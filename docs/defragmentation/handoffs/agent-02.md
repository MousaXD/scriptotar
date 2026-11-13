# Agent 02 — Rust services and backend API handoff

**Assigned priority:** P0-02 / P0-05  
**Repo:** https://github.com/MousaXD/scriptotar  
**Base branch:** `plan/scriptotar-defragmentation` (`2948918865c1960260a8c602e4dfe261c82cd0d0`)  
**Working branch:** `agent-02/defragmentation-rust-api`  
**Draft PR:** https://github.com/MousaXD/scriptotar/pull/103  
**Implementation commit at handoff preparation:** `ea6c2fad22afd18c80c93c30a19b62a6043faf92` (subsequent handoff documentation commit changes head).  
**Verdict:** **DRAFT / PARTIAL P0. No merge, no release.** Shared study/draft contract approval and exact-head CI are mandatory.

## Completed independent work
1. `get_ai_run(projectId, aiRunId)`: project-checked, asynchronous Tauri endpoint returning `UiAiRunDetail`: existing summary fields plus exact stored `projectId`, `prompt`, and nullable `result`. Uses narrow read-only SQL within Agent 02-owned `services/artifact_details.rs` because `AiRunRepository` has only list/insert. Preserves historical text; never regenerates or returns credentials.
2. `get_research_item(projectId, researchItemId)`: typed `UiResearchItemDetail`, using existing repository UUID lookup (no URL/title matching), project validation, creator label, scanned date and read-only queued-state lookup.
3. `save_copy_prompt(projectId, input)`: explicit save, returns the saved artifact ID/detail. Only accepts `mode: "copy"`; rejects nonblank API keys; never invokes a provider. Existing `build_ai_prompt` preview is side-effect-free and legacy `run_ai` copy mode is untouched for compatibility.
4. New commands use `on_blocking_worker`; additionally moved existing `search_transcripts`, `queue_research`, `enqueue_local_media`, `enqueue_url`, `cancel_job`, `retry_job`, `get_settings`, `save_settings` to blocking workers without changing their argument or result contracts.
5. Agent 02 API/navigation and proposed study handshake: `docs/defragmentation/proposals/agent-02-api.md`. Sent Agent 00/01 coordination to PR #102; proposals remain unfrozen.

## Typed IPC contract (new)
```ts
type UiAiRunDetail = UiAiRun & { projectId: string; prompt: string; result: string | null };
type UiResearchItemDetail = UiResearchItem & { projectId: string; scannedAt: string };

invoke<UiAiRunDetail>('get_ai_run', { projectId: string, aiRunId: string });
invoke<UiResearchItemDetail>('get_research_item', { projectId: string, researchItemId: string });
invoke<UiAiRunDetail>('save_copy_prompt', { projectId: string, input: AiPromptInput });
```
Project and artifact IDs must be UUID strings. The explicit `projectId` is the **originating active project**, not whichever project happens to be active when the result is rendered. The frontend should discard obsolete responses after switching projects. Legacy Library IDs such as `ai:<uuid>` and `research:<uuid>` must be parsed at the frontend edge; do not use them as DB record IDs.

## Added automated regression tests
- `services::tests::exact_artifact_details_are_project_scoped_and_survive_restart`
- `services::tests::preview_is_not_saved_but_explicit_copy_save_is_durable`
- `services::tests::artifact_read_surfaces_storage_failure_not_false_not_found`
- `tests::artifact_uuid_rejects_invalid_and_oversized_values`

Tests cover historical prompt/result equality, project-bound reads, missing/cross-project/stale IDs, research retrieval, restart recovery, Copy Prompt preview-vs-save, API-key rejection, corrupt/missing SQLite AI table and malformed identifiers. Existing tests remain unchanged.

## CI and reproducibility
No local cargo/rustfmt installation or GitHub network clone was available in the execution container; no local compilation or performance metrics can be claimed. GitHub Actions triggered automatically on PR commits:
- Prior `07d19a2...`: main CI succeeded (https://github.com/MousaXD/scriptotar/actions/runs/43441714939); Rust foundation failed only `cargo fmt --check` (https://github.com/MousaXD/scriptotar/actions/runs/43441716463).
- Prior `bfdd12a...`: Rustfmt reported additional formatting in async wrappers (https://github.com/MousaXD/scriptotar/actions/runs/43441996911).
- Formatting fixes committed through `ea6c2fa...`; at tested implementation head `fb243fadec37d75ef2a190792764e5c993c8d20f`, **Rust foundation PASSED** ([run](https://github.com/MousaXD/scriptotar/actions/runs/43442449931)), **main CI PASSED** ([run](https://github.com/MousaXD/scriptotar/actions/runs/43442450222)), and **security hygiene PASSED** ([run](https://github.com/MousaXD/scriptotar/actions/runs/43442450305)). The new Rust tests are explicitly reported passing in earlier [main CI #43441714939](https://github.com/MousaXD/scriptotar/actions/runs/43441714939). 
- **MCP E2E FAILED** on tested head `fb243fade...`: [run #43442449924](https://github.com/MousaXD/scriptotar/actions/runs/43442449924), due to expected exhaustive API parity guard, **not** a Rust compile/test failure. `tests/test_api_surface_contract.py` finds 37 registered Tauri commands but only 34 wired frontend commands, missing `get_ai_run`, `get_research_item`, `save_copy_prompt`. Also test's expected command count is 34 rather than 37. **Do not weaken this test.** 
- Exact-head [Tauri migration #43442449823](https://github.com/MousaXD/scriptotar/actions/runs/43442449823), [Windows #43442449928](https://github.com/MousaXD/scriptotar/actions/runs/43442449928), and [Linux #43442449977](https://github.com/MousaXD/scriptotar/actions/runs/43442449977) were still running when this note was written. The documentation-only commit adding this status will also move HEAD; recheck its workflows before merge.

## Security, limitations, required integration
- Existing API-key boundaries, endpoint policy, media/job recovery and MCP permission checks have not been relaxed.
- The exact-read adapter duplicates the legacy AI-run SQLite selection; once Agent 01 exposes a canonical project-filtered single-item repository API, replace the adapter without changing Tauri wire shapes.
- `get_research_item` consults scoped creators to format the established UI shape; optimize lookup only if a measured performance gate shows it necessary.
- Explicit Copy Prompt save is not yet wired into the UI (frontend owned elsewhere). Library navigation must supply exact `projectId` plus stored UUIDs; switching projects should clear stale selection. The frontend owner must update `apps/desktop-ui/src/api/client.ts`, `tauriClient.ts`, `mockClient.ts`, and `tauriClient.test.ts`, then coordinate `tests/test_api_surface_contract.py` command and API inventory counts (34→37 and 36→39). See comments on PRs #102 and #103.
- New detail reads have no special cancellation token; they are bounded SQLite reads with project revalidation. They do not implement autonomous runs, agent cancellation, evidence APIs or drafts.
- Some legacy synchronous commands still exist (native pickers, watchlist status, migration/shell operations); a broader asynchronous conversion requires separate approval and integration QA.
- Do not implement `create_study`, `attach_study_source`, `save_brief`, `save_draft_version`, evidence, progress or P1 orchestration until Agent 00/01 freeze the matching models, migrations and ownership/security semantics. Agent 04 owns the agent engine.
- No installed-app timing, cold-start measurements or EN/AR visual checks were performed in this branch.

**Agent 00 requested action:** review the proposal and coordinate the freeze with Agent 01; merge only after exact-head CI and frontend integration demonstrate the P0 narrow vertical slice.
