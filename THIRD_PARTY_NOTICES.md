# Third-party notices

Scriptotar source code is distributed under the Apache License 2.0. Scriptotar Next also distributes and links third-party software under each component's own license. This document describes the current Windows NSIS and Linux Debian preview packages; it is an engineering inventory, not a legal opinion or warranty of compliance.

## Machine-readable package inventory

Every Scriptotar Next package contains a self-contained transcription runtime. During packaging, `sidecars/transcription/runtime_licenses.py` writes `transcription-runtime/RUNTIME-LICENSES.json` and a `transcription-runtime/legal/` directory.

That generated manifest records the exact installed Python dependency closure and versions used for the package, copies license/NOTICE files exposed by installed Python distributions, records the embedded Python interpreter license, and audits **both** FFmpeg distribution paths used by Scriptotar Next:

1. the standalone `ffmpeg`/`ffprobe` executables fetched through `static-ffmpeg`;
2. the FFmpeg shared/native libraries bundled inside the PyAV binary wheel.

For the standalone executables, the manifest records binary SHA-256 values, the exact FFmpeg build configuration and the `static-ffmpeg` download record when available. For PyAV, it records the output-derived FFmpeg configuration/license groups and hashes the wheel's bundled native libraries. Packaging rejects `--enable-nonfree` and rejects FFmpeg license drift away from the currently audited GPLv3 baseline.

The package workflows already run `sidecars/transcription/validate_runtime.py` before Tauri packaging and again against the installed package. The validator now requires the runtime legal bundle and both FFmpeg inventories, so a package that drops the notices or changes the audited FFmpeg licensing fails validation.

When this document and the generated manifest disagree, treat the discrepancy as a release blocker and investigate the built artifact.

## Components bundled in Scriptotar Next

### Rust/Tauri desktop application

- **Scriptotar Rust crates** are Apache-2.0 and are compiled into the desktop executable.
- **Tauri 2** and its Rust runtime are compiled/linked into the desktop application. Upstream Tauri declares `Apache-2.0 OR MIT`. Exact dependency versions are locked by `Cargo.lock`.
- **rusqlite** and **libsqlite3-sys** are MIT-licensed upstream. This repository enables rusqlite's `bundled` feature, which compiles SQLite into the application instead of relying on a system SQLite library. The core SQLite code delivered this way is dedicated to the public domain by the SQLite project. Exact Rust dependency identity is recorded in `Cargo.lock`.

The Rust toolchain and Tauri CLI used by CI are build tools; their command-line programs are not copied into the installer.

### Svelte frontend

Scriptotar Next ships the generated static frontend assets consumed by Tauri. Svelte runtime code needed by the compiled application can therefore be present in those generated assets. Svelte is MIT-licensed upstream.

The repository's Node development toolchain, including Vite, TypeScript, Vitest, jsdom and testing-library packages, is used to build/test the frontend. `node_modules` itself is not copied into the Tauri package.

### Packaged Python transcription runtime

A separate system Python installation is **not** required by Scriptotar Next. PyInstaller builds the supervisor, engine and dedicated yt-dlp executable and embeds the Python runtime/dependencies they need.

Important bundled Python components include, with exact versions and discovered license files recorded in `RUNTIME-LICENSES.json`:

- **faster-whisper**: transcription/inference integration;
- **CTranslate2**: inference runtime used by faster-whisper;
- **yt-dlp**: media metadata extraction/downloading;
- **curl-cffi**: the precompiled native HTTP/TLS client selected by the yt-dlp extra used by Scriptotar;
- **huggingface-hub**: model acquisition/cache client used by the inference stack;
- **tokenizers**: tokenizer runtime;
- **PyAV (`av`)** and the remaining installed transitive dependencies required by the engine.

Installed wheel metadata is used as the primary license source. Some wheels do not expose sufficient license metadata. `tokenizers 0.23.1`, for example, does not expose a usable license declaration/file in the wheel metadata seen by package CI, so Scriptotar uses a version-specific Apache-2.0 fallback fetched from the exact upstream `v0.23.1` tag and verifies its SHA-256. A tokenizers version change therefore fails rather than inheriting an old assumption.

**PyInstaller 6.21.0** is primarily a build tool, but its bootloader is part of generated executables. PyInstaller is GPL-2.0-or-later with its upstream bootloader exception, which permits distribution of applications built with the bootloader under the application's chosen license subject to the exception's terms. The package copies PyInstaller's installed license material into the runtime legal directory for reference.

### FFmpeg and ffprobe executables

**FFmpeg and ffprobe are bundled binaries. They are not system-provided in Scriptotar Next.**

`static-ffmpeg==3.0` is the build-time fetcher. It resolves FFmpeg 8.0-era platform binaries and Scriptotar copies the resulting `ffmpeg` and `ffprobe` executables into `transcription-runtime/ffmpeg/`. The `static-ffmpeg` Python package itself is not copied into the runtime.

The upstream `static-ffmpeg` binary-generation workflow documents these sources for the v8.0 archive lineage used by the package:

- Linux x86-64/ARM64 archives: BtbN/FFmpeg-Builds **GPL** variants;
- Windows x86-64 archive: gyan.dev **essentials** build.

Scriptotar does not trust those labels alone. Packaging executes the selected FFmpeg binary, records its full `configuration:` switches, records SHA-256 hashes for the resolved binaries, and classifies the effective FFmpeg license from `--enable-gpl`, `--enable-version3` and `--enable-nonfree`. The currently supported/audited package baseline is **GPL-3.0-or-later**. A different classification fails packaging until the notices and legal artifacts are reviewed.

### FFmpeg libraries bundled through PyAV

PyAV is not just Python code. Upstream explicitly provides binary wheels with FFmpeg bundled. At the currently audited dependency resolution, PyAV `18.0.0` uses FFmpeg `8.1.2` in its binary wheels, and the PyAV tag points its Windows vendor fetch to the `PyAV-Org/pyav-ffmpeg` `8.1.2-1` release lineage.

Scriptotar therefore treats PyAV's native FFmpeg libraries as a **second FFmpeg distribution path**, independent of the standalone `static-ffmpeg` executables. During packaging, Scriptotar runs `python -m av --version`, reads every FFmpeg library configuration/license group reported by the installed wheel, classifies the configuration switches using the same GPL/version3/nonfree rules, and hashes the wheel's `av.libs`/native-library payload.

The current audited PyAV FFmpeg baseline is also **GPL-3.0-or-later**. Packaging fails if a PyAV FFmpeg group changes license, lacks the expected GPLv3 configuration, or includes `--enable-nonfree`.

The runtime includes a verified verbatim copy of GNU GPL version 3 as `legal/FFMPEG-GPL-3.0.txt`, referenced by both FFmpeg inventory entries.

## Downloaded later, not bundled in the installer

Whisper model weights are intentionally not included in Scriptotar Next installers. A selected model is downloaded on first uncached use and cached in the user's Scriptotar application-data location. Model repositories can have their own licenses, notices and usage terms; those model artifacts are outside the installer inventory above.

Remote media downloaded at the user's request is likewise not a component distributed in the Scriptotar installer.

## System-provided runtime components

On Linux, **WebKitGTK/GTK and their platform libraries are system package/runtime dependencies**, not files copied into Scriptotar's `transcription-runtime` resource directory. The Debian package also declares `zenity` as a system dependency. Build CI installs development packages so Tauri can compile; end users receive runtime libraries through their operating-system package manager.

On Windows, the Tauri webview/platform runtime is supplied by the Windows/WebView2 environment rather than by Scriptotar's transcription-runtime resources.

## Items still requiring human/legal review

The safeguards above improve the accuracy and reproducibility of the package inventory, but they do **not** constitute a legal compliance guarantee.

1. **GPL Corresponding Source for both FFmpeg payloads.** Scriptotar currently records provenance/configuration and ships GPLv3 terms, but this repository does not yet mirror a Corresponding Source bundle for the exact standalone FFmpeg binaries **or** for the FFmpeg/native library set distributed through the PyAV wheel and their GPL-covered linked components. Before distributing installers, release owners should verify a GPL-compliant Corresponding Source delivery method or publish/mirror the required sources and build material. A bare upstream hyperlink should not be assumed to satisfy that obligation.
2. **curl-cffi native dependency notices.** `curl-cffi` is precompiled and its current upstream release line includes a native curl-impersonation stack. Scriptotar copies license/NOTICE files exposed by the installed wheel, but a human should confirm that the wheel's own native third-party components and attributions are fully represented before claiming exhaustive compliance.
3. **Exhaustive Rust/frontend transitive notices.** `Cargo.lock` and the frontend lockfile determine the build graph, and this notice identifies the principal shipped runtime layers, but the generated legal manifest currently focuses on the self-contained transcription runtime. It does not yet claim to be a complete license-text aggregation for every compiled Rust crate or every piece of frontend code folded into production assets. That remains a separate release/legal-review item.

## Legacy Scriptotar Classic

Scriptotar Classic remains a separate Python/Tkinter application in this repository. Its Tk/Tkinter and Linux secret-service integration are legacy/runtime concerns and should not be confused with what the Scriptotar Next installers bundle. This notice focuses on the current Scriptotar Next Windows and Linux distribution path.

## Upstream references

- FFmpeg: https://ffmpeg.org/ and https://github.com/FFmpeg/FFmpeg
- static-ffmpeg: https://github.com/zackees/static_ffmpeg
- static-ffmpeg binary archive: https://github.com/zackees/ffmpeg_bins
- BtbN FFmpeg builds: https://github.com/BtbN/FFmpeg-Builds
- gyan.dev FFmpeg builds: https://www.gyan.dev/ffmpeg/builds/
- PyAV: https://github.com/PyAV-Org/PyAV
- PyAV FFmpeg binary builds: https://github.com/PyAV-Org/pyav-ffmpeg
- PyInstaller: https://pyinstaller.org/
- faster-whisper: https://github.com/SYSTRAN/faster-whisper
- CTranslate2: https://github.com/OpenNMT/CTranslate2
- yt-dlp: https://github.com/yt-dlp/yt-dlp
- curl-cffi: https://github.com/lexiforest/curl_cffi
- Hugging Face Hub: https://github.com/huggingface/huggingface_hub
- tokenizers: https://github.com/huggingface/tokenizers
- rusqlite: https://github.com/rusqlite/rusqlite
- SQLite: https://www.sqlite.org/copyright.html
- Tauri: https://github.com/tauri-apps/tauri
- Svelte: https://github.com/sveltejs/svelte
