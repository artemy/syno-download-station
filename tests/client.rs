mod utils;

use crate::utils::body_from_file;
use std::fs;
use syno_download_station::client::{SynoDS, SynoError};
use utils::form_param;
use wiremock::matchers::{header, header_regex, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

// Helper function to create a client with a mock server
async fn setup_client() -> (MockServer, SynoDS) {
    // Start a lightweight mock server.
    let server = MockServer::start().await;
    let url = server.uri();

    let synods = SynoDS::builder()
        .url(url)
        .username("test")
        .password("test123")
        .build()
        .unwrap();

    (server, synods)
}

// Helper function to create a mock for login
async fn create_login_mock(server: &mut MockServer) {
    // Create a mock on the server.
    Mock::given(method("POST"))
        .and(path("/webapi/entry.cgi"))
        .and(header("content-type", "application/x-www-form-urlencoded"))
        .and(form_param("api", "SYNO.API.Auth"))
        .and(form_param("version", "7"))
        .and(form_param("method", "login"))
        .and(form_param("account", "test"))
        .and(form_param("passwd", "test123"))
        .and(form_param("format", "sid"))
        .respond_with(
            ResponseTemplate::new(200)
                .append_header("content-type", "application/json")
                .set_body_string(body_from_file("test-files/login_success.json")),
        )
        .mount(server)
        .await;
}

// Helper function to create a mock for any API call
async fn create_api_mock(server: &mut MockServer, params: Vec<(&str, &str)>, response_file: &str) {
    // Create a mock on the server.
    let mut builder = Mock::given(method("POST"))
        .and(path("/webapi/entry.cgi"))
        .and(header("content-type", "application/x-www-form-urlencoded"));
    for (key, value) in params {
        builder = builder.and(form_param(key, value));
    }
    builder
        .and(form_param("_sid", "456"))
        .respond_with(
            ResponseTemplate::new(200)
                .append_header("content-type", "application/json")
                .set_body_string(body_from_file(response_file)),
        )
        .mount(server)
        .await;
}

// Helper function to create a mock for file upload API call
async fn create_file_upload_mock(server: &mut MockServer, response_file: &str) {
    Mock::given(method("POST"))
        .and(path("/webapi/entry.cgi"))
        .and(header_regex("content-type", "multipart/form-data"))
        .respond_with(
            ResponseTemplate::new(200)
                .append_header("content-type", "application/json")
                .set_body_string(body_from_file(response_file)),
        )
        .mount(server)
        .await;
}

#[tokio::test]
async fn test_login() {
    let (mut server, synods) = setup_client().await;

    create_login_mock(&mut server).await;

    synods.authorize().await.unwrap();
    assert!(synods.is_authorized().await);

    server.verify().await;
}

#[tokio::test]
async fn test_get_tasks() {
    let (mut server, synods) = setup_client().await;

    create_login_mock(&mut server).await;
    synods.authorize().await.unwrap();

    let params = vec![
        ("api", "SYNO.DownloadStation2.Task"),
        ("version", "2"),
        ("method", "list"),
        (
            "additional",
            r#"["transfer","tracker","peer","file","detail"]"#,
        ),
    ];

    create_api_mock(&mut server, params, "test-files/get_tasks_success.json").await;

    let tasks = synods.get_tasks().await.unwrap();

    server.verify().await;

    // Verify the response data
    assert_eq!(tasks.total, 2);
    assert_eq!(tasks.task.len(), 2);
    assert_eq!(tasks.task[0].id, "task_id_1");
    assert_eq!(tasks.task[0].title, "Test Torrent 1");
    assert_eq!(tasks.task[1].id, "task_id_2");
    assert_eq!(tasks.task[1].title, "Test Torrent 2");
}

#[tokio::test]
async fn test_get_task() {
    let (mut server, synods) = setup_client().await;

    // First, we need to log in
    create_login_mock(&mut server).await;
    synods.authorize().await.unwrap();

    let task_id = "task_id_1";
    let params = vec![
        ("api", "SYNO.DownloadStation2.Task"),
        ("version", "2"),
        ("method", "get"),
        ("id", task_id),
        (
            "additional",
            r#"["transfer","tracker","peer","file","detail"]"#,
        ),
    ];

    create_api_mock(&mut server, params, "test-files/get_task_success.json").await;

    let task_info = synods.get_task(vec![task_id.to_string()]).await.unwrap();

    server.verify().await;

    // Verify the response data
    assert_eq!(task_info.task.len(), 1);
    assert_eq!(task_info.task[0].id, "task_id_1");
    assert_eq!(task_info.task[0].title, "Test Torrent 1");

    // Verify additional details
    let additional = task_info.task[0].additional.as_ref().unwrap();

    // Check file info
    if let Some(files) = &additional.file {
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].filename, "test_file_1.mp4");
        assert_eq!(files[0].size, 1_073_741_824);
    } else {
        panic!("File information missing");
    }

    // Check peer info
    if let Some(peers) = &additional.peer {
        assert_eq!(peers.len(), 1);
        assert_eq!(peers[0].address, "192.168.1.100:12345");
        assert_eq!(peers[0].agent, "uTorrent/3.5.5");
    } else {
        panic!("Peer information missing");
    }

    // Check tracker info
    if let Some(trackers) = &additional.tracker {
        assert_eq!(trackers.len(), 1);
        assert_eq!(trackers[0].url, "udp://tracker.example.com:80/announce");
    } else {
        panic!("Tracker information missing");
    }
}

#[tokio::test]
async fn test_create_task() {
    let (mut server, synods) = setup_client().await;

    create_login_mock(&mut server).await;
    synods.authorize().await.unwrap();

    let uri = "https://example.com/test.iso";
    let destination = "/downloads";

    let params = vec![
        ("api", "SYNO.DownloadStation2.Task"),
        ("version", "2"),
        ("method", "create"),
        ("type", "\"url\""),
        ("destination", destination),
        ("url", uri),
        ("create_list", "false"),
    ];

    create_api_mock(&mut server, params, "test-files/create_task_success.json").await;

    let result = synods.create_task(uri, destination).await;

    server.verify().await;

    // Verify the operation was successful
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_create_task_from_file() {
    let (mut server, synods) = setup_client().await;

    create_login_mock(&mut server).await;
    synods.authorize().await.unwrap();

    let file_path = "test-files/test.torrent";
    let file_name = "test.torrent";
    let destination = "/downloads";

    // Read the test file
    let file_data = fs::read(file_path).expect("Failed to read test file");

    // Create a mock for the file upload
    create_file_upload_mock(&mut server, "test-files/create_task_from_file_success.json").await;

    let result = synods
        .create_task_from_file(&file_data, file_name, destination)
        .await;

    server.verify().await;

    // Verify the operation was successful
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_pause() {
    let (mut server, synods) = setup_client().await;

    create_login_mock(&mut server).await;
    synods.authorize().await.unwrap();

    let task_id = "task_id_1";

    let params = vec![
        ("api", "SYNO.DownloadStation2.Task"),
        ("version", "2"),
        ("method", "pause"),
        ("id", task_id),
    ];

    create_api_mock(&mut server, params, "test-files/pause_success.json").await;

    let result = synods.pause(task_id).await;

    server.verify().await;

    // Verify the operation was successful
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_resume() {
    let (mut server, synods) = setup_client().await;

    create_login_mock(&mut server).await;
    synods.authorize().await.unwrap();

    let task_id = "task_id_1";

    let params = vec![
        ("api", "SYNO.DownloadStation2.Task"),
        ("version", "2"),
        ("method", "resume"),
        ("id", task_id),
    ];

    create_api_mock(&mut server, params, "test-files/resume_success.json").await;

    let result = synods.resume(task_id).await;

    server.verify().await;

    // Verify the operation was successful
    assert!(result.is_ok());

    // Verify the response data
    assert_eq!(result.unwrap().failed_task.len(), 0);
}

#[tokio::test]
async fn test_complete() {
    let (mut server, synods) = setup_client().await;

    create_login_mock(&mut server).await;
    synods.authorize().await.unwrap();

    let task_id = "task_id_1";

    let params = vec![
        ("api", "SYNO.DownloadStation2.Task.Complete"),
        ("version", "1"),
        ("method", "start"),
        ("id", task_id),
    ];

    create_api_mock(&mut server, params, "test-files/complete_success.json").await;

    let result = synods.complete(task_id).await;

    server.verify().await;

    // Verify the operation was successful
    assert!(result.is_ok());

    // Verify the response data
    let task_completed = result.unwrap();
    assert_eq!(task_completed.task_id, "task_id_1");
}

#[tokio::test]
async fn test_delete_task() {
    let (mut server, synods) = setup_client().await;

    create_login_mock(&mut server).await;
    synods.authorize().await.unwrap();

    let task_id = "task_id_1";
    let force_complete = true;

    let params = vec![
        ("api", "SYNO.DownloadStation2.Task"),
        ("version", "2"),
        ("method", "delete"),
        ("id", task_id),
        ("force_complete", "true"),
    ];

    create_api_mock(&mut server, params, "test-files/delete_task_success.json").await;

    let result = synods.delete_task(task_id, force_complete).await;

    server.verify().await;

    // Verify the operation was successful
    assert!(result.is_ok());

    // Verify the response data
    let task_operation = result.unwrap();
    assert_eq!(task_operation.failed_task.len(), 0);
}

#[tokio::test]
async fn test_clear_completed() {
    let (mut server, synods) = setup_client().await;

    create_login_mock(&mut server).await;
    synods.authorize().await.unwrap();

    // The finished status is 5 (TaskStatus::Finished)
    let params = vec![
        ("api", "SYNO.DownloadStation2.Task"),
        ("version", "2"),
        ("method", "delete_condition"),
        ("status", "5"),
    ];

    create_api_mock(
        &mut server,
        params,
        "test-files/clear_completed_success.json",
    )
    .await;

    let result = synods.clear_completed().await;

    server.verify().await;

    // Verify the operation was successful
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_session_expired_auto_retry() {
    let (mut server, synods) = setup_client().await;

    // Initial login
    create_login_mock(&mut server).await;
    synods.authorize().await.unwrap();

    server.reset().await;

    // Re-mount login mock for re-auth
    create_login_mock(&mut server).await;

    // Mount expired response (only matches once)
    Mock::given(method("POST"))
        .and(path("/webapi/entry.cgi"))
        .and(header("content-type", "application/x-www-form-urlencoded"))
        .and(form_param("api", "SYNO.DownloadStation2.Task"))
        .and(form_param("method", "list"))
        .and(form_param("_sid", "456"))
        .respond_with(
            ResponseTemplate::new(200)
                .append_header("content-type", "application/json")
                .set_body_string(body_from_file("test-files/session_expired.json")),
        )
        .up_to_n_times(1)
        .mount(&server)
        .await;

    // Mount success response for retry (matches after expired mock is exhausted)
    Mock::given(method("POST"))
        .and(path("/webapi/entry.cgi"))
        .and(header("content-type", "application/x-www-form-urlencoded"))
        .and(form_param("api", "SYNO.DownloadStation2.Task"))
        .and(form_param("method", "list"))
        .and(form_param("_sid", "456"))
        .respond_with(
            ResponseTemplate::new(200)
                .append_header("content-type", "application/json")
                .set_body_string(body_from_file("test-files/get_tasks_success.json")),
        )
        .mount(&server)
        .await;

    // Re-authorize after server reset
    synods.authorize().await.unwrap();

    let tasks = synods.get_tasks().await.unwrap();
    assert_eq!(tasks.total, 2);
}

#[tokio::test]
async fn test_session_expired_reauth_fails() {
    let (mut server, synods) = setup_client().await;

    // Initial login
    create_login_mock(&mut server).await;
    synods.authorize().await.unwrap();

    server.reset().await;

    // First call returns session expired
    Mock::given(method("POST"))
        .and(path("/webapi/entry.cgi"))
        .and(header("content-type", "application/x-www-form-urlencoded"))
        .and(form_param("api", "SYNO.DownloadStation2.Task"))
        .and(form_param("method", "list"))
        .and(form_param("_sid", "456"))
        .respond_with(
            ResponseTemplate::new(200)
                .append_header("content-type", "application/json")
                .set_body_string(body_from_file("test-files/session_expired.json")),
        )
        .up_to_n_times(1)
        .mount(&server)
        .await;

    // Re-auth fails (returns failure)
    Mock::given(method("POST"))
        .and(path("/webapi/entry.cgi"))
        .and(header("content-type", "application/x-www-form-urlencoded"))
        .and(form_param("api", "SYNO.API.Auth"))
        .and(form_param("method", "login"))
        .respond_with(
            ResponseTemplate::new(200)
                .append_header("content-type", "application/json")
                .set_body_string(r#"{"success": false}"#),
        )
        .mount(&server)
        .await;

    let result = synods.get_tasks().await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_api_error_preserves_code() {
    let (mut server, synods) = setup_client().await;

    create_login_mock(&mut server).await;
    synods.authorize().await.unwrap();

    let params = vec![
        ("api", "SYNO.DownloadStation2.Task"),
        ("version", "2"),
        ("method", "list"),
        (
            "additional",
            r#"["transfer","tracker","peer","file","detail"]"#,
        ),
    ];

    create_api_mock(&mut server, params, "test-files/api_error.json").await;

    let result = synods.get_tasks().await;
    assert!(result.is_err());

    let err = result.unwrap_err();
    let syno_err = err.downcast_ref::<SynoError>().expect("expected SynoError");
    match syno_err {
        SynoError::Api { code, .. } => assert_eq!(*code, 403),
        other => panic!("Expected SynoError::Api, got: {other:?}"),
    }
}

#[tokio::test]
async fn test_session_expired_file_upload_retry() {
    let (mut server, synods) = setup_client().await;

    // Initial login
    create_login_mock(&mut server).await;
    synods.authorize().await.unwrap();

    server.reset().await;

    // Re-mount login mock for re-auth
    create_login_mock(&mut server).await;

    // First file upload returns session expired
    Mock::given(method("POST"))
        .and(path("/webapi/entry.cgi"))
        .and(header_regex("content-type", "multipart/form-data"))
        .respond_with(
            ResponseTemplate::new(200)
                .append_header("content-type", "application/json")
                .set_body_string(body_from_file("test-files/session_expired.json")),
        )
        .up_to_n_times(1)
        .mount(&server)
        .await;

    // Retry succeeds
    Mock::given(method("POST"))
        .and(path("/webapi/entry.cgi"))
        .and(header_regex("content-type", "multipart/form-data"))
        .respond_with(
            ResponseTemplate::new(200)
                .append_header("content-type", "application/json")
                .set_body_string(body_from_file(
                    "test-files/create_task_from_file_success.json",
                )),
        )
        .mount(&server)
        .await;

    let file_data = fs::read("test-files/test.torrent").expect("Failed to read test file");
    let result = synods
        .create_task_from_file(&file_data, "test.torrent", "/downloads")
        .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_auth_error_preserves_code() {
    let (server, synods) = setup_client().await;

    Mock::given(method("POST"))
        .and(path("/webapi/entry.cgi"))
        .and(header("content-type", "application/x-www-form-urlencoded"))
        .and(form_param("api", "SYNO.API.Auth"))
        .and(form_param("method", "login"))
        .respond_with(
            ResponseTemplate::new(200)
                .append_header("content-type", "application/json")
                .set_body_string(body_from_file("test-files/auth_error.json")),
        )
        .mount(&server)
        .await;

    let result = synods.authorize().await;
    assert!(result.is_err());

    let err = result.unwrap_err();
    let syno_err = err.downcast_ref::<SynoError>().expect("expected SynoError");
    match syno_err {
        SynoError::Auth {
            code: Some(400), ..
        } => {}
        other => panic!("Expected SynoError::Auth with code 400, got: {other:?}"),
    }
}

#[tokio::test]
async fn test_create_task_error_preserves_code() {
    let (mut server, synods) = setup_client().await;

    create_login_mock(&mut server).await;
    synods.authorize().await.unwrap();

    let params = vec![
        ("api", "SYNO.DownloadStation2.Task"),
        ("version", "2"),
        ("method", "create"),
        ("type", "\"url\""),
        ("destination", "/downloads"),
        ("url", "https://example.com/test.iso"),
        ("create_list", "false"),
    ];

    create_api_mock(&mut server, params, "test-files/api_error.json").await;

    let result = synods
        .create_task("https://example.com/test.iso", "/downloads")
        .await;
    assert!(result.is_err());

    let err = result.unwrap_err();
    let syno_err = err.downcast_ref::<SynoError>().expect("expected SynoError");
    match syno_err {
        SynoError::Api { code: 403, .. } => {}
        other => panic!("Expected SynoError::Api with code 403, got: {other:?}"),
    }
}

#[tokio::test]
async fn test_resume_error_preserves_code() {
    let (mut server, synods) = setup_client().await;

    create_login_mock(&mut server).await;
    synods.authorize().await.unwrap();

    let params = vec![
        ("api", "SYNO.DownloadStation2.Task"),
        ("version", "2"),
        ("method", "resume"),
        ("id", "task_id_1"),
    ];

    create_api_mock(&mut server, params, "test-files/api_error.json").await;

    let result = synods.resume("task_id_1").await;
    assert!(result.is_err());

    let err = result.unwrap_err();
    let syno_err = err.downcast_ref::<SynoError>().expect("expected SynoError");
    match syno_err {
        SynoError::Api { code: 403, .. } => {}
        other => panic!("Expected SynoError::Api with code 403, got: {other:?}"),
    }
}

#[tokio::test]
async fn test_delete_task_error_preserves_code() {
    let (mut server, synods) = setup_client().await;

    create_login_mock(&mut server).await;
    synods.authorize().await.unwrap();

    let params = vec![
        ("api", "SYNO.DownloadStation2.Task"),
        ("version", "2"),
        ("method", "delete"),
        ("id", "task_id_1"),
    ];

    create_api_mock(&mut server, params, "test-files/api_error.json").await;

    let result = synods.delete_task("task_id_1", false).await;
    assert!(result.is_err());

    let err = result.unwrap_err();
    let syno_err = err.downcast_ref::<SynoError>().expect("expected SynoError");
    match syno_err {
        SynoError::Api { code: 403, .. } => {}
        other => panic!("Expected SynoError::Api with code 403, got: {other:?}"),
    }
}

#[tokio::test]
async fn test_clear_completed_error_preserves_code() {
    let (mut server, synods) = setup_client().await;

    create_login_mock(&mut server).await;
    synods.authorize().await.unwrap();

    let params = vec![
        ("api", "SYNO.DownloadStation2.Task"),
        ("version", "2"),
        ("method", "delete_condition"),
        ("status", "5"),
    ];

    create_api_mock(&mut server, params, "test-files/api_error.json").await;

    let result = synods.clear_completed().await;
    assert!(result.is_err());

    let err = result.unwrap_err();
    let syno_err = err.downcast_ref::<SynoError>().expect("expected SynoError");
    match syno_err {
        SynoError::Api { code: 403, .. } => {}
        other => panic!("Expected SynoError::Api with code 403, got: {other:?}"),
    }
}
