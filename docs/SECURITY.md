# ClipKeep Security Model

**Status:** Dependency acquisition controls implemented; no independent security review performed
**Notice:** This is not a security certification. Authorized real-media testing and distribution licensing review remain outstanding.

## Scope and assets

ClipKeep is a Windows-first local desktop application that will accept URLs/metadata, manage external media tools, and write to a user-selected directory. Assets include user files, local preferences/history, dependency binaries, process integrity, and privacy.

Trust boundaries:

1. UI to Tauri commands: frontend input is untrusted; Rust must repeat validation.
2. App to remote source: URLs, responses, metadata, thumbnails, filenames, stdout/stderr are untrusted.
3. Dependency acquisition to execution: artifacts remain untrusted until verified.
4. App to filesystem: paths and existing files require explicit checks.
5. Process output to UI/logs: output may be malformed, excessive, or sensitive.

## Threats and controls

| Threat | Required control |
|---|---|
| Command injection | Typed argument construction in Rust; no user-composed shell command; no generic process API. |
| Unsafe URL | Parse in Rust; HTTPS only; exact normalized host allow-list; reject credentials, arbitrary schemes, and unsupported hosts. |
| Host confusion | Avoid suffix matching; test lookalikes, ports, userinfo, malformed paths, redirects, and encoded input. |
| Malicious metadata | Escape text, treat filenames as data, avoid unsafe HTML, and do not fetch remote thumbnails without a deliberate policy. |
| Path traversal/overwrite | Explicit destination, safe filename generation, app-owned temporary paths, collision check, no silent overwrite. |
| Unverified executable | Use pinned HTTPS artifact URLs and SHA-256 values; bound downloads; extract only fixed ZIP entries; check PE x64 architecture and installed-file digests before use; fail closed. |
| Excessive permissions | Narrow Tauri capabilities; no broad shell or filesystem access. |
| Output/resource exhaustion | One active job, bounded stdout/stderr/diagnostics, timeouts where suitable, no deadlocks. |
| Cancellation leaks | Track and terminate the process tree, await exit, clean only app-owned temporary files. |
| Partial file appears complete | Temporary output, successful process exit, output validation, and safe finalization before completion. |
| Sensitive URL leakage | Minimize URL retention; redact query and fragment in logs/history; no telemetry. |
| Unsafe update | User-triggered installation only; fixed release URLs and hashes; no background update or moving `latest` URL. |
| Remote navigation/script injection | Restrictive CSP, no unnecessary opener permission, render remote text as text. |
| Shutdown during work | Decide explicit shutdown behavior; interrupted work is never marked completed. |

## Security invariants

- Backend validation is authoritative.
- No unverified executable runs.
- One active job is enforced in the backend.
- Completion follows output validation/finalization only.
- Cancellation never deletes arbitrary user files.
- No browser cookies, credentials, or authentication automation.
- No ClipKeep analytics, accounts, advertising, or cloud media processing. Source platforms receive URL requests; tool and model providers receive user-requested downloads.
- Errors are explicit; verification failures are not replaced by success-shaped defaults.

## Privacy

Diagnostics should be local, bounded, and redacted. Do not log cookies, authorization headers, full sensitive URLs, or user file contents. History stores the minimum useful outcome and avoids full URLs. Any diagnostic export should be user-initiated and reviewed before sharing.

## Milestone 1 notes

- Frontend validation rejects non-HTTPS URLs, non-allow-listed hosts, credentials, and explicit ports. YouTube paths are validated strictly; other supported host paths are treated as extractor candidates and remain subject to yt-dlp checks. Validation is not a security boundary.
- Tauri capability includes `core:default` and `dialog:allow-open`; no shell, broad filesystem, or opener permission.
- CSP allows same-origin assets, Tauri IPC, and local Vite HMR; no remote font/thumbnail source is configured.
- Mock preview always terminates as failed or cancelled; no output path is produced.

## Milestone 2 notes

- Rust repeats URL validation and normalizes accepted YouTube links to a fixed watch URL. Other exact allow-listed Vimeo, Facebook, TikTok, and Instagram hosts retain their HTTPS URL after removing fragments. Credentials, ports, non-HTTPS schemes, unapproved hosts, and playlist query links are rejected.
- Metadata and download processes use explicit argument vectors, ignore yt-dlp configuration, disable netrc, drain process streams, and apply output/time limits. No shell command string or generic process command is exposed.
- The Windows download process is assigned to a kill-on-close Job Object. The backend enforces a maximum of three active jobs.

## Milestone 3 notes

- MP4 and MP3 use fixed yt-dlp argument vectors and a resolved FFmpeg path. MP4 selectors prefer compatible MP4 video plus M4A audio streams, with an MP4 combined-stream fallback; MP3 audio bitrates are passed as fixed FFmpeg post-processing options.
- Each job stages output in a UUID-named subfolder inside the selected destination. Staging is removed on worker exit, including failure and cancellation. Final output is validated with ffprobe for the expected container and streams.
- Finalization uses a same-volume hard link, which atomically refuses an existing final name instead of overwriting it. The app reports collisions rather than auto-renaming.
- Unit tests cover stream-quality filtering, argument construction, expected container/codec validation, collision checks, temporary-folder cleanup, missing dependency status, corrupt archives, digest comparison, stale-staging cleanup, and PE architecture. No real processing has run because the verified tools have not been installed and exercised in an authorized integration test.

## Milestone 4 notes

- The Windows x64 dependency manager downloads only fixed release URLs, permits HTTPS redirects only to a short host allow-list, caps transfer size, checks pinned archive hashes before extraction, reads only fixed ZIP entries, checks pinned executable hashes and PE architecture, and stages/activates the tool set without overwriting a prior directory.
- The installed manifest must match the pinned executable hashes; every status/resolve operation hashes files and checks architecture before returning paths. Missing or invalid files block execution.
- Interrupted staging directories are cleaned on a user-initiated retry; previous version folders are preserved.
- Dependency downloads use yt-dlp 2026.10.21, Deno 2.9.8, and Gyan.dev FFmpeg Essentials 9.0.2. The FFmpeg candidate is GPLv3 and remains a release-licensing blocker; see `LICENSING.md`.
- No real metadata or download run has been verified.

## Verification plan

Test URL normalization/allow-listing, argument vectors, path traversal and collisions, process output bounds, process-tree cancellation, integrity failure, and partial-file cleanup. Inspect CSP/capabilities. Conduct a separate review before packaging. Passing tests provide evidence only for tested cases.
