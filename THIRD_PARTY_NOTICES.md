# Third-party notices

Scriptotar source code is distributed under the Apache License 2.0. Scriptotar Next also redistributes and links third-party software under each component's own license. This document is the repository-level map for that distribution evidence. The package-specific files generated during Windows and Linux builds are the source of truth for the exact bytes shipped by a release candidate.

This is an engineering inventory, not a legal opinion or warranty of compliance.

## Generated package evidence

Every Scriptotar Next package contains a self-contained `transcription-runtime` resource directory. The package build now generates and the installed-package validator requires:

- `RUNTIME-LICENSES.json`: exact installed Python dependency closure, Python/PyInstaller notices, standalone FFmpeg build flags and hashes, and the PyAV-reported FFmpeg core configuration;
- `RUNTIME-PROVENANCE.json`: exact standalone FFmpeg provider URL, byte-pinned provider archive identity, runtime hashes/configuration and the source-delivery status;
- `NATIVE-COMPONENTS.json`: exact PyAV and curl-cffi wheel filenames/hashes, native-file hashes and reviewed native source/build lineage;
- `RUST-LICENSES.json`: the locked Rust/Tauri dependency closure reachable from `scriptotar-desktop`, categorized as shipped runtime or build/proc-macro support;
- `FRONTEND-LICENSES.json`: the exact frontend lockfile closure, categorized as production or development/build-only;
- `legal/THIRD_PARTY_NOTICES.md`: generated human-readable package summary;
- `legal/rust/`: license/notice material discovered for shipped registry Rust crates;
- `legal/python/`: license/notice material discovered from installed Python distributions;
- `legal/python-runtime/`: embedded CPython license material;
- `legal/build-tools/`: PyInstaller license/bootloader material exposed by the installed build distribution;
- `legal/FFMPEG-GPL-3.0.txt` and `legal/FFMPEG-LGPL-3.0.txt`: verified GNU license texts required by the audited FFmpeg paths;
- Scriptotar's `NOTICE` and Apache-2.0 license.

Both Windows and Linux package workflows run `sidecars/transcription/validate_runtime.py` before packaging and again against the packaged/installed runtime. Validation fails closed when required evidence disappears or when the installed FFmpeg/native payload no longer matches the generated manifests.

## Standalone FFmpeg and ffprobe

Scriptotar Next bundles `ffmpeg` and `ffprobe`; they are not system-provided.

`static-ffmpeg==3.0` defines the provider mapping used by the project. Scriptotar no longer accepts the provider's floating `main/v8.0` archive URL without verification. `sidecars/transcription/distribution_compliance.py` pins the reviewed Linux and Windows archive identities to the Git LFS SHA-256 OIDs committed by `zackees/ffmpeg_bins`, verifies the complete downloaded archive before extraction, then records the installed executable hashes, `ffmpeg -version` output and full configure flags.

The current audited standalone builds enable GPL and version-3 terms and do not enable `--enable-nonfree`; package validation rejects drift from that baseline.

The upstream/provider archive does not currently give Scriptotar a complete deterministic mapping from those prebuilt binaries to all source/build inputs for every statically linked GPL component. `RUNTIME-PROVENANCE.json` therefore records Corresponding Source status as unresolved instead of presenting a generic upstream link as proof of compliance.

## PyAV and its bundled native FFmpeg stack

PyAV binary wheels are a second native distribution path independent of the standalone FFmpeg executables.

The currently reviewed PyAV version is `18.0.0`. PyAV's own build recipe points the FFmpeg 8.1 wheel line to `PyAV-Org/pyav-ffmpeg` release `8.1.2-1`. The reviewed pyav-ffmpeg recipe pins the FFmpeg 8.1.2 source archive and each enabled codec/TLS/build dependency to explicit source URLs and SHA-256 values. Those coordinates are copied into `NATIVE-COMPONENTS.json` and a PyAV version change fails generation until reviewed.

The FFmpeg core libraries reported by `python -m av --version` currently identify an LGPL-3.0-or-later configuration with `--enable-version3`, without `--enable-gpl` or `--enable-nonfree`. That core report must not be treated as the license of every separate native library carried by the wheel. The reviewed wheel recipe also includes separately licensed components such as x264 and x265, so `NATIVE-COMPONENTS.json` records the native stack component-by-component.

The package includes GPLv3/LGPLv3 texts and exact source coordinates, hashes and native-file fingerprints. Whether the final installer mechanism satisfies every applicable LGPL replacement/relinking requirement, and how GPL obligations for separately conveyed codec libraries apply to the complete distribution, remains a legal/distribution interpretation rather than a software-detectable fact.

## curl-cffi native stack

`curl-cffi` is not treated as pure Python. The current reviewed version is `0.15.0`.

Upstream `curl-cffi` 0.15.0 builds against `curl-impersonate` 1.5.2 and curl 8.15.0. Its build configuration uses a statically linked native archive on Linux and a dynamic libcurl-impersonate/native dependency set on Windows. `NATIVE-COMPONENTS.json` records the exact resolved wheel filename/hash, every native file exposed by that wheel, and the reviewed curl/curl-impersonate source lineage. A curl-cffi version change therefore fails until the native recipe is reviewed again.

The top-level curl-cffi MIT declaration is intentionally not used as a blanket license for libcurl-impersonate or the TLS, HTTP/2, HTTP/3, compression and other libraries linked into its native payload. Those components retain their own terms. The machine-readable evidence makes native-shape drift detectable, but exhaustive component-by-component notice/source treatment must follow the exact upstream build inputs rather than being inferred from the Python package metadata.

## Rust/Tauri inventory

The shipped Rust dependency closure is derived reproducibly with `cargo metadata --locked` from the exact `Cargo.lock` graph rooted at `scriptotar-desktop` for the package target. Development-only dependencies are excluded from the shipped closure; build dependencies and proc-macro support are recorded separately as build-only.

Generation fails if a reachable package has no license metadata or if a shipped package contains one of the project's documented blocked/unreviewed license markers. Registry-crate license/notice files found in the local Cargo source tree are copied into `legal/rust/` and referenced from `RUST-LICENSES.json`.

## Frontend inventory

The production frontend inventory is derived from `apps/desktop-ui/package-lock.json` lockfile version 3. Entries marked `dev: true` are categorized as development/build-only; the remaining locked entries form the production dependency inventory. Generation fails on missing license metadata or blocked/unreviewed production license markers.

The installer contains the generated frontend assets, not the `node_modules` directory or the Node/Vite/TypeScript command-line toolchain.

## Packaged Python runtime

The transcription supervisor, engine and dedicated yt-dlp executable are built with PyInstaller and include the Python runtime and imported dependencies they require. Important roots include faster-whisper, CTranslate2, yt-dlp, curl-cffi, Hugging Face Hub, tokenizers and PyAV. The exact installed closure, versions, project URLs, declared license metadata and discovered license files are recorded in `RUNTIME-LICENSES.json`.

If a bundled Python wheel exposes neither usable license metadata nor a license file, generation fails unless a narrow exact-version fallback has been reviewed and hash-pinned. The existing tokenizers and hosted-CPython fallbacks follow that rule.

PyInstaller is primarily a build tool, but its bootloader is included in generated executables. Its installed legal material is therefore copied into the package evidence.

## System-provided and downloaded later

On Linux, WebKitGTK/GTK and declared Debian runtime dependencies are supplied by the operating system package manager rather than copied into the transcription runtime. On Windows, the webview/platform runtime is supplied by the Windows environment.

Whisper model weights are downloaded on first uncached use and are not installer payloads. User-requested remote media is likewise not a Scriptotar-distributed component.

## Remaining release/legal decisions

The generated evidence is designed to eliminate inventory guesswork and make distribution drift fail CI. It does not decide legal questions that software cannot decide.

The remaining release decisions are narrowly:

1. establish a GPL-compliant Corresponding Source delivery strategy for the exact byte-pinned standalone FFmpeg/ffprobe provider archives, or replace that provider with one whose complete source/build lineage can be proven and validated;
2. confirm the applicable LGPL replacement/relinking mechanism for the PyAV wheel and the GPL source/notice treatment required for its separately conveyed GPL codec libraries;
3. confirm the complete notice/source set for the exact native dependency stack folded into each curl-cffi platform wheel once the upstream build inputs have been exhaustively enumerated.

Rust/frontend transitive inventory generation, native wheel fingerprinting, installed package evidence checks and provenance drift detection are no longer deferred to manual review.

## Upstream references

- FFmpeg: https://ffmpeg.org/ and https://github.com/FFmpeg/FFmpeg
- static-ffmpeg: https://github.com/zackees/static_ffmpeg
- static-ffmpeg binary provider: https://github.com/zackees/ffmpeg_bins
- PyAV: https://github.com/PyAV-Org/PyAV
- PyAV FFmpeg builds: https://github.com/PyAV-Org/pyav-ffmpeg
- PyInstaller: https://pyinstaller.org/
- faster-whisper: https://github.com/SYSTRAN/faster-whisper
- CTranslate2: https://github.com/OpenNMT/CTranslate2
- yt-dlp: https://github.com/yt-dlp/yt-dlp
- curl-cffi: https://github.com/lexiforest/curl_cffi
- curl-impersonate: https://github.com/lexiforest/curl-impersonate
- Hugging Face Hub: https://github.com/huggingface/huggingface_hub
- tokenizers: https://github.com/huggingface/tokenizers
- CPython: https://github.com/python/cpython
- rusqlite: https://github.com/rusqlite/rusqlite
- SQLite: https://www.sqlite.org/copyright.html
- Tauri: https://github.com/tauri-apps/tauri
- Svelte: https://github.com/sveltejs/svelte
