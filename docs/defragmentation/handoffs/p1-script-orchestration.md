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
- No paid provider adapter is included. Agent 01's review found that existing `AiService` does not have hard output-token limits or verified usage receipts; its previous legacy adapter was removed from this branch to prevent accidental paid execution. There is deliberately **no central command registration or approved production effect-gate binding**.
- Optional per-candidate model review (always suggestions needing human review), targeted revision proposal and `apply_reviewed_beat_revision`; the latter checks old narration, refuses manual-edit replacement unless explicitly allowed, keeps untouched beats, clears revised stale citations, and recomputes the advisory audit.
- An explicit `AtomicCandidateStore::commit_three` contract requires all three candidate IDs/versions/source links and an idempotent receipt in ONE atomic transaction; `VersionedCandidateStore` contract requires CAS and immutable versions for edited revisions.
- `apps/desktop/src-tauri/tests/study_writing.rs`: 17 offline controlled tests with a transaction-backed SQLite fixture sink, including persisted test batches across SQLite reopen, duplicate execution without rebilling, denial, uncertain paid effect, malformed output, cancellation, stale evidence, fake citations, angle/script duplication, source/project refusal, atomic rollback, original manual text protection, Arabic and human-review findings.

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

`StudyWritingContext::validate` checks referential shape, not authentication: the caller must construct it only after loading project-scoped canonical data. `FixedStudy` exists as an offline fixture helper, not as a live project authorization source. Existing legacy `AiService` is not an approved writing provider. No paid generation route is registered by this PR.

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

## 2026-11-16 hardening continuation

- A forged serialized GenerationPlan (changed three-angle prompt or invalid run ID) now fails **before** approval/effect reservation/provider access. Each step recomputes the exact typed prompt from the canonical context.
- Removed the legacy unbounded AiService provider wrapper: paid execution still needs Agent 01's hard-bounded model transport and Agent 02's durable actual usage/response receipts. The WritingTextProvider trait is implemented only by controlled test providers in this PR.
- Export includes shot suggestions, beat evidence IDs, optional human-review findings and quality advisories without presenting any of them as independent fact checking.
- Two further tests assert zero provider/ledger calls for prompt/run tampering and zero persisted output when a model forges source IDs.
- Pre-hardening commit `893dc2f64ebaeaed81376b7ce1edf87a2b4763a6` passed Rustfmt, workspace check, Clippy and workspace tests. The new commit requires fresh exact-head CI; do not infer green from that previous commit.


## Agent 05 recovery — canonical batch persistence implementation (2026-11-16)

Recovery was performed on the existing PR #121 branch, not from a fresh scaffold. The original `study_writing.rs` orchestration and its 17 offline tests were preserved.

### Newly committed (source pending exact-head CI)
- `apps/desktop/src-tauri/src/services/study_writing_persistence.rs` contains `CanonicalSqliteCandidateStore`, an actual **IMMEDIATE transaction** writing three rows to canonical `creative_drafts`, three immutable version-1 `draft_versions`, all selected `draft_source_links`, one Study revision compare-and-swap, and an immutable JSON batch receipt. It does not use three sequential single-draft calls.
- Each stored candidate has a human-editable text export in the canonical body and its structured angles, beats, claims, source locators, research findings, evidence statuses and source bindings in the receipt payload. Model-provided `Supported` claims are conservatively saved as `Unverified`; model review findings always require human review.
- Run ID is globally unique; an exact repeated batch returns the same three canonical IDs after a lost reply, even when the Study revision advanced or the user subsequently edited a draft. A changed payload or mismatched project/Study cannot replay the receipt. Missing initial versions fail closed; incomplete writes roll back.
- The sink rereads `CurrentStudy` after obtaining the SQLite writer lock, checks active Study/project/revision and verifies that each exact canonical Study source resolves to its original project-owned reference. **The production CurrentStudy loader must be Agent 00's locked, live `scoped_writing_input_locked` adapter**; fixed/mocked loaders in tests are not production authorization.
- Added canonical real-DB regression tests (in `tests/study_writing.rs`) for 3 drafts/versions/links, SQLite restart, lost-reply idempotency without discarding manual edits, injected mid-batch rollback, cross-project/changed-run rejection, evidence changes, and withdrawn source origins. The previous fixture contract tests remain.
- The new adapter is compiled from the Rust integration test via `#[path]`. **Do not claim it is compiled as a production Tauri command yet.**

### Required Agent 00 central migration and host registration

The new module supplies `RECEIPT_MIGRATION`: additive `study_writing_batches` with immutable `run_id`, exact project/Study identity, bounded structured `payload_json`, original three candidate UUIDs and commit timestamp. Tests explicitly install this schema in disposable SQLite test databases. **Production startup does not apply it**; a versioned migration on the canonical `scriptotar-db` ledger must be allocated by Agent 00 alongside Agent 02's migration before registering any command. Do not run ad hoc `CREATE TABLE IF NOT EXISTS` on customer startup.

Agent 00 owns `services.rs`, Tauri `lib.rs` command registration, DTOs, Studio UI and versioned migration numbering. Register both `study_writing` and `study_writing_persistence` only after this schema migration exists, the real source/evidence/brief loader can be reread under SQLite IMMEDIATE, and an active-project lock is held for each final acceptance. Add an installed-app acceptance test: one approved Study with three real source revisions → three selectable editable drafts → restart → manual edit → exact-run reconcile, with EN/AR, source evidence review, and no paid invocation.

### Cross-agent blockers still outstanding

- **Agent 01**: durable provider identity/preview/consent, egress scope, output-token and price caps, revocation.
- **Agent 02**: actual checkpoint/effect claims and verified response/usage receipts, including unknown-payment recovery; `DurablePermissionBoundary` is still an interface, not wired.
- **Agent 03**: include only grounded and attributable research observations, never treat engagement comparisons as causal proof.
- **Agent 04**: independent fact verification; receipt field `Unverified` never becomes verified from matching citation IDs.
- **Agent 00**: production module registration, real scoped loader, migration coordination and typed three-draft Studio UI. Until all are complete, **no paid dispatch, no release, no merge to main.**

### Exact-head validation

As of writing this addendum, the first canonical test push failed Rustfmt; formatting was applied and re-pushed. Exact-head workflows must be rechecked on the final commit. Tests cannot certify real provider usage or packaged GUI paths without the owner integration. Never count earlier seven green workflows as proof of this new implementation.


### Additional recovery hardening — targeted revisions and source stamps

- `SelectedWritingSource` now preserves the exact canonical `fingerprint`, `source_revision`, and `collected_at` fields. `from_scoped_study` builds these from actual attached source rows; `StudyWritingContext::validate` rejects missing stamps; the canonical commit checks those fields under the IMMEDIATE SQLite lock. **A source's attached revision is not a cryptographic digest of an edited transcript**; Agent 00 must also build a live content-revision/digest check for sources where original text can mutate without changing these metadata fields.
- The additive receipt migration now contains **two** tables: `study_writing_batches` (idempotent original batch provenance) and `study_writing_revisions` (immutable structured per-draft versions). Allocate both in the same Agent 00-approved numbered migration; neither is automatically installed by this PR.
- `CanonicalSqliteCandidateStore` implements `VersionedCandidateStore` for targeted beat edits: expected draft revision CAS, exact prior narration, no changed unrelated beats/citations, one immutable `draft_versions` row plus one structured revision row in the same transaction. Human Studio edits lacking a matching structured revision fail closed rather than being overwritten. Manual editing through existing Studio remains unchanged.
- Tests cover stale/duplicate targeted revision, unchanged other beats and preserved human text; a fabricated lineage excerpt must fail before any draft write. Test fixture research origins are deterministic source-link fixtures, **not** evidence of actual transcript authenticity or independent fact verification.
- Agent 00 should evaluate project-lock ordering and read stability when calling the live scoped context gateway during an IMMEDIATE transaction, including concurrent editor save and evidence-review writers. If the current gateway cannot meet this contract, keep Tauri registration disabled until it can.


### Exact completed-run recovery (additional follow-up)

The first persistence implementation could reconcile `commit_three`, but `generate_three` checked the original Study revision before reaching that sink. A lost response after a successful batch would therefore fail the *whole* retry. The `AtomicCandidateStore` interface now has a read-only `recover_committed` hook, and orchestration checks the exact regenerated plan/prompt then probes the receipt **before** current-Study validation, budget reservation or provider dispatch. The canonical sink validates the stored run, project, Study, exact original context and all three initial canonical draft/version rows. Only a committed exact run can bypass a stale Study; a missing receipt still requires a fresh active approval and effect fence.

A regression constructs a completed three-script transaction, manually edits one saved draft, advances the Study revision, and retries orchestration using a **denied provider gate**. Recovery must return the three original IDs with zero new authorization/dispatch attempts and preserve the human edit. A modified prompt fails closed.

Agent 00 must expose this as a **read-only reconcile action** on a project-scoped persisted run. Never accept a `GenerationPlan` supplied directly by an untrusted webview as authorization; reconstruct and match it to the authorized original run/Study. This receipt is not a provider acknowledgement or factual verification.
