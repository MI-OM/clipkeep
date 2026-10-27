use std::path::PathBuf;

use super::types::AppError;

#[derive(Debug, Clone)]
pub struct MediaTools {
    pub yt_dlp: PathBuf,
    pub deno: PathBuf,
    pub ffmpeg: PathBuf,
    pub ffprobe: PathBuf,
}

pub trait ToolResolver: Send + Sync {
    fn resolve(&self) -> Result<MediaTools, AppError>;
}
