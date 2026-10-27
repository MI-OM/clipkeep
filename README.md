# ClipKeep

ClipKeep is a Windows-first desktop app for saving media locally when you own it or are otherwise authorized to download it. It is built with Tauri 2, React, TypeScript, and Rust.

> **Release status:** Source is licensed under Apache-2.0. Desktop installers are not cleared for redistribution yet; review the third-party and validation gates in [the release checklist](./docs/RELEASE_CHECKLIST.md). Only YouTube has been validated end to end. Other listed platform links are experimental and unverified.

## Features

- Accept individual video URL patterns from YouTube, Vimeo, Facebook, TikTok, and Instagram. Only YouTube has been validated end to end; the other platforms are experimental. Actual downloads depend on extractor support, source permissions/settings, and current platform terms.
- Download supported media as MP4 or audio as MP3, using locally managed media tools.
- Install pinned Windows x64 versions of yt-dlp, Deno, FFmpeg, and ffprobe from Settings. ClipKeep verifies checksums and executable architecture before use.
- Transcribe local audio and video on-device with a user-selected, separately downloaded speech model; edit and export the transcript as TXT. Model quality and hardware behavior remain under evaluation.
- Keep settings, download history, media processing, and files on the user's computer.

ClipKeep does not provide a browser downloader, account, cloud conversion service, or access-control bypass. Support and release claims are limited to Windows x64 until other platforms have been separately built and tested.

## Requirements

- Windows x64
- Node.js and npm
- Rust and Cargo
- The [Tauri Windows prerequisites](https://v2.tauri.app/start/prerequisites/)

## Run from source

```powershell
npm ci
npm run tauri dev
```

To build the frontend or Windows installer:

```powershell
npm run build
npm run tauri build
```

## Prepare a Windows release

GitHub Actions builds the frontend on pushes and pull requests. After publishing the source repository, push a version tag such as `v0.1.0` to build Windows x64 MSI and NSIS installers. The workflow checks that the tag matches all three app manifests, then creates a draft GitHub release with installer SHA-256 checksums. Review the draft and complete the distribution gates in [the release checklist](./docs/RELEASE_CHECKLIST.md) before publishing it. Installers are currently unsigned, and third-party licensing review is still required.

Users can install the media tools from **Settings > Install media tools**. The app fetches pinned third-party artifacts at the user's request; their licenses and notices are separate from the ClipKeep source license and remain under release review.

## License

ClipKeep-authored source code is available under the Apache License 2.0; see [LICENSE](./LICENSE). This license does not cover third-party tools, libraries, speech models, media, or trademarks. Their licenses and notices remain separate.

## Project documentation

- [Product requirements](./docs/PRD.md)
- [Architecture](./docs/ARCHITECTURE.md)
- [Security model](./docs/SECURITY.md)
- [Licensing and dependency review](./docs/LICENSING.md)
- [Test plan](./docs/TEST_PLAN.md)
- [Open decisions](./docs/DECISIONS.md)
- [Release checklist](./docs/RELEASE_CHECKLIST.md)
- [Privacy notice](./docs/PRIVACY.md)

## Responsible use

Only save media you own, have permission to download, or are otherwise authorized to keep. Respect copyright, platform terms, and applicable law. ClipKeep is not intended to bypass DRM, private-video restrictions, or other access controls.
