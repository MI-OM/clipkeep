use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    process::Command,
    sync::{Mutex, TryLockError},
    time::Instant,
};

use reqwest::{blocking::Client, redirect::Policy};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sherpa_onnx::{
    OfflineNemoEncDecCtcModelConfig, OfflineRecognizer, OfflineRecognizerConfig,
    OfflineTransducerModelConfig, OfflineWhisperModelConfig, Wave,
};
use tauri::{Emitter, State};
use uuid::Uuid;

use crate::download::{AppError, AppState, ToolResolver};

const MODEL_DIR: &str = "parakeet-tdt-0.6b-v2-int8";
const MODEL_FILES: [&str; 4] = [
    "encoder.int8.onnx",
    "decoder.int8.onnx",
    "joiner.int8.onnx",
    "tokens.txt",
];
const MODEL_REVISION: &str = "1ab9323";
const MODEL_REPOSITORY: &str =
    "https://huggingface.co/csukuangfj/sherpa-onnx-nemo-parakeet-tdt-0.6b-v2-int8/resolve/1ab9323";
const SMALL_ID: &str = "parakeet-tdt-ctc-110m-int8";
const LARGE_ID: &str = "parakeet-tdt-0.6b-v2-int8";
const SMALL_DIR: &str = "parakeet-tdt-ctc-110m-int8";
const SMALL_REPOSITORY: &str =
    "https://huggingface.co/VocaHQ/sherpa-onnx-nemo-parakeet-tdt-ctc-110m-en-int8/resolve/main";
const WHISPER_ID: &str = "whisper-tiny-en-int8";
const WHISPER_DIR: &str = "whisper-tiny-en-int8";
const WHISPER_REPOSITORY: &str = "https://huggingface.co/csukuangfj/sherpa-onnx-whisper-tiny.en/resolve/d026532c022fa99fd789d6b32446a1df7b6bfc43";
const WHISPER_BASE_ID: &str = "whisper-base-en-int8";
const WHISPER_BASE_DIR: &str = "whisper-base-en-int8";
const WHISPER_BASE_REPOSITORY: &str = "https://huggingface.co/csukuangfj/sherpa-onnx-whisper-base.en/resolve/8c8c1656435deee77c31d4b4f37b6fa0ef711c15";
static MODEL_INSTALL_LOCK: Mutex<()> = Mutex::new(());
static TRANSCRIPTION_LOCK: Mutex<()> = Mutex::new(());

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelInstallProgress {
    pub file: String,
    pub phase: String,
    pub percent: Option<u8>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptionStatus {
    pub models: Vec<ModelOption>,
    pub total_ram_mb: u64,
    pub available_ram_mb: u64,
    pub cpu_threads: usize,
    pub recommended_model_id: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelOption {
    pub id: String,
    pub name: String,
    pub description: String,
    pub download_mb: u32,
    pub minimum_ram_gb: u8,
    pub recommended_ram_gb: u8,
    pub minimum_available_ram_mb: u64,
    pub installed: bool,
    pub recommended: bool,
    pub can_run_now: bool,
    pub license: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptResult {
    pub source_path: String,
    pub title: String,
    pub text: String,
    pub duration_seconds: f64,
    pub elapsed_seconds: f64,
    pub model_name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveTranscriptRequest {
    pub path: String,
    pub text: String,
}

#[tauri::command]
pub async fn get_transcription_status(
    state: State<'_, AppState>,
) -> Result<TranscriptionStatus, AppError> {
    let root = state.transcription_root.clone();
    tauri::async_runtime::spawn_blocking(move || Ok(model_status(&root)))
        .await
        .map_err(|_| {
            AppError::new(
                "transcription_status_failed",
                "ClipKeep could not check the local transcription model.",
                true,
            )
        })?
}

#[tauri::command]
pub async fn install_transcription_model(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    model_id: String,
) -> Result<TranscriptionStatus, AppError> {
    let root = state.transcription_root.clone();
    tauri::async_runtime::spawn_blocking(move || {
        install_model(&root, &model_id, |progress| {
            app.emit("transcription-model-progress", progress)
                .map_err(|_| {
                    AppError::new(
                        "transcription_progress_failed",
                        "ClipKeep could not update model setup progress.",
                        true,
                    )
                })
        })
    })
    .await
    .map_err(|_| {
        AppError::new(
            "transcription_model_install_failed",
            "ClipKeep could not finish setting up the transcription model.",
            true,
        )
    })?
}

#[tauri::command]
pub async fn transcribe_file(
    state: State<'_, AppState>,
    source_path: String,
    model_id: String,
) -> Result<TranscriptResult, AppError> {
    let root = state.transcription_root.clone();
    let dependencies = state.dependencies.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let tools = dependencies.resolve()?;
        transcribe(&root, &tools.ffmpeg, PathBuf::from(source_path), &model_id)
    })
    .await
    .map_err(|_| {
        AppError::new(
            "transcription_task_failed",
            "ClipKeep could not complete the transcription operation.",
            true,
        )
    })?
}

#[tauri::command]
pub async fn save_transcript(request: SaveTranscriptRequest) -> Result<(), AppError> {
    tauri::async_runtime::spawn_blocking(move || {
        let path = PathBuf::from(request.path);
        if !matches!(
            path.extension().and_then(|extension| extension.to_str()),
            Some("txt")
        ) {
            return Err(AppError::new(
                "invalid_transcript_path",
                "Choose a .txt file for this transcript export.",
                false,
            ));
        }
        if request.text.len() > 32 * 1024 * 1024 {
            return Err(AppError::new(
                "transcript_too_large",
                "This transcript is too large to export safely.",
                false,
            ));
        }
        fs::write(path, request.text).map_err(|_| {
            AppError::new(
                "transcript_save_failed",
                "ClipKeep could not save the transcript to that location.",
                true,
            )
        })
    })
    .await
    .map_err(|_| {
        AppError::new(
            "transcript_save_task_failed",
            "ClipKeep could not save the transcript.",
            true,
        )
    })?
}

fn model_directory(root: &Path, model_id: &str) -> Result<PathBuf, AppError> {
    let directory = match model_id {
        SMALL_ID => SMALL_DIR,
        LARGE_ID => MODEL_DIR,
        WHISPER_ID => WHISPER_DIR,
        WHISPER_BASE_ID => WHISPER_BASE_DIR,
        _ => {
            return Err(AppError::new(
                "transcription_model_unknown",
                "Choose a transcription model from the available list.",
                false,
            ));
        }
    };
    Ok(root.join("models").join(directory))
}

fn model_status(root: &Path) -> TranscriptionStatus {
    let (total_ram_mb, available_ram_mb) = system_memory_mb();
    // This CTC model closed the app on an 8 GB machine; keep it available but restrict execution while investigated.
    let small_can_run = total_ram_mb >= 16 * 1024 && available_ram_mb >= 2 * 1024;
    let whisper_can_run = total_ram_mb >= 2 * 1024 && available_ram_mb >= 768;
    let large_can_run = total_ram_mb >= 16 * 1024 && available_ram_mb >= 6 * 1024;
    let cpu_threads = std::thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(1);
    let recommended_model_id =
        if total_ram_mb >= 16 * 1024 && available_ram_mb >= 6 * 1024 && cpu_threads >= 8 {
            LARGE_ID
        } else if total_ram_mb >= 8 * 1024 && available_ram_mb >= 1536 {
            WHISPER_BASE_ID
        } else if total_ram_mb >= 8 * 1024 && whisper_can_run {
            WHISPER_ID
        } else {
            WHISPER_ID
        };
    let small_directory = model_directory(root, SMALL_ID).expect("known id");
    let large_directory = model_directory(root, LARGE_ID).expect("known id");
    TranscriptionStatus {
        models: vec![
            ModelOption {
                id: SMALL_ID.into(),
                name: "Parakeet TDT CTC 110M INT8".into(),
                description: "Experimental compact model. It closed ClipKeep on an 8 GB system, so running it is restricted to systems with 16 GB RAM or more while investigated.".into(),
                download_mb: 126,
                minimum_ram_gb: 16,
                recommended_ram_gb: 16,
                minimum_available_ram_mb: 2 * 1024,
                installed: small_directory.join("model.int8.onnx").is_file()
                    && small_directory.join("tokens.txt").is_file(),
                recommended: recommended_model_id == SMALL_ID,
                can_run_now: small_can_run,
                license: "CC BY 4.0".into(),
            },
            ModelOption {
                id: WHISPER_ID.into(),
                name: "Whisper Tiny English INT8".into(),
                description: "Fastest Whisper option with lower recognition quality than Base. Useful when free memory is limited.".into(),
                download_mb: 99,
                minimum_ram_gb: 2,
                recommended_ram_gb: 8,
                minimum_available_ram_mb: 768,
                installed: ["tiny.en-encoder.int8.onnx", "tiny.en-decoder.int8.onnx", "tiny.en-tokens.txt"].iter().all(|file| model_directory(root, WHISPER_ID).expect("known id").join(file).is_file()),
                recommended: recommended_model_id == WHISPER_ID,
                can_run_now: whisper_can_run,
                license: "MIT (Whisper)".into(),
            },
            ModelOption {
                id: WHISPER_BASE_ID.into(),
                name: "Whisper Base English INT8".into(),
                description: "Higher-capacity Whisper model for better recognition quality; about 154 MB.".into(),
                download_mb: 154,
                minimum_ram_gb: 4,
                recommended_ram_gb: 8,
                minimum_available_ram_mb: 1536,
                installed: ["base.en-encoder.int8.onnx", "base.en-decoder.int8.onnx", "base.en-tokens.txt"].iter().all(|file| model_directory(root, WHISPER_BASE_ID).expect("known id").join(file).is_file()),
                recommended: recommended_model_id == WHISPER_BASE_ID,
                can_run_now: total_ram_mb >= 4 * 1024 && available_ram_mb >= 1536,
                license: "MIT (Whisper)".into(),
            },
            ModelOption {
                id: LARGE_ID.into(),
                name: "Parakeet TDT 0.6B v2 INT8".into(),
                description:
                    "Larger English model; recommended for systems with 16 GB RAM or more.".into(),
                download_mb: 661,
                minimum_ram_gb: 16,
                recommended_ram_gb: 16,
                minimum_available_ram_mb: 6 * 1024,
                installed: MODEL_FILES
                    .iter()
                    .all(|file| large_directory.join(file).is_file()),
                recommended: recommended_model_id == LARGE_ID,
                can_run_now: large_can_run,
                license: "CC BY 4.0".into(),
            },
        ],
        total_ram_mb,
        available_ram_mb,
        cpu_threads,
        recommended_model_id: recommended_model_id.into(),
    }
}

#[cfg(windows)]
fn system_memory_mb() -> (u64, u64) {
    use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
    let mut status = MEMORYSTATUSEX {
        dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32,
        ..Default::default()
    };
    if unsafe { GlobalMemoryStatusEx(&mut status) } != 0 {
        (
            status.ullTotalPhys / 1_048_576,
            status.ullAvailPhys / 1_048_576,
        )
    } else {
        (0, 0)
    }
}
#[cfg(not(windows))]
fn system_memory_mb() -> (u64, u64) {
    (0, 0)
}

fn install_model(
    root: &Path,
    model_id: &str,
    mut progress: impl FnMut(ModelInstallProgress) -> Result<(), AppError>,
) -> Result<TranscriptionStatus, AppError> {
    let _guard = MODEL_INSTALL_LOCK.lock().map_err(|_| {
        AppError::new(
            "transcription_setup_busy",
            "Transcription model setup is unavailable.",
            true,
        )
    })?;
    let destination = model_directory(root, model_id)?;
    let files: &[&str] = match model_id {
        SMALL_ID => &["model.int8.onnx", "tokens.txt"],
        WHISPER_ID => &[
            "tiny.en-encoder.int8.onnx",
            "tiny.en-decoder.int8.onnx",
            "tiny.en-tokens.txt",
        ],
        WHISPER_BASE_ID => &[
            "base.en-encoder.int8.onnx",
            "base.en-decoder.int8.onnx",
            "base.en-tokens.txt",
        ],
        _ => &MODEL_FILES,
    };
    if files.iter().all(|file| destination.join(file).is_file()) {
        return Ok(model_status(root));
    }
    let models_root = root.join("models");
    fs::create_dir_all(&models_root).map_err(|_| {
        AppError::new(
            "transcription_storage_unavailable",
            "ClipKeep could not prepare local model storage.",
            true,
        )
    })?;
    let staging = models_root.join(format!(".staging-{}", Uuid::new_v4()));
    fs::create_dir(&staging).map_err(|_| {
        AppError::new(
            "transcription_storage_unavailable",
            "ClipKeep could not create a temporary model folder.",
            true,
        )
    })?;
    let install_result = match model_id {
        SMALL_ID => download_small_model_files(&staging, &mut progress),
        WHISPER_ID | WHISPER_BASE_ID => download_whisper_files(&staging, model_id, &mut progress),
        _ => download_model_files(&staging, &mut progress),
    };
    if let Err(error) = install_result {
        let _ = fs::remove_dir_all(&staging);
        return Err(error);
    }
    if destination.exists() {
        fs::remove_dir_all(&destination).map_err(|_| {
            AppError::new(
                "transcription_model_replace_failed",
                "ClipKeep could not replace the existing incomplete model files.",
                true,
            )
        })?;
    }
    fs::rename(&staging, &destination).map_err(|_| {
        AppError::new(
            "transcription_model_install_failed",
            "ClipKeep could not activate the verified local model.",
            true,
        )
    })?;
    let status = model_status(root);
    if !status
        .models
        .iter()
        .any(|model| model.id == model_id && model.installed)
    {
        return Err(AppError::new(
            "transcription_model_verification_failed",
            "The model setup did not produce all required files.",
            true,
        ));
    }
    Ok(status)
}

fn download_model_files(
    staging: &Path,
    progress: &mut impl FnMut(ModelInstallProgress) -> Result<(), AppError>,
) -> Result<(), AppError> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(60 * 60))
        .redirect(Policy::custom(|attempt| {
            if attempt.url().scheme() == "https"
                && matches!(
                    attempt.url().host_str(),
                    Some("huggingface.co")
                        | Some("cdn-lfs.huggingface.co")
                        | Some("cas-bridge.xethub.hf.co")
                        | Some("us.aws.cdn.hf.co")
                )
            {
                attempt.follow()
            } else {
                attempt.stop()
            }
        }))
        .build()
        .map_err(|_| {
            AppError::new(
                "transcription_network_unavailable",
                "ClipKeep could not prepare a secure connection to the model source.",
                true,
            )
        })?;
    let pinned_hashes = [
        "a32b12d17bbbc309d0686fbbcc2987b5e9b8333a7da83fa6b089f0a2acd651ab",
        "b6bb64963457237b900e496ee9994b59294526439fbcc1fecf705b31a15c6b4e",
        "7946164367946e7f9f29a122407c3252b680dbae9a51343eb2488d057c3c43d2",
        "",
    ];
    for (index, filename) in MODEL_FILES.iter().enumerate() {
        progress(ModelInstallProgress {
            file: (*filename).into(),
            phase: "downloading".into(),
            percent: Some(0),
        })?;
        let url = format!("{MODEL_REPOSITORY}/{filename}");
        let mut response = client.get(url).send().map_err(|_| {
            AppError::new(
                "transcription_model_download_failed",
                format!("ClipKeep could not download the {filename} model file."),
                true,
            )
        })?;
        if !response.status().is_success() || response.url().scheme() != "https" {
            return Err(AppError::new(
                "transcription_model_download_failed",
                "The model file did not come from the approved HTTPS model source.",
                true,
            ));
        }
        let max_size = if index == 0 {
            700 * 1024 * 1024
        } else if index == 3 {
            64 * 1024
        } else {
            16 * 1024 * 1024
        };
        if response
            .content_length()
            .is_some_and(|length| length > max_size)
        {
            return Err(AppError::new(
                "transcription_model_file_too_large",
                "A model file exceeded its expected safe size limit.",
                false,
            ));
        }
        let path = staging.join(filename);
        let mut output = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|_| {
                AppError::new(
                    "transcription_storage_unavailable",
                    "ClipKeep could not create a temporary model file.",
                    true,
                )
            })?;
        let expected_length = response.content_length();
        let mut hasher = Sha256::new();
        let mut buffer = [0_u8; 64 * 1024];
        let mut total = 0_u64;
        let mut last_percent = 0_u8;
        loop {
            let count = response.read(&mut buffer).map_err(|_| {
                AppError::new(
                    "transcription_model_download_failed",
                    format!("The {filename} download was interrupted."),
                    true,
                )
            })?;
            if count == 0 {
                break;
            }
            total += count as u64;
            if total > max_size {
                return Err(AppError::new(
                    "transcription_model_file_too_large",
                    "A model file exceeded its expected safe size limit.",
                    false,
                ));
            }
            hasher.update(&buffer[..count]);
            output.write_all(&buffer[..count]).map_err(|_| {
                AppError::new(
                    "transcription_storage_unavailable",
                    "ClipKeep could not write a model file locally.",
                    true,
                )
            })?;
            if let Some(length) = expected_length.filter(|length| *length > 0) {
                let percent = (total.saturating_mul(100) / length).min(100) as u8;
                if percent != last_percent {
                    progress(ModelInstallProgress {
                        file: (*filename).into(),
                        phase: "downloading".into(),
                        percent: Some(percent),
                    })?;
                    last_percent = percent;
                }
            }
        }
        output.flush().map_err(|_| {
            AppError::new(
                "transcription_storage_unavailable",
                "ClipKeep could not finish writing a model file.",
                true,
            )
        })?;
        if expected_length.is_some_and(|length| length != total) {
            return Err(AppError::new(
                "transcription_model_download_incomplete",
                format!("The {filename} download was incomplete."),
                true,
            ));
        }
        let digest = format!("{:x}", hasher.finalize());
        if !pinned_hashes[index].is_empty() && digest != pinned_hashes[index] {
            return Err(AppError::new(
                "transcription_model_checksum_mismatch",
                format!("The {filename} file did not match its pinned SHA-256 checksum."),
                true,
            ));
        }
        if index == 3 {
            let tokens = fs::read_to_string(&path).map_err(|_| {
                AppError::new(
                    "transcription_model_invalid",
                    "The downloaded token dictionary is invalid.",
                    true,
                )
            })?;
            if !tokens.starts_with("<unk> 0\n") || tokens.lines().count() < 1000 {
                return Err(AppError::new(
                    "transcription_model_invalid",
                    "The downloaded token dictionary is invalid.",
                    true,
                ));
            }
        }
        progress(ModelInstallProgress {
            file: (*filename).into(),
            phase: "verified".into(),
            percent: Some(100),
        })?;
    }
    let mut marker = File::create(staging.join("model-source.txt")).map_err(|_| {
        AppError::new(
            "transcription_storage_unavailable",
            "ClipKeep could not record model attribution locally.",
            true,
        )
    })?;
    marker.write_all(format!("Model: NVIDIA Parakeet TDT 0.6B v2 INT8\nModel source: https://huggingface.co/nvidia/parakeet-tdt-0.6b-v2\nONNX conversion and package: csukuangfj / sherpa-onnx\nConverted model source: https://huggingface.co/csukuangfj/sherpa-onnx-nemo-parakeet-tdt-0.6b-v2-int8/tree/{MODEL_REVISION}\nLicense: CC BY 4.0 (https://creativecommons.org/licenses/by/4.0/)\n").as_bytes()).map_err(|_| AppError::new("transcription_storage_unavailable", "ClipKeep could not record model attribution locally.", true))?;
    Ok(())
}

fn download_small_model_files(
    staging: &Path,
    progress: &mut impl FnMut(ModelInstallProgress) -> Result<(), AppError>,
) -> Result<(), AppError> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(60 * 60))
        .redirect(Policy::custom(|attempt| {
            if attempt.url().scheme() == "https"
                && matches!(
                    attempt.url().host_str(),
                    Some("huggingface.co")
                        | Some("cdn-lfs.huggingface.co")
                        | Some("cas-bridge.xethub.hf.co")
                        | Some("us.aws.cdn.hf.co")
                )
            {
                attempt.follow()
            } else {
                attempt.stop()
            }
        }))
        .build()
        .map_err(|_| {
            AppError::new(
                "transcription_network_unavailable",
                "ClipKeep could not prepare a secure connection to the model source.",
                true,
            )
        })?;
    let files = [
        (
            "model.int8.onnx",
            "9177a9146cf32ee0cc8152276ef95116f312018d316be37ccf57f7efea81fc1a",
            131_652_171_u64,
        ),
        (
            "tokens.txt",
            "450e56bd2f036fe5b6aa821865838cc5aa9d8b0106134ce9a9ba0664abe6cd10",
            9_953_u64,
        ),
    ];
    for (filename, expected_hash, expected_size) in files {
        progress(ModelInstallProgress {
            file: filename.into(),
            phase: "downloading".into(),
            percent: Some(0),
        })?;
        let mut response = client
            .get(format!("{SMALL_REPOSITORY}/{filename}"))
            .send()
            .map_err(|_| {
                AppError::new(
                    "transcription_model_download_failed",
                    format!("ClipKeep could not download {filename}."),
                    true,
                )
            })?;
        if !response.status().is_success() || response.url().scheme() != "https" {
            return Err(AppError::new(
                "transcription_model_download_failed",
                "The model file did not come from the approved HTTPS model source.",
                true,
            ));
        }
        let path = staging.join(filename);
        let mut output = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|_| {
                AppError::new(
                    "transcription_storage_unavailable",
                    "ClipKeep could not create a temporary model file.",
                    true,
                )
            })?;
        let mut hasher = Sha256::new();
        let mut total = 0_u64;
        let mut buffer = [0_u8; 64 * 1024];
        let mut last_percent = 0;
        loop {
            let count = response.read(&mut buffer).map_err(|_| {
                AppError::new(
                    "transcription_model_download_failed",
                    "The model download was interrupted.",
                    true,
                )
            })?;
            if count == 0 {
                break;
            }
            total += count as u64;
            if total > expected_size {
                return Err(AppError::new(
                    "transcription_model_file_too_large",
                    "A model file exceeded its verified size.",
                    false,
                ));
            }
            hasher.update(&buffer[..count]);
            output.write_all(&buffer[..count]).map_err(|_| {
                AppError::new(
                    "transcription_storage_unavailable",
                    "ClipKeep could not write a model file locally.",
                    true,
                )
            })?;
            let percent = (total * 100 / expected_size).min(100) as u8;
            if percent != last_percent {
                progress(ModelInstallProgress {
                    file: filename.into(),
                    phase: "downloading".into(),
                    percent: Some(percent),
                })?;
                last_percent = percent;
            }
        }
        output.flush().map_err(|_| {
            AppError::new(
                "transcription_storage_unavailable",
                "ClipKeep could not finish writing a model file.",
                true,
            )
        })?;
        if total != expected_size || format!("{:x}", hasher.finalize()) != expected_hash {
            return Err(AppError::new(
                "transcription_model_checksum_mismatch",
                "The downloaded model file failed its size or SHA-256 check.",
                true,
            ));
        }
        progress(ModelInstallProgress {
            file: filename.into(),
            phase: "verified".into(),
            percent: Some(100),
        })?;
    }
    fs::write(staging.join("model-source.txt"), "NVIDIA Parakeet TDT-CTC 110M English, quantized ONNX package by VocaHQ / sherpa-onnx. License: CC BY 4.0 https://creativecommons.org/licenses/by/4.0/\nSource: https://huggingface.co/VocaHQ/sherpa-onnx-nemo-parakeet-tdt-ctc-110m-en-int8\n").map_err(|_| AppError::new("transcription_storage_unavailable", "ClipKeep could not record model attribution locally.", true))?;
    Ok(())
}

fn download_whisper_files(
    staging: &Path,
    model_id: &str,
    progress: &mut impl FnMut(ModelInstallProgress) -> Result<(), AppError>,
) -> Result<(), AppError> {
    let (repository, files) = if model_id == WHISPER_BASE_ID {
        (
            WHISPER_BASE_REPOSITORY,
            [
                (
                    "base.en-encoder.int8.onnx",
                    "ef6b936f4c9b1d90a3b68634b60c4ed8576b26172b33c2535ec0e933c9edb823",
                    29_120_534_u64,
                ),
                (
                    "base.en-decoder.int8.onnx",
                    "f7162ad6db2dbef16cfaeaa7f945b9d7dd9c1b8d472f6aca82f2273d185e4d41",
                    130_669_978_u64,
                ),
                ("base.en-tokens.txt", "", 2_000_000_u64),
            ],
        )
    } else {
        (
            WHISPER_REPOSITORY,
            [
                (
                    "tiny.en-encoder.int8.onnx",
                    "0ce578b827c94a961aacb8fa14b02f096504b337e5c94be37c36238cbe3e8bc6",
                    12_937_772_u64,
                ),
                (
                    "tiny.en-decoder.int8.onnx",
                    "06c0e6ff6348d427e51839219d1c886c18cfdf411e629e33f5e1679bff9c1527",
                    89_853_865_u64,
                ),
                ("tiny.en-tokens.txt", "", 2_000_000_u64),
            ],
        )
    };
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(60 * 60))
        .redirect(Policy::custom(|attempt| {
            if attempt.url().scheme() == "https"
                && matches!(
                    attempt.url().host_str(),
                    Some("huggingface.co")
                        | Some("cdn-lfs.huggingface.co")
                        | Some("cas-bridge.xethub.hf.co")
                        | Some("us.aws.cdn.hf.co")
                )
            {
                attempt.follow()
            } else {
                attempt.stop()
            }
        }))
        .build()
        .map_err(|_| {
            AppError::new(
                "transcription_network_unavailable",
                "ClipKeep could not prepare a secure connection to the Whisper model source.",
                true,
            )
        })?;
    for (filename, expected_hash, expected_size) in files {
        progress(ModelInstallProgress {
            file: filename.into(),
            phase: "downloading".into(),
            percent: Some(0),
        })?;
        let mut response = client
            .get(format!("{repository}/{filename}"))
            .send()
            .map_err(|_| {
                AppError::new(
                    "transcription_model_download_failed",
                    format!("ClipKeep could not download {filename}."),
                    true,
                )
            })?;
        if !response.status().is_success() || response.url().scheme() != "https" {
            return Err(AppError::new(
                "transcription_model_download_failed",
                "The Whisper model file did not come from the approved HTTPS source.",
                true,
            ));
        }
        let path = staging.join(filename);
        let mut output = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|_| {
                AppError::new(
                    "transcription_storage_unavailable",
                    "ClipKeep could not create a temporary Whisper model file.",
                    true,
                )
            })?;
        let mut hasher = Sha256::new();
        let mut total = 0_u64;
        let mut buffer = [0_u8; 64 * 1024];
        let mut last_percent = 0_u8;
        loop {
            let count = response.read(&mut buffer).map_err(|_| {
                AppError::new(
                    "transcription_model_download_failed",
                    "The Whisper model download was interrupted.",
                    true,
                )
            })?;
            if count == 0 {
                break;
            }
            total += count as u64;
            if total > expected_size {
                return Err(AppError::new(
                    "transcription_model_file_too_large",
                    "A Whisper model file exceeded its safe size limit.",
                    false,
                ));
            }
            hasher.update(&buffer[..count]);
            output.write_all(&buffer[..count]).map_err(|_| {
                AppError::new(
                    "transcription_storage_unavailable",
                    "ClipKeep could not write a Whisper model file locally.",
                    true,
                )
            })?;
            let percent = (total.saturating_mul(100) / expected_size).min(100) as u8;
            if percent != last_percent {
                progress(ModelInstallProgress {
                    file: filename.into(),
                    phase: "downloading".into(),
                    percent: Some(percent),
                })?;
                last_percent = percent;
            }
        }
        output.flush().map_err(|_| {
            AppError::new(
                "transcription_storage_unavailable",
                "ClipKeep could not finish writing a Whisper model file.",
                true,
            )
        })?;
        if total == 0
            || (expected_hash.is_empty() && total > 1_000_000)
            || (!expected_hash.is_empty()
                && (total != expected_size || format!("{:x}", hasher.finalize()) != expected_hash))
        {
            return Err(AppError::new(
                "transcription_model_checksum_mismatch",
                "A Whisper model file failed its pinned size or SHA-256 check.",
                true,
            ));
        }
        if filename.ends_with("tokens.txt") {
            let tokens = fs::read_to_string(&path).map_err(|_| {
                AppError::new(
                    "transcription_model_invalid",
                    "The Whisper token dictionary is invalid.",
                    true,
                )
            })?;
            if tokens.lines().count() < 50_000 {
                return Err(AppError::new(
                    "transcription_model_invalid",
                    "The Whisper token dictionary is incomplete.",
                    true,
                ));
            }
        }
        progress(ModelInstallProgress {
            file: filename.into(),
            phase: "verified".into(),
            percent: Some(100),
        })?;
    }
    let (name, source) = if model_id == WHISPER_BASE_ID {
        (
            "Whisper base.en",
            "https://huggingface.co/csukuangfj/sherpa-onnx-whisper-base.en",
        )
    } else {
        (
            "Whisper tiny.en",
            "https://huggingface.co/csukuangfj/sherpa-onnx-whisper-tiny.en",
        )
    };
    fs::write(staging.join("model-source.txt"), format!("OpenAI {name} exported and INT8 quantized for sherpa-onnx. Model license: MIT. Source: {source}\n"))
        .map_err(|_| AppError::new("transcription_storage_unavailable", "ClipKeep could not record Whisper model attribution locally.", true))?;
    Ok(())
}

fn transcribe(
    root: &Path,
    ffmpeg: &Path,
    source: PathBuf,
    model_id: &str,
) -> Result<TranscriptResult, AppError> {
    let _run_guard = match TRANSCRIPTION_LOCK.try_lock() {
        Ok(guard) => guard,
        Err(TryLockError::WouldBlock) => {
            return Err(AppError::new(
                "transcription_busy",
                "Another transcription is already running. Wait for it to finish before starting another.",
                false,
            ));
        }
        Err(_) => {
            return Err(AppError::new(
                "transcription_busy",
                "Transcription is temporarily unavailable.",
                true,
            ));
        }
    };
    let started = Instant::now();
    if !source.is_file() {
        return Err(AppError::new(
            "transcription_source_missing",
            "Choose an existing audio or video file.",
            false,
        ));
    }
    let status = model_status(root);
    let selected = status
        .models
        .iter()
        .find(|model| model.id == model_id)
        .ok_or_else(|| {
            AppError::new(
                "transcription_model_unknown",
                "Choose a transcription model from the available list.",
                false,
            )
        })?;
    if !selected.can_run_now {
        return Err(AppError::new(
            "transcription_memory_insufficient",
            format!(
                "{} needs more free memory. This device currently has about {} MB available. Close other apps and refresh this page, or choose another model.",
                selected.name, status.available_ram_mb
            ),
            false,
        ));
    }
    if !selected.installed {
        return Err(AppError::new(
            "transcription_model_missing",
            format!(
                "The selected {} model is not installed. Install it from this page.",
                selected.name
            ),
            true,
        ));
    }

    let temp_dir = root.join("temp");
    fs::create_dir_all(&temp_dir).map_err(|_| {
        AppError::new(
            "transcription_storage_unavailable",
            "ClipKeep could not prepare local temporary audio storage.",
            true,
        )
    })?;
    let wav_path = temp_dir.join(format!("{}.wav", Uuid::new_v4()));
    let _cleanup = TempFile(wav_path.clone());
    let conversion = Command::new(ffmpeg)
        .args(["-nostdin", "-v", "error", "-y", "-i"])
        .arg(&source)
        .args([
            "-map",
            "0:a:0",
            "-ac",
            "1",
            "-ar",
            "16000",
            "-c:a",
            "pcm_s16le",
            "-f",
            "wav",
        ])
        .arg(&wav_path)
        .output()
        .map_err(|_| {
            AppError::new(
                "transcription_audio_decode_failed",
                "ClipKeep could not start the local audio decoder. Check Settings for media tools.",
                true,
            )
        })?;
    if !conversion.status.success() {
        return Err(AppError::new(
            "transcription_audio_decode_failed",
            "ClipKeep could not read an audio track from this file.",
            true,
        ));
    }

    let wav_size_mb = fs::metadata(&wav_path)
        .map_err(|_| {
            AppError::new(
                "transcription_audio_invalid",
                "ClipKeep could not inspect the prepared audio file.",
                true,
            )
        })?
        .len()
        .div_ceil(1_048_576);
    let required_before_decode_mb = selected
        .minimum_available_ram_mb
        .saturating_add(wav_size_mb.saturating_mul(2));
    let (_, free_before_decode_mb) = system_memory_mb();
    if free_before_decode_mb < required_before_decode_mb {
        return Err(AppError::new(
            "transcription_memory_insufficient",
            format!(
                "This audio needs about {} MB of available memory to transcribe safely; this device currently has {} MB free. Close other programs or choose a shorter file.",
                required_before_decode_mb, free_before_decode_mb
            ),
            false,
        ));
    }

    let wav_name = wav_path.to_string_lossy();
    let wave = Wave::read(&wav_name).ok_or_else(|| {
        AppError::new(
            "transcription_audio_invalid",
            "The decoded audio could not be prepared for transcription.",
            true,
        )
    })?;
    let duration_seconds = wave.samples().len() as f64 / wave.sample_rate() as f64;
    if duration_seconds <= 0.0 {
        return Err(AppError::new(
            "transcription_audio_empty",
            "This file does not contain usable audio.",
            false,
        ));
    }

    let (_, free_after_decode_mb) = system_memory_mb();
    if free_after_decode_mb < selected.minimum_available_ram_mb {
        return Err(AppError::new(
            "transcription_memory_insufficient",
            format!(
                "Only {} MB remains free after preparing the audio. This model needs at least {} MB before it can load safely.",
                free_after_decode_mb, selected.minimum_available_ram_mb
            ),
            false,
        ));
    }

    let model = model_directory(root, model_id)?;
    let mut config = OfflineRecognizerConfig::default();
    if model_id == SMALL_ID {
        config.model_config.nemo_ctc = OfflineNemoEncDecCtcModelConfig {
            model: Some(model.join("model.int8.onnx").to_string_lossy().into_owned()),
        };
        config.model_config.model_type = Some("nemo_ctc".into());
    } else if model_id == WHISPER_ID || model_id == WHISPER_BASE_ID {
        let prefix = if model_id == WHISPER_BASE_ID {
            "base.en"
        } else {
            "tiny.en"
        };
        config.model_config.whisper = OfflineWhisperModelConfig {
            encoder: Some(
                model
                    .join(format!("{prefix}-encoder.int8.onnx"))
                    .to_string_lossy()
                    .into_owned(),
            ),
            decoder: Some(
                model
                    .join(format!("{prefix}-decoder.int8.onnx"))
                    .to_string_lossy()
                    .into_owned(),
            ),
            language: Some("en".into()),
            task: Some("transcribe".into()),
            ..Default::default()
        };
        config.model_config.model_type = Some("whisper".into());
    } else {
        config.model_config.transducer = OfflineTransducerModelConfig {
            encoder: Some(model.join(MODEL_FILES[0]).to_string_lossy().into_owned()),
            decoder: Some(model.join(MODEL_FILES[1]).to_string_lossy().into_owned()),
            joiner: Some(model.join(MODEL_FILES[2]).to_string_lossy().into_owned()),
        };
        config.model_config.model_type = Some("nemo_transducer".into());
    }
    config.model_config.tokens = Some(
        model
            .join(if model_id == SMALL_ID {
                "tokens.txt"
            } else if model_id == WHISPER_ID {
                "tiny.en-tokens.txt"
            } else if model_id == WHISPER_BASE_ID {
                "base.en-tokens.txt"
            } else {
                MODEL_FILES[3]
            })
            .to_string_lossy()
            .into_owned(),
    );
    config.model_config.num_threads = 1;

    let recognizer = OfflineRecognizer::create(&config).ok_or_else(|| {
        AppError::new(
            "transcription_model_invalid",
            "ClipKeep could not load the selected transcription model. Confirm available memory and retry with the recommended model.",
            true,
        )
    })?;
    let stream = recognizer.create_stream();
    stream.accept_waveform(wave.sample_rate(), wave.samples());
    recognizer.decode(&stream);
    let result = stream.get_result().ok_or_else(|| {
        AppError::new(
            "transcription_failed",
            "The transcription engine did not return a result.",
            true,
        )
    })?;
    let title = source
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or("Transcript")
        .to_string();
    Ok(TranscriptResult {
        source_path: source.to_string_lossy().into_owned(),
        title,
        text: result.text,
        duration_seconds,
        elapsed_seconds: started.elapsed().as_secs_f64(),
        model_name: selected.name.clone(),
    })
}

struct TempFile(PathBuf);

impl Drop for TempFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}
