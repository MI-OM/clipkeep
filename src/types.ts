export type MediaFormat = "mp4" | "mp3";

export const MAX_ACTIVE_DOWNLOADS = 3;

export type Quality =
  | "best"
  | "1080p"
  | "720p"
  | "480p"
  | "320kbps"
  | "192kbps"
  | "128kbps";

export type DownloadStatus =
  | "queued"
  | "validating"
  | "fetching_metadata"
  | "downloading"
  | "merging"
  | "converting"
  | "finalizing"
  | "completed"
  | "failed"
  | "cancelling"
  | "cancelled";

export interface DownloadRequest {
  url: string;
  title?: string | null;
  format: MediaFormat;
  quality: Quality;
  destinationDirectory: string;
}

export interface MediaMetadata {
  title: string;
  durationSeconds: number | null;
  thumbnailUrl: string | null;
  availableVideoQualities: Quality[];
  availableAudioQualities: Quality[];
}

export interface AppError {
  code: string;
  message: string;
  recoverable: boolean;
}

export interface DownloadJob {
  id: string;
  title: string | null;
  format: MediaFormat;
  quality: Quality;
  status: DownloadStatus;
  progressPercent: number | null;
  bytesPerSecond: number | null;
  etaSeconds: number | null;
  outputPath: string | null;
  error: AppError | null;
  createdAt: number;
}

export interface ProgressEvent {
  jobId: string;
  status: DownloadStatus;
  progressPercent: number | null;
  bytesPerSecond: number | null;
  etaSeconds: number | null;
  outputPath: string | null;
  error: AppError | null;
}

export interface Settings {
  preferredFormat: MediaFormat;
  preferredQuality: Quality;
  destinationDirectory: string;
  rememberDestination: boolean;
}
