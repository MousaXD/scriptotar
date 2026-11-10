# Agent 04 — Dashboard and Jobs workspace handoff

- **Repository:** `MousaXD/scriptotar`
- **Branch:** `agent-04/dashboard-jobs-overhaul`
- **PR:** [#95](https://github.com/MousaXD/scriptotar/pull/95) (draft, not approved for merge)
- **Audited baseline:** `f741244634b5143602a21a554094a498a0005b11`
- **Latest implementation/test commit before this handoff:** `3554b13cf671314552002fe6cf077ef08c86388f`
- **Owner:** Agent 04; coordinator Agent 00

## Implementation

### Dashboard
- Removed the redundant metric strip, repetitive eyebrow headings, and dense four-card grid.
- Made **Add transcription job** the primary call to action (navigates to the existing safe Jobs workflow); retained create project and research actions.
- Shows failed/interrupted items before active jobs, with backend failure details and visible active progress.
- Recent transcripts remain one-click accessible; creator research and AI work stay reachable in compact secondary sections.
- Project switcher remains owned and rendered by `AppShell.svelte` (untouched). The dashboard displays the selected project's name and keeps create-project support.
- Uses native semantic buttons, responsive grids, logical CSS properties, semantic tokens, and `dir="auto"` on user-created names.

### Jobs
- Local-file selection and URL ingestion are distinct, with the original Tauri file picker, native drag/drop subscription, URL drop, clipboard multi-URL ingestion, advanced manual path, and backend queue actions preserved.
- The browser fallback no longer sends `File.name` as if it were a validated absolute path. Only explicit desktop paths are selectable; native Tauri drop continues to receive actual paths. Backend validation is still the final authority.
- Filter counts cover all/active/attention/finished; cancel/retry/open transcript and detailed failure diagnostics remain available.
- Queues initially render 30 entries, expanding in 30-row increments to avoid rendering the entire large history at once.
- URLs and advanced paths support form submission with Enter. Busy and error/success feedback remain visible.

### JobRow
- Readable job name, source, updated timestamp, semantic status, backend-supplied detail, localized recovery guidance, active stage, clamped progress, and an explicit unknown-progress state.
- Preserves existing status test hooks and diagnostic text.

## Source files owned / changed
- `apps/desktop-ui/src/views/DashboardView.svelte`
- `apps/desktop-ui/src/views/JobsView.svelte`
- `apps/desktop-ui/src/components/JobRow.svelte`
- New `apps/desktop-ui/src/views/DashboardView.test.ts`
- New `apps/desktop-ui/src/views/JobsView.test.ts`
- This handoff document only

No global CSS, shared localization dictionaries, shared components, backend APIs, App shell, or `main` were modified.

## Validation / evidence
- Original `JobRow.test.ts` remains unchanged, including clamping, NaN, Arabic state, and failure text assertions.
- Added interaction tests for empty and busy/failure states, dashboard navigation/project creation and identity, progress, transcript opening, queue filters, safe picker, URL ingestion, native drop, filename-only browser drop rejection, clipboard batches, manual path, retry/cancel/open, 75-job chunked rendering, and Arabic.
- Remote PR actions were triggered automatically. **At handoff writing, the exact-head CI runs were still queued/pending; no green status is claimed.**
- No installed-app visual verification, true native Tauri GUI drag/drop exercise, screenshot comparison, or real keyboard/RTL/narrow-window field certification was performed. These must remain QA gates.

## Integration dependencies / outstanding validation
1. Agent 01's shared button/control contract may supersede the currently functional native buttons. No premature duplicate component was introduced.
2. Agent 02 owns startup and AppShell. Project switcher is retained there; this PR does not change its behavior.
3. Agent 03's backend/DTO changes may affect the `Job` payload; re-run exact-head checks on the approved integration base.
4. The one new progressive-list label is localized inline for English and Arabic because shared locale dictionaries are out of scope; move it to the normal dictionary only after integration coordination.
5. Manually verify 1440px, 1100px, 760px and smaller widths, both language directions, keyboard focus, all job states, and native file drops in the packaged desktop app.

## Required commands / acceptance
From `apps/desktop-ui`:
```sh
npm ci
npm run check
npm test
npm run build
```

Then examine exact-head GitHub Actions, do installed-app smoke testing, and request independent UI/QA review. Do not merge based solely on this document.