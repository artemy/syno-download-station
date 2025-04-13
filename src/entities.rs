use chrono::serde::ts_seconds;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_repr::{Deserialize_repr, Serialize_repr};

/// Response from Synology API
#[derive(Deserialize, Debug)]
pub struct SynologyResponse<D> {
    pub success: bool,
    pub data: Option<D>,
    pub error: Option<SynoError>,
}

/// Authentication response data
#[allow(unused)]
#[derive(Deserialize, Debug)]
pub struct AuthData {
    #[serde(default)]
    pub account: String,
    #[serde(default)]
    pub device_id: String,
    #[serde(default)]
    pub ik_message: String,
    #[serde(default)]
    pub is_portal_port: bool,
    /// Session ID used for authenticated requests
    pub sid: String,
    #[serde(default)]
    pub synotoken: String,
}

/// Collection of download tasks
#[derive(Deserialize, Debug)]
pub struct Tasks {
    pub offset: i8,
    pub task: Vec<Task>,
    pub total: i32,
}

/// Detailed information about specific tasks
#[derive(Deserialize, Debug)]
pub struct TaskInfo {
    pub task: Vec<Task>,
}

/// Individual download task information
#[derive(Deserialize, Debug)]
pub struct Task {
    /// Unique identifier for the task
    pub id: String,
    pub username: String,
    /// Type of download task (e.g., "bt" for `BitTorrent`)
    #[serde(rename = "type")]
    pub task_type: String,
    /// Task title/name
    pub title: String,
    /// Total size in bytes
    pub size: u64,
    /// Current status of the task
    pub status: TaskStatus,
    /// Extra task details
    pub status_extra: Option<StatusExtra>,
    /// Additional detailed information about the task
    pub additional: Option<AdditionalTaskInfo>,
}

/// Extra task details
#[derive(Deserialize, Debug)]
pub struct StatusExtra {
    pub error_detail: Option<String>,
    pub unzip_progress: Option<i32>,
}

/// Additional detailed information about a task
#[derive(Deserialize, Default, Debug)]
pub struct AdditionalTaskInfo {
    pub detail: Option<Detail>,
    pub file: Option<Vec<File>>,
    pub peer: Option<Vec<Peer>>,
    pub tracker: Option<Vec<Tracker>>,
    pub transfer: Option<Transfer>,
}

/// Detailed task information
#[derive(Deserialize, Debug)]
pub struct Detail {
    #[serde(with = "ts_seconds")]
    pub completed_time: DateTime<Utc>,
    pub connected_leechers: u32,
    pub connected_peers: u32,
    pub connected_seeders: u32,
    #[serde(with = "ts_seconds")]
    pub created_time: DateTime<Utc>,
    pub destination: String,
    pub seed_elapsed: u64,
    #[serde(with = "ts_seconds")]
    pub started_time: DateTime<Utc>,
    pub total_peers: u32,
    pub total_pieces: u32,
    pub uri: String,
    pub unzip_password: Option<String>,
    pub waiting_seconds: u32,
}

/// Information about a file within a download task
#[derive(Deserialize, Debug)]
pub struct File {
    pub filename: String,
    pub index: u32,
    pub priority: String,
    pub size: u64,
    pub size_downloaded: u64,
    pub wanted: bool,
}

/// Information about a connected peer
#[derive(Deserialize, Debug)]
pub struct Peer {
    pub address: String,
    pub agent: String,
    pub progress: f32,
    pub speed_download: u64,
    pub speed_upload: u64,
}

/// Information about a tracker
#[derive(Deserialize, Debug)]
pub struct Tracker {
    pub peers: i32,
    pub seeds: i32,
    pub status: String,
    pub update_timer: u32,
    pub url: String,
}

/// Transfer statistics
#[derive(Deserialize, Default, Debug)]
pub struct Transfer {
    pub downloaded_pieces: u32,
    pub size_downloaded: u64,
    pub size_uploaded: u64,
    pub speed_download: u64,
    pub speed_upload: u64,
}

/// Download task status enum
#[derive(Serialize_repr, Deserialize_repr, Debug)]
#[repr(u8)]
pub enum TaskStatus {
    Waiting = 1,
    Downloading = 2,
    Paused = 3,
    Finishing = 4,
    Finished = 5,
    HashChecking = 6,
    PreSeeding = 7,
    Seeding = 8,
    FilehostingWaiting = 9,
    Extracting = 10,
    Preprocessing = 11,
    PreprocessPass = 12,
    Downloaded = 13,
    Postprocessing = 14,
    CaptchaNeeded = 15,
    Error = 101,
    ErrorBrokenLink = 102,
    ErrorDestNoExist = 103,
    ErrorDestDeny = 104,
    ErrorDiskFull = 105,
    ErrorQuotaReached = 106,
    ErrorTimeout = 107,
    ErrorExceedMaxFsSize = 108,
    ErrorExceedMaxTempFsSize = 109,
    ErrorExceedMaxDestFsSize = 110,
    ErrorNameTooLongEncryption = 111,
    ErrorNameTooLong = 112,
    ErrorTorrentDuplicate = 113,
    ErrorFileNoExist = 114,
    ErrorRequiredPremium = 115,
    ErrorNotSupportType = 116,
    ErrorFtpEncryptionNotSupportType = 117,
    ErrorExtractFail = 118,
    ErrorExtractWrongPassword = 119,
    ErrorExtractInvalidArchive = 120,
    ErrorExtractQuotaReached = 121,
    ErrorExtractDiskFull = 122,
    ErrorTorrentInvalid = 123,
    ErrorRequiredAccount = 124,
    ErrorTryItLater = 125,
    ErrorEncryption = 126,
    ErrorMissingPython = 127,
    ErrorPrivateVideo = 128,
    ErrorExtractFolderNotExist = 129,
    ErrorNzbMissingArticle = 130,
    ErrorEd2KLinkDuplicate = 131,
    ErrorDestFileDuplicate = 132,
    ErrorParchiveRepairFailed = 133,
    ErrorInvalidAccountPassword = 134,
}

/// Error information from Synology API
#[derive(Deserialize, Debug)]
pub struct SynoError {
    pub code: i32,
    pub errors: Option<TaskOperation>,
}

#[derive(Deserialize, Debug)]
pub struct TaskCompleted {
    pub task_id: String,
}

#[derive(Deserialize, Debug)]
pub struct TaskCreated {
    pub list_id: Vec<String>,
    pub task_id: Vec<String>,
}

#[derive(Deserialize, Debug)]
pub struct TaskOperation {
    pub failed_task: Vec<FailedTask>,
}

#[derive(Deserialize, Debug)]
pub struct FailedTask {
    pub error: i32,
    pub id: String,
}
