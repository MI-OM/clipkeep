use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Seek, Write},
    path::{Path, PathBuf},
    sync::Mutex,
    time::Duration,
};

use reqwest::{
    blocking::{Client, Response},
    redirect::Policy,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;
use zip::ZipArchive;

use super::{
    tools::{MediaTools, ToolResolver},
    types::AppError,
};

const BUNDLE_ID: &str = "yt-dlp-2026.10.21-deno-2.9.8-ffmpeg-9.0.2";
const MAX_DOWNLOAD_BYTES: u64 = 512 * 1024 * 1024;
const MAX_EXECUTABLE_BYTES: u64 = 256 * 1024 * 1024;
const DOWNLOAD_CHUNK_SIZE: usize = 64 * 1024;

const YT_DLP_URL: &str = "https://github.com/yt-dlp/yt-dlp/releases/download/2026.10.21/yt-dlp.exe";
const YT_DLP_SHA256: &str = "33121d192d706e62f684a06f80918e3f3a8ba52ac3a65213b6951c39c7d9f555";
const DENO_URL: &str =
    "https://github.com/denoland/deno/releases/download/v2.9.8/deno-x86_64-pc-windows-msvc.zip";
const DENO_SHA256: &str = "cba84b26e0df5ae7f74fc66b86a95b866872a86292ebd84fa1e959927edd93bf";
const DENO_BINARY_SHA256: &str = "924397800533d3eb0ed0c889b20ad70e18b970d93acd8135bc043f425e528266";
const FFMPEG_URL: &str =
    "https://www.gyan.dev/ffmpeg/builds/packages/ffmpeg-9.0.2-essentials_build.zip";
const FFMPEG_SHA256: &str = "60f467265b1e312373dbcd92200c2618a74850f98d3d078e94296bb3fa2047ba";
const FFMPEG_BINARY_SHA256: &str =
    "3256173f3f8bffd7df12227c68adf68025edb1832273a9530688a7bb1ed8edec";
const FFPROBE_BINARY_SHA256: &str =
    "f0d36ecbbdd3bcfac3efa078c96c7271c2e68b3810595552ac3b7f17e9a65c52";
const DENO_ARCHIVE_ENTRY: &str = "deno.exe";
const FFMPEG_ARCHIVE_ROOT: &str = "ffmpeg-9.0.2-essentials_build/bin/";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DependencyToolStatus {
    pub name: String,
    pub version: String,
    pub state: DependencyToolState,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DependencyToolState {
    Missing,
    Verified,
    Invalid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DependencyStatus {
    pub available: bool,
    pub tools: Vec<DependencyToolStatus>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DependencyProgress {
    pub tool: String,
    pub phase: String,
    pub percent: Option<u8>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InstalledManifest {
    bundle_id: String,
    yt_dlp_sha256: String,
    deno_sha256: String,
    ffmpeg_sha256: String,
    ffprobe_sha256: String,
}

pub struct DependencyManager {
    root: PathBuf,
    install_lock: Mutex<()>,
}

impl DependencyManager {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            install_lock: Mutex::new(()),
        }
    }

    pub fn status(&self) -> Result<DependencyStatus, AppError> {
        let expected = expected_paths(&self.root);
        let manifest_path = self.root.join(BUNDLE_ID).join("installed.json");
        let manifest = match fs::read(&manifest_path) {
            Ok(bytes) => serde_json::from_slice::<InstalledManifest>(&bytes).ok(),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(_) => {
                return Err(AppError::new(
                    "dependency_status_failed",
                    "ClipKeep could not read its installed media-tool records.",
                    true,
                ));
            }
        };

        let manifest_matches = manifest.as_ref().is_some_and(|manifest| {
            manifest.bundle_id == BUNDLE_ID
                && manifest.yt_dlp_sha256 == YT_DLP_SHA256
                && manifest.deno_sha256 == DENO_BINARY_SHA256
                && manifest.ffmpeg_sha256 == FFMPEG_BINARY_SHA256
                && manifest.ffprobe_sha256 == FFPROBE_BINARY_SHA256
        });

        let files = [
            ("yt-dlp", "2026.10.21", &expected.yt_dlp, YT_DLP_SHA256),
            ("Deno", "2.9.8", &expected.deno, DENO_BINARY_SHA256),
            ("FFmpeg", "9.0.2", &expected.ffmpeg, FFMPEG_BINARY_SHA256),
            ("ffprobe", "9.0.2", &expected.ffprobe, FFPROBE_BINARY_SHA256),
        ];
        let mut tools = Vec::with_capacity(files.len());
        for (name, version, path, digest) in files {
            let state = if !manifest_matches {
                if path.exists() {
                    DependencyToolState::Invalid
                } else {
                    DependencyToolState::Missing
                }
            } else {
                match sha256_file(path) {
                    Ok(actual) if actual == digest => match verify_pe_x64(path) {
                        Ok(()) => DependencyToolState::Verified,
                        Err(_) => DependencyToolState::Invalid,
                    },
                    Ok(_) => DependencyToolState::Invalid,
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                        DependencyToolState::Missing
                    }
                    Err(_) => {
                        return Err(AppError::new(
                            "dependency_status_failed",
                            "ClipKeep could not verify an installed media tool.",
                            true,
                        ));
                    }
                }
            };
            tools.push(DependencyToolStatus {
                name: name.to_string(),
                version: version.to_string(),
                state,
            });
        }
        Ok(DependencyStatus {
            available: tools
                .iter()
                .all(|tool| tool.state == DependencyToolState::Verified),
            tools,
        })
    }

    pub fn install(
        &self,
        mut progress: impl FnMut(DependencyProgress) -> Result<(), AppError>,
    ) -> Result<DependencyStatus, AppError> {
        let _guard = self.install_lock.lock().map_err(|_| {
            AppError::new(
                "dependency_install_busy",
                "ClipKeep could not safely start media-tool setup.",
                true,
            )
        })?;
        if !cfg!(all(target_os = "windows", target_arch = "x86_64")) {
            return Err(AppError::new(
                "dependency_platform_unsupported",
                "Managed media tools are currently available only for 64-bit Windows.",
                false,
            ));
        }

        fs::create_dir_all(&self.root).map_err(|_| {
            AppError::new(
                "dependency_storage_unavailable",
                "ClipKeep could not create its local media-tool folder.",
                true,
            )
        })?;
        self.cleanup_stale_staging()?;
        let current = self.status()?;
        if current.available {
            return Ok(current);
        }
        let staging = self.root.join(format!(".staging-{}", Uuid::new_v4()));
        fs::create_dir(&staging).map_err(|_| {
            AppError::new(
                "dependency_storage_unavailable",
                "ClipKeep could not prepare a safe staging folder for media tools.",
                true,
            )
        })?;

        let result = self.install_staged(&staging, &mut progress);
        if result.is_err() {
            if let Err(cleanup_error) = fs::remove_dir_all(&staging) {
                return Err(AppError::new(
                    "dependency_cleanup_failed",
                    format!(
                        "Media-tool setup failed, and its temporary files could not be removed: {cleanup_error}"
                    ),
                    true,
                ));
            }
        }
        result?;
        let status = self.status()?;
        if !status.available {
            return Err(AppError::new(
                "dependency_verification_failed",
                "Media-tool setup finished, but the installed files did not pass verification.",
                true,
            ));
        }
        Ok(status)
    }

    fn cleanup_stale_staging(&self) -> Result<(), AppError> {
        let entries = fs::read_dir(&self.root).map_err(|_| {
            AppError::new(
                "dependency_storage_unavailable",
                "ClipKeep could not inspect its local media-tool folder.",
                true,
            )
        })?;
        for entry in entries {
            let entry = entry.map_err(|_| {
                AppError::new(
                    "dependency_storage_unavailable",
                    "ClipKeep could not inspect a temporary media-tool folder.",
                    true,
                )
            })?;
            let file_type = entry.file_type().map_err(|_| {
                AppError::new(
                    "dependency_storage_unavailable",
                    "ClipKeep could not inspect a temporary media-tool folder.",
                    true,
                )
            })?;
            let is_stale_staging =
                entry.file_name().to_string_lossy().starts_with(".staging-") && file_type.is_dir();
            if is_stale_staging {
                fs::remove_dir_all(entry.path()).map_err(|_| {
                    AppError::new(
                        "dependency_cleanup_failed",
                        "ClipKeep could not remove files left by an interrupted media-tool setup.",
                        true,
                    )
                })?;
            }
        }
        Ok(())
    }

    fn install_staged(
        &self,
        staging: &Path,
        progress: &mut impl FnMut(DependencyProgress) -> Result<(), AppError>,
    ) -> Result<(), AppError> {
        let yt_dlp_archive = staging.join("yt-dlp.exe");
        let deno_archive = staging.join("deno.zip");
        let ffmpeg_archive = staging.join("ffmpeg.zip");
        let client = build_http_client()?;

        download_verified(
            &client,
            YT_DLP_URL,
            YT_DLP_SHA256,
            &yt_dlp_archive,
            "yt-dlp",
            progress,
        )?;
        download_verified(
            &client,
            DENO_URL,
            DENO_SHA256,
            &deno_archive,
            "Deno",
            progress,
        )?;
        download_verified(
            &client,
            FFMPEG_URL,
            FFMPEG_SHA256,
            &ffmpeg_archive,
            "FFmpeg",
            progress,
        )?;

        let binaries = staging.join("bin");
        fs::create_dir(&binaries).map_err(|_| {
            AppError::new(
                "dependency_storage_unavailable",
                "ClipKeep could not prepare the verified media-tool files.",
                true,
            )
        })?;

        progress(DependencyProgress {
            tool: "yt-dlp".into(),
            phase: "installing".into(),
            percent: None,
        })?;
        fs::rename(&yt_dlp_archive, binaries.join("yt-dlp.exe")).map_err(|_| {
            AppError::new(
                "dependency_install_failed",
                "ClipKeep could not stage the verified yt-dlp executable.",
                true,
            )
        })?;
        extract_verified_entry(
            &deno_archive,
            DENO_ARCHIVE_ENTRY,
            &binaries.join("deno.exe"),
            "Deno",
            progress,
        )?;
        extract_verified_entry(
            &ffmpeg_archive,
            &format!("{FFMPEG_ARCHIVE_ROOT}ffmpeg.exe"),
            &binaries.join("ffmpeg.exe"),
            "FFmpeg",
            progress,
        )?;
        extract_verified_entry(
            &ffmpeg_archive,
            &format!("{FFMPEG_ARCHIVE_ROOT}ffprobe.exe"),
            &binaries.join("ffprobe.exe"),
            "ffprobe",
            progress,
        )?;

        for archive in [&deno_archive, &ffmpeg_archive] {
            fs::remove_file(archive).map_err(|_| {
                AppError::new(
                    "dependency_cleanup_failed",
                    "ClipKeep could not remove a verified dependency archive after extraction.",
                    true,
                )
            })?;
        }

        for name in ["yt-dlp.exe", "deno.exe", "ffmpeg.exe", "ffprobe.exe"] {
            verify_pe_x64(&binaries.join(name)).map_err(|_| {
                AppError::new(
                    "dependency_architecture_invalid",
                    format!(
                        "The downloaded {name} executable is not a valid 64-bit Windows program."
                    ),
                    true,
                )
            })?;
        }

        for (name, expected_digest) in [
            ("yt-dlp.exe", YT_DLP_SHA256),
            ("deno.exe", DENO_BINARY_SHA256),
            ("ffmpeg.exe", FFMPEG_BINARY_SHA256),
            ("ffprobe.exe", FFPROBE_BINARY_SHA256),
        ] {
            let actual_digest = sha256_file(&binaries.join(name)).map_err(|_| {
                AppError::new(
                    "dependency_install_failed",
                    format!("ClipKeep could not verify {name} after extraction."),
                    true,
                )
            })?;
            if actual_digest != expected_digest {
                return Err(AppError::new(
                    "dependency_checksum_mismatch",
                    format!("The extracted {name} does not match its pinned SHA-256 checksum."),
                    true,
                ));
            }
        }

        let manifest = InstalledManifest {
            bundle_id: BUNDLE_ID.to_string(),
            yt_dlp_sha256: YT_DLP_SHA256.to_string(),
            deno_sha256: DENO_BINARY_SHA256.to_string(),
            ffmpeg_sha256: FFMPEG_BINARY_SHA256.to_string(),
            ffprobe_sha256: FFPROBE_BINARY_SHA256.to_string(),
        };
        let manifest_bytes = serde_json::to_vec(&manifest).map_err(|_| {
            AppError::new(
                "dependency_install_failed",
                "ClipKeep could not record the verified media-tool versions.",
                true,
            )
        })?;
        write_new_file(&staging.join("installed.json"), &manifest_bytes)?;

        let installed = self.root.join(BUNDLE_ID);
        if installed.exists() {
            let quarantine = self.root.join(format!(".invalid-{}", Uuid::new_v4()));
            fs::rename(&installed, quarantine).map_err(|_| {
                AppError::new(
                    "dependency_repair_failed",
                    "ClipKeep could not safely preserve the invalid installation before repair.",
                    true,
                )
            })?;
        }
        fs::rename(staging, installed).map_err(|_| {
            AppError::new(
                "dependency_install_failed",
                "ClipKeep could not atomically activate the verified media tools.",
                true,
            )
        })?;
        Ok(())
    }
}

impl ToolResolver for DependencyManager {
    fn resolve(&self) -> Result<MediaTools, AppError> {
        let status = self.status()?;
        if !status.available {
            return Err(AppError::new(
                "dependency_not_ready",
                "Verified media tools are not installed. Open Settings to install or repair them.",
                true,
            ));
        }
        let paths = expected_paths(&self.root);
        Ok(MediaTools {
            yt_dlp: paths.yt_dlp,
            deno: paths.deno,
            ffmpeg: paths.ffmpeg,
            ffprobe: paths.ffprobe,
        })
    }
}

struct ToolPaths {
    yt_dlp: PathBuf,
    deno: PathBuf,
    ffmpeg: PathBuf,
    ffprobe: PathBuf,
}

fn expected_paths(root: &Path) -> ToolPaths {
    let bin = root.join(BUNDLE_ID).join("bin");
    ToolPaths {
        yt_dlp: bin.join("yt-dlp.exe"),
        deno: bin.join("deno.exe"),
        ffmpeg: bin.join("ffmpeg.exe"),
        ffprobe: bin.join("ffprobe.exe"),
    }
}

fn build_http_client() -> Result<Client, AppError> {
    Client::builder()
        .timeout(Duration::from_secs(15 * 60))
        .redirect(Policy::custom(|attempt| {
            match attempt.url().host_str() {
                Some("github.com")
                | Some("release-assets.githubusercontent.com")
                | Some("objects.githubusercontent.com")
                | Some("www.gyan.dev") => attempt.follow(),
                _ => attempt.stop(),
            }
        }))
        .build()
        .map_err(|_| {
            AppError::new(
                "dependency_network_unavailable",
                "ClipKeep could not prepare a secure connection to the approved dependency sources.",
                true,
            )
        })
}

fn download_verified(
    client: &Client,
    url: &str,
    expected_sha256: &str,
    destination: &Path,
    tool: &str,
    progress: &mut impl FnMut(DependencyProgress) -> Result<(), AppError>,
) -> Result<(), AppError> {
    progress(DependencyProgress {
        tool: tool.into(),
        phase: "downloading".into(),
        percent: Some(0),
    })?;
    let mut response = client.get(url).send().map_err(|_| {
        AppError::new(
            "dependency_download_failed",
            format!("ClipKeep could not download {tool} from its pinned release source."),
            true,
        )
    })?;
    if !response.status().is_success() || !approved_final_host(&response) {
        return Err(AppError::new(
            "dependency_download_failed",
            format!("The {tool} download did not come from an approved HTTPS release host."),
            true,
        ));
    }
    if response
        .content_length()
        .is_some_and(|length| length > MAX_DOWNLOAD_BYTES)
    {
        return Err(AppError::new(
            "dependency_download_too_large",
            format!("The {tool} download exceeds ClipKeep's safe size limit."),
            false,
        ));
    }

    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)
        .map_err(|_| {
            AppError::new(
                "dependency_storage_unavailable",
                "ClipKeep could not create a temporary file for a media-tool download.",
                true,
            )
        })?;
    let digest = copy_bounded(&mut response, &mut file, tool, progress)?;
    file.flush().map_err(|_| {
        AppError::new(
            "dependency_storage_unavailable",
            "ClipKeep could not finish writing a media-tool download.",
            true,
        )
    })?;
    if !matches_sha256(&digest, expected_sha256) {
        return Err(AppError::new(
            "dependency_checksum_mismatch",
            format!("The downloaded {tool} did not match its pinned SHA-256 checksum."),
            true,
        ));
    }
    progress(DependencyProgress {
        tool: tool.into(),
        phase: "verified".into(),
        percent: Some(100),
    })?;
    Ok(())
}

fn copy_bounded(
    response: &mut Response,
    output: &mut File,
    tool: &str,
    progress: &mut impl FnMut(DependencyProgress) -> Result<(), AppError>,
) -> Result<String, AppError> {
    let expected_length = response.content_length();
    let mut hasher = Sha256::new();
    let mut total = 0_u64;
    let mut last_reported_percent = None;
    let mut buffer = [0_u8; DOWNLOAD_CHUNK_SIZE];
    loop {
        let count = response.read(&mut buffer).map_err(|_| {
            AppError::new(
                "dependency_download_failed",
                format!("The {tool} download was interrupted."),
                true,
            )
        })?;
        if count == 0 {
            break;
        }
        total += count as u64;
        if total > MAX_DOWNLOAD_BYTES {
            return Err(AppError::new(
                "dependency_download_too_large",
                format!("The {tool} download exceeds ClipKeep's safe size limit."),
                false,
            ));
        }
        hasher.update(&buffer[..count]);
        output.write_all(&buffer[..count]).map_err(|_| {
            AppError::new(
                "dependency_storage_unavailable",
                "ClipKeep could not write a media-tool download to local storage.",
                true,
            )
        })?;
        if let Some(length) = expected_length.filter(|length| *length > 0) {
            let percent = ((total.saturating_mul(100) / length).min(100)) as u8;
            if last_reported_percent != Some(percent) {
                progress(DependencyProgress {
                    tool: tool.into(),
                    phase: "downloading".into(),
                    percent: Some(percent),
                })?;
                last_reported_percent = Some(percent);
            }
        }
    }
    if expected_length.is_some_and(|length| length != total) {
        return Err(AppError::new(
            "dependency_download_incomplete",
            format!("The {tool} download was incomplete."),
            true,
        ));
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn approved_final_host(response: &Response) -> bool {
    response.url().scheme() == "https"
        && matches!(
            response.url().host_str(),
            Some("github.com")
                | Some("release-assets.githubusercontent.com")
                | Some("objects.githubusercontent.com")
                | Some("www.gyan.dev")
        )
}

fn extract_verified_entry(
    archive_path: &Path,
    entry_name: &str,
    destination: &Path,
    tool: &str,
    progress: &mut impl FnMut(DependencyProgress) -> Result<(), AppError>,
) -> Result<(), AppError> {
    progress(DependencyProgress {
        tool: tool.into(),
        phase: "installing".into(),
        percent: None,
    })?;
    let archive_file = File::open(archive_path).map_err(|_| {
        AppError::new(
            "dependency_archive_invalid",
            format!("ClipKeep could not open the verified {tool} archive."),
            true,
        )
    })?;
    let mut archive = ZipArchive::new(archive_file).map_err(|_| {
        AppError::new(
            "dependency_archive_invalid",
            format!("The verified {tool} archive is corrupt or unsupported."),
            true,
        )
    })?;
    let entry = archive.by_name(entry_name).map_err(|_| {
        AppError::new(
            "dependency_archive_invalid",
            format!("The verified {tool} archive does not contain the expected executable."),
            true,
        )
    })?;
    if entry.size() == 0 || entry.size() > MAX_EXECUTABLE_BYTES {
        return Err(AppError::new(
            "dependency_archive_invalid",
            format!("The {tool} executable has an invalid size."),
            true,
        ));
    }
    let entry_size = entry.size();
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)
        .map_err(|_| {
            AppError::new(
                "dependency_storage_unavailable",
                "ClipKeep could not create a media-tool executable during installation.",
                true,
            )
        })?;
    let copied =
        std::io::copy(&mut entry.take(MAX_EXECUTABLE_BYTES + 1), &mut output).map_err(|_| {
            AppError::new(
                "dependency_archive_invalid",
                format!("ClipKeep could not safely extract {tool} from its verified archive."),
                true,
            )
        })?;
    output.flush().map_err(|_| {
        AppError::new(
            "dependency_storage_unavailable",
            "ClipKeep could not finish writing an extracted media tool.",
            true,
        )
    })?;
    if copied != entry_size {
        return Err(AppError::new(
            "dependency_archive_invalid",
            format!("The extracted {tool} executable did not match its archive entry size."),
            true,
        ));
    }
    Ok(())
}

fn verify_pe_x64(path: &Path) -> std::io::Result<()> {
    let mut file = File::open(path)?;
    let length = file.metadata()?.len();
    if length < 64 || length > MAX_EXECUTABLE_BYTES {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "invalid executable length",
        ));
    }
    let mut dos_header = [0_u8; 64];
    file.read_exact(&mut dos_header)?;
    if &dos_header[..2] != b"MZ" {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "invalid DOS signature",
        ));
    }
    let pe_offset = u32::from_le_bytes(dos_header[60..64].try_into().unwrap()) as u64;
    if pe_offset + 6 > length || pe_offset > 1024 * 1024 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "invalid PE header offset",
        ));
    }
    file.seek(std::io::SeekFrom::Start(pe_offset))?;
    let mut signature_and_machine = [0_u8; 6];
    file.read_exact(&mut signature_and_machine)?;
    if &signature_and_machine[..4] != b"PE\0\0"
        || u16::from_le_bytes([signature_and_machine[4], signature_and_machine[5]]) != 0x8664
    {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "not an x64 Windows PE executable",
        ));
    }
    Ok(())
}

fn sha256_file(path: &Path) -> std::io::Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; DOWNLOAD_CHUNK_SIZE];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn matches_sha256(actual: &str, expected: &str) -> bool {
    actual == expected
}

fn write_new_file(path: &Path, bytes: &[u8]) -> Result<(), AppError> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|_| {
            AppError::new(
                "dependency_storage_unavailable",
                "ClipKeep could not save its verified media-tool record.",
                true,
            )
        })?;
    file.write_all(bytes).map_err(|_| {
        AppError::new(
            "dependency_storage_unavailable",
            "ClipKeep could not save its verified media-tool record.",
            true,
        )
    })?;
    file.sync_all().map_err(|_| {
        AppError::new(
            "dependency_storage_unavailable",
            "ClipKeep could not securely finish its verified media-tool record.",
            true,
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verifies_only_valid_x64_pe_headers() {
        let directory = std::env::temp_dir().join(format!("clipkeep-pe-{}", Uuid::new_v4()));
        fs::create_dir(&directory).unwrap();
        let path = directory.join("valid.exe");
        let mut bytes = vec![0_u8; 128];
        bytes[..2].copy_from_slice(b"MZ");
        bytes[60..64].copy_from_slice(&64_u32.to_le_bytes());
        bytes[64..68].copy_from_slice(b"PE\0\0");
        bytes[68..70].copy_from_slice(&0x8664_u16.to_le_bytes());
        fs::write(&path, bytes).unwrap();
        assert!(verify_pe_x64(&path).is_ok());

        let wrong_architecture = directory.join("wrong-architecture.exe");
        let mut bytes = fs::read(&path).unwrap();
        bytes[68..70].copy_from_slice(&0x014c_u16.to_le_bytes());
        fs::write(&wrong_architecture, bytes).unwrap();
        assert!(verify_pe_x64(&wrong_architecture).is_err());
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn status_fails_closed_when_tools_are_missing() {
        let directory = std::env::temp_dir().join(format!("clipkeep-tools-{}", Uuid::new_v4()));
        let manager = DependencyManager::new(directory.clone());
        let status = manager.status().unwrap();
        assert!(!status.available);
        assert_eq!(status.tools.len(), 4);
        assert!(
            status
                .tools
                .iter()
                .all(|tool| tool.state == DependencyToolState::Missing)
        );
        assert!(manager.resolve().is_err());
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn installed_manifest_is_not_trusted_without_expected_artifact_version() {
        let directory = std::env::temp_dir().join(format!("clipkeep-manifest-{}", Uuid::new_v4()));
        let install_dir = directory.join(BUNDLE_ID);
        fs::create_dir_all(&install_dir).unwrap();
        fs::write(
            install_dir.join("installed.json"),
            br#"{"bundleId":"unexpected","ytDlpSha256":"0000000000000000000000000000000000000000000000000000000000000000","denoSha256":"0000000000000000000000000000000000000000000000000000000000000000","ffmpegSha256":"0000000000000000000000000000000000000000000000000000000000000000","ffprobeSha256":"0000000000000000000000000000000000000000000000000000000000000000"}"#,
        )
        .unwrap();
        let status = DependencyManager::new(directory.clone()).status().unwrap();
        assert!(!status.available);
        assert!(
            status
                .tools
                .iter()
                .all(|tool| tool.state == DependencyToolState::Missing)
        );
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn checksum_comparison_rejects_modified_artifacts() {
        assert!(matches_sha256("abc123", "abc123"));
        assert!(!matches_sha256("abc123", "abc124"));
    }

    #[test]
    fn corrupt_archives_are_rejected_before_extraction() {
        let directory = std::env::temp_dir().join(format!("clipkeep-zip-{}", Uuid::new_v4()));
        fs::create_dir(&directory).unwrap();
        let archive = directory.join("corrupt.zip");
        let output = directory.join("output.exe");
        fs::write(&archive, b"not a zip archive").unwrap();

        let result =
            extract_verified_entry(&archive, "expected.exe", &output, "test tool", &mut |_| {
                Ok(())
            });
        assert_eq!(result.unwrap_err().code, "dependency_archive_invalid");
        assert!(!output.exists());
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn interrupted_staging_is_cleaned_without_removing_installed_versions() {
        let directory = std::env::temp_dir().join(format!("clipkeep-repair-{}", Uuid::new_v4()));
        let stale_staging = directory.join(".staging-interrupted");
        let installed = directory.join("previous-version");
        fs::create_dir_all(&stale_staging).unwrap();
        fs::create_dir_all(&installed).unwrap();
        fs::write(stale_staging.join("partial.zip"), b"partial").unwrap();
        fs::write(installed.join("tool.exe"), b"preserve").unwrap();

        DependencyManager::new(directory.clone())
            .cleanup_stale_staging()
            .unwrap();
        assert!(!stale_staging.exists());
        assert!(installed.join("tool.exe").exists());
        fs::remove_dir_all(directory).unwrap();
    }
}
