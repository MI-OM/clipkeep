mod dependencies;
mod manager;
mod process;
mod tools;
mod types;
mod url;

pub use dependencies::{DependencyManager, DependencyProgress, DependencyStatus};
pub use manager::DownloadManager;
pub use tools::{MediaTools, ToolResolver};
pub use types::{AppError, DownloadJob, DownloadRequest, MediaMetadata};
pub use url::validate_media_url;

use std::sync::Arc;

pub struct AppState {
    pub downloads: Arc<DownloadManager>,
    pub dependencies: Arc<DependencyManager>,
    pub transcription_root: std::path::PathBuf,
}

impl AppState {
    pub fn new(
        dependency_root: std::path::PathBuf,
        transcription_root: std::path::PathBuf,
    ) -> Self {
        Self {
            downloads: Arc::new(DownloadManager::default()),
            dependencies: Arc::new(DependencyManager::new(dependency_root)),
            transcription_root,
        }
    }
}

pub fn get_metadata(
    executable: std::path::PathBuf,
    deno: std::path::PathBuf,
    url: String,
) -> Result<MediaMetadata, AppError> {
    process::get_metadata(&executable, &deno, &url)
}

pub fn start_download(
    manager: &DownloadManager,
    app: tauri::AppHandle,
    tools: MediaTools,
    request: DownloadRequest,
) -> Result<DownloadJob, AppError> {
    manager.start(app, tools, request)
}

pub fn cancel_download(manager: &DownloadManager, job_id: String) -> Result<(), AppError> {
    manager.cancel(&job_id)
}
