# Agent 02 — Frontend startup and performance handoff

## Branch and scope
- Repository: `MousaXD/scriptotar`
- Assigned branch: `agent-02/frontend-startup-performance`
- Audited main baseline: `f741244634b5143602a21a554094a498a0005b11`
- PR: https://github.com/MousaXD/scriptotar/pull/90
- Branch was absent when assigned and was created from the exact baseline. No earlier Agent 02 work existed to replace.
- All changes confined to `App.svelte`, `AppShell.svelte`, `main.ts`, and new dedicated startup component/test. No modifications to global design files or backend contracts.

## Root causes observed in baseline
1. `App.svelte` rendered the full-screen `boot-screen` until all of `api.bootstrap()` finished; the shell and navigation were withheld even with an available WebView.
2. Seven workspace view modules were eagerly statically imported, competing with initial shell rendering.
3. Job and operational refreshes could commit asynchronously returned data after a project switch. Full refreshes could also race with a project switch.
4. App initialization applied the persisted appearance only after bootstrap; initial localization was launched after Svelte mounted.

## Changes
- `AppShell` now mounts immediately. A small `WorkspaceSkeleton` fills only its main content area while bootstrap or a project switch is pending. No artificial delay or full-screen logo animation.
- If boot fails, the retryable error stays within the shell. If bootstrap returns no usable project, a localized, functional first-project form appears instead of an empty screen.
- The project selector is disabled while loading or switching, and placeholder text is shown until project metadata is available. Navigation, keyboard shortcuts and command palette remain mounted.
- Dashboard stays eager; Research, Jobs, Transcripts, AI Studio, Library, and Settings use demand-driven dynamic imports. Imports are single-flight; loading fallback is workspace-local. Project-keyed AI state remains isolated.
- Project transitions increment a local generation and guard against delayed job polling, operational status polling, workspace refresh, and watchlist-save replies committing stale data. Search IDs and debounce timers are invalidated on switch. Cross-project search selection stops when switching fails.
- Appearance and localization initialize before the Svelte app is mounted.
- Search result mapping skips work for empty query strings and exits when the existing ten-result limit is reached, avoiding scans of the remainder of large lists.
- View import failures now render a retryable error inside the selected workspace rather than leaving an indefinite skeleton.

## Dedicated test scenarios
`apps/desktop-ui/src/startup-performance.test.ts`:
1. Shell, accessible loading state, project control, and navigation exist before a deliberately unresolved bootstrap.
2. Bootstrap failure leaves the shell visible and is recoverable through retry.
3. Stale in-flight jobs cannot appear after selecting another project.
4. Bootstrap with zero available projects displays the first-project form and successfully creates a workspace.

Existing interaction tests also cover project-switch failures, Arabic content and UI, appearance persistence, command palette, persisted jobs, search, and migration recovery.

## Performance evidence
- **Architectural measurements:** Initial route now excludes six inactive view imports by using Vite dynamic-import boundaries; a lightweight workspace skeleton replaces a full application loading view. This is a code-level observation, not a benchmark.
- **Cold-start time:** not measured — packaged Tauri binary not available in this connected environment.
- **User Timing instrumentation:** `scriptotar.shell-mounted`, `scriptotar.bootstrap-start`, `scriptotar.bootstrap-ready`, and the `scriptotar.bootstrap` measure are available when supported by the WebView Performance API. These are instrumentation names, not measured numbers.
- **Warm-start time:** not measured — packaged Tauri binary not available.
- **Initial JavaScript bytes:** not yet measured against the baseline build.
- **Acceptance target:** Agent 08 should record shell-first-paint, bootstrap completion, and navigation timings on the packaged build in both English/Arabic and dark/system appearance.

## Validation
- GitHub Actions Desktop UI PR workflow on latest *source* commit `d3b455b8d7b5a1549aff865b565974590c3e84e7`: https://github.com/MousaXD/scriptotar/actions/runs/42924518988 — queued when checked. Earlier PR runs were superseded/cancelled by later source commits.
- Local Svelte checks, Vitest and Vite build cannot be executed in this session: repository dependencies are not available locally and outbound GitHub clone is unavailable. CI status is **not** assumed successful.
- PR is intentionally a draft pending checks and independent QA.

## Coordination and remaining work
- **Agent 03:** staged bootstrap data requires an explicit reviewed backend/API contract; this branch does not invent or silently change the bootstrap interface. Existing whole-bootstrap request still controls when workspace *data* becomes available (but no longer blocks shell).
- **Agent 01:** merge design-token/control work separately. This change does not edit global CSS; shell native select styling remains Agent 01's ownership.
- **Agent 08:** validate real packaged cold/warm startup, slow/failed database, large workspaces, Arabic/English, system/dark, keyboard palette, project switches, and layout across widths.
- **Risk:** dynamic import failure now has an explicit retry control, but it still requires CI and packaged-binary verification.
- **Do not merge** based on source-only evidence or the presence of unit tests; use exact-head checks and packaged application acceptance.

## Final state
Latest validated *source review* SHA before this handoff update: `d3b455b8d7b5a1549aff865b565974590c3e84e7`. The GitHub PR's head SHA is the authoritative final branch SHA, including this documentation commit. All CI and packaged tests remain unverified while runners are queued. No code is merged to `main`.
