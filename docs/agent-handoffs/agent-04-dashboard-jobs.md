# Agent 04 — Dashboard and Jobs workspace handoff

## Identity and release verdict

- **Repository:** `MousaXD/scriptotar`
- **Existing branch:** `agent-04/dashboard-jobs-overhaul`; `main` untouched
- **PR:** [#95](https://github.com/MousaXD/scriptotar/pull/95) (draft)
- **Baseline:** `f741244634b5143602a21a554094a498a0005b11`
- **Latest source/test commit:** `ccbaa3a7ab6814236da037857ff3b07c63a73ae3`
- **Final branch HEAD:** This handoff's update commit; use PR #95 head SHA as the authoritative full commit hash.
- **Verdict:** **BLOCKED for merge/release certification** until exact-final-HEAD checks and installed desktop UX acceptance are complete. Implementation and all specifically verified UI tests remain in Agent 04's scope.

## Completed changes

- Dashboard replaces repetitive cards and metrics with a compact transcription-first workspace. The existing **Jobs** navigation, new-project form, research, AI, transcript opener, failed/interrupted job details, and active progress remain available. A failed-job backlog reserves space for two active jobs.
- Project name and description remain project-controlled content; creation errors retain the **Create project** heading, explicit alert and editable form, and switching remains in the existing AppShell.
- Recent transcript list uses its own scoped CSS class to avoid the Transcript workspace's narrow-window `transcript-list` styling.
- Jobs presents clearly separated local-file and URL intake, safe Tauri picker/native drop, clipboard batch URLs, URL drag/drop, advanced manual path, filters, cancel, retry, transcript opening, failure diagnostics and progress.
- Browser `File.name` is not treated as a trusted absolute path. A filename-only browser drop now provides a localized actionable message and never calls the backend with a fabricated filename. Native Tauri paths still pass through backend validation.
- Local picker displays both filename and actual selected path (LTR for path in Arabic). Queue history initially renders 30 rows and increases in chunks.
- JobRow exposes readable title/source/state/date, clamped progress or unknown progress, stage details, backend error messages, and localized recovery text.

## Exclusive ownership preserved

Modified:
- `apps/desktop-ui/src/views/DashboardView.svelte`
- `apps/desktop-ui/src/views/JobsView.svelte`
- `apps/desktop-ui/src/components/JobRow.svelte`
- Added `apps/desktop-ui/src/views/DashboardView.test.ts`
- Added `apps/desktop-ui/src/views/JobsView.test.ts`
- This handoff

No edits to AppShell, App.svelte, global CSS, shared locale dictionaries, backend/API, other agents' components, or CI configuration. The original `JobRow.test.ts` remains intact.

## Independent regression audit

Review targeted all changed components, existing app integration tests, and the repository design contract. Known issues discovered and addressed:

1. **Five historical app/GUI tests regressed:** the first redesign removed an explicit **Jobs** button and a **Create project** heading and stopped displaying the absolute picker path. Restored all three without weakening historical assertions. Commit `bd0cb09b03638d73ff321f761bf018b210759f35`.
2. **Narrow-window style collision:** dashboard's `transcript-list` class conflicted with global transcript-reader styling. Renamed to `dashboard-transcripts` with its own local grid.
3. **Failed-job backlog crowding:** six or more failures could entirely hide active work. Now reserves up to two active slots while preserving failed-first ordering.
4. **Misleading browser drop error:** filename-only drops incorrectly reported that the native listener could not be enabled. Now correctly explains how to use the local picker, with English/Arabic text, while still rejecting unsafe pseudo-paths.
5. Tests cover empty, active, failed/interrupted, completed, retry/cancel/open, safe picker, exact native drop paths, browser drop rejection, clipboard batches, manual paths, filters, many jobs, project identity and creation failures, Arabic and status accessibility.

## Actual validation evidence

**Verified at source commit `bd0cb09b03638d73ff321f761bf018b210759f35`:**
- [Desktop UI #42985237369](https://github.com/MousaXD/scriptotar/actions/runs/42985237369): **success** — npm dependency installation, Svelte/type check, **95/95 Vitest tests across 14 files**, production Vite build.
- [CI #42985237518](https://github.com/MousaXD/scriptotar/actions/runs/42985237518): **success**.
- [Security Hygiene #42985237368](https://github.com/MousaXD/scriptotar/actions/runs/42985237368): **success**.
- [Previous SHA Desktop UI #42924865851](https://github.com/MousaXD/scriptotar/actions/runs/42924865851): **failure** in five historical tests; addressed as above.
- [Previous SHA Rust Supply Chain](https://github.com/MousaXD/scriptotar/actions/runs/42924865838): **failure** involving `libsqlite3-sys 0.30.1` / `RUSTSEC-2026-0459` (patched upgrade advertised at `>=0.39.0`). This is **not** a frontend-owned change; do not disable the advisory gate.

**At the time this handoff was authored:** the latest `ccbaa3a7ab6814236da037857ff3b07c63a73ae3` source changes and the documentation commit had not yet completed exact-head workflows. They must be verified separately; no claim of pass is made.

## Remaining dependencies / release blockers

1. **Exact-final-HEAD CI**: confirm Svelte check, complete Vitest suite, frontend production build, repository CI, Rust supply chain, packaging and security checks. Do not conflate source-commit checks and final handoff-commit checks.
2. **Installed desktop UX**: manual native Tauri file picker/drag-drop and keyboard focus, EN LTR and AR RTL, and 1440/1100/760px/narrow windows in the packaged binary are unverified. Source interaction tests do not equal installed-app certification.
3. **Shared control contract**: Agent 01's shared UI components should be reconciled only after integration is approved; this branch deliberately keeps functional native controls and no shared-component duplication.
4. **Backend contracts**: Agent 03-owned changes must be tested on final coordinated integration baseline. Do not merge another agent's branch here.
5. **Localized inline labels**: pagination and browser-drop fallback currently have EN/AR inline text because the shared dictionary is outside Agent 04 ownership; coordinator can migrate them after shared translations are available.
6. **Security advisory**: backend/supply-chain owner must resolve the Rust advisory within their own permitted files; do not weaken `cargo audit`.

## Handoff acceptance and commands

From `apps/desktop-ui` on this branch:
```sh
npm ci
npm run check
npm test
npm run build
```

Then review **the exact PR HEAD** workflow conclusions and perform packaged-app smoke checks. PR remains draft and `main` unchanged.