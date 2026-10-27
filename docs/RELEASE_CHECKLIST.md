# Release readiness

ClipKeep's source now declares Apache-2.0. The source tree and the Windows installer have separate release gates: publishing source does not grant rights to third-party tools or model files, and does not establish that the installer is ready.

## Automated build flow

- Pull requests and pushes run the frontend production build on Windows through GitHub Actions.
- Pushing a `v*` tag that matches the versions in `package.json`, `src-tauri/tauri.conf.json`, and `src-tauri/Cargo.toml` builds Windows x64 MSI and NSIS installers and opens a **draft** GitHub release with SHA-256 checksums.
- The draft is a review artifact, not approval to distribute. Complete the applicable gates below, review the installer and checksum, and configure code signing before publishing. The workflow does not sign installers.
- Required repository setup: enable GitHub Actions and allow the release workflow to write repository contents. No signing certificate or secret is configured in this source tree.

## Before publishing the source repository

- [x] Owner reports having rights to license the ClipKeep-authored code under Apache-2.0; the project began from ChatGPT research and was completed with AI coding agents.
- [ ] Confirm rights to the included app icon/artwork and project name/logo before publication.
- [ ] Review the initial public source commit and ensure it contains no local media, private planning documents, generated installers, environment files, or build directories. The root `.gitignore` now excludes these items.
- [ ] Keep README claims accurate: only YouTube has an authorized end-to-end validation record; Vimeo, Facebook, TikTok, and Instagram are candidate URL patterns and remain unverified.
- [ ] Review the privacy notice against actual storage and network behavior and for the intended audience/jurisdictions. Contact email and revision date are now present.
- [ ] Create/configure the intended GitHub repository and publish the reviewed source commit.

## Before distributing a Windows installer

- [ ] Inventory and provide all third-party license texts, notices, attributions, and corresponding-source materials required by the exact shipped artifacts. Findings are in [LICENSING.md](./LICENSING.md).
- [ ] Resolve the static `sherpa-onnx` native archive: verify its digest, identify its bundled components, preserve their notices/source, and make builds reproducible. The current build script downloads the archive without an observed digest check.
- [ ] Include yt-dlp's matching release `THIRD_PARTY_LICENSES.txt` where users can access it. The current managed tool setup extracts only `yt-dlp.exe`.
- [ ] Decide and document the redistribution/compliance plan for the selected static GPLv3 Gyan FFmpeg build, including its exact configuration, libraries, source, and notices.
- [ ] Verify each speech model's exact revision, source, license, conversion changes, attribution, notices, and permission to fetch/distribute it. Preserve the required model-specific attribution with the installed model.
- [ ] Generate and review a complete SBOM for the application, npm/Cargo dependencies, native archive, managed tools, and model files.
- [ ] Pass current frontend/Rust checks, authorized MP4/MP3 integration, cancellation and failure handling, installer clean install/upgrade/uninstall, and transcription checks on a clean supported Windows x64 machine.
- [ ] Verify any platform before claiming it is supported. Only YouTube has been validated end to end; the other accepted platform URL patterns are unverified. Obtain platform authorization where required.
- [ ] Have qualified counsel review any terms addressing user responsibility/liability; do not rely on an absolute “not liable for user downloads” statement.
- [ ] Inspect the final installer and release artifacts for user data, private notes, unreviewed binaries, notices, and checksums.

## Before each release

- [ ] Recheck dependency versions, source URLs, hashes/signatures, licenses, notices, and corresponding-source availability.
- [ ] Build from the reviewed tagged commit and record check/test results from that exact source.
- [ ] Publish source and binary artifacts with matching version/tag, checksums, notices, and known limitations.
- [ ] Sign the Windows installers with the release certificate and verify the signatures before publishing; the automated workflow currently produces unsigned installers.
