use crate::client::SynoError::*;
use crate::entities::TaskStatus::Finished;
use crate::entities::{
    AuthData, SynologyResponse, TaskCompleted, TaskCreated, TaskInfo, TaskOperation, Tasks,
};
use anyhow::{Context, Result};
use log::debug;
use reqwest::multipart::Part;
use reqwest::{multipart, Client};
use std::env;
use std::time::Duration;
use thiserror::Error;

const API_PATH: &str = "/webapi/entry.cgi";

/// Custom error types for the [`SynoDS`] client
#[derive(Error, Debug)]
pub enum SynoError {
    #[error("Authentication error: {0}")]
    Auth(String),

    #[error("Synology API error: code={code}, message={message}")]
    Api { code: i32, message: String },

    #[error("Network request error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("URL parsing error: {0}")]
    UrlParse(String),

    #[error("Environment variable error: {0}")]
    Environment(#[from] env::VarError),

    #[error("JSON serialization/deserialization error: {0}")]
    InvalidResponse(String),

    #[error("Invalid input parameter: {0}")]
    InvalidInput(String),

    #[error("Task creation failed: {0}")]
    TaskCreation(String),

    #[error("Task modification failed: {0}")]
    TaskModification(String),

    #[error("Configuration error: {0}")]
    Configuration(String),
}

/// Synology Download Station client
pub struct SynoDS {
    url: String,
    username: String,
    password: String,
    client: Client,
    sid: String,
}

const DEFAULT_PARAMS: &[(&str, &str)] =
    &[("api", "SYNO.DownloadStation2.Task"), ("version", "2")];

impl SynoDS {
    /// Creates a new `SynoDS` client with the given url, credentials and timeout
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Username, password, or host URL is empty
    /// - URL doesn't start with "http://" or "https://"
    #[allow(clippy::needless_pass_by_value)]
    pub fn new(url: String, username: String, password: String, timeout_ms: u64) -> Result<Self> {
        // Validate all required configuration parameters
        if username.is_empty() {
            return Err(Configuration("Username cannot be empty".into()).into());
        }

        if password.is_empty() {
            return Err(Configuration("Password cannot be empty".into()).into());
        }

        if url.is_empty() {
            return Err(Configuration("Host URL cannot be empty".into()).into());
        }

        // Validate host URL format
        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Err(Configuration(format!(
                "Host URL must start with http:// or https://, got: {url}"
            ))
            .into());
        }

        // Remove trailing slash from host URL if present
        let url = url.trim_end_matches('/').to_string();

        let client = Self::create_client(timeout_ms);

        Ok(Self {
            url,
            username,
            password,
            client,
            sid: String::new(),
        })
    }

    /// Creates a configured HTTP client
    fn create_client(timeout: u64) -> Client {
        Client::builder()
            .timeout(Duration::from_millis(timeout))
            .build()
            .unwrap_or_default()
    }

    /// Creates a new `SynoDS` client with a builder pattern
    #[must_use]
    pub fn builder() -> SynoDSBuilder {
        SynoDSBuilder::default()
    }

    /// Authorizes the client by getting a session ID
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Network request fails
    /// - Authentication fails
    /// - Response cannot be parsed
    pub async fn authorize(&mut self) -> Result<()> {
        let params = [
            ("api", "SYNO.API.Auth"),
            ("version", "7"),
            ("method", "login"),
            ("account", &self.username),
            ("passwd", &self.password),
            ("format", "sid"),
        ];

        let response = self
            .make_api_request::<SynologyResponse<AuthData>>(&params)
            .await
            .context("Failed to authorize")?;

        if response.success {
            match response.data {
                Some(data) => {
                    self.sid = data.sid;
                    Ok(())
                }
                None => Err(InvalidResponse("No data received".into()).into()),
            }
        } else {
            Err(InvalidResponse("Failed to authenticate".into()).into())
        }
    }

    /// Gets all Download Station tasks
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Network request fails
    /// - API returns an error response
    /// - Response cannot be parsed
    /// - Session is invalid or expired
    pub async fn get_tasks(&self) -> Result<Tasks> {
        let all_params = {
            let mut params = DEFAULT_PARAMS.to_vec();
            params.push(("method", "list"));
            params.push(("additional", r#"["transfer","detail"]"#));
            params
        };

        let response = self
            .make_api_request::<SynologyResponse<Tasks>>(&all_params)
            .await
            .context("Failed to get tasks")?;

        if response.success {
            match response.data {
                Some(tasks) => Ok(tasks),
                None => Err(InvalidResponse("No data received".into()).into()),
            }
        } else {
            Err(InvalidResponse("Failed to get tasks".into()).into())
        }
    }

    /// Gets detailed information about specific task(s)
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - IDs vector is empty
    /// - Network request fails
    /// - API returns an error response
    /// - Response cannot be parsed
    /// - Session is invalid or expired
    pub async fn get_task(&self, ids: Vec<String>) -> Result<TaskInfo> {
        if ids.is_empty() {
            return Err(InvalidInput("Task IDs cannot be empty".into()).into());
        }

        let id_string = ids.join(",");
        let all_params = {
            let mut params = DEFAULT_PARAMS.to_vec();
            params.push(("method", "get"));
            params.push(("id", &id_string));
            params.push(("additional", r#"["transfer","detail"]"#));
            params
        };

        let response = self
            .make_api_request::<SynologyResponse<TaskInfo>>(&all_params)
            .await
            .context("Failed to get task details")?;

        if response.success {
            match response.data {
                Some(task_info) => Ok(task_info),
                None => Err(InvalidResponse("No data received".into()).into()),
            }
        } else if let Some(error) = response.error {
            Err(Api {
                code: error.code,
                message: "Failed to get task".into(),
            }
            .into())
        } else {
            Err(InvalidResponse("Failed to get task, unknown error".into()).into())
        }
    }

    /// Creates a new download task from a URI (HTTP/HTTPS URL or magnet link)
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - URI or destination is empty
    /// - URI doesn't start with http://, https://, or magnet:
    /// - Session ID is not available (must call [`Self::authorize()`] first)
    /// - Network request fails
    /// - API returns an error response
    /// - Response cannot be parsed
    pub async fn create_task(&self, uri: &str, destination: &str) -> Result<()> {
        // Validate input parameters
        if uri.is_empty() {
            return Err(InvalidInput("URI cannot be empty".into()).into());
        }

        if destination.is_empty() {
            return Err(InvalidInput("Destination path cannot be empty".into()).into());
        }

        // Basic URI validation
        if !uri.starts_with("http://")
            && !uri.starts_with("https://")
            && !uri.starts_with("magnet:")
        {
            return Err(InvalidInput(format!(
                "URI must start with http://, https://, or magnet:, got: {uri}"
            ))
            .into());
        }

        // Check if we have a session ID
        if self.sid.is_empty() {
            return Err(Auth(
                "No session ID available. Make sure to call authorize() first".into(),
            )
            .into());
        }

        debug!("Creating download task. URI: {uri}, Destination: {destination}");

        // Parameters for the create task API call
        let all_params = {
            let mut params = DEFAULT_PARAMS.to_vec();
            params.push(("method", "create"));
            params.push(("type", "\"url\""));
            params.push(("destination", destination));
            params.push(("url", uri));
            params.push(("create_list", "false"));
            params
        };

        // Use the make_api_request method to create the task via POST request
        let response = self
            .make_api_request::<SynologyResponse<TaskCreated>>(&all_params)
            .await
            .context("Failed to create download task")?;

        if response.success {
            debug!("Successfully created download task for URI: {uri}");
            Ok(())
        } else {
            Err(InvalidResponse("Failed to create task".into()).into())
        }
    }

    /// Creates a new download task from a torrent file
    /// Uses multipart/form-data with POST for file uploads
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - File data is empty
    /// - File name or destination is empty
    /// - Session ID is not available (must call [`Self::authorize()`] first)
    /// - Network request fails
    /// - API returns an error response
    /// - Response cannot be parsed
    /// - Torrent file is invalid or corrupted
    pub async fn create_task_from_file(
        &self,
        file_data: &[u8],
        file_name: &str,
        destination: &str,
    ) -> Result<()> {
        // Validate input parameters
        if file_data.is_empty() {
            return Err(InvalidInput("File data cannot be empty".into()).into());
        }

        if file_name.is_empty() {
            return Err(InvalidInput("File name cannot be empty".into()).into());
        }

        if destination.is_empty() {
            return Err(InvalidInput("Destination path cannot be empty".into()).into());
        }

        // Check if we have a session ID
        if self.sid.is_empty() {
            return Err(Auth(
                "No session ID available. Make sure to call authorize() first".into(),
            )
            .into());
        }

        // Basic file validation
        if !file_name.ends_with(".torrent") {
            debug!("Warning: File name does not end with .torrent extension: {file_name}");
        }

        debug!(
            "Creating download task from file. Name: {}, Size: {} bytes, Destination: {}",
            file_name,
            file_data.len(),
            destination
        );

        // For file uploads, we must still use multipart/form-data POST request
        // There's no way to upload files via GET request efficiently

        // Create the part for the torrent file
        let file_part = Part::bytes(file_data.to_vec())
            .file_name(file_name.to_string())
            .mime_str("application/x-bittorrent")
            .context("Failed to create file part")?;

        // Create the multipart form
        let form = multipart::Form::new()
            .text("api", "SYNO.DownloadStation2.Task")
            .text("version", "2")
            .text("method", "create")
            .text("type", "\"file\"")
            .text("file", "[\"torrent\"]")
            .text("destination", format!("\"{destination}\""))
            .text("create_list", "false")
            .part("torrent", file_part);

        // Create the URL for the API call with session ID
        let url = format!("{}{}?_sid={}", self.url, API_PATH, self.sid);

        // Make the POST request with the multipart form
        let client = &self.client;
        let response = client
            .post(&url)
            .multipart(form)
            .send()
            .await
            .context("Failed to send file upload request")?
            .json::<SynologyResponse<TaskCreated>>()
            .await?;

        // Handle the response
        if response.success {
            debug!("Successfully created download task for file: {file_name}");
            Ok(())
        } else if let Some(error) = response.error {
            Err(Api {
                code: error.code,
                message: "Failed to create task".into(),
            }
            .into())
        } else {
            Err(InvalidResponse("Failed to create task, unknown error".into()).into())
        }
    }

    /// Pause a specific task
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Network request fails
    /// - API returns an error response
    /// - Task ID is invalid
    /// - Task cannot be paused (e.g., already paused or in a state that cannot be paused)
    /// - Session is invalid or expired
    pub async fn pause(&self, id: &str) -> Result<()> {
        let all_params = {
            let mut params = DEFAULT_PARAMS.to_vec();
            params.push(("method", "pause"));
            params.push(("id", id));
            params
        };

        let response = self
            .make_api_request::<SynologyResponse<()>>(&all_params)
            .await
            .context("Failed to pause download task")?;

        if response.success {
            Ok(())
        } else if let Some(error) = response.error {
            Err(Api {
                code: error.code,
                message: "Failed to pause task".into(),
            }
            .into())
        } else {
            Err(InvalidResponse("Failed to pause task, unknown error".into()).into())
        }
    }

    /// Resume a specific task
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Network request fails
    /// - API returns an error response
    /// - Task ID is invalid
    /// - Task cannot be resumed (e.g., not paused or in a state that cannot be resumed)
    /// - Session is invalid or expired
    /// - Response data is missing or invalid
    pub async fn resume(&self, id: &str) -> Result<TaskOperation> {
        let all_params = {
            let mut params = DEFAULT_PARAMS.to_vec();
            params.push(("method", "resume"));
            params.push(("id", id));
            params
        };

        let response = self
            .make_api_request::<SynologyResponse<TaskOperation>>(&all_params)
            .await
            .context("Failed to resume download task")?;

        if response.success {
            match response.data {
                Some(task_operation) => Ok(task_operation),
                None => Err(InvalidResponse("No data received".into()).into()),
            }
        } else {
            Err(TaskModification(format!("Failed to resume download task id: {}", &id)).into())
        }
    }

    /// Complete a specific task
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Network request fails
    /// - API returns an error response
    /// - Task ID is invalid
    /// - Task cannot be completed (e.g., in a state that cannot be completed)
    /// - Session is invalid or expired
    /// - Response data is missing or invalid
    pub async fn complete(&self, id: &str) -> Result<TaskCompleted> {
        let params = [
            ("api", "SYNO.DownloadStation2.Task.Complete"),
            ("version", "1"),
            ("method", "start"),
            ("id", id),
        ];

        let response = self
            .make_api_request::<SynologyResponse<TaskCompleted>>(&params)
            .await
            .context("Failed to complete download task")?;

        if response.success {
            match response.data {
                Some(task_completed) => Ok(task_completed),
                None => Err(InvalidResponse("No data received".into()).into()),
            }
        } else {
            Err(TaskModification(format!("Failed to complete download task id: {}", &id)).into())
        }
    }

    /// Delete a specific task
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Network request fails
    /// - API returns an error response
    /// - Task ID is invalid
    /// - Task cannot be deleted (e.g., in a state that prevents deletion)
    /// - Session is invalid or expired
    /// - Response data is missing or invalid
    pub async fn delete_task(&self, id: &str, force_complete: bool) -> Result<TaskOperation> {
        let all_params = {
            let mut params = DEFAULT_PARAMS.to_vec();
            params.push(("method", "delete"));
            params.push(("id", id));
            if force_complete {
                params.push(("force_complete", "true"));
            }
            params
        };

        let response = self
            .make_api_request::<SynologyResponse<TaskOperation>>(&all_params)
            .await
            .context("Failed to delete download task")?;

        if response.success {
            match response.data {
                Some(task_operation) => Ok(task_operation),
                None => Err(InvalidResponse("No data received".into()).into()),
            }
        } else {
            Err(TaskModification(format!("Failed to delete download task id: {}", &id)).into())
        }
    }

    /// Clear completed tasks
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Network request fails
    /// - API returns an error response
    /// - No completed tasks exist
    /// - Session is invalid or expired
    pub async fn clear_completed(&self) -> Result<()> {
        let finished_index = (Finished as u8).to_string();
        let all_params = {
            let mut params = DEFAULT_PARAMS.to_vec();
            params.push(("method", "delete_condition"));
            params.push(("status", &finished_index));
            params
        };

        let response = self
            .make_api_request::<SynologyResponse<()>>(&all_params)
            .await
            .context("Failed to clear completed tasks")?;

        if response.success {
            Ok(())
        } else {
            Err(TaskModification("Failed to clear completed tasks".to_string()).into())
        }
    }

    /// Makes a POST API request with form parameters
    async fn make_api_request<R>(&self, params: &[(&str, &str)]) -> Result<R>
    where
        R: for<'de> serde::Deserialize<'de>,
    {
        // Create combined parameters with session ID if needed
        let mut all_params = params.to_vec();
        if !self.sid.is_empty() {
            all_params.push(("_sid", &self.sid));
        }

        // Build the base URL
        let base_url = format!("{}{}", self.url, API_PATH);
        debug!(
            "Making API request to: {} with {} parameters",
            base_url,
            all_params.len()
        );

        // Send the POST request with form data
        let client = &self.client;
        let response = client
            .post(&base_url)
            .form(&all_params)
            .send()
            .await
            .context("Failed to make API request")?;

        debug!("API request status: {}", response.status());

        // Process the response
        let status = response.status();
        if !status.is_success() {
            return Err(Api {
                code: i32::from(status.as_u16()),
                message: format!(
                    "HTTP request failed with status: {} ({})",
                    status.as_u16(),
                    status.canonical_reason().unwrap_or("Unknown")
                ),
            }
            .into());
        }

        response
            .json::<R>()
            .await
            .context("Failed to parse API response".to_string())
    }
}

/// Builder for [`SynoDS`] client
#[derive(Default)]
pub struct SynoDSBuilder {
    url: Option<String>,
    username: Option<String>,
    password: Option<String>,
    timeout: Option<u64>,
}

impl SynoDSBuilder {
    /// Sets the host URL
    #[must_use]
    pub fn url(mut self, url: impl Into<String>) -> Self {
        self.url = Some(url.into());
        self
    }

    /// Sets the username
    #[must_use]
    pub fn username(mut self, username: impl Into<String>) -> Self {
        self.username = Some(username.into());
        self
    }

    /// Sets the password
    #[must_use]
    pub fn password(mut self, password: impl Into<String>) -> Self {
        self.password = Some(password.into());
        self
    }

    /// Sets the request timeout in milliseconds
    #[must_use]
    pub fn timeout(mut self, timeout_millis: u64) -> Self {
        self.timeout = Some(timeout_millis);
        self
    }

    /// Builds the [`SynoDS`] client
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Required fields (url, username, password) are not provided
    /// - Host URL doesn't start with "http://" or "https://"
    /// - Any field contains invalid data
    pub fn build(self) -> Result<SynoDS> {
        let url = self
            .url
            .ok_or_else(|| Configuration("Host URL is required".into()))?;
        let username = self
            .username
            .ok_or_else(|| Configuration("Username is required".into()))?;
        let password = self
            .password
            .ok_or_else(|| Configuration("Password is required".into()))?;

        let timeout = self.timeout.unwrap_or(3000);

        let client = SynoDS::new(url, username, password, timeout)?;

        Ok(client)
    }
}
