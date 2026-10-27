use tauri::{AppHandle, Emitter, State};

use crate::download::{
    self, AppError, AppState, DependencyProgress, DependencyStatus, DownloadJob, DownloadRequest,
    MediaMetadata, ToolResolver,
};

#[tauri::command]
pub async fn get_media_metadata(
    state: State<'_, AppState>,
    url: String,
) -> Result<MediaMetadata, AppError> {
    let normalized_url = download::validate_media_url(&url)?;
    let dependencies = state.dependencies.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let tools = dependencies.resolve()?;
        download::get_metadata(tools.yt_dlp, tools.deno, normalized_url)
    })
    .await
    .map_err(|_| {
        AppError::new(
            "metadata_task_failed",
            "Could not read video details.",
            true,
        )
    })?
}

#[tauri::command]
pub async fn start_download(
    app: AppHandle,
    state: State<'_, AppState>,
    request: DownloadRequest,
) -> Result<DownloadJob, AppError> {
    let normalized_url = download::validate_media_url(&request.url)?;
    let dependencies = state.dependencies.clone();
    let downloads = state.downloads.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let tools = dependencies.resolve()?;
        let mut request = request;
        request.url = normalized_url;
        download::start_download(&downloads, app, tools, request)
    })
    .await
    .map_err(|_| {
        AppError::new(
            "download_task_failed",
            "ClipKeep could not prepare the download.",
            true,
        )
    })?
}

#[tauri::command]
pub fn cancel_download(state: State<'_, AppState>, job_id: String) -> Result<(), AppError> {
    download::cancel_download(&state.downloads, job_id)
}

#[tauri::command]
pub async fn get_dependency_status(
    state: State<'_, AppState>,
) -> Result<DependencyStatus, AppError> {
    let dependencies = state.dependencies.clone();
    tauri::async_runtime::spawn_blocking(move || dependencies.status())
        .await
        .map_err(|_| {
            AppError::new(
                "dependency_status_failed",
                "ClipKeep could not check media-tool status.",
                true,
            )
        })?
}

#[tauri::command]
pub async fn install_media_tools(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<DependencyStatus, AppError> {
    let dependencies = state.dependencies.clone();
    tauri::async_runtime::spawn_blocking(move || {
        dependencies.install(|progress: DependencyProgress| {
            app.emit("dependency-setup-progress", progress)
                .map_err(|_| {
                    AppError::new(
                        "dependency_progress_failed",
                        "ClipKeep could not update the media-tool setup status.",
                        true,
                    )
                })
        })
    })
    .await
    .map_err(|_| {
        AppError::new(
            "dependency_install_failed",
            "ClipKeep could not complete media-tool setup.",
            true,
        )
    })?
}
