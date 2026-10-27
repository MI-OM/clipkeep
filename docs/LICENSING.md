# ClipKeep Licensing and Dependency Review

**Status:** ClipKeep-authored source is designated Apache-2.0 in `LICENSE`. The owner reports having rights to license the authored code, which they say began from ChatGPT research and was completed with AI coding agents. Rights to the included icon/artwork and project name/logo still need confirmation. Third-party license and notice review is incomplete; do not claim that third-party licenses are all taken care of or distribute the installer.
**Disclaimer:** Engineering checklist, not legal advice.

## Repository audit snapshot — 2026-10-27

This is a source and metadata inventory, not a complete SBOM or redistribution approval. The npm/Cargo package-count scan below was performed on 2026-10-26; the pinned yt-dlp and FFmpeg publisher pages were reviewed on 2026-10-27. The current evidence does **not** confirm that all third-party license obligations have been satisfied.

- `package-lock.json` contains 142 third-party package records. Their declared license expressions include MIT, Apache-2.0, BSD, ISC, CC0, BlueOak, and MPL-2.0; 12 records identify MPL-2.0 (Lightning CSS and platform package variants). No package record in the lockfile declares GPL/AGPL. The root package now declares Apache-2.0. Package metadata is not a substitute for checking each package's shipped license/notice files.
- Offline `cargo metadata` resolved 503 Rust package records including the root package. Five dependency records declare MPL-2.0 (`cssparser`, `cssparser-macros`, `dtoa-short`, `option-ext`, and `selectors`). `r-efi` declares MIT OR Apache-2.0 OR LGPL-2.1-or-later; the selected option and target inclusion must be recorded. Other expressions include Apache/MIT/BSD/ISC/Zlib/Unicode and CDLA-Permissive-2.0. The root package now declares Apache-2.0. This expression scan does not provide the texts/notices or demonstrate all dependencies' inclusion in the shipped binary.
- `sherpa-onnx` 1.13.8 uses `sherpa-onnx-sys` 1.13.8 with its default `static` feature. The crate build script downloads a versioned Windows x64 static native archive from GitHub when it is not cached. The inspected script does not verify an archive digest. The precise archive, its bundled ONNX Runtime and other third-party components, their licenses/notices, and corresponding source have not been inventoried. This is a release blocker for a reproducible, reviewable build and notice set.
- The pinned yt-dlp 2026.10.21 Windows PyInstaller executable is not covered only by yt-dlp's Unlicense: the upstream release README says its PyInstaller executables include GPLv3+ code and the combined work is GPLv3+. The matching `THIRD_PARTY_LICENSES.txt` is available from the exact upstream tag. ClipKeep currently downloads/extracts only `yt-dlp.exe`; it does not install that notice file. Review and provide the exact notice and determine the corresponding-source and Microsoft Distributable Code terms for the release artifact.
- The selected Gyan.dev FFmpeg Essentials artifact is a static GPLv3 build. The publisher describes all its builds as static and GPLv3. The exact package's build configuration, complete libraries list, corresponding source availability, and source/notice delivery plan remain unverified. ClipKeep currently has no compliance materials bundled with the managed download flow.
- The pinned large Parakeet model is CC BY 4.0 and has an install-time source note. The compact Parakeet model is CC BY 4.0, downloaded from a mutable `main` URL, with file hashes pinned in code and a short install-time attribution note. Whisper model weights are sourced from fixed Hugging Face revisions and the app writes an MIT/source note. Review each exact conversion's attribution, changes, notices, and license text; model cards and repository metadata can differ from the source model's license.
- `package.json` no longer marks the package `private: true`; npm and Cargo metadata now identify Apache-2.0. The owner reports rights to license the authored code; rights for the included artwork and project name/logo remain unconfirmed.

### Status by component

| Component | Preliminary result | Release status |
|---|---|---|
| ClipKeep source | Apache-2.0 license text and package metadata are present; owner reports rights to authored code | Confirm rights to included artwork and project name/logo before publication |
| npm dependencies | Lockfile metadata scan done; MPL-2.0 packages present | Preserve exact licenses/notices and generate reviewed SBOM |
| Cargo dependencies | Cargo metadata scan done; MPL-2.0 packages present; `r-efi` has license alternatives | Preserve licenses/notices and generate reviewed SBOM |
| sherpa-onnx native archive | Static archive fetched by build script; no digest check observed; bundled notices/source not inventoried | Blocked |
| yt-dlp executable | Exact pinned PyInstaller release is a GPLv3+ combined work; its upstream tag provides a third-party license bundle | Blocked until the exact notice is delivered with the installed tool and source/distribution terms are reviewed |
| Deno | MIT identified; exact release notice delivery not checked | Complete release artifact and bundled notice review |
| FFmpeg/ffprobe | Gyan static GPLv3 build | Blocked pending exact source, build configuration, libraries, and compliance plan |
| Speech models | Separate model terms; two CC BY 4.0 Parakeet models, Whisper MIT source model | Verify exact revisions/conversion notices and user-visible attribution |

The source license applies to ClipKeep-authored material only. It does not relicense these dependencies, model weights, media, or trademarks.

## Pinned Windows x64 dependency candidates

| Component | Pinned artifact and integrity source | Licence/status |
|---|---|---|
| yt-dlp | [2026.10.21 `yt-dlp.exe`](https://github.com/yt-dlp/yt-dlp/releases/download/2026.10.21/yt-dlp.exe); SHA-256 `33121d192d706e62f684a06f80918e3f3a8ba52ac3a65213b6951c39c7d9f555`, listed in the release's [SHA2-256SUMS](https://github.com/yt-dlp/yt-dlp/releases/download/2026.10.21/SHA2-256SUMS) | yt-dlp source is Unlicense; the pinned Windows PyInstaller executable includes GPLv3+ code and Microsoft Distributable Code. Provide the exact tag's [`THIRD_PARTY_LICENSES.txt`](https://github.com/yt-dlp/yt-dlp/blob/2026.10.21/THIRD_PARTY_LICENSES.txt) and review its distribution terms. |
| Deno | [v2.9.8 Windows x64 ZIP](https://github.com/denoland/deno/releases/download/v2.9.8/deno-x86_64-pc-windows-msvc.zip); archive SHA-256 `cba84b26e0df5ae7f74fc66b86a95b866872a86292ebd84fa1e959927edd93bf`; extracted `deno.exe` SHA-256 `924397800533d3eb0ed0c889b20ad70e18b970d93acd8135bc043f425e528266` | MIT; release source contains `LICENSE.md`. |
| FFmpeg and ffprobe | [Gyan.dev FFmpeg 9.0.2 Essentials ZIP](https://www.gyan.dev/ffmpeg/builds/packages/ffmpeg-9.0.2-essentials_build.zip); archive SHA-256 `60f467265b1e312373dbcd92200c2618a74850f98d3d078e94296bb3fa2047ba`; extracted `ffmpeg.exe` SHA-256 `3256173f3f8bffd7df12227c68adf68025edb1832273a9530688a7bb1ed8edec`; `ffprobe.exe` SHA-256 `f0d36ecbbdd3bcfac3efa078c96c7271c2e68b3810595552ac3b7f17e9a65c52` | Gyan describes these static builds as GPLv3 and lists included libraries, including libmp3lame, libx264, and libx265. Third-party build; the sidecar is not an independent publisher signature. |
| sherpa-onnx Rust runtime | `sherpa-onnx` 1.13.8 pinned in `src-tauri/Cargo.toml`; native library is fetched by the crate build script | Apache-2.0 project; audit the exact native archive, its notices, and linked components before redistribution. |
| Parakeet TDT 0.6B v2 INT8 model | [Pinned model revision](https://huggingface.co/csukuangfj/sherpa-onnx-nemo-parakeet-tdt-0.6b-v2-int8/tree/1ab9323); ONNX SHA-256 values are pinned in `src-tauri/src/transcription.rs` | Model card identifies CC BY 4.0. Retain attribution; verify exact source checkpoint and redistribution obligations before shipping model files. The app downloads it after a user action rather than bundling it in the installer. |

The extracted Deno and FFmpeg executable digests were computed from the downloaded ZIP
entries only after their archive digests matched the pinned values; the executables were
not run. The yt-dlp standalone bundles `yt-dlp-ejs`; Deno is selected explicitly as the
JavaScript runtime. The pinned Deno version is above yt-dlp's documented 2.3.0 minimum.
ffprobe is extracted from the same FFmpeg archive as ffmpeg. Deno's own `LICENSE.md` is
MIT; a complete release-time inventory of any bundled third-party notices is still needed.

This table is not an SBOM, legal review, or approval record. The pins are engineering
inputs and must be checked again before public release.

## Required artifact record

For each shipped binary/library, record component, exact version, official/approved source, OS and architecture, filename, digest/signature result, licence and licence text, included components, redistribution obligations, notices, corresponding-source obligations, date checked, and approval state.

Unknown values block release; never invent versions, hashes, URLs, or licence metadata.
The application does not currently bundle dependency executables in its installer, but
the app-managed download still needs a reviewed license/notice/source-compliance plan.

## Risks to verify

- yt-dlp's PyInstaller executable includes third-party components. The selected release's third-party license inventory and required notice delivery must be reviewed and made available alongside the installed executable before redistribution.
- Gyan.dev identifies the selected FFmpeg archive as a static GPLv3 build. Determine the exact build configuration, all included component licenses, corresponding-source obligations, and how to retain/provide the applicable source and notices before redistribution.
- Apache-2.0 applies to ClipKeep-authored source only. Confirm the owner has rights to license all included code and assets; keep third-party licenses separate.
- Current YouTube support may require yt-dlp-ejs and a supported JavaScript runtime. Verify official documentation for the exact pinned release.
- The app's own licence does not authorize downloading media or remove obligations from bundled dependencies.
- The Parakeet model is a separate CC BY 4.0 work. Keep attribution with installed model files and complete a model-specific redistribution review before any public model bundling.
- Review applicable platform terms and copyright law before public distribution.

## Release controls

1. Record the owner's rights confirmation for authored code; confirm rights to included artwork and the project name/logo before public source release.
2. Complete artifact-specific review for the selected yt-dlp and FFmpeg builds.
3. Provide the exact notices and corresponding-source materials required for any redistributed binaries.
4. Generate a complete dependency inventory/SBOM for application, development, and downloaded components.
5. Fail release checks for missing license records or digest mismatch.
6. Re-review every dependency update and retire old pins only with an explicit upgrade plan.

## Fastest practical path

1. Publish a source-only repository after confirming rights to the icon/artwork and project name/logo and reviewing the exact commit. Keep the Apache-2.0 grant scoped to ClipKeep-authored material; the lockfiles identify third-party dependencies but do not relicense them.
2. Hold the Windows installer release until the artifact records, notices, SBOM, and source/compliance plan above are complete. The current workflow makes a draft release, but that draft is not a licensing approval.
3. For the Windows installer, first decide whether to replace the pinned Gyan static GPLv3 build with a build whose license and feature set fit the intended distribution, or keep it and obtain a reviewed plan for the exact binary and its corresponding source. FFmpeg's own legal guidance distinguishes its LGPL and GPL configurations; the exact Gyan build is described as static GPLv3.
4. Add the matching yt-dlp license bundle to the user's installed tool directory and audit the exact static `sherpa-onnx` archive before enabling public binary distribution.

These are engineering release controls, not a legal conclusion. Ask qualified counsel to review the final distribution model if you will publish installers or let the app acquire third-party executables.

References: [yt-dlp 2026.10.21 licensing notes](https://github.com/yt-dlp/yt-dlp/blob/2026.10.21/README.md), [yt-dlp 2026.10.21 third-party licenses](https://github.com/yt-dlp/yt-dlp/blob/2026.10.21/THIRD_PARTY_LICENSES.txt), [FFmpeg legal guidance](https://ffmpeg.org/legal.html), [Gyan.dev build information](https://www.gyan.dev/ffmpeg/builds/), [GNU GPL FAQ](https://www.gnu.org/licenses/gpl-faq.en.html), [sherpa-onnx 1.13.8 license](https://github.com/k2-fsa/sherpa-onnx/blob/v1.13.8/LICENSE), [Tauri Windows signing](https://v2.tauri.app/distribute/sign/windows/).
