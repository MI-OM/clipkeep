# ClipKeep Architecture

**Status:** Approved for staged implementation

## Principles

- Process media locally.
- Keep frontend components unaware of executable paths, process arguments, and unrestricted filesystem access.
- Validate typed requests in Rust even when the frontend provides early validation.
- Fail closed if a dependency cannot be verified.
- Allow up to three active jobs, with a backend-enforced limit.
- Treat remote metadata, filenames, thumbnails, and process output as untrusted.

## System shape

```text
React + TypeScript screens
  Save media / Downloads / Settings / About
  Typed command and event client
                |
                | narrow Tauri commands and typed events
                v
Rust services
  URL validation | job manager | settings/history | file manager
  transcription model manager | local audio preparation | sherpa-onnx ASR
  dependency manager | process lifecycle | output validation
                |
                | fixed executable + explicit argument vector
                v
yt-dlp and pinned media tools
                |
                v
User-selected output + app-local settings/history
```

The browser preview remains a mock service and does not execute yt-dlp or FFmpeg. The
Windows x64 desktop app uses a pinned dependency manifest, downloads into app-local
staging, checks artifact hashes before extraction, validates expected ZIP members and
PE architecture, and activates the verified set atomically. The resolver fails closed
until all installed files pass their recorded integrity checks.

## Component responsibilities

| Component | Responsibility |
|---|---|
| React UI | URL, format/quality, destination, progress, settings, accessible errors |
| Frontend validation | Early UX feedback; never treated as a security boundary |
| Tauri commands | Typed input/output, repeated validation, narrow authorization |
| URL validator | HTTPS and exact allow-list for YouTube, Vimeo, Facebook, TikTok, and Instagram; strict YouTube ID parsing and generic non-empty paths for other platforms |
| Job manager | Unique IDs, bounded concurrency, state transitions, events, cancellation |
| Process adapter | Fixed executable, argument array, bounded output drains, process-tree lifecycle |
| Dependency manager | Platform selection, integrity/version/architecture checks, health and repair |
| File manager | User-selected destination, safe temporary output, traversal/collision handling, validation/finalization |
| Settings/history | Local preferences and minimal recent outcomes |
| Diagnostics | Bounded and redacted local details; explicit failures |

## Typed contracts

Proposed frontend models:

```ts
type MediaFormat = "mp4" | "mp3";
type Quality =
  | "best" | "1080p" | "720p" | "480p"
  | "320kbps" | "192kbps" | "128kbps";

interface DownloadRequest {
  url: string;
  format: MediaFormat;
  quality: Quality;
  destinationDirectory: string;
}

interface MediaMetadata {
  title: string;
  durationSeconds: number | null;
  thumbnailUrl: string | null;
  availableQualities: Quality[];
}

type DownloadStatus =
  | "queued" | "validating" | "fetching_metadata" | "downloading"
  | "merging" | "converting" | "finalizing"
  | "completed" | "failed" | "cancelling" | "cancelled";

interface DownloadJob {
  id: string;
  title: string | null;
  format: MediaFormat;
  quality: Quality;
  status: DownloadStatus;
  progressPercent: number | null;
  bytesPerSecond: number | null;
  etaSeconds: number | null;
  outputPath: string | null;
  error: { code: string; message: string; recoverable: boolean } | null;
}

interface ProgressEvent {
  jobId: string;
  status: DownloadStatus;
  progressPercent: number | null;
  bytesPerSecond: number | null;
  etaSeconds: number | null;
}
```

Proposed Tauri operations: `validate_media_url`, `get_media_metadata`, `start_download`, `cancel_download`, `list_download_history`, `clear_download_history`, `get_settings`, `save_settings`, `get_dependency_status`, and `choose_destination`. There is no generic command-execution command. Exact serialization and command names are finalized with the Rust milestone.

## Job lifecycle

```text
queued -> validating -> fetching_metadata -> downloading
  downloading -> merging -> finalizing       (MP4 when needed)
  downloading -> converting -> finalizing    (MP3)
  downloading ----------------> finalizing
  any active state -> cancelling -> cancelled
  any active state -> failed
  finalizing -> completed
```

`completed`, `failed`, and `cancelled` are terminal. Completion requires process success, output validation, and finalization. Cancellation races resolve to one terminal outcome. Retries use a new job ID and never reuse partial output.

## Persistence and repository boundaries

Preferences/history are local and minimal; full source URLs are not persisted by default. App-managed tools and temporary files are separate from user media. Avoid a database server. The exact local persistence format and app-shutdown policy remain open decisions.

The current shell uses Tauri's dialog plugin for directory selection and grants only `dialog:allow-open` alongside core defaults. Shared UI does not access the filesystem directly.

## Milestone 2-3 implementation boundary

The current backend accepts HTTPS URLs on exact YouTube, Vimeo, Facebook, TikTok, and Instagram host allow-lists, exposes metadata,
start, and cancel commands, and emits typed progress events. Process arguments are
constructed as a fixed vector, output is bounded, jobs are limited to three at a time,
and Windows child processes are assigned to a kill-on-close Job Object. MP4 selects
compatible MP4 video/audio streams and merges them when required. MP3 selects an audio
stream and uses FFmpeg extraction at the requested output bitrate. Work is staged in a
job-specific temporary directory; ffprobe verifies the output container and streams,
and a no-replace hard link finalizes the file. The browser preview remains an explicitly
simulated, non-success path.

The production resolver verifies installed files before returning paths. Metadata and
downloads use the app-managed Deno runtime with yt-dlp's JavaScript challenge support.
The user initiates dependency installation and repair from Settings; there is no
background update check. The selected FFmpeg candidate is a third-party GPLv3 build,
so public redistribution remains blocked on complete license/source notices and review.
MP4/MP3 processing is not yet verified against authorized real media, and this does not
claim clean-install release readiness.

The non-YouTube host additions only route candidate links to the pinned yt-dlp extractor.
Extractor reliability, permissions, and platform terms have not been verified per host.
No browser cookies, credentials, or access-control circumvention are supported. Output is
staged under a fixed `media` basename in a per-job directory so different platforms do not
need a YouTube-specific ID field.

The initial transcription path uses sherpa-onnx 1.13.8 through its Rust API. The user
initiates installation of Parakeet TDT 0.6B v2 INT8 to app-local storage; ONNX files are
checksum verified before activation. A selected local file is decoded to temporary 16 kHz
mono PCM by the verified FFmpeg and passed to the offline recognizer. Recognized text is
editable and exports as TXT. This first slice does not yet provide transcription
cancellation, resumable segments, timed subtitle exports, or transcription history.
Parakeet is provisional pending target-hardware benchmarking.

## References

- [Tauri sidecars](https://v2.tauri.app/develop/sidecar/)
- [Tauri capabilities](https://v2.tauri.app/security/capabilities/)
- [yt-dlp project](https://github.com/yt-dlp/yt-dlp)
- [FFmpeg legal guidance](https://ffmpeg.org/legal.html)
