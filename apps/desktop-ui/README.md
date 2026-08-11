# Scriptotar Next desktop UI

This directory contains the Svelte + TypeScript + Vite frontend for Scriptotar Next.

The frontend is integrated with the Tauri host through a single typed `ScriptotarApi` boundary. A browser-only development run can use the mock client; the production Tauri window injects the real invoke-backed client before mounting the application.

## Development

```bash
npm ci
npm run dev
```

Quality gate:

```bash
npm run check
npm run test
npm run build
```

## Structure

- `src/components/`: reusable shell and state components.
- `src/views/`: dashboard, research, jobs, transcript, AI Studio, library, and settings workspaces.
- `src/api/`: the frontend's host boundary, including mock and Tauri-backed clients.
- `src/types.ts`: typed frontend/domain contract shapes shared across views.
- `src/test/`: test setup.

## Host boundary

Components should depend on `ScriptotarApi` instead of importing persistence, process, or database behavior.

The Tauri-backed client maps UI operations to the current command surface, including:

- `bootstrap_app`
- `list_jobs`
- `select_project`
- `create_project`
- `enqueue_local_media`
- `enqueue_url`
- native media/output-directory selection
- `cancel_job`
- `retry_job`
- settings load/save
- legacy import
- watchlist/research operations
- AI prompt construction and BYOK execution

Keep new host functionality behind this boundary so views remain testable without launching Tauri.

## Secrets

BYOK tokens may exist in component memory for the current interaction, but the frontend must not persist them to localStorage, IndexedDB, cookies, ordinary settings, or mock persistence.

Endpoint policy and provider request execution belong below the frontend in Rust. Copy Prompt mode remains available without an API key.

Appearance is presentation-only state and may use the frontend's local appearance persistence. Transcription/output/provider settings that affect application behavior remain Rust-owned.

## UX model

The Next desktop experience is organized around:

```text
Project -> Research -> Jobs -> Transcript -> AI Studio -> Library / Export
```

The UI supports project workspaces, creator research, persisted job states, native media/output selection, transcript search/export, Arabic RTL transcript presentation, AI prompt/BYOK flows, library search, settings, and local workspace navigation.

Job progress is rendered from backend state rather than fabricated in the frontend. Failed/interrupted jobs remain distinguishable so retry/recovery behavior matches the Rust lifecycle.

## Production integration

`src/main.ts` selects the real Tauri-backed client when running inside the desktop shell. The frontend bundle is built as part of the integrated Tauri and package workflows.

The frontend does not own the Next SQLite database and does not launch the Python sidecar directly. Those boundaries belong to Rust/Tauri and the orchestrator.
