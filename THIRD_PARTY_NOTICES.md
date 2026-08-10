# Third-party notices

Scriptotar source code is distributed under the Apache License 2.0. Scriptotar Next also distributes and links third-party software under each component's own license. This document describes the current Windows NSIS and Linux Debian preview packages; it is an engineering inventory, not a legal opinion or warranty of compliance.

## Machine-readable package inventory

Every Scriptotar Next package contains a self-contained transcription runtime. During packaging, `sidecars/transcription/runtime_licenses.py` writes `transcription-runtime/RUNTIME-LICENSES.json` and a `transcription-runtime/legal/` directory.

That generated manifest records the exact installed Python dependency closure and versions used for the package, copies license/NOTICE files exposed by installed Python distributions, records the Python interpreter license, records the exact FFmpeg build configuration and binary download record, and refuses to package FFmpeg if its audited license baseline changes or `--enable-nonfree` is present.

The package workflows validate those files before uploading installers. When this document and the generated manifest disagree, treat the discrepancy as a release blocker and investigate the built artifact.

## Components bundled in Scriptotar Next

### Rust/Tauri desktop application

- **Scriptotar Rust crates** are Apache-2.0 and are compiled into the desktop executable.
- **Tauri 2** and its Rust runtime are compiled/linked into the desktop application. Upstream Tauri declares `Apache-2.0 OR MIT`. Exact dependency versions are locked by `Cargo.lock`.
- **rusqlite** is built with its `bundled` feature in this repository, so the desktop application uses a bundled SQLite build rather than relying on a system SQLite installation. Exact Rust dependency identity is recorded in `Cargo.lock`.

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
- **curl-cffi**: HTTP/TLS impersonation support selected by the yt-dlp extra used by Scriptotar;
- **huggingface-hub**: model acquisition/cache client used by the inference stack;
- **tokenizers**: tokenizer runtime;
- **PyAV (`av`)** and the remaining installed transitive dependencies required by the engine.

**PyInstaller 6.21.0** is primarily a build tool, but its bootloader is part of generated executables. PyInstaller is GPL-2.0-or-later with its upstream bootloader exception, which permits distribution of applications built with the bootloader under the application's chosen license subject to the exception's terms. The package copies PyInstaller's installed license material into the runtime legal directory for reference.

### FFmpeg and ffprobe

**FFmpeg and ffprobe are bundled binaries. They are not system-provided in Scriptotar Next.**

`static-ffmpeg==3.0` is the build-time fetcher. It resolves FFmpeg 8.0-era platform binaries and Scriptotar copies the resulting `ffmpeg` and `ffprobe` executables into `transcription-runtime/ffmpeg/`. The `static-ffmpeg` Python package itself is not copied into the runtime.

The upstream `static-ffmpeg` binary-generation workflow documents these sources for the v8.0 archive lineage used by the package:

- Linux x86-64/ARM64 archives: BtbN/FFmpeg-Builds **GPL** variants;
- Windows x86-64 archive: gyan.dev **essentials** build.

Scriptotar does not trust those labels alone. Packaging executes the selected FFmpeg binary, records its full `configuration:` switches and classifies the effective FFmpeg license from `--enable-gpl`, `--enable-version3` and `--enable-nonfree`. The currently supported/audited package baseline is **GPL-3.0-or-later**. A different classification fails packaging until the notices and legal artifacts are reviewed.

The runtime includes a verified verbatim copy of GNU GPL version 3 as `legal/FFMPEG-GPL-3.0.txt`. `RUNTIME-LICENSES.json` also records the actual `static-ffmpeg` download record when available.

**Corresponding Source remains a release responsibility.** GPL object-code redistribution requires qualifying Corresponding Source availability under the GPL's terms. Scriptotar currently records binary provenance and ships the GPL text, but this repository does not yet mirror a source bundle for the exact prebuilt FFmpeg binary and all GPL-covered linked components. Before distributing preview installers, release owners should verify that the exact Corresponding Source remains available in a GPL-compliant manner or publish/mirror the required source material. This is intentionally listed as an unresolved human/legal review item rather than being claimed solved by a hyperlink.

## Downloaded later, not bundled in the installer

Whisper model weights are intentionally not included in Scriptotar Next installers. A selected model is downloaded on first uncached use and cached in the user's Scriptotar application-data location. Model repositories can have their own licenses, notices and usage terms; those model artifacts are outside the installer inventory above.

Remote media downloaded at the user's request is likewise not a component distributed in the Scriptotar installer.

## System-provided runtime components

On Linux, **WebKitGTK/GTK and their platform libraries are system package/runtime dependencies**, not files copied into Scriptotar's `transcription-runtime` resource directory. The Debian package also declares `zenity` as a system dependency. Build CI installs development packages so Tauri can compile; end users receive runtime libraries through their operating-system package manager.

On Windows, the Tauri webview/platform runtime is supplied by the Windows/WebView2 environment rather than by Scriptotar's transcription-runtime resources.

## Legacy Scriptotar Classic

Scriptotar Classic remains a separate Python/Tkinter application in this repository. Its Tk/Tkinter and Linux secret-service integration are legacy/runtime concerns and should not be confused with what the Scriptotar Next installers bundle. This notice focuses on the current Scriptotar Next Windows and Linux distribution path.

## Upstream references

- FFmpeg: https://ffmpeg.org/ and https://github.com/FFmpeg/FFmpeg
- static-ffmpeg: https://github.com/zackees/static_ffmpeg
- static-ffmpeg binary archive: https://github.com/zackees/ffmpeg_bins
- BtbN FFmpeg builds: https://github.com/BtbN/FFmpeg-Builds
- gyan.dev FFmpeg builds: https://www.gyan.dev/ffmpeg/builds/
- PyInstaller: https://pyinstaller.org/
- faster-whisper: https://github.com/SYSTRAN/faster-whisper
- CTranslate2: https://github.com/OpenNMT/CTranslate2
- yt-dlp: https://github.com/yt-dlp/yt-dlp
- curl-cffi: https://github.com/lexiforest/curl_cffi
- Hugging Face Hub: https://github.com/huggingface/huggingface_hub
- tokenizers: https://github.com/huggingface/tokenizers
- Tauri: https://github.com/tauri-apps/tauri
- Svelte: https://github.com/sveltejs/svelte
