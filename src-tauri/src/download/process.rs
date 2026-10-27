use std::{
    collections::BTreeSet,
    fs,
    io::{self, Read},
    path::{Path, PathBuf},
    process::{Child, Command, ExitStatus, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, SyncSender, TryRecvError},
    },
    thread,
    time::{Duration, Instant},
};

use sha2::{Digest, Sha256};
use tauri::{AppHandle, Emitter};

use super::tools::MediaTools;
use super::types::{
    AppError, DownloadJob, DownloadRequest, DownloadStatus, MediaFormat, MediaMetadata,
    ProgressEvent, Quality,
};
use super::url::validate_media_url;

const METADATA_LIMIT: usize = 4 * 1024 * 1024;
const LINE_LIMIT: usize = 8 * 1024;
const MAX_FILENAME_UTF16_UNITS: usize = 160;
const PROCESS_TIMEOUT: Duration = Duration::from_secs(60 * 60 * 4);
const PROGRESS_EVENT: &str = "download-progress";
const PROGRESS_PREFIX: &str = "CK_PROGRESS:";
const POSTPROCESS_PREFIX: &str = "CK_POSTPROCESS:";

pub fn metadata_arguments(url: &str, deno: &Path) -> Vec<String> {
    vec![
        "--ignore-config".into(),
        "--no-playlist".into(),
        "--no-warnings".into(),
        "--js-runtimes".into(),
        format!("deno:{}", deno.to_string_lossy()),
        "--skip-download".into(),
        "--dump-single-json".into(),
        url.into(),
    ]
}

pub fn download_arguments(
    request: &DownloadRequest,
    normalized_url: &str,
    destination: &Path,
    ffmpeg: &Path,
    deno: &Path,
) -> Result<Vec<String>, AppError> {
    let mut args = vec![
        "--ignore-config".into(),
        "--no-playlist".into(),
        "--no-warnings".into(),
        "--js-runtimes".into(),
        format!("deno:{}", deno.to_string_lossy()),
        "--no-overwrites".into(),
        "--restrict-filenames".into(),
        "--newline".into(),
        "--progress".into(),
        "--progress-template".into(),
        format!(
            "download:{PROGRESS_PREFIX}%(progress._percent_str)s|%(progress._speed_str)s|%(progress._eta_str)s"
        ),
        "--progress-template".into(),
        format!("postprocess:{POSTPROCESS_PREFIX}%(progress.status)s"),
        "--paths".into(),
        destination.to_string_lossy().into_owned(),
        "--output".into(),
        "media.%(ext)s".into(),
        "--ffmpeg-location".into(),
        ffmpeg.to_string_lossy().into_owned(),
    ];

    match request.format {
        MediaFormat::Mp4 => {
            args.extend([
                "--format".into(),
                request.quality.mp4_selector()?,
                "--merge-output-format".into(),
                "mp4".into(),
            ]);
        }
        MediaFormat::Mp3 => {
            args.extend([
                "--format".into(),
                "bestaudio/best".into(),
                "--extract-audio".into(),
                "--audio-format".into(),
                "mp3".into(),
                "--audio-quality".into(),
                request.quality.mp3_bitrate()?.into(),
            ]);
        }
    }
    args.push(normalized_url.into());
    Ok(args)
}

pub fn get_metadata(
    executable: &Path,
    deno: &Path,
    input_url: &str,
) -> Result<MediaMetadata, AppError> {
    let url = validate_media_url(input_url)?;
    let args = metadata_arguments(&url, deno);
    let (stdout, status) = run_capture(executable, &args, METADATA_LIMIT, Duration::from_secs(90))?;
    if !status.success() {
        return Err(AppError::new(
            "metadata_failed",
            "Could not read this video's details. Check the link and try again.",
            true,
        ));
    }

    let value: serde_json::Value = serde_json::from_slice(&stdout).map_err(|_| {
        AppError::new(
            "metadata_invalid",
            "The media engine returned unreadable video details.",
            true,
        )
    })?;

    let title = value
        .get("title")
        .and_then(serde_json::Value::as_str)
        .filter(|title| !title.trim().is_empty())
        .map(|title| title.chars().take(300).collect())
        .ok_or_else(|| {
            AppError::new(
                "metadata_missing_title",
                "The video details did not include a title.",
                true,
            )
        })?;

    let formats = value
        .get("formats")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let available_video_qualities = mp4_qualities(&formats);
    let available_audio_qualities = mp3_qualities(&formats);
    let thumbnail_url = value
        .get("thumbnail")
        .and_then(serde_json::Value::as_str)
        .filter(|thumbnail| UrlSafety::is_https(thumbnail))
        .map(ToOwned::to_owned);

    Ok(MediaMetadata {
        title,
        duration_seconds: value.get("duration").and_then(serde_json::Value::as_u64),
        thumbnail_url,
        available_video_qualities,
        available_audio_qualities,
    })
}

pub fn start_download(
    app: AppHandle,
    tools: MediaTools,
    request: DownloadRequest,
    job_id: String,
    cancel: Arc<AtomicBool>,
) -> Result<(DownloadJob, thread::JoinHandle<()>), AppError> {
    let normalized_url = validate_media_url(&request.url)?;
    match request.format {
        MediaFormat::Mp4 => {
            request.quality.mp4_selector()?;
        }
        MediaFormat::Mp3 => {
            request.quality.mp3_bitrate()?;
        }
    }
    let destination = PathBuf::from(&request.destination_directory)
        .canonicalize()
        .map_err(|_| {
            AppError::new(
                "invalid_destination",
                "Choose an existing save folder before starting a download.",
                false,
            )
        })?;
    if !destination.is_dir() {
        return Err(AppError::new(
            "invalid_destination",
            "The save location must be a folder.",
            false,
        ));
    }
    let source_key = Sha256::digest(normalized_url.as_bytes());
    let fallback_key = source_key[..6]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let fallback_title = format!("video-{fallback_key}");
    let title = resolve_output_title(request.title.as_deref(), &fallback_title, || {
        get_metadata(&tools.yt_dlp, &tools.deno, &normalized_url).map(|metadata| metadata.title)
    });
    let filename_stem = sanitize_filename(&title, &fallback_title);
    let extension = match request.format {
        MediaFormat::Mp4 => "mp4",
        MediaFormat::Mp3 => "mp3",
    };
    let final_output = destination.join(format!("{filename_stem}.{extension}"));
    ensure_output_available(&final_output)?;

    let temp_directory = destination.join(format!(".clipkeep-{job_id}"));
    fs::create_dir(&temp_directory).map_err(|_| {
        AppError::new(
            "temporary_directory_failed",
            "A temporary work folder could not be created in the save location.",
            true,
        )
    })?;
    let staged_output = temp_directory.join(format!("media.{extension}"));

    let job = DownloadJob {
        id: job_id.clone(),
        title: Some(title),
        format: request.format,
        quality: request.quality,
        status: DownloadStatus::Queued,
        progress_percent: None,
        bytes_per_second: None,
        eta_seconds: None,
        output_path: None,
        error: None,
        created_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64,
    };
    let args = match download_arguments(
        &request,
        &normalized_url,
        &temp_directory,
        &tools.ffmpeg,
        &tools.deno,
    ) {
        Ok(args) => args,
        Err(error) => {
            let _ = fs::remove_dir(&temp_directory);
            return Err(error);
        }
    };
    let format = request.format;
    let cleanup_directory = temp_directory.clone();

    let worker = thread::Builder::new()
        .name(format!("clipkeep-download-{job_id}"))
        .spawn(move || {
            run_download(
                app,
                tools,
                args,
                staged_output,
                final_output,
                temp_directory,
                format,
                job_id,
                cancel,
            );
        });
    let worker = match worker {
        Ok(worker) => worker,
        Err(_) => {
            if let Err(error) = fs::remove_dir(&cleanup_directory) {
                if error.kind() != io::ErrorKind::NotFound {
                    return Err(AppError::new(
                        "temporary_cleanup_failed",
                        "The job could not start, and its temporary folder could not be removed.",
                        true,
                    ));
                }
            }
            return Err(AppError::new(
                "job_start_failed",
                "The download could not be started.",
                true,
            ));
        }
    };

    Ok((job, worker))
}

fn run_download(
    app: AppHandle,
    tools: MediaTools,
    args: Vec<String>,
    staged_output: PathBuf,
    final_output: PathBuf,
    temp_directory: PathBuf,
    format: MediaFormat,
    job_id: String,
    cancel: Arc<AtomicBool>,
) {
    let mut workspace = TempWorkspace(temp_directory);
    let result = run_download_inner(
        &app,
        &tools,
        args,
        &staged_output,
        &final_output,
        format,
        &job_id,
        &cancel,
    );
    if workspace.cleanup().is_err() {
        if let Ok(Some(path)) = &result {
            let _ = fs::remove_file(path);
        }
        emit_failure(
            &app,
            &job_id,
            AppError::new(
                "temporary_cleanup_failed",
                "Temporary media files could not be safely cleaned. The job was not marked complete.",
                true,
            ),
        );
        return;
    }

    match result {
        Ok(Some(path)) => {
            let mut event = ProgressEvent::new(&job_id, DownloadStatus::Completed);
            event.output_path = Some(path);
            emit(&app, event);
        }
        Ok(None) => emit(&app, ProgressEvent::new(&job_id, DownloadStatus::Cancelled)),
        Err(error) => emit_failure(&app, &job_id, error),
    }
}

fn run_download_inner(
    app: &AppHandle,
    tools: &MediaTools,
    args: Vec<String>,
    staged_output: &Path,
    final_output: &Path,
    format: MediaFormat,
    job_id: &str,
    cancel: &AtomicBool,
) -> Result<Option<String>, AppError> {
    if cancel.load(Ordering::SeqCst) {
        return Ok(None);
    }
    emit(app, ProgressEvent::new(job_id, DownloadStatus::Validating));

    let mut command = Command::new(&tools.yt_dlp);
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(windows_sys::Win32::System::Threading::CREATE_NO_WINDOW);
    }

    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(_) => {
            return Err(AppError::new(
                "process_start_failed",
                "The verified media engine could not be started.",
                true,
            ));
        }
    };

    #[cfg(windows)]
    let job_object = match WindowsJob::assign(&child) {
        Ok(job) => Some(job),
        Err(_) => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(AppError::new(
                "process_isolation_failed",
                "The media process could not be safely isolated.",
                false,
            ));
        }
    };

    let stdout = match child.stdout.take() {
        Some(stdout) => stdout,
        None => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(AppError::new(
                "process_pipe_failed",
                "Could not read media progress.",
                true,
            ));
        }
    };
    let stderr = child.stderr.take();
    let (sender, receiver) = mpsc::sync_channel(32);
    let output_reader = thread::spawn(move || read_progress(stdout, sender));
    let stderr_reader = stderr.map(|stderr| thread::spawn(move || drain(stderr)));

    emit(app, ProgressEvent::new(job_id, DownloadStatus::Downloading));
    let result = wait_for_download(
        &mut child,
        &receiver,
        app,
        job_id,
        &cancel,
        format,
        PROCESS_TIMEOUT,
        #[cfg(windows)]
        job_object.as_ref(),
    );

    let output_read_result = output_reader.join();
    let stderr_read_result = stderr_reader.map(|reader| reader.join());
    drain_progress(&receiver, app, job_id, format);
    let output_read_result = output_read_result.map_err(|_| {
        AppError::new(
            "process_read_failed",
            "Could not read media progress.",
            true,
        )
    })?;
    output_read_result.map_err(|_| {
        AppError::new(
            "process_read_failed",
            "Could not read media progress.",
            true,
        )
    })?;
    if let Some(stderr_read_result) = stderr_read_result {
        stderr_read_result
            .map_err(|_| {
                AppError::new("process_read_failed", "Could not read media errors.", true)
            })?
            .map_err(|_| {
                AppError::new("process_read_failed", "Could not read media errors.", true)
            })?;
    }

    match result {
        Ok(DownloadStatus::Completed) => {
            emit(app, ProgressEvent::new(job_id, DownloadStatus::Finalizing));
            if !staged_output.is_file()
                || fs::metadata(&staged_output)
                    .map(|metadata| metadata.len() == 0)
                    .unwrap_or(true)
            {
                return Err(AppError::new(
                    "output_missing",
                    "The media engine finished without creating the expected file.",
                    true,
                ));
            }
            validate_media_output(&tools.ffprobe, staged_output, format)?;
            if let Err(error) = fs::hard_link(staged_output, final_output) {
                let (code, message) = if error.kind() == io::ErrorKind::AlreadyExists {
                    (
                        "file_exists",
                        "A file with this video ID already exists. Choose another save folder.",
                    )
                } else {
                    (
                        "output_finalize_failed",
                        "The validated file could not be safely finalized in the save folder.",
                    )
                };
                return Err(AppError::new(code, message, true));
            }
            Ok(Some(final_output.to_string_lossy().into_owned()))
        }
        Ok(DownloadStatus::Cancelled) => Ok(None),
        Ok(_) => Err(AppError::new(
            "job_state_invalid",
            "The media process returned an unexpected job state.",
            false,
        )),
        Err(error) => Err(error),
    }
}

fn wait_for_download(
    child: &mut Child,
    receiver: &Receiver<String>,
    app: &AppHandle,
    job_id: &str,
    cancel: &AtomicBool,
    format: MediaFormat,
    timeout: Duration,
    #[cfg(windows)] job_object: Option<&WindowsJob>,
) -> Result<DownloadStatus, AppError> {
    let started = Instant::now();

    loop {
        drain_progress(receiver, app, job_id, format);

        if cancel.load(Ordering::SeqCst) {
            emit(app, ProgressEvent::new(job_id, DownloadStatus::Cancelling));
            terminate(
                child,
                #[cfg(windows)]
                job_object,
            );
            return Ok(DownloadStatus::Cancelled);
        }
        if started.elapsed() >= timeout {
            terminate(
                child,
                #[cfg(windows)]
                job_object,
            );
            return Err(AppError::new(
                "download_timeout",
                "The download took too long and was stopped.",
                true,
            ));
        }

        match child.try_wait() {
            Ok(Some(status)) if status.success() => {
                return Ok(DownloadStatus::Completed);
            }
            Ok(Some(_)) => {
                let (code, message) = match format {
                    MediaFormat::Mp4 => (
                        "mp4_processing_failed",
                        "MP4 download or stream merging failed. Check the link and try again.",
                    ),
                    MediaFormat::Mp3 => (
                        "mp3_conversion_failed",
                        "Audio download or MP3 conversion failed. Verify the conversion tools and try again.",
                    ),
                };
                return Err(AppError::new(code, message, true));
            }
            Ok(None) => thread::sleep(Duration::from_millis(50)),
            Err(_) => {
                terminate(
                    child,
                    #[cfg(windows)]
                    job_object,
                );
                return Err(AppError::new(
                    "process_wait_failed",
                    "The download process stopped unexpectedly.",
                    true,
                ));
            }
        }
    }
}

fn drain_progress(receiver: &Receiver<String>, app: &AppHandle, job_id: &str, format: MediaFormat) {
    loop {
        match receiver.try_recv() {
            Ok(line) => {
                if line.starts_with(POSTPROCESS_PREFIX) {
                    let status = match format {
                        MediaFormat::Mp4 => DownloadStatus::Merging,
                        MediaFormat::Mp3 => DownloadStatus::Converting,
                    };
                    emit(app, ProgressEvent::new(job_id, status));
                    continue;
                }
                if let Some(progress) = parse_progress(&line) {
                    let mut event = ProgressEvent::new(job_id, DownloadStatus::Downloading);
                    event.progress_percent = progress.0;
                    event.bytes_per_second = progress.1;
                    event.eta_seconds = progress.2;
                    emit(app, event);
                }
            }
            Err(TryRecvError::Empty | TryRecvError::Disconnected) => break,
        }
    }
}

fn read_progress<R: Read>(mut reader: R, sender: SyncSender<String>) -> io::Result<()> {
    let mut line = Vec::with_capacity(256);
    let mut discard = false;
    let mut buffer = [0u8; 1024];
    loop {
        let read = match reader.read(&mut buffer) {
            Ok(0) => break,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error),
            Ok(read) => read,
        };
        for &byte in &buffer[..read] {
            if byte == b'\n' {
                if !discard {
                    let text = String::from_utf8_lossy(&line).trim().to_string();
                    if !text.is_empty() {
                        match sender.try_send(text) {
                            Ok(()) => {}
                            Err(mpsc::TrySendError::Full(_)) => {}
                            Err(mpsc::TrySendError::Disconnected(_)) => return Ok(()),
                        }
                    }
                }
                line.clear();
                discard = false;
            } else if !discard {
                if line.len() < LINE_LIMIT {
                    line.push(byte);
                } else {
                    line.clear();
                    discard = true;
                }
            }
        }
    }
    if !discard && !line.is_empty() {
        let _ = sender.try_send(String::from_utf8_lossy(&line).trim().to_string());
    }
    Ok(())
}

fn drain<R: Read>(mut reader: R) -> io::Result<()> {
    let mut buffer = [0u8; 4096];
    loop {
        match reader.read(&mut buffer) {
            Ok(0) => return Ok(()),
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) => return Err(error),
        }
    }
}

fn run_capture(
    executable: &Path,
    args: &[String],
    output_limit: usize,
    timeout: Duration,
) -> Result<(Vec<u8>, ExitStatus), AppError> {
    let mut command = Command::new(executable);
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(windows_sys::Win32::System::Threading::CREATE_NO_WINDOW);
    }

    let mut child = command.spawn().map_err(|_| {
        AppError::new(
            "process_start_failed",
            "The verified media engine could not be started.",
            true,
        )
    })?;

    #[cfg(windows)]
    let job_object = WindowsJob::assign(&child).map_err(|_| {
        let _ = child.kill();
        AppError::new(
            "process_isolation_failed",
            "The media process could not be safely isolated.",
            false,
        )
    })?;

    let stdout = child.stdout.take().ok_or_else(|| {
        AppError::new("process_pipe_failed", "Could not read video details.", true)
    })?;
    let stderr = child.stderr.take();
    let too_large = Arc::new(AtomicBool::new(false));
    let too_large_reader = Arc::clone(&too_large);
    let output_reader = thread::spawn(move || {
        let mut bytes = Vec::new();
        let mut limited = stdout.take((output_limit + 1) as u64);
        let result = limited.read_to_end(&mut bytes);
        if bytes.len() > output_limit {
            too_large_reader.store(true, Ordering::SeqCst);
        }
        (bytes, result)
    });
    let stderr_reader = stderr.map(|stderr| thread::spawn(move || drain(stderr)));

    let started = Instant::now();
    let status = loop {
        if too_large.load(Ordering::SeqCst) || started.elapsed() >= timeout {
            terminate(
                &mut child,
                #[cfg(windows)]
                Some(&job_object),
            );
            break None;
        }
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) => thread::sleep(Duration::from_millis(50)),
            Err(_) => {
                terminate(
                    &mut child,
                    #[cfg(windows)]
                    Some(&job_object),
                );
                break None;
            }
        }
    };

    let output_result = output_reader.join();
    let stderr_result = stderr_reader.map(|reader| reader.join());
    let (stdout, read_result) = output_result
        .map_err(|_| AppError::new("process_read_failed", "Could not read video details.", true))?;
    if let Some(stderr_result) = stderr_result {
        stderr_result
            .map_err(|_| {
                AppError::new("process_read_failed", "Could not read media errors.", true)
            })?
            .map_err(|_| {
                AppError::new("process_read_failed", "Could not read media errors.", true)
            })?;
    }
    read_result
        .map_err(|_| AppError::new("process_read_failed", "Could not read video details.", true))?;
    if too_large.load(Ordering::SeqCst) {
        return Err(AppError::new(
            "metadata_too_large",
            "The video's details were too large to process safely.",
            false,
        ));
    }
    let status = status.ok_or_else(|| {
        AppError::new(
            "metadata_timeout",
            "Reading video details took too long and was stopped.",
            true,
        )
    })?;
    Ok((stdout, status))
}

fn parse_progress(line: &str) -> Option<(Option<f64>, Option<f64>, Option<u64>)> {
    let values = line
        .strip_prefix(PROGRESS_PREFIX)?
        .split('|')
        .collect::<Vec<_>>();
    if values.len() != 3 {
        return None;
    }
    Some((
        parse_percent(values[0]),
        parse_speed(values[1]),
        parse_time(values[2]),
    ))
}

fn parse_percent(value: &str) -> Option<f64> {
    value
        .trim()
        .trim_end_matches('%')
        .trim()
        .parse::<f64>()
        .ok()
        .filter(|percent| percent.is_finite() && (0.0..=100.0).contains(percent))
}

fn parse_speed(value: &str) -> Option<f64> {
    let value = value.trim();
    let (number, multiplier) = if let Some(value) = value.strip_suffix("GiB/s") {
        (value, 1024_f64.powi(3))
    } else if let Some(value) = value.strip_suffix("MiB/s") {
        (value, 1024_f64.powi(2))
    } else if let Some(value) = value.strip_suffix("KiB/s") {
        (value, 1024_f64)
    } else if let Some(value) = value.strip_suffix("B/s") {
        (value, 1.0)
    } else {
        return None;
    };
    number
        .trim()
        .parse::<f64>()
        .ok()
        .map(|number| number * multiplier)
        .filter(|speed| speed.is_finite() && *speed >= 0.0)
}

fn parse_time(value: &str) -> Option<u64> {
    let mut parts = value.trim().split(':').rev();
    let seconds = parts.next()?.parse::<u64>().ok()?;
    let minutes = parts
        .next()
        .map(str::parse::<u64>)
        .transpose()
        .ok()?
        .unwrap_or(0);
    let hours = parts
        .next()
        .map(str::parse::<u64>)
        .transpose()
        .ok()?
        .unwrap_or(0);
    if parts.next().is_some() {
        return None;
    }
    Some(hours.saturating_mul(3600) + minutes.saturating_mul(60) + seconds)
}

fn mp3_qualities(formats: &[serde_json::Value]) -> Vec<Quality> {
    if formats.iter().any(|format| {
        format
            .get("acodec")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|codec| codec != "none")
    }) {
        vec![
            Quality::Best,
            Quality::Kbps320,
            Quality::Kbps192,
            Quality::Kbps128,
        ]
    } else {
        Vec::new()
    }
}

fn mp4_qualities(formats: &[serde_json::Value]) -> Vec<Quality> {
    let has_m4a_audio = formats.iter().any(|format| {
        format.get("ext").and_then(serde_json::Value::as_str) == Some("m4a")
            && format
                .get("acodec")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|codec| codec != "none")
    });
    let mut heights = BTreeSet::new();
    for format in formats {
        if format.get("ext").and_then(serde_json::Value::as_str) != Some("mp4") {
            continue;
        }
        let has_video = format
            .get("vcodec")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|codec| codec != "none");
        let has_embedded_audio = format
            .get("acodec")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|codec| codec != "none");
        if !has_video || (!has_embedded_audio && !has_m4a_audio) {
            continue;
        }
        if let Some(height) = format.get("height").and_then(serde_json::Value::as_u64) {
            heights.insert(height);
        }
    }

    if heights.is_empty() {
        return Vec::new();
    }
    let mut result = vec![Quality::Best];
    for (height, quality) in [
        (1080, Quality::P1080),
        (720, Quality::P720),
        (480, Quality::P480),
    ] {
        if heights.iter().any(|available| *available >= height) {
            result.push(quality);
        }
    }
    result
}

fn ensure_output_available(path: &Path) -> Result<(), AppError> {
    match fs::symlink_metadata(path) {
        Ok(_) => Err(AppError::new(
            "file_exists",
            "A file with this title already exists. Choose another save folder.",
            true,
        )),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(_) => Err(AppError::new(
            "output_check_failed",
            "The save location could not be checked for an existing file.",
            true,
        )),
    }
}

fn sanitize_filename(title: &str, fallback: &str) -> String {
    let mut filename = String::new();
    let mut utf16_units = 0;
    for character in title.trim().chars() {
        let safe_character = if character.is_control() || "<>:\"/\\|?*".contains(character) {
            '_'
        } else {
            character
        };
        let units = safe_character.len_utf16();
        if utf16_units + units > MAX_FILENAME_UTF16_UNITS {
            break;
        }
        filename.push(safe_character);
        utf16_units += units;
    }

    let filename = filename.trim_end_matches([' ', '.']).to_string();
    if filename.is_empty() {
        return fallback.to_string();
    }

    let device_name = filename
        .split('.')
        .next()
        .unwrap_or_default()
        .trim_end_matches([' ', '.'])
        .to_ascii_uppercase();
    let is_reserved_device_name = matches!(
        device_name.as_str(),
        "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
    ) || (device_name.len() == 4
        && (device_name.starts_with("COM") || device_name.starts_with("LPT"))
        && matches!(device_name.as_bytes()[3], b'1'..=b'9'));

    if is_reserved_device_name {
        format!("_{filename}")
    } else {
        filename
    }
}

fn resolve_output_title(
    supplied_title: Option<&str>,
    fallback: &str,
    load_metadata_title: impl FnOnce() -> Result<String, AppError>,
) -> String {
    if let Some(title) = supplied_title
        .map(str::trim)
        .filter(|title| !title.is_empty())
    {
        return title.to_string();
    }
    match load_metadata_title() {
        Ok(title) if !title.trim().is_empty() => title,
        _ => fallback.to_string(),
    }
}

fn validate_media_output(ffprobe: &Path, path: &Path, format: MediaFormat) -> Result<(), AppError> {
    let args = vec![
        "-v".into(),
        "error".into(),
        "-show_entries".into(),
        "format=format_name:stream=codec_type,codec_name".into(),
        "-of".into(),
        "json".into(),
        path.to_string_lossy().into_owned(),
    ];
    let (output, status) = run_capture(ffprobe, &args, 64 * 1024, Duration::from_secs(30))?;
    if status.success() && is_valid_media_probe(&output, format) {
        Ok(())
    } else {
        Err(AppError::new(
            "output_validation_failed",
            "The processed file did not pass media validation and was not saved.",
            true,
        ))
    }
}

fn is_valid_media_probe(output: &[u8], format: MediaFormat) -> bool {
    let Ok(probe) = serde_json::from_slice::<serde_json::Value>(output) else {
        return false;
    };
    let format_names = probe
        .get("format")
        .and_then(|format| format.get("format_name"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .split(',')
        .collect::<Vec<_>>();
    let streams = probe
        .get("streams")
        .and_then(serde_json::Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default();
    match format {
        MediaFormat::Mp4 => {
            format_names
                .iter()
                .any(|name| matches!(*name, "mp4" | "mov"))
                && streams.iter().any(|stream| {
                    stream.get("codec_type").and_then(serde_json::Value::as_str) == Some("video")
                })
                && streams.iter().any(|stream| {
                    stream.get("codec_type").and_then(serde_json::Value::as_str) == Some("audio")
                })
        }
        MediaFormat::Mp3 => {
            format_names.contains(&"mp3")
                && streams.iter().any(|stream| {
                    stream.get("codec_type").and_then(serde_json::Value::as_str) == Some("audio")
                        && stream.get("codec_name").and_then(serde_json::Value::as_str)
                            == Some("mp3")
                })
        }
    }
}

struct TempWorkspace(PathBuf);

impl TempWorkspace {
    fn cleanup(&mut self) -> io::Result<()> {
        match fs::remove_dir_all(&self.0) {
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            result => result,
        }
    }
}

impl Drop for TempWorkspace {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}

fn emit(app: &AppHandle, event: ProgressEvent) {
    let _ = app.emit(PROGRESS_EVENT, event);
}

fn emit_failure(app: &AppHandle, job_id: &str, error: AppError) {
    let mut event = ProgressEvent::new(job_id, DownloadStatus::Failed);
    event.error = Some(error);
    emit(app, event);
}

struct UrlSafety;

impl UrlSafety {
    fn is_https(value: &str) -> bool {
        url::Url::parse(value).is_ok_and(|url| url.scheme() == "https")
    }
}

fn terminate(child: &mut Child, #[cfg(windows)] job: Option<&WindowsJob>) {
    #[cfg(windows)]
    if let Some(job) = job {
        job.terminate();
    }
    let _ = child.kill();
    let _ = child.wait();
}

#[cfg(windows)]
struct WindowsJob(windows_sys::Win32::Foundation::HANDLE);

#[cfg(windows)]
impl WindowsJob {
    fn assign(child: &Child) -> Result<Self, ()> {
        use std::{mem, os::windows::io::AsRawHandle};
        use windows_sys::Win32::{
            Foundation::{CloseHandle, HANDLE},
            System::JobObjects::{
                AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
                JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
                SetInformationJobObject,
            },
        };

        let handle = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
        if handle.is_null() {
            return Err(());
        }

        let mut limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = unsafe { mem::zeroed() };
        limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        let configured = unsafe {
            SetInformationJobObject(
                handle,
                JobObjectExtendedLimitInformation,
                &limits as *const _ as *const _,
                mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            )
        };
        let assigned = unsafe { AssignProcessToJobObject(handle, child.as_raw_handle() as HANDLE) };
        if configured == 0 || assigned == 0 {
            unsafe { CloseHandle(handle) };
            return Err(());
        }
        Ok(Self(handle))
    }

    fn terminate(&self) {
        use windows_sys::Win32::System::JobObjects::TerminateJobObject;
        unsafe {
            TerminateJobObject(self.0, 1);
        }
    }
}

#[cfg(windows)]
impl Drop for WindowsJob {
    fn drop(&mut self) {
        use windows_sys::Win32::Foundation::CloseHandle;
        unsafe {
            CloseHandle(self.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{
        AppError, DownloadRequest, MAX_FILENAME_UTF16_UNITS, MediaFormat, Quality, TempWorkspace,
        download_arguments, ensure_output_available, is_valid_media_probe, metadata_arguments,
        mp3_qualities, mp4_qualities, parse_progress, parse_speed, parse_time,
        resolve_output_title, sanitize_filename,
    };
    use std::fs;

    fn request() -> DownloadRequest {
        DownloadRequest {
            url: "https://www.youtube.com/watch?v=BaW_jenozKc".into(),
            title: None,
            format: MediaFormat::Mp4,
            quality: Quality::P720,
            destination_directory: "C:\\videos".into(),
        }
    }

    #[test]
    fn metadata_arguments_are_fixed_and_ignore_local_configuration() {
        let args = metadata_arguments(
            "https://www.youtube.com/watch?v=BaW_jenozKc",
            Path::new("C:\\tools\\deno.exe"),
        );
        assert!(args.contains(&"--ignore-config".into()));
        assert!(!args.contains(&"--no-netrc".into()));
        assert!(args.contains(&"--no-playlist".into()));
        assert!(args.contains(&"--dump-single-json".into()));
        assert!(args.contains(&"deno:C:\\tools\\deno.exe".into()));
        assert_eq!(
            args.last().map(String::as_str),
            Some("https://www.youtube.com/watch?v=BaW_jenozKc")
        );
    }

    #[test]
    fn download_arguments_use_separate_fixed_arguments_and_safe_output_template() {
        let args = download_arguments(
            &request(),
            "https://www.youtube.com/watch?v=BaW_jenozKc",
            Path::new("C:\\user supplied\\folder"),
            Path::new("C:\\tools\\ffmpeg.exe"),
            Path::new("C:\\tools\\deno.exe"),
        )
        .unwrap();
        assert!(
            args.windows(2)
                .any(|pair| { pair[0] == "--output" && pair[1] == "media.%(ext)s" })
        );
        assert!(
            args.windows(2)
                .any(|pair| { pair[0] == "--no-overwrites" && pair[1] == "--restrict-filenames" })
        );
        assert!(!args.contains(&"--no-netrc".into()));
        assert!(
            args.windows(2)
                .any(|pair| { pair[0] == "--merge-output-format" && pair[1] == "mp4" })
        );
        assert!(
            args.windows(2).any(|pair| {
                pair[0] == "--ffmpeg-location" && pair[1] == "C:\\tools\\ffmpeg.exe"
            })
        );
        assert!(args.contains(&"deno:C:\\tools\\deno.exe".into()));
        assert!(args.contains(&"C:\\user supplied\\folder".into()));
        assert_eq!(
            args.last().map(String::as_str),
            Some("https://www.youtube.com/watch?v=BaW_jenozKc")
        );
        assert!(
            args.iter()
                .all(|arg| !arg.contains("C:\\user supplied\\folder "))
        );
    }

    #[test]
    fn mp3_arguments_extract_and_convert_audio_at_requested_bitrate() {
        let mut request = request();
        request.format = MediaFormat::Mp3;
        request.quality = Quality::Kbps192;
        let args = download_arguments(
            &request,
            "https://www.youtube.com/watch?v=BaW_jenozKc",
            Path::new("C:\\videos"),
            Path::new("C:\\tools\\ffmpeg.exe"),
            Path::new("C:\\tools\\deno.exe"),
        )
        .unwrap();

        assert!(
            args.windows(2)
                .any(|pair| { pair[0] == "--format" && pair[1] == "bestaudio/best" })
        );
        assert!(args.contains(&"--extract-audio".into()));
        assert!(
            args.windows(2)
                .any(|pair| { pair[0] == "--audio-format" && pair[1] == "mp3" })
        );
        assert!(
            args.windows(2)
                .any(|pair| { pair[0] == "--audio-quality" && pair[1] == "192K" })
        );
        assert_eq!(Quality::Best.mp3_bitrate().unwrap(), "0");
        assert!(Quality::P720.mp3_bitrate().is_err());
    }

    #[test]
    fn progress_parsing_handles_known_units_and_rejects_invalid_values() {
        assert_eq!(
            parse_progress("CK_PROGRESS:42.5%|1.5MiB/s|00:12"),
            Some((Some(42.5), Some(1.5 * 1024_f64.powi(2)), Some(12)))
        );
        assert_eq!(parse_progress("noise"), None);
        assert_eq!(
            parse_progress("CK_PROGRESS:101%|N/A|N/A"),
            Some((None, None, None))
        );
        assert_eq!(parse_speed("N/A"), None);
        assert_eq!(parse_time("1:02:03"), Some(3723));
        assert_eq!(parse_time("N/A"), None);
    }

    #[test]
    fn available_qualities_separate_mp4_and_mp3_source_support() {
        let formats = vec![
            serde_json::json!({
                "ext": "mp4", "vcodec": "avc1", "acodec": "none", "height": 1080
            }),
            serde_json::json!({
                "ext": "m4a", "vcodec": "none", "acodec": "mp4a"
            }),
        ];

        assert_eq!(
            mp4_qualities(&formats),
            vec![Quality::Best, Quality::P1080, Quality::P720, Quality::P480]
        );
        assert_eq!(
            mp3_qualities(&formats),
            vec![
                Quality::Best,
                Quality::Kbps320,
                Quality::Kbps192,
                Quality::Kbps128
            ]
        );
        assert!(mp4_qualities(&[]).is_empty());
        assert!(mp3_qualities(&[]).is_empty());
        assert_eq!(
            mp4_qualities(&[serde_json::json!({
                "ext": "mp4", "vcodec": "avc1", "acodec": "mp4a", "height": 480
            })]),
            vec![Quality::Best, Quality::P480]
        );
        assert_eq!(
            mp3_qualities(&[serde_json::json!({
                "ext": "m4a", "vcodec": "none", "acodec": "mp4a"
            })]),
            vec![
                Quality::Best,
                Quality::Kbps320,
                Quality::Kbps192,
                Quality::Kbps128
            ]
        );
    }

    #[test]
    fn generated_process_arguments_keep_untrusted_values_as_distinct_arguments() {
        let mut hostile_request = request();
        hostile_request.destination_directory = "C:\\Videos & --exec".into();
        let args = download_arguments(
            &hostile_request,
            "https://www.youtube.com/watch?v=BaW_jenozKc",
            Path::new("C:\\Videos & --exec"),
            Path::new("C:\\tools\\ffmpeg.exe"),
            Path::new("C:\\tools\\deno.exe"),
        )
        .unwrap();
        assert!(args.contains(&"C:\\Videos & --exec".into()));
        assert!(args.iter().all(|argument| !argument.contains("&&")));
    }

    #[test]
    fn output_collision_check_does_not_allow_overwriting_existing_files() {
        let directory =
            std::env::temp_dir().join(format!("clipkeep-output-test-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&directory).unwrap();
        let output = directory.join("video.mp4");
        assert!(ensure_output_available(&output).is_ok());
        fs::write(&output, b"existing").unwrap();
        assert_eq!(
            ensure_output_available(&output).unwrap_err().code,
            "file_exists"
        );
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn filename_uses_title_and_replaces_windows_path_characters() {
        assert_eq!(
            sanitize_filename("  My Video: Part 1/2?  ", "video-id"),
            "My Video_ Part 1_2_"
        );
    }

    #[test]
    fn filename_avoids_reserved_windows_device_names() {
        assert_eq!(sanitize_filename("CON", "video-id"), "_CON");
        assert_eq!(sanitize_filename("com1", "video-id"), "_com1");
        assert_eq!(
            sanitize_filename("Ordinary Title", "video-id"),
            "Ordinary Title"
        );
    }

    #[test]
    fn filename_uses_video_id_fallback_for_empty_or_dot_titles() {
        assert_eq!(
            sanitize_filename("...  ", "video-BaW_jenozKc"),
            "video-BaW_jenozKc"
        );
    }

    #[test]
    fn metadata_size_failure_falls_back_to_video_id_for_filename() {
        let title = resolve_output_title(None, "video-BaW_jenozKc", || {
            Err(AppError::new(
                "metadata_too_large",
                "The video's details were too large to process safely.",
                false,
            ))
        });
        assert_eq!(title, "video-BaW_jenozKc");
    }

    #[test]
    fn supplied_preview_title_skips_metadata_lookup() {
        let title = resolve_output_title(Some("A verified title"), "video-id", || {
            panic!("metadata lookup should not run when preview title is supplied")
        });
        assert_eq!(title, "A verified title");
    }

    #[test]
    fn filename_length_is_limited_by_windows_utf16_units() {
        let filename = sanitize_filename(&"😀".repeat(200), "video-id");
        assert_eq!(filename.encode_utf16().count(), MAX_FILENAME_UTF16_UNITS);
    }

    #[test]
    fn temporary_workspace_is_removed_on_drop() {
        let directory =
            std::env::temp_dir().join(format!("clipkeep-workspace-test-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&directory).unwrap();
        fs::write(directory.join("partial.mp4"), b"partial").unwrap();

        drop(TempWorkspace(directory.clone()));

        assert!(!directory.exists());
    }

    #[test]
    fn media_validation_requires_expected_container_and_streams() {
        let mp4 = serde_json::json!({
            "format": {"format_name": "mov,mp4,m4a"},
            "streams": [{"codec_type": "video"}, {"codec_type": "audio"}]
        });
        let audio_only = serde_json::json!({
            "format": {"format_name": "mp3"},
            "streams": [{"codec_type": "audio", "codec_name": "mp3"}]
        });
        let bad_mp3 = serde_json::json!({
            "format": {"format_name": "mp3"},
            "streams": [{"codec_type": "audio", "codec_name": "aac"}]
        });

        assert!(is_valid_media_probe(
            mp4.to_string().as_bytes(),
            MediaFormat::Mp4
        ));
        assert!(!is_valid_media_probe(
            audio_only.to_string().as_bytes(),
            MediaFormat::Mp4
        ));
        assert!(is_valid_media_probe(
            audio_only.to_string().as_bytes(),
            MediaFormat::Mp3
        ));
        assert!(!is_valid_media_probe(
            bad_mp3.to_string().as_bytes(),
            MediaFormat::Mp3
        ));
    }
}
