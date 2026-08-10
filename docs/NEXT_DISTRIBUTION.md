# Scriptotar Next distribution

Scriptotar Next `0.1.0` is the current Rust/Tauri/Svelte preview line. Windows and Linux packages include a self-contained transcription runtime, while Scriptotar Classic `1.2.0` keeps its separate Linux release channel and package formats.

## Release identity

The rolling Next GitHub prerelease uses:

```text
tauri-next-latest
```

Package workflows first build versioned artifacts from the Tauri application version:

```text
Scriptotar-Next-0.1.0-x64-setup.exe
Scriptotar-Next-0.1.0-amd64.deb
```

The rolling preview publishes stable channel filenames:

```text
Scriptotar-Next-latest-x64-setup.exe
Scriptotar-Next-latest-amd64.deb
```

`latest` identifies the rolling preview channel; it is not a separate application version. See `docs/VERSIONING.md` for the complete Classic/Next version policy.

## Runtime architecture

A production Next package contains this resource layout:

```text
transcription-runtime/
  scriptotar-transcription[.exe]   JSONL protocol v1 supervisor
  sidecar.py                        packaged runtime marker / compatibility path
  scriptotar-ytdlp[.exe]           dedicated yt-dlp CLI for creator research
  engine/
    scriptotar-engine[.exe]         isolated heavy transcription engine
    ...                             PyInstaller runtime and Python dependencies
  ffmpeg/
    ffmpeg[.exe]
    ffprobe[.exe]
  RUNTIME-VERSIONS.txt
```

Rust owns durable jobs and the application database. The supervisor speaks JSON Lines protocol v1 to the Rust orchestrator. The heavy engine remains isolated so cancellation can terminate media/transcription descendants without moving Python execution into Rust or Svelte.

Creator research uses the dedicated packaged `scriptotar-ytdlp` executable. Production does not treat the transcription supervisor as a general Python interpreter.

Release startup resolves the runtime from Tauri's application resource directory. Development builds keep source-tree/Python overrides unless `SCRIPTOTAR_SIDECAR_*` or `SCRIPTOTAR_YTDLP_EXECUTABLE` overrides are supplied.

## What is bundled

The packaging build includes:

- a Python 3.12 runtime through PyInstaller;
- Scriptotar's transcription supervisor and engine worker;
- a dedicated packaged yt-dlp command for creator research;
- `faster-whisper==1.2.1` and its packaged runtime dependencies;
- `yt-dlp[default,curl-cffi]==2026.7.4`;
- FFmpeg and ffprobe from the pinned `static-ffmpeg==3.0` packaging dependency;
- protocol/runtime provenance in `RUNTIME-VERSIONS.txt`.

The packaged Next application does not require users to install Python, Faster Whisper, yt-dlp, FFmpeg, or ffprobe separately.

## What is downloaded later

Whisper model weights are intentionally not embedded in the installer. The selected Faster Whisper model is resolved on first transcription and cached through `HF_HOME` under the Next application-data directory:

```text
<Scriptotar Next app data>/models
```

An uncached model therefore requires network access on first use. Model retrieval failures are surfaced as retryable engine failures rather than fake completion or silent model substitution.

After the requested model is cached, local-file transcription does not need another model download. URL transcription and creator research can still require network access to the source platform.

## CPU and CUDA behavior

The engine's device policy remains authoritative:

- `cpu` forces CPU inference with the configured CPU compute mode;
- `cuda` explicitly requests CUDA and reports an engine error when required host GPU/driver/runtime support is unavailable;
- `auto` probes for usable NVIDIA support and otherwise uses CPU, including a CPU retry when automatic CUDA model construction fails.

The package does not install or modify NVIDIA drivers. CPU remains the portable path across supported Windows and Linux packages.

## FFmpeg and ffprobe

The package puts its private `ffmpeg` directory first on the child-process `PATH`. Scriptotar's ffprobe calls and yt-dlp post-processing therefore resolve the bundled executables without changing protocol v1.

System FFmpeg is not required for the packaged Next application.

## Windows package

`.github/workflows/windows-tauri.yml` builds the runtime, validates it, embeds it as a Tauri resource, and produces an NSIS current-user installer.

Its installed-package smoke test verifies:

1. packaged engine imports and FFmpeg/ffprobe resolution;
2. supervisor protocol `ready` / `ping` / `shutdown` behavior;
3. silent installation into a clean temporary directory;
4. launch of the installed Tauri executable;
5. creation of a fresh Rust-owned SQLite database in an isolated application-data directory;
6. discovery/validation of the runtime from installed files instead of the repository tree;
7. successful NSIS uninstall invocation.

The Windows `0.1.x` package is a preview. CI does not claim Authenticode signing when signing credentials are absent.

## Linux package

`.github/workflows/linux-tauri.yml` builds the Next Debian preview without replacing the Classic Linux release workflows.

The Linux lane verifies:

- the self-contained runtime before bundling;
- the `.deb` contains the supervisor, dedicated yt-dlp command, engine, FFmpeg and ffprobe;
- the generated Debian package has the declared desktop dependency required by the current native picker path;
- installation on a clean GitHub-hosted Ubuntu runner;
- packaged-runtime discovery and protocol/dependency self-test from the installed resource directory;
- package removal;
- SHA-256 checksum metadata for the staged artifacts.

Next currently publishes a Debian package on Linux. Next AppImage and Flatpak packages are not part of this preview line; those formats remain available through Scriptotar Classic.

## Runtime validation

`sidecars/transcription/validate_runtime.py` is the package runtime smoke test. It proves that the packaged engine can import its pinned runtime, resolves private FFmpeg/ffprobe executables, exposes the dedicated yt-dlp command, and that the public supervisor can complete protocol-v1 startup, ping and orderly shutdown.

It intentionally does not contact social platforms or download a Whisper model. Those are separate operational/canary concerns, not deterministic installer-smoke prerequisites.

## Application data and model cache

The Tauri bundle identifier is:

```text
io.github.mousaxd.scriptotar.next
```

Next stores its Rust-owned database and model cache beneath Tauri's application-data directory. `SCRIPTOTAR_DATA_DIR` can replace that root for tests and diagnostics.

The package does not reuse the Classic application database in place. Legacy import is staged and performed by the migration bridge described in `docs/NEXT_MIGRATION.md`.

## Development overrides

These environment variables remain supported for development, test fixtures and diagnostics:

- `SCRIPTOTAR_SIDECAR_PYTHON`
- `SCRIPTOTAR_SIDECAR_SCRIPT`
- `SCRIPTOTAR_SIDECAR_ENGINE_EXECUTABLE`
- `SCRIPTOTAR_SIDECAR_ENGINE_WORKER`
- `SCRIPTOTAR_YTDLP_EXECUTABLE`
- `SCRIPTOTAR_DATA_DIR`
- `HF_HOME`

Production startup supplies packaged defaults only when an override is not already present.

## Release integrity and signing

Package workflows stage runtime provenance and SHA-256 metadata. The rolling Next release combines validated Windows and Linux artifacts and emits a combined checksum manifest.

No signing credential is stored in the repository, and unsigned artifacts are not described as signed.

Future Windows signing belongs after bundle creation and before final checksum/publication. A future macOS lane must add Apple code signing, hardened-runtime/entitlement review, notarization and stapling before any macOS artifact is called release-ready.

## Clean-machine behavior

For a packaged Next build:

1. install Scriptotar Next;
2. launch the app;
3. Scriptotar creates its application-data directory and Rust-owned SQLite database;
4. transcription launches the bundled supervisor/engine from application resources;
5. creator research invokes the bundled dedicated yt-dlp command;
6. bundled FFmpeg/ffprobe and Python dependencies are used automatically;
7. an uncached Whisper model downloads into the app-data model cache on first use;
8. later jobs can reuse the model cache and persistent engine process.

No repository checkout, manual virtual environment, `pip install`, or system FFmpeg installation is part of the packaged end-user path.

## Known limitations

- Next is a `0.1.x` preview, not the stable release channel.
- Whisper model weights are not bundled.
- GPU acceleration depends on compatible host NVIDIA support.
- Windows preview artifacts are unsigned unless external signing credentials are configured.
- No macOS distributable is currently published.
- Linux Next packaging is Debian-only in this release line.
- Package smoke tests intentionally avoid live Instagram/TikTok/YouTube availability and large model downloads.
- Scriptotar Classic remains supported; successful Next packaging does not by itself authorize deleting the Classic application or release line.
