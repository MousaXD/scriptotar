# Agent 02 — Scriptotar startup, loading and frontend performance

## Ownership / source verification
- Repository: `MousaXD/scriptotar` (verified via connected GitHub account)
- Original assigned branch: `agent-02/frontend-startup-performance`; **resumed** rather than recreated during this audit
- PR: https://github.com/MousaXD/scriptotar/pull/90 (draft)
- Original main baseline: `f741244634b5143602a21a554094a498a0005b11`; main not modified or merged
- Latest audited **code** commit: `4e9a71648e751fbb10331bdec1d3edd432c695c5`
- This report is a subsequent documentation commit; use PR #90's live HEAD SHA as the authoritative final branch commit.
- Connected workspace is not a local Git checkout (`pwd=/`, `git rev-parse` reports not a repository). Direct clone failed because `github.com` could not be resolved. Source was verified/updated through GitHub's authenticated contents and PR APIs. No pretend local test results.

## Delivered functionality
1. **No blocking splash.** `AppShell` mounts immediately with navigation, search/command palette, language control and visible project selector while backend bootstrap is pending. A small accessible `WorkspaceSkeleton` renders **inside** the main content region only. Real init failures retain a retry action.
2. **Project lifecycle safety.** In-flight project switches hide the previous project's workspace; concurrent project creation/selection is prevented. Failed ordinary switch requests preserve the previous project. Inconsistent backend switch **or project-creation** snapshots are treated as unsafe: clear actionable data, show recovery UI, and do not silently pick a different project's content. A successful bootstrap retry clears obsolete switch errors.
3. **No project empty state.** A localized first-project creation form replaces the previous empty/dead-end screen. Duplicate submissions are disabled; backend errors are visible. Dashboard project-creation forms are no longer unmounted while requests are pending, preserving existing validation behavior.
4. **Stale-response protection.** Generation-scoped guards stop old bootstrap, job updates, watchlist status, transcript search and research saves from overwriting a newer workspace. New-project polling can proceed even when the old project's request never resolves. Search clears on project changes; cross-project search selection aborts on failure and checks transcript IDs.
5. **Search optimization.** Workspace search skips scanning when blank and stops once the existing 10-result limit is reached. Backend transcript search remains debounced at 160ms.
6. **First paint preferences.** Persisted appearance and localization initialize before Svelte mount; legacy and shell translation capabilities remain intact.
7. **Measurements hooks.** Supported WebViews receive performance marks `scriptotar.shell-mounted`, `scriptotar.bootstrap-start`, `scriptotar.bootstrap-ready`, and the `scriptotar.bootstrap` duration measure.
8. **Design consistency.** The skeleton uses semantic tokens, avoids flashing animation and network assets, and has responsive 900px/540px adjustments. No global design tokens/CSS were modified.

## Audit finding: lazy-loading regression and correction
- The first implementation dynamically imported six inactive views. The real Desktop UI workflow on `1857ba8363ebf36c42dcbb2a3fb1764ac44bee08` **passed Type and Svelte checks but failed interaction tests**: 36 failures, 46 passes, 82 total; the production build was skipped. Evidence: https://github.com/MousaXD/scriptotar/actions/runs/42924625080
- Root cause: existing view/AI/project tests (and corresponding established navigation behavior) expect a destination view to be usable synchronously after navigation. The new lazy import rendered a placeholder in place of inputs and controls. One project-creation failure was separate: an in-flight create unmounted the dashboard form and hid its validation error.
- **Correction:** restored original eager view imports to preserve all existing keyboard/UI/workflow contracts, while retaining the nonblocking startup shell and guarded refreshes. Restored dashboard creation-form continuity independently. No assertions or other agents' tests were weakened.
- Future view chunking must be separately designed with an explicit no-flicker/keyboard contract and Agent 08 certification; shipping broken lazy navigation to reduce JS bytes was rejected.

## Startup-specific tests
`apps/desktop-ui/src/startup-performance.test.ts` contains dedicated scenarios:
- Shell/nav/status visible before a deliberately unresolved bootstrap; user can navigate ahead of loading data.
- Recoverable bootstrap failure keeps shell available.
- Old project job results cannot appear after a switch.
- A pending older project request cannot block new project job refresh.
- Invalid bootstrap active-project ID shows fail-closed error, not another project's data.
- Invalid project-switch response shows fail-closed error; no stale project remains actionable; a subsequent valid bootstrap retry restores the workspace and removes the obsolete error.
- Invalid project-creation response shows fail-closed error instead of exposing the former project.
- RTL shell/status is available while Arabic bootstrap is pending.
- Empty database allows first-project creation.

Existing `App.test.ts`, `gui-overhaul.test.ts`, `project-management.test.ts`, and AI source-selection tests remain unchanged and are required to pass.

## Validation evidence and remaining gates
- GitHub Actions before regression fixes: Desktop UI [run 42924625080](https://github.com/MousaXD/scriptotar/actions/runs/42924625080): Svelte/type checks **passed**, interaction tests **failed (36/82)**, production build **skipped**. No claim of success.
- Latest **code** head: `4e9a71648e751fbb10331bdec1d3edd432c695c5`, Desktop UI [run 42927888090](https://github.com/MousaXD/scriptotar/actions/runs/42927888090) was **queued** at audit. Subsequent documentation-only commit creates a new exact HEAD and will require a fresh status check.
- Local `npm ci`, Svelte checks, Vitest and Vite build could not execute because the session has no network-accessible Git clone or installed project dependencies. Rely on actual GitHub Actions once runners complete.
- **Packaged cold/warm startup time:** not measured. **Bundle JS bytes:** not measured; views were kept eager for compatibility. Do not claim a size win. Project shell-first paint and bootstrap interval can be measured via marks in the packaged app after integration.
- Must independently validate 1440/1100/760px layouts; dark and system-light themes; English and Arabic; large workspace; empty/slow/failing boot; keyboard palette; rapid project changes; and installed Windows/Linux desktop behavior.

## Coordination and unmerged dependencies
- **Agent 01:** global design tokens, custom/native controls, global CSS. This branch changes none of them.
- **Agent 03:** [PR #92](https://github.com/MousaXD/scriptotar/pull/92) introduces an opt-in typed `bootstrapSummary()` / `bootstrap_app({summary:true})`. Agent 02 currently uses unchanged full `api.bootstrap()`; that means workspace data still waits for the full bootstrap call. Do **not** activate summary without coordinated Agent 06 on-demand `getTranscript` hydration, search, AI source, export and project authorization. Explicit coordination comment has been posted on PR #92.
- **Agent 06:** transcript/AI workspaces must handle metadata-only snapshots before the summary contract can be adopted.
- **Agent 08:** independent current-head tests/build, packaged-app measurements, keyboard/RTL/accessibility and cross-PR regression verdict.
- No other agent branches were merged or edited.

## Readiness verdict
**BLOCKED — NOT READY FOR AGENT 00 REVIEW/MERGE** until the latest exact-head Desktop UI tests/build and required integrated/packaged gates are genuinely green. Code regression fixes have been pushed and tests added, but queued CI is not passing CI. PR #90 remains a draft; main is unchanged.
