# Scriptotar Next migration

Scriptotar Next is the in-progress Tauri 2 replacement for the legacy Python/Tkinter desktop application. The legacy application remains supported during this migration wave and must not be removed until parity is demonstrated in a later deliberate change.

## Current status

The integrated architecture is:

```text
Svelte desktop UI
      |
      | typed Tauri commands
      v
Tauri command adapters
      |
      v
Rust application services
      |
      +--------------------------+
      |                          |
      v                          v
SQLite repositories       Job orchestrator
                                 |
                                 | JSON Lines protocol v1
                                 v
                        Python transcription sidecar
                                 |
                         yt-dlp / FFmpeg /
                         faster-whisper
```

Rust owns application persistence and orchestration. Svelte owns presentation. Python owns media/transcription execution only.

The frontend must never open the application SQLite database or launch Python. The transcription sidecar must never open or mutate Scriptotar's application database. Tauri command functions are adapters and should contain no business logic.

## Crate ownership and dependency direction

- `scriptotar-core`: stable domain models, repository contracts, job states and cross-layer DTO-independent contracts.
- `scriptotar-db`: SQLite schema migrations and repository implementations. This is the only crate that owns the application database schema.
- `scriptotar-jobs`: application-level job lifecycle operations such as enqueue, retry and startup recovery.
- `scriptotar-media`: media validation and the Rust representation of sidecar protocol v1.
- `scriptotar-orchestrator`: persistent sidecar process management, serialized execution, cancellation, progress handling and transcript-result commits.
- `scriptotar-ai`: AI provider contracts, provider execution and endpoint security policy.
- `scriptotar-research`: research-provider execution and network URL policy.
- `apps/desktop/src-tauri`: Tauri composition root and thin typed commands.
- `apps/desktop-ui`: Svelte/TypeScript presentation and a single `ScriptotarApi` host seam.
- `sidecars/transcription`: Python media/transcription worker and its protocol implementation.

Dependencies should point inward toward domain contracts. Core code does not import Tauri, Svelte or Python implementation details. Provider implementations may depend on core contracts, but core contracts must not depend on provider implementations.

## IPC boundary

The browser-facing API is `apps/desktop-ui/src/api/`. Components call `ScriptotarApi`; they do not scatter raw Tauri calls throughout the view tree. In a browser-only development run the mock client implements that interface. In a Tauri window `main.ts` injects the real Tauri invoke function.

Tauri handlers call `AppServices`. A new command should therefore be implemented in this order:

1. define or reuse a domain/application contract;
2. implement the operation in a service or repository;
3. expose a small serializable command DTO;
4. add the command adapter;
5. add it to `ScriptotarApi` if the frontend needs it.

Do not put SQL, process spawning, API-key policy or provider network policy in command functions.

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

Transitions are validated by `JobState`. A caller cannot write an arbitrary lifecycle string.

A job is inserted as `queued` before the orchestrator receives it. The orchestrator moves it to `preparing` before it sends work to the sidecar. Sidecar progress drives validated stage transitions. A transcript result is persisted together with the final `processing -> completed` change in a database transaction.

On startup, jobs left in active states are classified `interrupted`. Scriptotar does not claim that an interrupted Whisper operation resumed. Retry is an explicit `interrupted -> queued` transition. Durable restart behavior is tested by closing and reopening the same SQLite database.

A sidecar error records `failed`. A sidecar cancellation acknowledgement records `cancelled`. Unexpected sidecar closure classifies an active job as `interrupted`. The queue continues after controlled errors and engine crashes.

## Transcription sidecar contract

Protocol v1 is documented in `sidecars/transcription/PROTOCOL.md`. The host launches the sidecar without a shell and pipes stdin, stdout and stderr independently.

- stdout is JSON Lines protocol only;
- stderr is diagnostic output only;
- the host waits for `ready` and verifies protocol compatibility;
- `transcribe` contains flat `job_id`, `input`, `output` and `options` fields;
- `cancel` targets a job ID;
- orderly application shutdown sends `shutdown` and retains a host-side kill fallback;
- unexpected sidecar death is never treated as completion.

The Rust protocol structs intentionally mirror the strict Python schema. Do not loosen Python unknown-field validation to accommodate a mismatched Rust message.

## Database migration policy

The new database uses transactional migrations, foreign keys and explicit schema-version records. Additive integration migrations are versioned separately so a partially applied migration cannot be mistaken for success.

Rules for future migrations:

1. never mutate schema ad hoc from UI code;
2. run schema changes in a transaction whenever SQLite permits it;
3. add indexes for new query paths deliberately;
4. preserve foreign-key relationships;
5. make upgrade tests start from an older schema and prove the resulting version;
6. do not store AI API keys in SQLite;
7. retain restrictive local file permissions on Unix-like systems.

Transcript search uses SQLite FTS5 when available with a conservative fallback. Vector/semantic search is intentionally not part of this migration wave.

## Legacy data import

The legacy `history.sqlite3` contains projects, job history, research items, watchlists and AI-run history. The Next importer:

- refuses to import the destination database into itself;
- creates a backup of the legacy database before import;
- fingerprints the source by canonical path, size and modification time;
- records completed imports;
- skips an already imported fingerprint;
- imports in a single destination transaction;
- maps arbitrary legacy identifiers to deterministic UUID v5 values;
- imports unknown/running legacy job states as `interrupted` rather than pretending they completed or resumed.

The legacy database is not deleted. Import is migration assistance, not permission to remove the old application.

## AI providers

`scriptotar-ai` owns provider contracts, HTTPS execution, response parsing, timeouts and endpoint policy. Production BYOK execution currently supports:

- OpenAI through the Responses API;
- Anthropic through the Messages API;
- Gemini through `generateContent`;
- common OpenAI-compatible `/chat/completions` servers.

Provider rules:

1. pass BYOK secrets only for the request that needs them;
2. never add API keys to `ApplicationSettings`, browser storage, SQLite, provider errors or normal logs;
3. apply `EndpointPolicy` before attaching a key to a custom endpoint;
4. require HTTPS for non-loopback custom endpoints;
5. allow plaintext HTTP only for localhost or loopback development endpoints;
6. reject embedded URL credentials and malformed endpoints;
7. keep model validation structural rather than maintaining a fragile exhaustive remote-model allowlist;
8. cap prompt/provider-response sizes and return explicit timeout/provider/parse failures.

Copy Prompt mode requires no API key and does not contact an AI provider. Preparing a Copy Prompt run can be stored in local AI history. Completed BYOK runs persist project, task, provider, model, prompt, result and timestamps, but never the session API key.

Secure credential-store persistence is not implemented in this wave. Keys remain session-only. Local-model provider execution also remains explicitly unavailable rather than pretending to succeed.

Provider tests use local/mock HTTP boundaries and response fixtures. CI does not spend provider credits or require live OpenAI, Anthropic or Gemini availability.

## Research providers

`scriptotar-research` owns URL/network policy and the production creator-scan provider. Creator scanning uses `yt-dlp` without a shell and accepts only the intended Instagram, TikTok and YouTube domain allowlist. Lookalike hosts, embedded credentials, unsupported schemes and non-standard ports are rejected before provider execution. Provider-returned media URLs are validated again before persistence or queueing.

Research observations are normalized into Rust-owned `ResearchItem` records. The application stores useful normalized metadata such as source URL, platform, title, view/like/comment counts, publish date, duration, creator relationship and scan timestamp. Only a small sanitized metadata subset is retained from extractor output, with explicit size limits.

Research persistence is project-scoped and deduplicates repeated scans by project plus source URL. Selected research items enter transcription through the normal `JobService` and orchestrator boundary; research code does not duplicate transcription execution.

Watchlists support:

- local project-scoped persistence;
- manual refresh from the Research view;
- configured in-process refresh intervals while the desktop app is running;
- last-successful-scan timestamps;
- deduplicated research refreshes.

Saving a watchlist itself does not contact a social platform. Automatic refresh is not a closed-app daemon: when Scriptotar is not running, watchlists are not scanned in the background.

The research provider discovers `yt-dlp` through `SCRIPTOTAR_YTDLP_EXECUTABLE`, the configured sidecar Python environment, or the system `yt-dlp` executable. Production packaging/provisioning of that executable remains a distribution concern and must be validated by the packaging work before self-contained installation is claimed.

Research tests use local fixture executables and policy tests rather than depending on live social-platform availability.

## Adding a job type

A new job type needs a domain-level input/contract, validated lifecycle behavior, repository persistence and an orchestrator implementation. UI buttons should enqueue typed work through Tauri rather than creating their own background tasks. If the job uses an external worker, keep that worker behind a host contract and persist job state before starting execution.

## Developer setup

### Rust

```bash
cargo fmt --all -- --check
cargo check --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

Tauri development additionally requires the normal Linux WebKit/GTK dependencies on Linux.

### Frontend

```bash
cd apps/desktop-ui
npm ci
npm run check
npm run test
npm run build
```

A normal Vite browser run uses mock data. A Tauri run injects the real command client.

### Sidecar

```bash
cd sidecars/transcription
PYTHONPATH=. python3 -m compileall -q .
PYTHONPATH=. python3 -m unittest discover -s tests -v
python3 -m pip install -r requirements-engine.txt
python3 -m pip check
```

Engine dependencies are installed separately. CI verifies dependency consistency plus `yt_dlp` and `faster_whisper` imports without downloading a Whisper model.

## CI quality gate

`.github/workflows/integration.yml` separates Rust, frontend, sidecar, integration, supply-chain and Tauri-build responsibilities. Standard GitHub-hosted runners are used. The workflow does not rely on the repository owner's Azure/self-hosted machines.

The integration lane uses local fixtures/fakes so Instagram, TikTok and YouTube availability cannot determine CI success. AI provider tests likewise use local/mock boundaries rather than spending API credits. The Tauri lane must compile the integrated desktop application, not merely run unit tests.

Supply-chain checks include Rust advisories, npm high-severity auditing, Python dependency auditing and Dependabot configuration.

## Packaging status

**Compile-ready application: yes.** PR validation builds the production Svelte assets and compiles the integrated Tauri application with `cargo tauri build --no-bundle`.

**Fully self-contained distributable: no.** The migration does not yet prove a bundled Python runtime, transcription sidecar environment, FFmpeg distribution, Whisper model/runtime resources, installed-package behavior, or release/upgrade path for the Tauri replacement.

The existing legacy Debian/AppImage/Flatpak release pipeline is intentionally preserved. The Tauri app does not silently replace the stable legacy packages.

A later packaging parity change should establish Tauri Debian/AppImage/Flatpak artifacts, sidecar/Python-environment placement, FFmpeg/model distribution, upgrade behavior and rollback testing before changing release names or replacing stable downloads.

## Legacy versus Next

The legacy Python/Tkinter application remains the production/reference application during this wave. Scriptotar Next now has real BYOK AI-provider execution and creator research in addition to the integrated persistence, UI and transcription architecture, but replacement parity is not claimed merely because those services work or a Tauri binary compiles.

Known intentionally deferred parity includes local-model AI execution, production Next packaging/runtime placement and broader feature-by-feature validation against the legacy UI.

Until those remaining gaps are closed and the final integration agent validates the exact integration SHA, the correct release recommendation remains **not ready to replace the legacy application**.
