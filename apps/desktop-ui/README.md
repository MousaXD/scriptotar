# Scriptotar Desktop UI

Standalone Svelte + TypeScript + Vite frontend for the Scriptotar Tauri migration.

This directory intentionally has no dependency on the Rust workspace or the legacy Python application. During migration it runs against `createMockClient()`. Agent 4 can inject a Tauri-backed client by calling `setApi(createTauriClient(invoke))` before mounting the application, or by passing a `ScriptotarApi` to `App` in tests/host glue.

## Development

```bash
npm install
npm run dev
```

Quality gate:

```bash
npm run check
npm run test
npm run build
```

## Structure

- `src/components/`: reusable application-shell and state components.
- `src/views/`: dashboard, research, jobs, transcript, AI Studio, library, and settings workspaces.
- `src/api/`: the only frontend boundary to host functionality.
- `src/types.ts`: typed frontend/domain contract shapes shared across views.
- `src/test/`: test setup.

The UI does not import Tauri APIs directly. It talks to `ScriptotarApi` only.

## Backend contract

`src/api/client.ts` defines the integration seam. The first command mapping in `tauriClient.ts` is deliberately small:

| UI operation | Proposed Tauri command |
| --- | --- |
| Bootstrap workspace | `bootstrap_app` |
| Select active project | `select_project` |
| Scan creator/profile | `scan_creator` |
| Queue research media | `queue_research` |
| Cancel job | `cancel_job` |
| Build AI prompt | `build_ai_prompt` |
| Run AI request | `run_ai` |
| Save settings | `save_settings` |

Agent 4 should reconcile names and payloads against Agent 1 rather than forcing Rust to match this mock contract mechanically. `createTauriClient()` takes an injected `invoke` function so the Svelte bundle remains testable without `@tauri-apps/api`.

## Secrets

The UI may hold a BYOK token in component memory for the current interaction, but it does not persist keys to localStorage, IndexedDB, cookies, settings, or the mock client. Secure persistence and custom-endpoint policy belong to Rust. Copy Prompt mode requires no key and is presented as the default local workflow.

## UX model

The migration replaces the old seven-tab utility with a creator workstation flow:

`Project → Research → Jobs → Transcript → AI Studio → Library / Export`

Jobs render the persisted lifecycle states supplied by the host. A percentage is shown only when the backend supplies one. Arabic transcript content uses explicit RTL direction and remains searchable by timestamped segment.
