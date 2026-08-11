# Scriptotar Next migration

Scriptotar Next is the Rust/Tauri replacement line for the legacy Python/Tkinter application. The migration is **integrated and distributable**, but Next remains a **0.1.x preview** and Classic remains supported.

This document describes the current architecture and migration boundary. It no longer treats already-integrated AI, research, sidecar, or packaging work as future agent tasks.

## Current status

The integrated architecture is:

```text
Svelte desktop UI
      |
      | typed ScriptotarApi / Tauri commands
      v
Tauri command adapters
      |
      v
Rust application services
      |
      +-------------------------------+
      |               |               |
      v               v               v
SQLite repos      Job orchestrator   AI / research services
                      |
                      | JSON Lines protocol v1
                      v
             Python transcription sidecar
                      |
              yt-dlp / FFmpeg /
                faster-whisper
```

Rust owns application persistence, orchestration, AI/research service execution, settings, and the host boundary. Svelte owns presentation. Python owns media/transcription execution only.

The frontend must not open the application SQLite database or spawn Python directly. The sidecar must not open or mutate Scriptotar's application database. Tauri commands are thin adapters over application services.

## Product identity during migration

Two application lines intentionally coexist:

- **Scriptotar Next 0.1.0**: current preview, Windows NSIS and Linux Debian packages.
- **Scriptotar Classic 1.2.0**: supported Python/Tkinter line with Debian, AppImage, and Flatpak packages.

The version numbers are independent. See `docs/VERSIONING.md` for the release/tag policy.

## Crate ownership and dependency direction

- `scriptotar-core`: stable domain models, repository contracts, job states and cross-layer contracts.
- `scriptotar-db`: SQLite schema migrations and repository implementations. This is the only crate that owns the Next application database schema.
- `scriptotar-jobs`: job lifecycle operations such as enqueue, retry and startup recovery.
- `scriptotar-media`: media validation and the Rust representation of sidecar protocol v1.
- `scriptotar-orchestrator`: persistent sidecar process management, serialized execution, cancellation, progress handling and transcript-result commits.
- `scriptotar-ai`: AI provider contracts, request execution and endpoint security policy.
- `scriptotar-research`: research URL policy and yt-dlp-backed creator/profile scanning.
- `apps/desktop/src-tauri`: Tauri composition root and typed command adapters.
- `apps/desktop-ui`: Svelte/TypeScript presentation behind the `ScriptotarApi` seam.
- `sidecars/transcription`: Python media/transcription supervisor and engine.

Dependencies should point inward toward domain contracts. Core code does not depend on Tauri, Svelte or Python implementation details.

## IPC boundary

The browser-facing API is `apps/desktop-ui/src/api/`. Components call `ScriptotarApi`; they do not scatter raw Tauri calls through the view tree.

A browser-only Vite run can use mock data. In the Tauri window, the application injects the real Tauri invoke implementation.

A new command should normally be implemented in this order:

1. define or reuse a domain/application contract;
2. implement the operation in a service or repository;
3. expose the smallest serializable command DTO;
4. add the Tauri command adapter;
5. add the operation to `ScriptotarApi` if the frontend needs it.

Do not put SQL, process spawning, API-key policy or provider network policy in UI components or command adapters.

## Job lifecycle and recovery

Persisted job states are:

```text
queued
preparing
downloading
transcribing
processing
completed
failed
cancelled
interrupted
```

Transitions are validated by the Rust `JobState` state machine.

A job is stored as `queued` before orchestration starts. Sidecar progress drives validated stage transitions. A completed transcription is persisted into the Rust-owned database rather than being treated as frontend-only state.

On startup, work left in active states is classified `interrupted`. Scriptotar does not pretend an interrupted Whisper operation resumed from the middle. Retry is an explicit transition back to `queued`.

Unexpected sidecar death is not completion. Controlled cancellation and failures remain distinguishable and the queue can recover for later work.

## Transcription sidecar contract

Protocol v1 is documented in `sidecars/transcription/PROTOCOL.md`.

The host:

- launches the sidecar without a shell;
- pipes stdin/stdout/stderr separately;
- waits for `ready` and verifies protocol compatibility;
- persists jobs before sending transcription work;
- translates progress/result/error events into Rust-owned state;
- sends `cancel` for user cancellation;
- sends `shutdown` on orderly application exit with a kill fallback;
- treats unexpected process death as interrupted/failed work, never success.

Packaged Next builds use the bundled supervisor/engine executables described in `docs/NEXT_DISTRIBUTION.md`. Development mode can still use source-tree/Python overrides.

## Next data directory

The production application uses Tauri's application-data directory for bundle identifier:

```text
io.github.mousaxd.scriptotar.next
```

Inside it, the Rust application stores:

```text
scriptotar.sqlite3        Next application database
models/                   Whisper model cache
transcription-output/     fallback output root
history.sqlite3           temporary/idempotent legacy-import staging snapshot when migration is needed
```

`SCRIPTOTAR_DATA_DIR` overrides this root for tests and diagnostics.

## Legacy data import

Classic stores its normal Linux data under `scriptotar/history.sqlite3`; the older WesamBoss branding used `wesamboss/history.sqlite3`. Classic Flatpak data lives under its sandboxed XDG data root.

Before `AppServices` opens the Next database, the migration bridge searches supported legacy roots for `scriptotar/history.sqlite3` and `wesamboss/history.sqlite3`.

Safety behavior:

- no candidate: startup continues normally;
- exactly one valid candidate: create a SQLite online-backup snapshot in the Next data directory, then import it;
- multiple distinct candidates: refuse to choose automatically and report the candidates;
- symlinked/non-regular/truncated candidates: reject them;
- source database: never overwritten or deleted;
- import: fingerprinted/idempotent so the same staged legacy database is not repeatedly duplicated.

The SQLite backup path is WAL-aware, so committed WAL state is included in the staged snapshot.

Legacy running/unknown job states are not promoted to fake completion. Migration maps them conservatively into recoverable Next state.

## AI providers

`scriptotar-ai` owns provider execution and endpoint policy.

Current Next behavior includes:

- Copy Prompt mode with no API key;
- BYOK execution for OpenAI;
- BYOK execution for Anthropic;
- BYOK execution for Gemini;
- BYOK execution for OpenAI-compatible chat endpoints;
- endpoint validation that blocks unsafe credential transport;
- request-time API keys that are not stored in normal settings or SQLite.

Provider requests are made below the UI/Tauri command layer. Do not move secret handling into browser storage.

## Research and watchlists

`scriptotar-research` owns source URL/network policy and the yt-dlp-backed research provider.

Current Next behavior includes:

- creator/profile scanning through the dedicated packaged yt-dlp command;
- normalized public research metadata;
- Rust-owned research persistence and deduplication;
- queueing selected research items into the transcription job path;
- saved watchlists with in-app refresh/backoff behavior.

CI does not depend on live Instagram/TikTok/YouTube responses. Deterministic fixtures are used for release health; live platform behavior remains an operational dependency for users.

## Database migration policy

The Next database uses transactional migrations, foreign keys and explicit schema-version records.

Rules for future migrations:

1. never mutate schema from UI code;
2. run schema changes in transactions whenever SQLite permits it;
3. add indexes for new query paths deliberately;
4. preserve foreign-key relationships;
5. make upgrade tests start from an older schema and prove the resulting version;
6. never store AI API keys in SQLite;
7. retain restrictive local file permissions on Unix-like systems;
8. keep legacy import separate from destructive source-database mutation.

## Packaging status

**Self-contained Next runtime: yes.**

Windows and Linux Next packages include the Python supervisor/engine runtime, Faster Whisper dependencies, dedicated yt-dlp executable, FFmpeg and ffprobe. End users do not need a repository checkout or a separate Python/FFmpeg installation for the packaged Next app.

**Whisper models bundled: no.**

Selected model weights download on first uncached use into the Next model cache.

**Current release-ready package formats:**

- Windows NSIS preview installer;
- Linux Debian preview package.

**Not currently published for Next:**

- Linux AppImage;
- Linux Flatpak;
- macOS package/signing/notarization output.

The existing Classic Linux release lanes remain intentionally separate.

## Remaining migration limitations

The integrated Next app is no longer merely compile-ready, but it is not yet the stable channel.

Known current boundaries include:

- unsigned Windows preview packages unless signing credentials are configured externally;
- no macOS distributable;
- no Next AppImage/Flatpak;
- first-use Whisper model download requirement;
- external source-platform variability;
- active project selection currently falls back to Inbox after restart;
- package smoke tests validate the installed runtime boundary but intentionally do not download a large Whisper model or depend on live social sites.

Classic should not be removed merely because Next packages successfully. Deprecation/removal requires a separate deliberate compatibility and release decision.

## Developer validation

Use the commands in `CONTRIBUTING.md` for the Rust workspace, Svelte frontend, sidecar, integrated Tauri build, and preserved Classic tests.

The main integrated quality gate is `.github/workflows/integration.yml`; installed package validation is performed by the Windows and Linux Tauri package workflows.
