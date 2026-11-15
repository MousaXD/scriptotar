# Agent 05 — P1 original script orchestration handoff

**Date:** 2026-11-16  
**Base:** `integration/scriptotar-defragmentation-agent00-20261113` @ `3009989a3aa73248b6432ba0cd84821c59110993`  
**Branch:** `agent-05/p1-script-orchestration-20261116`  
**Owner:** Agent 00 is the only integrator. No main merge or release.

## Reused instead of rebuilt

- Agent 06's `scriptotar_ai::writing` structured prompt/angle/draft/review/revision types, `parse_angles`, `parse_draft`, `parse_review`, `apply_beat_revision`, and advisory audit.
- Existing `AiService<P: AiProvider>`, `ProviderConfig`, and request-time session key, without a new HTTP stack.
- Agent 00's `scoped_writing_input_locked` in `services/study_gateway.rs`: it already reads actual Study, brief, attached transcript segment/timecodes, and unverified source observations, excluding disputed ones. It is PRIVATE to that sibling module, so Agent 00 must expose an owner-approved scoped adapter rather than duplicating it here.
- Existing v6 `creative_drafts`, `draft_versions`, `draft_source_links`, Study revision CAS, v7 checkpoint, and v8 effect reservation. NO new schema or unsafe ownership changes were made here.

## Implemented on this branch

- `apps/desktop/src-tauri/src/services/study_writing.rs`: a scoped local plan for exactly three candidate angles from at least three distinct selected source bindings, each with attributable evidence. Rejects missing/foreign evidence source IDs, duplicate source IDs, and model-created corroboration of transcript/user observations.
- `copy_angles_prompt` works completely offline; preparing it does not construct a provider or require a credential.
- `generate_three`: four typed model tasks (1 angle proposal + 3 independent scripts); consent/reservation boundary invoked BEFORE every model call; durable recorded-result reuse only for exact step/prompt; uncertain provider failures/reconciliation required, no retries. Revalidate full Study context after each result, not just the Study revision.
- Strict `parse_angles`/`parse_draft`, similar-angle and duplicate-script rejection, source/angle/claim/beat lineage, audit findings and exact locator export. Outputs retain structured `ScriptDraft` and `EvidenceRef`, not only rendered prose.
- `ApprovedAiAdapter` uses the existing `AiService` but cannot be called from Tauri in this branch: there is deliberately **no central command registration or approved production effect-gate binding**.
- Optional per-candidate model review (always suggestions needing human review), targeted revision proposal and `apply_reviewed_beat_revision`; the latter checks old narration, refuses manual-edit replacement unless explicitly allowed, keeps untouched beats, clears revised stale citations, and recomputes the advisory audit.
- An explicit `AtomicCandidateStore::commit_three` contract requires all three candidate IDs/versions/source links and an idempotent receipt in ONE atomic transaction; `VersionedCandidateStore` contract requires CAS and immutable versions for edited revisions.
- `apps/desktop/src-tauri/tests/study_writing.rs`: 14 offline controlled tests with a transaction-backed SQLite fixture sink, including persisted test batches across SQLite reopen, duplicate execution without rebilling, denial, uncertain paid effect, malformed output, cancellation, stale evidence, fake citations, angle/script duplication, source/project refusal, atomic rollback, original manual text protection, Arabic and human-review findings.

## Critical remaining wiring — NOT silently enabled

1. **Agent 00 / canonical scoped context:** add `mod study_writing;` to `services.rs` and expose a project-lock-held context builder via the existing `scoped_writing_input_locked`; produce `SelectedWritingSource { study_source_id, origin_id }` from currently attached, available sources. If fewer than three attributable sources exist, surface a limitation rather than fabricate. The `CurrentStudy` implementation must reload exact transcript, brief, attached source and evidence state for every step. Hold the same project lock across final commit or re-read evidence atomically under SQLite IMMEDIATE.
2. **Agent 01 / provider policy:** implement a durable single-run authorization covering exact provider identity, approved prompt preview, data-egress details, per-call/token/US$ maximum, cancellation/revocation and expiry. A boolean from the frontend is NOT sufficient.
3. **Agent 02 / effect driver:** implement `DurablePermissionBoundary` with an actual v7 Running/Approved checkpoint and v8 **unique effect claim** before dispatch. `record_response` must durably save bounded provider text (secret-free) and settle the receipt in the correct order; existing/uncertain/accepted-but-unproven paid effects MUST block unless there is a verified saved response tied to exact run/step/prompt. Never assume a timeout means not applied. No automatic billing retries.
4. **Agent 00 / canonical atomic batch sink (new approved DB adapter):** implement `AtomicCandidateStore::commit_three` on real `creative_drafts`, `draft_versions`, `draft_source_links`, and a durable batch idempotency receipt in **one** IMMEDIATE transaction. Verify active project, Study CAS, current source membership and current evidence/brief snapshot **inside** that transaction. Existing `create_reviewed_study_draft` is single-draft transactional, therefore three sequential calls are NOT a safe substitute. If a partial batch exists from other code, block/reconcile before retry. Persist structured generated beats, review findings, citation status, original source locators and angle/run lineage in a durable structured artifact, not solely lossy Markdown.
5. **Agent 00 / Studio and DTO:** register typed start/approve/poll/cancel/reconcile, three-candidate selection/edit/export/review/revise calls, and bind proper EN/AR prompts. Handle project switches, startup/restart and stale async completion. Do not change Copy Prompt behavior.
6. **Agent 04 / fact verification:** do not promote model assertions to verified using citation IDs alone. This engine treats observations and transcripts as unverified/attributed. Agent 04 controls independent corroboration and status transitions, with human review for unsupported/high-stakes claims.
7. **Agent 06 foundation:** keep existing typed parser and manual-protection invariants. No source-provided instructions become tools/permissions.

## Contract and safety rationale

`DurablePermissionBoundary::begin` returns either an exclusive `Dispatch { effect_key }`, a bound already-durable `Recorded { response_json }`, or a blocking error. Implementations must refuse a second dispatch for reserved/accepted/unknown states. The response must be persisted durably before `Recorded` is ever returned; if persistence fails, treat the paid request as uncertain and do not retry. Every `StepAdmission` applies to the *exact* prompt, Study, run and step.

`AtomicCandidateStore` is intentionally NOT implemented using the current one-at-a-time `SqliteStore::create_reviewed_study_draft`, because this would violate all-or-nothing and exact-once batch requirements. The transaction-backed fixture proves the semantics but uses **test-only tables**, NOT the user's canonical Studio tables. This agent does not own Agent 00's schema/API or Agent 02's effect ledger.

`StudyWritingContext::validate` checks referential shape, not authentication: the caller must construct it only after loading project-scoped canonical data. `FixedStudy` exists as an offline fixture helper, not as a live project authorization source. `ApprovedAiAdapter` alone is not permission. No paid generation route is registered by this PR.

## Verification

Run in an environment with Rust installed:

```bash
cargo test -p scriptotar-desktop --test study_writing --locked
cargo fmt --all -- --check
cargo clippy -p scriptotar-desktop --all-targets -- -D warnings
cargo test --workspace --locked
```

This chat environment lacks Rust/Cargo and cannot execute these commands locally. CI results must be collected at the actual PR head; **tests have been written, not claimed green**. Then execute canonical end-to-end three-source fixture + real installed graphical WebView EN/AR and full workspace/security/packaging workflows. No live provider was called and no credentials were used.

## Release verdict

**P1 PARTIAL / integration contract awaiting Agent 00 + Agents 01/02/04. Not ready for production execution or merge to main.** Keep issue #116 and integration PR #114 open/draft until atomic canonical save, full permission/effect wiring and independent QA are complete.
