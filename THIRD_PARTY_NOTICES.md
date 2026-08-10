# Third-party notices

Scriptotar source code is distributed under the Apache License 2.0. Scriptotar Next also redistributes and links third-party software under each component's own license. This document is the repository-level map for that distribution evidence. The package-specific files generated during Windows and Linux builds are the source of truth for the exact bytes shipped by a release candidate.

This is an engineering inventory, not a legal opinion or warranty of compliance.

## Generated package evidence

Every Scriptotar Next package contains a self-contained `transcription-runtime` resource directory. The package build generates and the installed-package validators require:

- `RUNTIME-LICENSES.json`: exact installed Python dependency closure, Python/PyInstaller notices, standalone FFmpeg build flags and hashes, the PyAV-reported FFmpeg core configuration, and packaged curl-native notice paths;
- `RUNTIME-PROVENANCE.json`: exact standalone FFmpeg provider URL, byte-pinned provider archive identity, provider-generation lineage, runtime hashes/configuration, exact FFmpeg core source revision where proven, and explicit Corresponding Source status;
- `NATIVE-COMPONENTS.json`: exact PyAV and curl-cffi wheel filenames/hashes, native-file hashes, reviewed PyAV source recipe, curl-cffi native component inventory, binary evidence, and versioned native notice hashes;
- `RUST-LICENSES.json`: the locked Rust/Tauri dependency closure reachable from `scriptotar-desktop`, categorized as shipped runtime or build/proc-macro support;
- `FRONTEND-LICENSES.json`: lock/declaration metadata for the complete npm lockfile, retaining build/development classification as a separate audit signal;
- `FRONTEND-BUNDLE-LICENSES.json`: exact npm packages proven to contribute code to the Vite production output by generated source maps, regardless of whether npm labels them `dev` dependencies;
- `legal/THIRD_PARTY_NOTICES.md`: generated human-readable package summary;
- `legal/rust/`: license/notice material discovered for shipped registry Rust crates;
- `legal/frontend/`: exact license/notice files extracted from the integrity-verified npm tarballs that contribute to production output;
- `legal/python/`: license/notice material discovered from installed Python distributions;
- `legal/curl-native/`: exact versioned upstream license/notice files for the reviewed curl-cffi native stack, each hash-pinned in `NATIVE-COMPONENTS.json`;
- `legal/python-runtime/`: embedded CPython license material;
- `legal/build-tools/`: PyInstaller license/bootloader material exposed by the installed build distribution;
- `legal/FFMPEG-GPL-3.0.txt` and `legal/FFMPEG-LGPL-3.0.txt`: verified GNU license texts required by the audited FFmpeg paths;
- Scriptotar's `NOTICE` and Apache-2.0 license.

The runtime builder self-validates the final compliance bundle before returning success. Both Windows and Linux package workflows also run the original runtime validator and the final compliance validator against the runtime found in the installed package. Validation fails closed when required evidence disappears, native hashes or version markers drift, FFmpeg configuration changes, bundled frontend contributors change, or packaged notice bytes stop matching their recorded hashes.

## Standalone FFmpeg and ffprobe

Scriptotar Next bundles `ffmpeg` and `ffprobe`; they are not system-provided.

`static-ffmpeg==3.0` defines the provider mapping used by the project. Scriptotar no longer accepts the provider's floating `main/v8.0` archive URL without verification. `sidecars/transcription/distribution_compliance.py` pins the reviewed Linux and Windows archive identities to the Git LFS SHA-256 OIDs committed by `zackees/ffmpeg_bins`, verifies the complete downloaded archive before extraction, then records the installed executable hashes, `ffmpeg -version` output and full configure flags.

The `ffmpeg_bins` archive-generation commit and archived `static_ffmpeg` workflow identify BtbN/FFmpeg-Builds as the Linux x64 upstream provider and gyan.dev as the Windows x64 upstream provider. The Linux binary itself exposes its FFmpeg Git revision and the Windows provider's 8.0.1 release metadata identifies the FFmpeg core revision. Those exact core-source coordinates are recorded in `RUNTIME-PROVENANCE.json`.

The current audited standalone builds enable GPL and version-3 terms and do not enable `--enable-nonfree`; package validation rejects drift from that baseline.

The current prebuilt provider chain still does not prove a retained, complete mapping from the exact redistributed archive to all source/build inputs for every statically linked GPL dependency. `RUNTIME-PROVENANCE.json` therefore reports `partial-core-source-provenance-only` and explicitly sets the full Corresponding Source issue as technically unresolved. A generic FFmpeg source link is not presented as sufficient evidence.

## PyAV and its bundled native FFmpeg stack

PyAV binary wheels are a second native distribution path independent of the standalone FFmpeg executables.

The currently reviewed PyAV version is `18.0.0`. PyAV's own build recipe points the FFmpeg 8.1 wheel line to `PyAV-Org/pyav-ffmpeg` release `8.1.2-1`. The reviewed pyav-ffmpeg recipe pins the FFmpeg 8.1.2 source archive and each enabled codec/TLS/build dependency to explicit source URLs and SHA-256 values. Those coordinates are copied into `NATIVE-COMPONENTS.json`, and a PyAV version change fails generation until reviewed.

The FFmpeg core libraries reported by `python -m av --version` currently identify an LGPL-3.0-or-later configuration with `--enable-version3`, without `--enable-gpl` or `--enable-nonfree`. That core report is not treated as the license of every separate native library carried by the wheel. The reviewed wheel recipe also includes separately licensed components such as x264 and x265, so `NATIVE-COMPONENTS.json` records the native stack component-by-component.

The package includes GPLv3/LGPLv3 texts, exact source coordinates and hashes, the resolved PyAV wheel filename/hash, and hashes for the native files exposed by the installed PyAV distribution. Whether the final installer mechanism satisfies every applicable LGPL replacement/relinking requirement, and how GPL obligations for separately conveyed codec libraries apply to the complete distribution, remains a legal/distribution interpretation rather than a software-detectable fact.

## curl-cffi native stack

`curl-cffi` is not treated as pure Python. The current reviewed version is `0.15.0`; its upstream build script selects `curl-impersonate` `1.5.2` and curl `8.15.0`.

On Linux, curl-cffi statically folds the downloaded libcurl-impersonate mega archive into `curl_cffi/_wrapper`; the compliance generator therefore does not rely on filenames alone. It requires reviewed embedded version markers for curl 8.15.0, BoringSSL commit `673e61fc215b178a90c0e67858bbf162c8158993`, Brotli 1.2.0, libidn2 2.3.7, nghttp2 1.63.0, nghttp3 1.15.0, ngtcp2 1.20.0, zlib 1.3.1 and zstd 1.5.6, and it hash-pins the wrapper containing those markers.

On Windows, curl-cffi's reviewed build recipe dynamically links libcurl-impersonate and its native library set. The native wheel files are hashed, while Crypt32, Secur32, wldap32, Normaliz and iphlpapi are classified as Windows system libraries rather than copied third-party payloads.

For the reviewed third-party native stack, the package fetches license/notice material from immutable source revisions or a hash-pinned source archive, stores it under `legal/curl-native/`, and records the exact notice SHA-256 values in `NATIVE-COMPONENTS.json`. libidn2 additionally carries its GPLv2, LGPLv3 and Unicode terms. The top-level curl-cffi MIT declaration is never used as a blanket license for the linked native dependencies.

A curl-cffi version change, reviewed native component/version change, missing Linux marker, changed Windows native shape, missing notice or notice hash mismatch fails compliance validation.

## Rust/Tauri inventory

The shipped Rust dependency closure is derived reproducibly with `cargo metadata --locked` from the exact `Cargo.lock` graph rooted at `scriptotar-desktop` for the package target. Development-only dependencies are excluded from the shipped closure; build dependencies and proc-macro support are recorded separately as build-only.

Generation fails if a reachable package has no license metadata or if a shipped package contains one of the project's documented blocked/unreviewed license markers. Registry-crate license/notice files found in the local Cargo source tree are copied into `legal/rust/` and referenced from `RUST-LICENSES.json`.

## Frontend inventory

Frontend compliance deliberately keeps two views because npm declaration metadata and production output are not the same thing.

`FRONTEND-LICENSES.json` records the lock/declaration graph from `apps/desktop-ui/package-lock.json`. It is useful for detecting metadata drift and for separating the wider development/build toolchain.

`FRONTEND-BUNDLE-LICENSES.json` is the shipped-code inventory. The package builder runs the locked frontend build with source maps enabled, reads the Vite/Rollup source maps, maps every `node_modules` source that contributed to the production output back to its exact package-lock entry, and records version, license, resolved tarball URL and integrity. This means a package can be declared under `devDependencies` and still be correctly treated as production-bundled when its runtime code appears in the generated JavaScript.

For every production-bundled npm package, the builder downloads the exact locked tarball, verifies npm Subresource Integrity, extracts its top-level license/notice material into `legal/frontend/`, and hashes those packaged notice files. Missing license metadata, blocked/unreviewed license markers, tarball integrity failure, an empty/invalid source-map graph, or output-graph drift fails the final validator.

The installer contains generated frontend assets and the corresponding license evidence. It does not copy the Node/npm, Vite or TypeScript command-line toolchain merely because those tools were used during the build.

## Packaged Python runtime

The transcription supervisor, engine and dedicated yt-dlp executable are built with PyInstaller and include the Python runtime and imported dependencies they require. Important roots include faster-whisper, CTranslate2, yt-dlp, curl-cffi, Hugging Face Hub, tokenizers and PyAV. The exact installed closure, versions, project URLs, declared license metadata and discovered license files are recorded in `RUNTIME-LICENSES.json`.

If a bundled Python wheel exposes neither usable license metadata nor a license file, generation fails unless a narrow exact-version fallback has been reviewed and hash-pinned. The existing tokenizers and hosted-CPython fallbacks follow that rule.

PyInstaller is primarily a build tool, but its bootloader is included in generated executables. Its installed legal material is therefore copied into the package evidence.

## System-provided and downloaded later

On Linux, WebKitGTK/GTK and declared Debian runtime dependencies are supplied by the operating system package manager rather than copied into the transcription runtime. On Windows, the webview/platform runtime and the Windows libraries identified above are supplied by the Windows environment.

Whisper model weights are downloaded on first uncached use and are not installer payloads. User-requested remote media is likewise not a Scriptotar-distributed component.

## Technically resolved

The repository now enforces automatically:

- byte-pinned standalone FFmpeg provider archives and exact installed FFmpeg/ffprobe hashes/configuration;
- machine-readable FFmpeg provider/core-source provenance without overstating full Corresponding Source availability;
- exact PyAV wheel/native-file fingerprints plus a pinned native source recipe;
- curl-cffi wheel/native-file fingerprints, reviewed native component markers/shapes, versioned packaged notices and notice hashes;
- locked Rust transitive inventory and shipped-crate notice collection;
- frontend lock/declaration inventory plus exact Vite production-output inventory and integrity-verified npm notices;
- package inclusion and installed-runtime validation on Windows and Linux;
- fail-closed drift detection for the reviewed compliance evidence above.

## Remaining engineering blocker and legal interpretation

One distribution engineering blocker remains for the current standalone FFmpeg binaries: Scriptotar still needs a defensible complete Corresponding Source delivery path for the exact byte-pinned statically linked GPL builds, or it must replace/build those binaries from a source recipe whose complete source inputs can be retained and validated. The repository now proves this gap precisely, but software cannot manufacture missing historical provider source provenance.

Separately, genuine legal/distribution interpretation is still required to confirm the applicable LGPL replacement/relinking mechanism for the PyAV wheel and the GPL source/notice treatment of its separately conveyed GPL codec libraries. Those questions depend on how the completed distribution is legally characterized, not on missing inventory automation.

Rust/frontend transitive inventory generation, curl-native notice collection, native wheel fingerprinting, installed package evidence checks and provenance drift detection are not deferred to manual review.

## Upstream references

- FFmpeg: https://ffmpeg.org/ and https://github.com/FFmpeg/FFmpeg
- static-ffmpeg: https://github.com/zackees/static_ffmpeg
- static-ffmpeg binary provider: https://github.com/zackees/ffmpeg_bins
- BtbN FFmpeg builds: https://github.com/BtbN/FFmpeg-Builds
- gyan.dev FFmpeg builds: https://www.gyan.dev/ffmpeg/builds/
- PyAV: https://github.com/PyAV-Org/PyAV
- PyAV FFmpeg builds: https://github.com/PyAV-Org/pyav-ffmpeg
- PyInstaller: https://pyinstaller.org/
- faster-whisper: https://github.com/SYSTRAN/faster-whisper
- CTranslate2: https://github.com/OpenNMT/CTranslate2
- yt-dlp: https://github.com/yt-dlp/yt-dlp
- curl-cffi: https://github.com/lexiforest/curl_cffi
- curl-impersonate: https://github.com/lexiforest/curl-impersonate
- curl: https://github.com/curl/curl
- BoringSSL: https://github.com/google/boringssl
- Brotli: https://github.com/google/brotli
- libidn2: https://www.gnu.org/software/libidn/#libidn2
- nghttp2: https://github.com/nghttp2/nghttp2
- nghttp3: https://github.com/ngtcp2/nghttp3
- ngtcp2: https://github.com/ngtcp2/ngtcp2
- zlib: https://github.com/madler/zlib
- zstd: https://github.com/facebook/zstd
- Hugging Face Hub: https://github.com/huggingface/huggingface_hub
- tokenizers: https://github.com/huggingface/tokenizers
- CPython: https://github.com/python/cpython
- rusqlite: https://github.com/rusqlite/rusqlite
- SQLite: https://www.sqlite.org/copyright.html
- Tauri: https://github.com/tauri-apps/tauri
- Svelte: https://github.com/sveltejs/svelte
