# ClipKeep Decisions

**Status:** Approved for staged implementation; open decisions remain.

## Approved for Milestone 1

| ID | Decision | State |
|---|---|---|
| D-01 | Tauri 2, React, TypeScript, Vite, Rust | Approved |
| D-02 | Windows first; no macOS/Linux support claim without separate builds/tests | Approved |
| D-03 | Initial source family is individual video URLs from YouTube, Vimeo, Facebook, TikTok, and Instagram; only exact hosts and known single-item URL patterns are accepted. | Candidate scope; non-YouTube behavior and platform authorization remain unverified |
| D-04 | Allow up to three active downloads; enforce the same limit in the UI and Rust job manager. | Approved |
| D-05 | UI-first mock service; never simulate success | Approved |
| D-06 | Pin and verify dependencies; do not trust moving latest URLs or arbitrary mirrors | Required control |

## Implemented dependency decisions and unresolved questions

| ID | Question | Needed by |
|---|---|---|
| D-10 | License ClipKeep-authored source under Apache-2.0. Third-party software, models, art, and trademarks retain separate terms. | Selected for source metadata; owner reports rights to authored code; artwork and project name/logo rights still need confirmation |
| D-07 | Install pinned yt-dlp, Deno, and FFmpeg/ffprobe artifacts in app-local storage on explicit user action; do not bundle them into the app executable. | Implemented; release review pending |
| D-08 | Pin yt-dlp 2026.10.21, Deno 2.9.8, and Gyan.dev FFmpeg Essentials 9.0.2 for Windows x64. | Implemented; verify again before release |
| U-04 | Which Windows versions and architectures are supported? | Packaging |
| U-05 | Which quality presets should be exposed, and which audio bitrate labels are useful? | Format milestone |
| U-06 | How should filename collisions be handled without silent overwrite? | File handling |
| U-07 | What happens to an active job when the app closes? | Downloader integration |
| U-08 | Which history fields and retention period are acceptable? | History implementation |
| D-09 | Dependency installation and repair are user initiated; no background update checks. | Implemented |
| U-10 | Confirm ClipKeep name/icon/trademark review before public release. | Public release |
| U-11 | Should the app remember the last folder, a configurable default, or both? | Settings |

Record rationale, date, and effects on requirements when resolving an item. Do not approve a version, hash, URL, or licence until verified against the exact artifact.
