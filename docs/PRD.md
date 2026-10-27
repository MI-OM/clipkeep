# ClipKeep Product Requirements

**Status:** Approved for staged implementation
**Platform:** Windows first; cross-platform support is a future goal
**Current milestone:** 1 — application shell

## Product summary

ClipKeep is a local-first desktop utility for saving individual videos as MP4 or audio as MP3 when the user owns the content, has permission to save it, or is otherwise authorized. Media processing and file management happen on the user's computer. There are no accounts, ads, analytics, cloud conversion services, or automatic background downloads.

The working name is ClipKeep; verify trademarks and existing projects before public adoption.

## Goals

- Paste an individual video link from YouTube, Vimeo, Facebook, TikTok, or Instagram, choose a format and available quality, then save to a selected folder. Extractor behavior is platform-dependent and must be tested before claiming support.
- Show metadata when available, accurate progress, cancellation, retry, and clear errors.
- Keep jobs, settings, history, and media processing local.
- Manage required media tools transparently and verify them before execution.
- Transcribe local audio and video on-device, then review and export the result.
- Keep the interface accessible and responsive while work is active.

## Scope boundaries

**MVP:** individual supported video URLs from allow-listed hosts; MP4 with audio; MP3 conversion; quality selection; progress, cancellation, retry; save location; local history; managed dependencies; on-device transcription with editable TXT output. Platform terms and download settings apply; URL validation is not proof of permission.

**Not in MVP:** playlists/channels, DRM or access-control circumvention, private-video access, browser extension, built-in player, cloud conversion, account/sync, and automatic background work.

Do not import cookies or automate authentication. Target only media the user is authorized to download. Do not claim support for a format or quality unavailable from the source.

## Primary flow

1. Paste an individual video URL from an allow-listed platform.
2. Review title, duration, and thumbnail if returned.
3. Select MP4 or MP3 and a quality compatible with the source.
4. Select or confirm the save folder.
5. Start up to three jobs, monitor them independently, cancel or retry when appropriate.
6. Review the result in Downloads and local history.

Screens: Save media, Downloads, Settings, and About/licences. Include clear empty, validation, dependency, network, and processing states.

## Functional requirements

| ID | Requirement | Acceptance criteria |
|---|---|---|
| FR-01 | URL input | Accept allow-listed platform URL hosts; apply platform-specific path validation where implemented; reject malformed, non-HTTPS, and unapproved hosts with clear feedback. |
| FR-02 | Metadata preview | Display title, duration, and thumbnail when available; do not fabricate missing values. |
| FR-03 | Format | Choose MP4 or MP3 before starting. |
| FR-04 | Quality | Offer Best Available and compatible options only; do not promise unavailable quality. |
| FR-05 | Execution | Use a typed request and fixed executable with explicit arguments; never compose shell commands from user input. |
| FR-06 | Progress | Show lifecycle status and percentage/speed/ETA only when calculable. |
| FR-07 | Cancellation | Stop the process tree, clean app-owned temporary files, and report cancellation accurately. |
| FR-08 | Integrity | Report a final file only after successful exit, validation, and safe finalization. |
| FR-09 | Destination | Select a directory and remember it locally. |
| FR-10 | Error recovery | Map common failures to actionable messages; retain bounded, redacted local diagnostics. |
| FR-11 | Dependencies | Check versions and integrity before use; fail closed with repair guidance. |
| FR-12 | History | Keep a local list of recent outcomes with a clear-history action; avoid storing sensitive full URLs. |
| FR-13 | Privacy | No ClipKeep analytics, advertising, accounts, or cloud processing; disclose that source platforms receive download/metadata requests and publishers receive user-requested tool/model downloads. |
| FR-14 | Installer | Test a Windows installer on a clean supported machine before release claims. |
| FR-15 | Local transcription | Transcribe a user-selected local audio/video file on-device using the managed, pinned ASR engine and local FFmpeg decoding. |
| FR-16 | Transcript review | Let users edit recognized text and export TXT; remind users to check consequential passages. |
| FR-17 | Model setup | Make model installation user initiated, verify pinned model files, show progress, and retain model attribution locally. |

## Quality requirements

- UI stays responsive during downloads/conversion.
- Interrupted and incomplete work is never shown as complete.
- No arbitrary shell execution, untrusted executable execution, or broad filesystem permissions.
- Keyboard navigation, visible focus, readable contrast, semantic labels, and accessible progress.
- Platform-specific executable names, paths, and packaging stay outside shared UI code.
- One active job in the first release; no unnecessary local server or database server.

## Release gates

1. **Foundation:** Windows app launches; screens work with a mock service; validation and UI states are tested.
2. **Downloader:** authorized integration media can be inspected/downloaded through Rust; failures and cancellation are truthful.
3. **Formats/filesystem:** validated MP4/MP3 output; compatible quality options; safe cancellation and collision behavior.
4. **Dependencies/security:** pinned and verified tools, repair flow, and reviewed permissions work on a clean test environment.
5. **Installer:** clean install, launch, upgrade, uninstall, notices, SBOM, and acceptance tests pass.

Do not count a gate as complete without evidence. The current mock must never claim a successful download.

## Current implementation status

The app shell, UI models, frontend URL validation, local preferences, folder selection,
and an explicitly non-success browser preview exist. The Rust backend implements
metadata/process contracts and MP4/MP3 processing, including staged output and validation.
The Windows x64 desktop build has a user-initiated dependency manager for pinned
yt-dlp, Deno, FFmpeg, and ffprobe artifacts; it fails closed until every executable
passes checksum and architecture checks. Dependency acquisition and app code compile,
but no dependency payload or authorized real-media integration test has been run.
The first local transcription path uses the pinned sherpa-onnx Rust runtime and Parakeet
TDT v2 INT8, with user-initiated model setup, local audio decoding, editable text, and TXT
export. Frontend and Rust compilation pass; model installation and real-file inference
have not yet been exercised. Cancellation, timestamped subtitle exports, and transcription
history remain outstanding. No candidate benchmark has run, so Parakeet is provisional.
Public distribution remains blocked on the documented licensing/notices review, including
the model's CC BY 4.0 attribution.
