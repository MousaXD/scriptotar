# Third-party notices

Scriptotar source code is distributed under the Apache License 2.0. Scriptotar Next also distributes and links third-party software under each component's own license. This document describes the current Windows NSIS and Linux Debian preview packages. It is an engineering inventory, not a legal opinion or warranty of compliance.

## Generated package inventory

Every Scriptotar Next package contains a self-contained transcription runtime. During packaging, `sidecars/transcription/runtime_licenses.py` writes:

- `transcription-runtime/RUNTIME-LICENSES.json`;
- `transcription-runtime/legal/THIRD_PARTY_NOTICES.md`;
- the Scriptotar `NOTICE` and Apache-2.0 license;
- the embedded Python runtime license;
- license/NOTICE files exposed by installed Python distributions;
- a verified GNU GPLv3 text for the standalone FFmpeg/ffprobe payload;
- a verified GNU LGPLv3 text for FFmpeg libraries bundled by PyAV.

The generated manifest records the exact installed Python dependency closure and versions used by the package. It also audits two independent FFmpeg distribution paths:

1. standalone `ffmpeg` and `ffprobe` executables fetched through `static-ffmpeg`;
2. FFmpeg shared/native libraries carried inside the PyAV binary wheel.

The existing Windows and Linux packaging workflows run `sidecars/transcription/validate_runtime.py` before Tauri packaging and again against the packaged/installed runtime. The validator now requires the legal bundle and rejects unexpected FFmpeg licensing or `--enable-nonfree`.

When this document and a generated `RUNTIME-LICENSES.json` disagree, treat the package as a release blocker and inspect the built artifact.

## Rust and Tauri desktop application

Scriptotar's Rust crates are Apache-2.0 and are compiled into the desktop executable. Tauri 2 and its Rust runtime are compiled/linked into that application; upstream Tauri uses Apache-2.0 or MIT licensing. Exact Rust dependency versions are locked by `Cargo.lock`.

The application enables rusqlite's `bundled` feature. That means SQLite is compiled into Scriptotar rather than being supplied by the operating system. rusqlite/libsqlite3-sys are MIT-licensed upstream, while SQLite's core source is dedicated to the public domain by the SQLite project.

The Rust toolchain and Tauri CLI are build tools. Their command-line programs are not copied into the installer.

## Svelte frontend

Scriptotar Next ships the generated static frontend consumed by Tauri. Svelte runtime code needed by the compiled frontend can therefore be present in those generated assets. Svelte is MIT-licensed upstream.

Vite, TypeScript, Vitest, jsdom, testing-library packages and other development dependencies are used to build/test the frontend. `node_modules` itself is not copied into the Tauri package.

## Packaged Python transcription runtime

A separate system Python installation is not required by Scriptotar Next. PyInstaller builds the supervisor, engine and dedicated yt-dlp executable and embeds the Python runtime and imported dependencies they need.

Important bundled Python components include:

- `faster-whisper` for transcription/inference integration;
- CTranslate2 for inference;
- `yt-dlp` for media extraction/downloading;
- `curl-cffi` for the yt-dlp HTTP/TLS impersonation extra used by Scriptotar;
- `huggingface-hub` for model acquisition/cache behavior;
- `tokenizers`;
- PyAV (`av`), including its native FFmpeg libraries;
- the remaining installed transitive dependencies resolved from those roots.

Exact versions, project URLs, declared license metadata and discovered license files are recorded in `RUNTIME-LICENSES.json`.

Installed wheel metadata is the primary license source. If a bundled wheel exposes neither usable license metadata nor a license file, packaging fails unless there is a narrow, reviewed fallback for that exact version. The current fallback for `tokenizers 0.23.1` fetches the Apache-2.0 license from the exact upstream `v0.23.1` tag and verifies its SHA-256. A tokenizers version change therefore requires review instead of inheriting the old assumption.

The hosted Linux CPython 3.12.13 installation used by package CI does not always place its license beside the interpreter. When no local interpreter license is found, the builder permits only CPython 3.12.13 and fetches the license from the official `v3.12.13` source tag with a pinned SHA-256. Other interpreter/version combinations fail unless their license is locally available or reviewed.

PyInstaller 6.21.0 is primarily a build tool, but its bootloader is included in generated executables. PyInstaller is GPL-2.0-or-later with its upstream bootloader exception. The runtime legal directory includes PyInstaller license material exposed by the installed distribution.

## Standalone FFmpeg and ffprobe

**FFmpeg and ffprobe are bundled binaries in Scriptotar Next. They are not system-provided.**

`static-ffmpeg==3.0` is used at build time to acquire platform FFmpeg binaries. Scriptotar copies the resulting executables into `transcription-runtime/ffmpeg/`. The `static-ffmpeg` Python package itself is not copied into the runtime.

For each package build, Scriptotar executes the selected FFmpeg binary, records its version/configuration and binary SHA-256 values, then infers the effective FFmpeg license from the actual configure switches. `--enable-nonfree` is rejected. The currently audited standalone binary baseline on Windows and Linux is **GPL-3.0-or-later**, with `--enable-gpl` and `--enable-version3` present.

The package includes a hash-verified copy of GNU GPL version 3 at `legal/FFMPEG-GPL-3.0.txt`.

## FFmpeg libraries bundled through PyAV

PyAV binary wheels bundle FFmpeg libraries, so they are a second distribution path independent of the standalone `static-ffmpeg` executables.

Package CI for the currently resolved PyAV 18.0.0 wheels reports **LGPL-3.0-or-later** for the bundled FFmpeg library groups on both Windows and Linux. Scriptotar records the output of `python -m av --version`, requires the configuration to remain compatible with the audited LGPLv3 baseline, rejects GPL/nonfree drift, and hashes the wheel's native `av.libs`/equivalent payload.

The package includes a hash-verified copy of GNU LGPL version 3 at `legal/FFMPEG-LGPL-3.0.txt`. GNU LGPLv3 incorporates GPLv3 terms and requires the GPL and LGPL license documents to accompany covered object code in relevant cases, so both texts are present in the runtime legal directory.

## Downloaded later, not bundled

Whisper model weights are not included in Scriptotar Next installers. A selected model is downloaded on first uncached use and cached in the user's Scriptotar application-data location. Model repositories can have their own licenses, notices and usage terms, so those model artifacts are outside the installer inventory above.

Remote media downloaded at the user's request is also not a component distributed in the Scriptotar installer.

## System-provided runtime components

On Linux, WebKitGTK, GTK and their platform libraries are system package/runtime dependencies rather than files copied into Scriptotar's transcription runtime. The Debian package also declares `zenity` as a system dependency. Build CI installs development packages so Tauri can compile, while end users receive the runtime libraries through their operating-system package manager.

On Windows, the webview/platform runtime is supplied through the Windows/WebView2 environment rather than Scriptotar's transcription-runtime resource directory.

## Items requiring human or legal review

The safeguards above improve package accuracy and make license drift fail CI, but they do not establish a legal-compliance guarantee.

1. **Standalone GPL FFmpeg Corresponding Source.** Scriptotar records the exact executable configuration, provenance where available and SHA-256 values, and ships GPLv3 terms. This repository still does not mirror a Corresponding Source bundle for the exact prebuilt standalone FFmpeg/ffprobe binaries and their GPL-covered linked components. Release owners must verify a GPL-compliant source-delivery method before distributing installers.
2. **PyAV/FFmpeg LGPL obligations.** The current PyAV wheel's FFmpeg libraries audit as LGPL-3.0-or-later, not GPL. LGPLv3 has source, notice and relinking/modified-library requirements that depend on how the covered libraries are conveyed and linked. The package now ships the LGPL/GPL texts and records the native payload, but a human should verify the chosen distribution mechanism satisfies the applicable LGPL requirements for the exact wheel.
3. **curl-cffi native dependency notices.** `curl-cffi` is precompiled and includes a native curl-impersonation stack. Scriptotar copies license/NOTICE files exposed by its installed wheel, but a human should confirm that all native third-party components and attributions inside that wheel are fully represented before claiming exhaustive compliance.
4. **Exhaustive Rust/frontend transitive notices.** `Cargo.lock` and the frontend lockfile define those dependency graphs, but the generated legal manifest currently focuses on the self-contained transcription runtime. It does not yet claim to aggregate every license text for every compiled Rust crate or every third-party code fragment folded into production frontend assets.

## Scriptotar Classic

Scriptotar Classic remains a separate Python/Tkinter application in this repository. Its Tk/Tkinter and Linux secret-service integration should not be confused with what the Scriptotar Next installers bundle. This notice focuses on the current Scriptotar Next Windows and Linux distribution path.

## Upstream references

- FFmpeg: https://ffmpeg.org/ and https://github.com/FFmpeg/FFmpeg
- static-ffmpeg: https://github.com/zackees/static_ffmpeg
- PyAV: https://github.com/PyAV-Org/PyAV
- PyAV FFmpeg builds: https://github.com/PyAV-Org/pyav-ffmpeg
- PyInstaller: https://pyinstaller.org/
- faster-whisper: https://github.com/SYSTRAN/faster-whisper
- CTranslate2: https://github.com/OpenNMT/CTranslate2
- yt-dlp: https://github.com/yt-dlp/yt-dlp
- curl-cffi: https://github.com/lexiforest/curl_cffi
- Hugging Face Hub: https://github.com/huggingface/huggingface_hub
- tokenizers: https://github.com/huggingface/tokenizers
- CPython: https://github.com/python/cpython
- rusqlite: https://github.com/rusqlite/rusqlite
- SQLite: https://www.sqlite.org/copyright.html
- Tauri: https://github.com/tauri-apps/tauri
- Svelte: https://github.com/sveltejs/svelte
