# ClipKeep Test Plan

**Status:** Dependency manager implemented; integration and release-readiness work remain
**Latest verification:** `npm test` — 20 passed; `npm run build` — passed; `cargo test` — 17 passed; `cargo check` — passed. No dependency installation or authorized real-media integration test was run.

## Strategy

Use unit tests for URL validation and job lifecycle, component tests for accessible states, Rust tests for process arguments/filesystem behavior, then authorized integration and clean-machine tests. Mocks must be clearly identified and must not claim real download success.

## Milestone matrix

| Milestone | Coverage | Exit evidence |
|---|---|---|
| 1. Foundation | URL forms and rejection; format/quality controls; empty/error/progress/cancel/preview states; keyboard labels; typed models; CSP/capability review | Windows app launches; frontend checks pass; mock never reports success |
| 2. Downloader | Backend URL tests; fixed argument construction; metadata/progress parsing; bounded stdout/stderr; process failure; cancellation/child cleanup; one-job enforcement | Rust tests/build pass; authorized fixture in temp directory; true success/failure evidence recorded |
| 3. Formats/files | Combined/separate streams; MP4 merge; MP3 conversion; unsupported quality; conversion failure; cancellation; duplicate names; traversal; partial cleanup/output validation | Authorized fixtures produce valid files; no silent overwrite; incomplete output never completed |
| 4. Dependencies/security | Missing/corrupt/wrong-architecture/mismatched digest; interrupted staging cleanup; archive extraction; repair failure; CSP/capability review | Clean Windows x64 environment can install/repair only pinned, checksum- and architecture-verified tools; no dependency payload has been installed/executed in this work |
| 5. Installer | Clean install/launch; media smoke test; upgrade/uninstall; preserve user media; runtime prerequisites; notices/SBOM | Clean supported Windows environment passes release checklist |

## Focused cases

- URL: empty, malformed, non-HTTPS, credentials, explicit ports, lookalike hosts, suffix tricks, invalid video IDs, supported watch/short/embed/live forms.
- Lifecycle: unique IDs; maximum concurrency enforced at three; bounded progress per job; one terminal state per job; independent cancellation; process-exit race; retry uses a fresh ID.
- Process: output is drained without deadlock and bounded; timeout and child cleanup are tested.
- Filesystem: hostile titles cannot escape the destination; test Windows reserved names, Unicode, long names, collisions, and partial-file behavior.
- Dependencies: missing/corrupt/wrong-platform/wrong-architecture/digest mismatch fail closed; corrupt archives are rejected; interrupted staging is cleaned on retry; prior installed versions are preserved.
- Release: exact versions, digests, licences, notices, and SBOM are present.

## Test media and privacy

Use media that the project owner is authorized to download. Do not use cookies, credentials, playlists, or access-control circumvention. Keep integration outputs in temporary folders and avoid sensitive URL query data in snapshots/logs.

## Current commands and evidence

- `npm test` — 3 test files, 20 tests passed.
- `npm run build` — TypeScript check and Vite production build passed.
- `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` — passed after final Rust edits.
- `cargo test --quiet --manifest-path src-tauri/Cargo.toml` — 17 tests passed.
- `cargo check --manifest-path src-tauri/Cargo.toml` — passed after dependency-manager changes.
- `npm run tauri -- build --no-bundle` — an earlier Windows executable build succeeded after MP4/MP3 implementation; a later relink then failed with `Access is denied` while replacing the existing executable. No Tauri relink was run after dependency-manager changes. This command does not create an installer.
- Unit coverage includes MP4/MP3 arguments and quality profiles, stream checks, container/codec validation, collision refusal, and temporary workspace cleanup.
- The production resolver is available only after the Settings installer runs and verifies every executable. No real-media integration test has run, and no dependency binaries have been installed or executed.

Update this section only from observed command results. Document unavailable platform tests as outstanding.
