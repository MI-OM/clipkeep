use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppError {
    pub code: String,
    pub message: String,
    pub recoverable: bool,
}

impl AppError {
    pub fn new(code: &str, message: impl Into<String>, recoverable: bool) -> Self {
        Self {
            code: code.to_string(),
            message: message.into(),
            recoverable,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MediaFormat {
    Mp4,
    Mp3,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Quality {
    #[serde(rename = "best")]
    Best,
    #[serde(rename = "1080p")]
    P1080,
    #[serde(rename = "720p")]
    P720,
    #[serde(rename = "480p")]
    P480,
    #[serde(rename = "320kbps")]
    Kbps320,
    #[serde(rename = "192kbps")]
    Kbps192,
    #[serde(rename = "128kbps")]
    Kbps128,
}

impl Quality {
    pub fn mp4_selector(self) -> Result<String, AppError> {
        let height = match self {
            Self::Best => {
                return Ok("bestvideo[ext=mp4]+bestaudio[ext=m4a]/best[ext=mp4]".to_string());
            }
            Self::P1080 => 1080,
            Self::P720 => 720,
            Self::P480 => 480,
            Self::Kbps320 | Self::Kbps192 | Self::Kbps128 => {
                return Err(AppError::new(
                    "invalid_quality",
                    "Choose a video resolution for an MP4 download.",
                    false,
                ));
            }
        };
        Ok(format!(
            "bestvideo[ext=mp4][height<={height}]+bestaudio[ext=m4a]/best[ext=mp4][height<={height}]"
        ))
    }

    pub fn mp3_bitrate(self) -> Result<&'static str, AppError> {
        match self {
            Self::Best => Ok("0"),
            Self::Kbps320 => Ok("320K"),
            Self::Kbps192 => Ok("192K"),
            Self::Kbps128 => Ok("128K"),
            _ => Err(AppError::new(
                "invalid_quality",
                "Choose an audio bitrate for an MP3 download.",
                false,
            )),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadRequest {
    pub url: String,
    #[serde(default)]
    pub title: Option<String>,
    pub format: MediaFormat,
    pub quality: Quality,
    pub destination_directory: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MediaMetadata {
    pub title: String,
    pub duration_seconds: Option<u64>,
    pub thumbnail_url: Option<String>,
    pub available_video_qualities: Vec<Quality>,
    pub available_audio_qualities: Vec<Quality>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DownloadStatus {
    Queued,
    Validating,
    FetchingMetadata,
    Downloading,
    Merging,
    Converting,
    Finalizing,
    Completed,
    Failed,
    Cancelling,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadJob {
    pub id: String,
    pub title: Option<String>,
    pub format: MediaFormat,
    pub quality: Quality,
    pub status: DownloadStatus,
    pub progress_percent: Option<f64>,
    pub bytes_per_second: Option<f64>,
    pub eta_seconds: Option<u64>,
    pub output_path: Option<String>,
    pub error: Option<AppError>,
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressEvent {
    pub job_id: String,
    pub status: DownloadStatus,
    pub progress_percent: Option<f64>,
    pub bytes_per_second: Option<f64>,
    pub eta_seconds: Option<u64>,
    pub output_path: Option<String>,
    pub error: Option<AppError>,
}

impl ProgressEvent {
    pub fn new(job_id: &str, status: DownloadStatus) -> Self {
        Self {
            job_id: job_id.to_string(),
            status,
            progress_percent: None,
            bytes_per_second: None,
            eta_seconds: None,
            output_path: None,
            error: None,
        }
    }
}
