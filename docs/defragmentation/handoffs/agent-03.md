# Scriptotar — Agent 03 handoff: Unified Studio

**Owner:** Agent 03, Principal Product Designer / Svelte  
**Target PR base:** `plan/scriptotar-defragmentation`  
**Working branch:** `agent-03/defragmentation-studio-ui`  
**Status:** DRAFT / blocked on Agent 00 contract and Agent 01/02 backend. Not ready to merge or release.

## Implemented frontend slice

- New Studio with project-scoped source selection, on-demand transcript preview and editable script canvas.
- Real Research/Transcript → Studio ID handoffs; historical AI Library → Studio exact-ID routing (requires backend for actual historical result retrieval).
- Typed study adapter, mock-based save/reopen/version test path, explicit unsupported production state.
- Project jobs shown in a global activity popover and in Studio; full jobs interface preserved.
- TXT/Markdown export and existing local prompt builder preserved.
- English/Arabic labels, RTL-safe user text, keyboard operations using current custom controls and responsive layout.
- No production study storage, no invented IPC commands, no external AI call, no root/Rust/SQLite/Python changes, no modification to `qa/**`.

## API integration block

Agent 00 must freeze canonical DTOs, then Agent 01 supplies study/draft storage and Agent 02 supplies project-scoped methods. Reconcile `apps/desktop-ui/src/studio/contract.ts` with their contract instead of registering unverified commands. Required methods: list/create/get study, revision-checked updates and immutable versions, exact historical AI result detail, and typed source provenance. See `docs/defragmentation/proposals/agent-03-studio.md`.

## Remaining engineering work

- Wire the real Studio gateway once the backend commands are approved, with concurrent revision and project-switch guards.
- Resolve pending save on navigation/exit with real backend acknowledgement; do not claim restart durability before the installed-app test.
- Add study-associated source queue and typed evidence references instead of metadata-only research.
- Verify exact AI artifact content and permissions against production records.
- Run all frontend checks + CI on exact HEAD; independent Agent 07 UI QA and installed-app tests remain separate.
- P3 copy audit and five-item top-level consolidation only after integrated P0 path is certified.

## Validation evidence

Baseline and source files were inspected from GitHub. Unit/component tests in `src/studio/selection.test.ts` and `src/views/StudioView.test.ts` cover exact IDs, mock study reopening, version history, metadata truthfulness, source-to-script navigation, copy-only saved AI history, prompt invalidation, project boundaries, EN/AR and stale response rejection.

- **Verified green at code SHA `4a55b7bf08adbba5c3ff08030eaacfe50da39838`:** [Desktop UI workflow #43444899153](https://github.com/MousaXD/scriptotar/actions/runs/43444899153) — typecheck **0 errors**, all **197/197 tests in 24 files**, production Vite build **passed**. A harmless unused Studio selector warning was subsequently fixed.
- Subsequent commits fix a pending-prompt invalidation edge case and support nullable exact saved AI results; **the final head must pass its own exact-commit checks** before accepting the frontend handoff. See [PR #109 checks](https://github.com/MousaXD/scriptotar/pull/109/checks) for latest run status, not only the earlier green baseline.
- Installed-app rendering/screenshot/performance certification is **not** supplied by the unit tests. Agent 07 owns `apps/desktop-ui/qa/**`. CI tests with a mock gateway **do not** certify real database persistence, process restart or native autosave.

Local clone was blocked in this environment by unavailable DNS resolution to github.com, so code changes and test logs were inspected through the GitHub connector.

## Cross-agent coordination

- Agent 01 [PR #108](https://github.com/MousaXD/scriptotar/pull/108) has a proposed migration v6 with independently revisioned Study, CreativeBrief and Draft models; do not mistake the frontend's single `saveStudy` testing facade for the approved backend write contract.
- Agent 02 [PR #103](https://github.com/MousaXD/scriptotar/pull/103) has project-scoped `get_ai_run` and `get_research_item` on its **unmerged** branch, requiring frontend bindings and API-surface parity updates after Agent 00 approval.
- The independent QA work [PR #106](https://github.com/MousaXD/scriptotar/pull/106) has been notified of exact Studio test paths and RTL/activity scenarios.
- **Do not merge this draft PR yet:** Agent 00 must integrate/freeze the contracts and verify the P0 durable path under real installed-app conditions.

**Safety verdict:** suitable as an isolated draft frontend slice; **NOT GO** for P0 completion, merge, installed package or release.
