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
- `scriptotar-ai`: AI provider contracts and endpoint security policy.
- `scriptotar-research`: research-provider boundary and network URL policy.
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

`scriptotar-ai` owns provider contracts and endpoint policy. To add an AI provider:

1. add/extend a `ProviderKind` and provider implementation behind the provider trait;
2. keep request execution below Tauri/UI layers;
3. pass BYOK secrets only for the request that needs them;
4. never add API keys to `ApplicationSettings`, browser storage or SQLite;
5. apply `EndpointPolicy` before attaching a key to a custom endpoint;
6. keep non-loopback plaintext HTTP blocked by default;
7. add success, provider-error and recovery tests.

Copy Prompt mode requires no API key and remains useful even while live provider execution is incomplete. The desktop UI can build and copy that prompt locally. BYOK validates key presence and endpoint policy, then returns an explicit unavailable error until provider execution is implemented; it does not fake a successful AI response.

## Research providers

`scriptotar-research` owns URL/network policy. Local watchlists are implemented independently of external provider execution: the active project's watchlists are persisted in SQLite, idempotently upserted by profile URL, survive database reopen, and are surfaced through Tauri bootstrap state. Saving a watchlist does not contact a social platform.

Live profile scanning and queueing of provider research results remain intentionally unavailable until a real provider is integrated. Those commands validate their input boundary and then return an explicit unavailable error instead of substituting mock production data. Browser-only development may still use the mock client.

To add a research provider:

1. implement the research-provider contract below the UI layer;
2. validate the profile/source URL with the network policy before network access;
3. return typed/sanitized research items, not raw extractor dictionaries;
4. persist through Rust repositories only;
5. test lookalike domains, unsupported schemes, credentials/SSRF-style input and provider failures;
6. never make CI depend on a live social platform returning stable data.

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

The integration lane uses local fixtures/fakes so Instagram, TikTok and YouTube availability cannot determine CI success. The Tauri lane must compile the integrated desktop application, not merely run unit tests.

Supply-chain checks include Rust advisories, npm high-severity auditing, Python dependency auditing and Dependabot configuration.

## Packaging status

**Compile-ready application: yes.** PR validation builds the production Svelte assets and compiles the integrated Tauri application with `cargo tauri build --no-bundle`.

**Fully self-contained distributable: no.** The migration does not yet prove a bundled Python runtime, transcription sidecar environment, FFmpeg distribution, Whisper model/runtime resources, installed-package behavior, or release/upgrade path for the Tauri replacement.

The existing legacy Debian/AppImage/Flatpak release pipeline is intentionally preserved. The Tauri app does not silently replace the stable legacy packages.

A later packaging parity change should establish Tauri Debian/AppImage/Flatpak artifacts, sidecar/Python-environment placement, FFmpeg/model distribution, upgrade behavior and rollback testing before changing release names or replacing stable downloads.

## Legacy versus Next

The legacy Python/Tkinter application remains the production/reference application during this wave. Scriptotar Next has the integrated persistence, UI and transcription architecture, but replacement parity is not claimed merely because a Tauri binary compiles.

Known intentionally deferred parity includes live AI-provider execution, full live research-provider execution, production Next packaging/sidecar environment placement and broader feature-by-feature validation against the legacy UI.

Until those gaps are closed, the correct release recommendation is **not ready to replace the legacy application**.
