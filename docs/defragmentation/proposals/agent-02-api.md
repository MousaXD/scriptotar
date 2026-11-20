# Agent 02 — Rust/Tauri API contract proposal (P0-02 / P0-05)

**Status:** Proposed; **Agent 00 and Agent 01 must approve/freeze shared study and draft types before implementation.**  
**Based on:** `plan/scriptotar-defragmentation` at `2948918865c1960260a8c602e4dfe261c82cd0d0` (main `69e60f91c0b00560332dac06b732fc2259c1d632`).  
**Scope:** Desktop service, DTO, IPC and integration tests only. Additive. No schema, AI agent, frontend or compatibility API changes.

## Findings verified against current source
- `AiRunRepository::list_ai_runs(Some(project_id))` reads persisted prompt/result; bootstrap intentionally emits only `UiAiRun` summary. No single-run repository method exists.
- `ResearchRepository::get_research_items(&[id])` returns a full `ResearchItem`; UI bootstrap emits `UiResearchItem`.
- `get_transcript` already fetches on demand and checks active-project ownership.
- `build_ai_prompt` builds text without persistence. Legacy `run_ai` in copy mode persists a `copy_prompt` record, returning prompt text, whereas the UI's Copy Prompt action calls the preview builder. This is currently intentional but not user-visible as a Save action.
- Existing `on_blocking_worker` runs some SQLite calls and external tasks off Tauri's IPC executor; `run_ai` is already off-thread.

## Independent additive commands — implemented before study contract freeze
```text
get_ai_run({ projectId: UUID, aiRunId: UUID }) -> UiAiRunDetail
get_research_item({ projectId: UUID, researchItemId: UUID }) -> UiResearchItemDetail
save_copy_prompt({ projectId: UUID, input: AiPromptInput }) -> UiAiRunDetail
```
All require `projectId` to equal **current active project** and record ownership to equal that project. Deleted, missing or cross-project IDs return not-found, not data. Invalid UUIDs are rejected at IPC parsing. A queued UI command for the previous project must fail rather than resolving in the newly active project. Detail reads use background workers; no arbitrary network access. `save_copy_prompt` requires copy mode and rejects nonblank keys, persists only the generated text, returns the saved ID, and does **not** make any external request. Legacy `build_ai_prompt` remains a non-saving preview; legacy `run_ai` remains compatible.

`UiAiRunDetail` flattens existing `UiAiRun` fields and adds `projectId`, exact `prompt`, `result` (nullable). `UiResearchItemDetail` flattens existing `UiResearchItem` fields and adds `projectId`, `scannedAt`; original `ResearchItem` is selected by UUID, not title/URL. No credential is ever returned. No new persisted schema in this slice.

## Proposed shared IDs/navigation (requires Agent 00 freeze)
```ts
type EntityKind = 'research'|'transcript'|'ai_run'|'job'|'study'|'draft';
type ExactLocation = {
  projectId: string; kind: EntityKind; entityId: string;
  // optional sub-selection: segmentId / excerptId / draftVersionId,
  // each validated against the parent entity and project.
};
```
Do **not** use Library virtual row IDs (e.g. `research:<uuid>`) as data-layer identities. Parse those at the frontend boundary, then pass `projectId` + `kind` + entity UUID. Library rows are an index, not an independently persisted entity. Detect stale navigation by keeping originating project and ID. Avoid automatic project switching on item resolution.

## Proposed study/draft service handshake (NOT implemented until Agent 01/00 freeze)
Prefer an Agent 01-owned `StudyRepository` (and domain DTOs) for `create_study`, `get_study`, `list_studies`, `attach_study_source`, `remove_study_source`, `get_study_evidence`, `save_brief_if_revision`, `create_draft`, `save_draft_version_if_revision`, and `get_draft_versions`. Each accepts explicit `project_id` and `study_id`/record IDs and validates ownership **inside one database operation**, not only in a Tauri pre-check. Source kinds and citation locators must be Agent 01 domain enums, with stable UUIDs and transcript timing/excerpts. Concurrency uses expected revision on writes and conflict errors. No destructive schema edits. Agent 04 owns agent-run orchestration; this agent will only add cancellable, permission-checked entry points after freeze.

**Pending:** Agent 01 model and migration API names, Agent 00 wire-field approvals, frontend route consumption, exact-head GitHub CI. Do not consider P0 complete until the integrated UI and restart tests are certified.
