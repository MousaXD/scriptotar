# Agent 06 — Unified Agentic Studio UX handoff (2026-11-16)

**Repository:** MousaXD/scriptotar  
**Base:** integration/scriptotar-defragmentation-agent00-20261113 @ 3009989a3aa73248b6432ba0cd84821c59110993  
**Agent branch:** agent-06/p1-agentic-studio-20261116  
**Draft PR:** https://github.com/MousaXD/scriptotar/pull/123  
**Owner:** Agent 06 (Studio UI only). **Integration/release owner:** Agent 00.

## Implemented

- Retained existing canonical Study selection, provenance-preserving source picker, source preview, evidence review/uncertainty, local transcript inspection, three-angle Copy Prompt/paste validation, persisted drafts and immutable versions. No generic dashboard or duplicate backend.
- Added compact in-page workflow navigation connecting Sources → Evidence → Execution → Scripts → Export, with counts and explicitly **prompt-ready** (not provider-approved) status. Keyboard-focusable native anchors; English/Arabic labels and RTL direction.
- Added StudioAgentReview.svelte: exact source IDs and bound prompt preview, provider/model selection with the existing custom Select, text-entry token and USD ceilings (no native sliders), explicit consent, approved backend actions only, and honest statuses for waiting/running/blocked/failed/cancelled/completed.
- Supports backend-reported run refresh/reopening (when the optional adapter exists), denial, cancellation and user-initiated blocked reconciliation. Host returns are checked for project, Study, revision, source set, prompt and request limits; changed context invalidates consent and guards stale async resolutions. It NEVER invents paid-run results.
- Without a production adapter, the prompt remains local and provider execution controls are unavailable. The existing manual Copy Prompt path still operates.
- Added StudioCandidateCompare.svelte: loads up to three saved, distinct candidate bodies by exact draftId through the existing getStudy gateway; rejects cross-project/Study, wrong draft IDs or mismatched revisions. Allows switching into the **existing** safe revisioned editor. Unsaved text is never silently substituted into a saved comparison.
- Added Studio-only regression tests in apps/desktop-ui/src/studio/workflow.test.ts: no-provider boundary; exact consent; approval/denial; tampered scope refusal; stale in-flight project switch; persisted blocked recovery; EN/AR + RTL; three saved candidate loading/edit selection; unsaved edit warning; wrong-project candidate rejection; keyboard-focusable native button.

## Proposed Agent 00 shared production contract

See comment on issue #116 (2026-11-16). The optional *local UI* interface in studio/agenticRuntime.ts is **NOT a production Tauri declaration**.

- listStudioRuns(projectId, studyId): Promise<StudioRun[]>
- proposeStudioRun(projectId, studyId, expectedStudyRevision, { provider, model, exactPrompt, selectedSources, maxTokens, maxUsd }): Promise<StudioRun>
- decideStudioRun(projectId, studyId, runId, 'approve' | 'deny' | 'cancel' | 'recover'): Promise<StudioRun>

Authoritative snapshots carry runId, projectId, studyId, studyRevision, state, provider, model, **exactPrompt**, selectedSources, maxTokens, maxUsd; optional progress/error. Agent 00 must reconcile these with Agent 01 provider-consent, Agent 02 durable receipts and Agent 05 writing orchestration. Any rename needs UI/test changes together. Backend MUST bind explicit approval to the same exact input hash, limits and project and never replay uncertain paid effects. UI-side validation is defense-in-depth, not authorization.

## Validation / screenshots

- PR #123 is **draft**. CI results must be evaluated at the **last pushed SHA only**, not a previous green commit.
- Intended local commands in apps/desktop-ui: npm ci; npm run check; npm test -- --run; npm run build. The Frontend job in GitHub Actions is the available execution environment for this remote-only agent work; do not claim local execution or a passing result until Actions actually report it.
- **Native Windows/Linux graphical screenshots are not attached yet.** GitHub unit/CI success alone is not native WebView evidence. Agent 09/QA must capture EN/AR light/dark, narrow/wide screenshots and provide keyboard/screen-reader/manual verification in installed builds. Do not fabricate screenshot evidence.

## Not completed / P1 blockers

- Production agent host registration and typed Tauri API binding are Agent 00 work. This branch does not enable paid AI execution, independent research/fact checking, autonomous generation or backend-driven three-candidate production persistence. It uses existing honest, manual canonical paths.
- No cross-agent shared types, central API, database migrations, AppShell or non-Studio components changed.
- Independent claim-level citation verification is not represented as independently fact-checked; human evidence review and unverified/disputed labels remain intact. When Agent 04/00 ship verified claim DTOs, add explicit cite-to-excerpt navigation.
- Targeted model revisions and durable model lineage must be wired to Agent 05/00 backend. Existing manual editing, immutable history and candidate selection remain available; do not pretend this branch generated three scripts.
- Review populated Study with three real canonical candidates after integration; test exit/reopen with durable real run, stale project/error paths, real consent budget and installed native app.
- Main unchanged, no release, no merge recommendation until P0/P1 gates and exact-head CI/QA are satisfied.
