# Scriptotar Next distribution

Scriptotar Next packages the Tauri desktop application together with a self-contained transcription runtime. Production packages do not require a repository checkout or a separately installed Python interpreter.

The legacy Python/Tkinter packages remain supported and are not replaced by this work.

## Runtime architecture

A production bundle contains this resource layout:

```text
transcription-runtime/
  scriptotar-transcription[.exe]   JSONL protocol v1 supervisor
  sidecar.py                        packaged runtime marker / compatibility path
  engine/
    scriptotar-engine[.exe]         isolated heavy transcription engine
    ...                             PyInstaller runtime and Python dependencies
  ffmpeg/
    ffmpeg[.exe]
    ffprobe[.exe]
  RUNTIME-VERSIONS.txt
```

The Rust orchestrator still owns durable jobs and speaks the existing JSON Lines protocol v1. The supervisor remains a separate process, and the heavy engine remains a child process so cancellation can terminate transcription/media descendants without moving Python execution into Rust or Svelte.

Release startup resolves the runtime from Tauri's application resource directory. Development builds keep the existing source-tree/Python workflow unless `SCRIPTOTAR_SIDECAR_*` overrides are supplied.

## What is bundled

The packaging build pins and bundles:

- a Python 3.12 runtime through PyInstaller;
- Scriptotar's transcription supervisor and engine worker;
- `faster-whisper==1.2.1` and its packaged runtime dependencies;
- `yt-dlp[default,curl-cffi]==2026.7.4`;
- FFmpeg and ffprobe from the pinned `static-ffmpeg==3.0` packaging dependency;
- protocol/runtime provenance in `RUNTIME-VERSIONS.txt`.

The installed user does not need to install Python, faster-whisper, yt-dlp, FFmpeg, or ffprobe separately.

## What is downloaded later

Whisper model weights are intentionally not embedded in the installer. The selected Faster Whisper model is resolved on first transcription and then cached under Scriptotar Next's application data directory via `HF_HOME`:

```text
<Scriptotar Next app data>/models
```

This keeps installer size bounded and avoids shipping several large model variants that many users will never select.

A first transcription therefore needs network access when its selected model is not already cached. If model retrieval or another engine dependency fails, the job returns an explicit, retryable engine error that explains the first-use model requirement. Scriptotar does not report a fake successful transcription or silently fall back to another model.

Once the requested model is cached, local-file transcription does not require model download access. URL transcription can still require network access to the source platform.

## CPU and CUDA behavior

The existing engine device policy remains authoritative:

- `cpu` forces CPU inference with the existing int8 configuration;
- `cuda` explicitly requests CUDA and reports an engine error if the required host GPU/driver/runtime support is not usable;
- `auto` probes for a usable NVIDIA environment and otherwise uses CPU; if automatic CUDA model construction fails, it retries on CPU.

The package does not install or modify NVIDIA drivers. GPU acceleration therefore depends on compatible host hardware and drivers. CPU remains the portable supported path on Windows and Linux.

## FFmpeg and ffprobe

The package puts its private `ffmpeg` directory first on the child-process `PATH`. This lets both Scriptotar's direct ffprobe calls and yt-dlp's media post-processing use the packaged executables without changing the public sidecar protocol.

System FFmpeg is not required for the packaged Tauri application.

## Windows package

`.github/workflows/windows-tauri.yml` builds the runtime first, validates it, embeds it as a Tauri resource, and then produces an NSIS current-user installer.

The workflow validates more than compilation:

1. packaged engine imports and FFmpeg/ffprobe resolution;
2. supervisor protocol `ready` / `ping` / `shutdown` behavior;
3. silent installation into a clean temporary directory;
4. launch of the installed Tauri executable;
5. creation of a fresh Rust-owned application database in a clean application-data directory;
6. discovery and validation of the runtime from the installed files rather than the repository tree;
7. successful NSIS uninstall invocation.

The Windows artifact remains labeled a preview until the final integration/release-readiness audit and signing policy are complete.

## Linux package

`.github/workflows/linux-tauri.yml` builds a Tauri Debian package without changing or deleting the legacy Linux package workflows.

The Linux lane validates:

- the self-contained runtime before bundling;
- the `.deb` contents include the supervisor, engine, FFmpeg and ffprobe;
- package installation on a clean GitHub-hosted Ubuntu runner;
- packaged-runtime discovery and protocol/dependency self-test from the installed resource directory;
- package removal;
- SHA-256 checksums for the staged artifacts.

This is a Scriptotar Next preview artifact. It does not replace the legacy Debian, AppImage, or Flatpak release names.

## Runtime validation

`sidecars/transcription/validate_runtime.py` is the packaging smoke test. It does not contact social platforms or download a Whisper model. It proves that the packaged heavy engine can import the pinned runtime, resolve private FFmpeg/ffprobe executables, and that the public supervisor can complete protocol-v1 startup, ping and orderly shutdown.

This distinction is intentional: package CI should prove the installed runtime boundary without making success depend on a live third-party media platform or a multi-gigabyte model download.

## Development overrides

These environment variables remain supported for development, test fixtures and diagnostics:

- `SCRIPTOTAR_SIDECAR_PYTHON`
- `SCRIPTOTAR_SIDECAR_SCRIPT`
- `SCRIPTOTAR_SIDECAR_ENGINE_EXECUTABLE`
- `SCRIPTOTAR_SIDECAR_ENGINE_WORKER`
- `SCRIPTOTAR_DATA_DIR` for isolated test/application-data roots
- `HF_HOME` for model-cache placement

Production release startup supplies packaged defaults only when an override is not already present.

## Signing and release integrity

No signing credential is stored in the repository and CI does not claim unsigned artifacts are signed.

Current release-integrity behavior:

- Windows and Linux package jobs preserve a runtime version manifest;
- Linux produces SHA-256 checksums for the staged package/runtime manifest;
- the Windows preview release produces SHA-256 checksums before publication.

Future Windows signing should be inserted after bundle creation and before checksum generation/upload, using a CI secret-backed certificate or signing service. The workflow should verify the signature before release publication and remain capable of ordinary unsigned PR validation when release signing credentials are intentionally unavailable.

A future macOS lane must add Apple code signing, hardened-runtime/entitlement review, notarization and stapling before any macOS artifact is described as release-ready.

## Clean-machine behavior

For a supported packaged build:

1. install Scriptotar Next;
2. launch the app;
3. Scriptotar creates its application-data directory and Rust-owned SQLite database;
4. transcription starts the bundled supervisor and engine from application resources;
5. bundled FFmpeg/ffprobe and Python dependencies are used automatically;
6. on the first job using an uncached Whisper model, the model is downloaded to the app-data model cache;
7. subsequent jobs can reuse the cached model and persistent engine process.

No repository checkout, virtual environment setup, `pip install`, or system FFmpeg installation is part of the end-user path.

## Known limitations

- Whisper model weights are not bundled, so uncached models need a network connection on first use.
- GPU acceleration depends on compatible host NVIDIA support; the installer does not provision GPU drivers.
- Windows signing credentials are not configured by this repository change, so preview CI must not imply Authenticode signing.
- No macOS distributable is produced by this work.
- The Linux Next artifact is Debian-format only in this packaging wave; the legacy AppImage/Flatpak pipelines remain separate.
- Package smoke tests intentionally avoid live Instagram/TikTok/YouTube availability and do not spend bandwidth downloading Whisper weights.
- Final replacement of the Python/Tkinter application is an integration-level decision and is not implied by a successful Agent 1 package build.