# Scriptotar Next Rust backend architecture

This document defines the current Rust ownership boundaries for Scriptotar Next. The Rust/Tauri/Svelte application is now integrated and packaged as the `0.1.x` preview line. The Python/Tkinter Scriptotar Classic application remains supported separately.

## Dependency direction

The Rust workspace follows one-way dependencies. UI and host code depend inward on application/domain boundaries; domain crates do not depend on Tauri or Svelte.

```text
Svelte frontend
      |
      | ScriptotarApi / Tauri IPC
      v
apps/desktop/src-tauri
      |
      | application composition
      v
AppServices
      |
      +----------------------+-----------------------+
      |                      |                       |
      v                      v                       v
scriptotar-jobs       scriptotar-ai          scriptotar-research
      |                                              |
      v                                              v
scriptotar-core                              packaged yt-dlp
      ^
      |
scriptotar-db

AppServices -> scriptotar-orchestrator -> scriptotar-media -> Python sidecar protocol v1
```

The diagram shows ownership rather than every Cargo dependency edge.

## Crate responsibilities

### `scriptotar-core`

Owns stable domain models, repository traits, application settings that are safe to persist, and the job state machine. It contains no Tauri UI state, SQLite implementation, provider HTTP client, or Python process implementation.

### `scriptotar-db`

Owns the Next SQLite schema, migrations, repository implementations, transcript/search persistence, research/watchlist persistence, AI-run persistence, and legacy import logic. It is the only crate that owns the application database schema.

### `scriptotar-jobs`

Owns application-level job lifecycle operations such as enqueue, retry, and startup recovery. It cannot write arbitrary lifecycle strings; transitions pass through the typed `JobState` rules.

### `scriptotar-media`

Owns typed media policy and the Rust representation of the sidecar protocol. It does not reimplement Faster Whisper.

### `scriptotar-orchestrator`

Owns the persistent sidecar host process, serialized job execution, cancellation, progress persistence, unexpected-process recovery, and transcript-result commits.

### `scriptotar-ai`

Owns AI provider contracts, provider HTTP execution, and endpoint security policy. API keys are request-time values and do not belong in `ApplicationSettings` or normal SQLite persistence.

### `scriptotar-research`

Owns supported source/network policy and the yt-dlp-backed creator/profile research provider. Production uses the dedicated packaged `scriptotar-ytdlp` executable.

### `apps/desktop/src-tauri`

Owns application composition, Tauri lifecycle, native desktop picker adapters, packaged-resource configuration, and thin typed IPC commands. Business logic belongs in services/crates rather than command handlers.

## Frontend boundary

`apps/desktop-ui/src/api/` is the host seam for Svelte. Components call `ScriptotarApi`; they do not open SQLite, launch Python, or scatter raw Tauri calls through the component tree.

Browser-only development can inject a mock client. The production Tauri application injects the real invoke-backed client.

When adding a frontend-visible operation:

1. define/reuse the Rust domain or service contract;
2. implement persistence/provider/process behavior below Tauri;
3. expose a narrow serializable DTO;
4. add a thin Tauri command;
5. extend `ScriptotarApi` and its tests.

## Database ownership and migration rules

The Next database uses SQLite with foreign keys, WAL journaling, synchronous durability settings, busy-timeout handling, and transactional schema migrations. Schema/version records are Rust-owned.

Important rules:

- never mutate the schema from Svelte or Tauri command glue;
- use transactions for schema/data invariants whenever SQLite permits it;
- preserve foreign keys and indexes deliberately;
- test upgrades from older schemas;
- do not store AI API keys in the database;
- retain restrictive private-file behavior on Unix-like systems;
- never treat the Classic source database as the Next database in place.

Transcript search uses SQLite search infrastructure with a conservative fallback where required. Semantic/vector search remains outside the current foundation.

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

`completed` is terminal. Restart recovery changes active work to `interrupted`; queued work remains queued. Retrying failed/cancelled/interrupted work is an explicit transition back to `queued`.

A job is persisted before orchestration starts. Sidecar events drive validated state changes. Transcript persistence and completion are owned below the frontend.

## Sidecar contract

The Rust media/orchestrator boundary speaks protocol version `1` to `sidecars/transcription`.

Host process rules:

- spawn without a shell;
- keep stdin/stdout/stderr separate;
- stdout is protocol JSON only;
- wait for `ready` and negotiate protocol compatibility;
- persist work before sending `transcribe`;
- validate event order/identity;
- send explicit `cancel` and `shutdown` commands;
- classify unexpected sidecar death as interrupted/failed work, never completion.

Packaged builds resolve the supervisor/engine/yt-dlp/FFmpeg runtime from Tauri resources. Development mode can use source-tree or explicit environment overrides without changing the wire protocol.

## Legacy migration boundary

The new database is separate from Classic's `history.sqlite3`.

The desktop startup bridge discovers supported Classic/WesamBoss locations, rejects unsafe candidates, and stages exactly one valid candidate through SQLite online backup before import. Multiple distinct candidates are not guessed. The source database is not overwritten or deleted.

See `docs/NEXT_MIGRATION.md` for the current migration behavior.

## AI and research boundaries

Provider execution is integrated below the UI:

- AI: OpenAI, Anthropic, Gemini, and OpenAI-compatible BYOK flows plus Copy Prompt mode.
- Research: yt-dlp-backed creator/profile scanning, normalized metadata persistence, queueing, and watchlist refresh behavior.

Network/credential policy stays in the relevant Rust crates. CI uses deterministic local fixtures rather than depending on live social platforms or paid AI calls.

## Where new code belongs

Put a rule/model in `scriptotar-core` when it is true regardless of storage, UI, or provider implementation.

Put SQLite statements and migrations in `scriptotar-db`.

Put job lifecycle operations in `scriptotar-jobs`.

Put AI request execution and endpoint policy in `scriptotar-ai`.

Put source-platform research/network behavior in `scriptotar-research`.

Put sidecar protocol/media validation in `scriptotar-media` and host process orchestration in `scriptotar-orchestrator`.

Put Tauri lifecycle, native desktop adapters, packaged-resource setup, and narrow IPC adapters in `apps/desktop/src-tauri`.

Do not move logic into a Tauri command merely because the frontend needs it. Add a service/domain operation first, then expose the minimum command needed.

## Current architectural limitations

The current architecture is production-packaged but still a preview product line. Known follow-up areas include stable release/signing policy, macOS packaging, additional Linux Next formats, durable active-project selection across restarts, and deeper installed-app/real-model canary coverage.

Those follow-ups should extend the existing boundaries rather than bypassing them.
