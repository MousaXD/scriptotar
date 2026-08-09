# Scriptotar Rust backend foundation

This document defines the ownership boundaries for the Tauri rewrite while the Python/Tkinter application remains the production behavioral reference.

## Dependency direction

The Rust workspace follows one-way dependencies. UI and host code may depend inward; domain crates do not depend on Tauri.

```text
Svelte frontend (separate branch)
        |
        v
apps/desktop/src-tauri
        |
        +-------------------------------+
        | application services          |
        v                               v
scriptotar-jobs                    policy crates
        |                         /      |       \
        v                        v       v        v
scriptotar-core              ai     research   media
        ^
        |
scriptotar-db
```

`scriptotar-core` contains domain models, the job state machine, settings that are safe to persist, and repository interfaces. It contains no UI state, SQLite, HTTP, subprocesses, or Tauri types.

`scriptotar-db` owns the application SQLite database and migration runner. It implements core repository interfaces. Rust is the only owner of the new application database. Sidecars must return structured results to Rust rather than writing this database.

`scriptotar-jobs` owns lifecycle operations over `JobRepository`. It never invents free-form states. Restart recovery classifies active work as `interrupted`; it does not claim to resume a stage that cannot be safely resumed.

`scriptotar-ai` owns provider contracts and endpoint security policy. Provider secrets are request-time values, are not part of `ApplicationSettings`, and must not be stored in SQLite. Plaintext HTTP endpoints are rejected unless they are loopback addresses.

`scriptotar-research` owns the network allowlist below the UI boundary. The frontend may request research, but it cannot bypass supported-domain URL validation by constructing a raw network request through this crate.

`scriptotar-media` owns typed contracts for yt-dlp, FFmpeg, and the transcription sidecar. It does not reimplement faster-whisper and unit tests do not download models.

`apps/desktop/src-tauri` is an adapter. Tauri command handlers translate IPC arguments and delegate to `AppServices`; command handlers do not contain persistence or lifecycle logic.

## IPC boundary

The initial typed command surface is intentionally small:

- backend health/schema version
- create/list projects
- enqueue/list/cancel/retry jobs
- load/save application settings
- validate research URLs
- validate AI provider endpoints

Agent 4 should extend this surface with job progress channels and sidecar orchestration rather than allowing the frontend to spawn Python or write SQLite directly.

## Database ownership and migration rules

The new database uses SQLite with foreign keys, WAL journaling, `synchronous=FULL`, a busy timeout, and transactional migrations. `PRAGMA user_version` is the authoritative schema version and `schema_migrations` records the applied migration names and timestamps.

Migration 1 creates domain tables and indexes. Migration 2 creates transcript search infrastructure. When SQLite is built with FTS5, `transcript_fts` is a real FTS5 virtual table; otherwise the same migration installs a conservative indexed fallback so the application remains bootable. Triggers keep transcript search data synchronized. Semantic/vector search is deliberately deferred.

All state transitions that affect a job are validated against the core state machine and performed inside an immediate SQLite transaction. A stale or invalid transition fails instead of silently changing state.

On Unix-like systems, application-private storage created by the Rust backend uses mode `0700` for its new directory and the database file is forced to `0600`. API keys are intentionally absent from the persisted settings model.

## Job lifecycle

```text
queued
  -> preparing
       -> downloading -> transcribing -> processing -> completed
       -> transcribing -> processing -> completed

preparing/downloading/transcribing/processing
  -> failed
  -> cancelled
  -> interrupted

failed/cancelled/interrupted
  -> queued
```

`completed` is terminal. Restart recovery changes only active states to `interrupted`; queued work remains queued. Retrying an interrupted job is an explicit transition back to `queued`.

## Sidecar contract

The Rust media crate establishes protocol version `1` and typed host-side command/event shapes for ping, transcribe, cancel, shutdown, ready, progress, result, and error. Agent 3 owns the executable implementation and detailed protocol documentation under `sidecars/transcription/**`.

Expected process rules for integration:

- Rust spawns the sidecar.
- stdin/stdout carry versioned machine-readable messages.
- stdout is protocol-only; logs go to stderr.
- Rust owns the application database.
- cancellation is explicit and must terminate relevant child work.
- a process crash becomes a persisted `interrupted` or `failed` job according to the orchestration point; it is never labeled a successful resume.

## Where new code belongs

Put a rule or model in `scriptotar-core` when it is true regardless of storage, UI, or provider implementation.

Put SQLite statements and migration behavior in `scriptotar-db`.

Put job lifecycle orchestration in `scriptotar-jobs`.

Put AI provider contracts, endpoint rules, and future provider clients in `scriptotar-ai`.

Put source-platform research policy and research provider adapters in `scriptotar-research`.

Put yt-dlp/FFmpeg/sidecar host contracts in `scriptotar-media`.

Put only Tauri lifecycle, state construction, and thin IPC adapters in `apps/desktop/src-tauri`.

Do not move logic into Tauri handlers merely because the frontend needs it. Add a service method or domain capability first, then expose the smallest IPC command needed.

## Deferred work

This foundation intentionally does not port the production Svelte UI, perform live AI HTTP calls, launch the Python sidecar, migrate the legacy Python SQLite data, or implement vector search. Those are integration/provider tasks that can now be added without changing the domain and persistence boundaries.
