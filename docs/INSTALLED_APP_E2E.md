# Installed Scriptotar Next end-to-end test

The Linux Tauri packaging workflow includes a deterministic installed-app test for the Scriptotar Next transcription path.

## What the test proves

The package job builds the normal self-contained transcription runtime and Debian package, installs the generated `.deb`, validates the runtime from its installed filesystem location, and launches the installed Scriptotar executable under Xvfb to prove the packaged desktop shell reaches a stable running state.

The functional test then drives the same Tauri command functions used by the frontend through Tauri's mock-runtime IPC layer. It deliberately uses a clean application-data directory and the transcription supervisor copied from the installed Debian package.

The covered path is:

1. frontend Tauri client command-name/argument contract;
2. Tauri IPC command dispatch;
3. `AppServices` project creation and selection;
4. local-media validation and `JobService` persistence;
5. the production `JobOrchestrator` queue;
6. the installed packaged `scriptotar-transcription` supervisor and installed `sidecar.py` protocol host;
7. a deterministic subprocess fixture engine that emits the real internal engine protocol without network access or Whisper model downloads;
8. sidecar progress/result events back through the production protocol;
9. SQLite persistence of job, source, media and transcript records;
10. bootstrap/list APIs exposing the completed job, transcript and library item;
11. transcript artifacts written under the configured output root;
12. destruction and recreation of `AppServices` against the same application-data directory, followed by verification that the completed job and transcript remain available.

The functional round trip runs three times in the Linux package job to make synchronization flakes visible. Polling uses a bounded deadline rather than fixed multi-second sleeps.

## Fixture boundary

`sidecars/transcription/tests/installed_app_fixture_engine.py` replaces only the expensive inference worker. It is launched as a child process by the installed packaged sidecar supervisor. The fixture does not bypass Rust orchestration, Tauri command dispatch, the sidecar protocol, output artifact creation or SQLite persistence.

The fixture never contacts Instagram, TikTok, YouTube, Hugging Face or an AI provider and does not download a Whisper model. Real model acquisition and faster-whisper/CTranslate2 inference belong to the separate real-runtime canary.

## UI coverage boundary

The package job launches the actual installed desktop executable under a virtual X display, but it does not currently automate WebView clicks or DOM assertions. Frontend-to-command naming is covered by the frontend Tauri client contract test, while command execution is covered through Tauri IPC in Rust.

This layered design keeps the installed-package gate deterministic on Linux while still exercising the production backend boundary. Full WebDriver interaction can be added separately if it can be kept stable without weakening this package gate.

## Windows

The installed functional transcription E2E currently runs on Linux. Windows retains a strict installed-package smoke with two independent checks:

1. the normally installed GUI executable must launch and remain alive during its startup window;
2. the same installed executable is invoked with the CI-only `--scriptotar-installed-backend-smoke` flag against a clean `SCRIPTOTAR_DATA_DIR`; that path constructs the real `AppServices`, runs database migrations/recovery, creates the default project when needed, exercises bootstrap, exits successfully, and must leave `scriptotar.sqlite3` behind.

The Windows job then validates the packaged transcription runtime from the installed directory and requires a successful NSIS uninstall. Separating GUI liveness from backend initialization avoids making SQLite proof depend on WebView initialization in a headless Windows runner while preserving both assertions.
