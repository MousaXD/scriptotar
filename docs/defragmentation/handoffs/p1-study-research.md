# Agent 03 — P1/P2 Content Study research integration handoff

**Base:** `integration/scriptotar-defragmentation-agent00-20261113@3009989a3aa73248b6432ba0cd84821c59110993`  
**Owned branch:** `agent-03/p1-study-research-20261116`  
**Scope:** existing `scriptotar-research` intelligence, a new host-side Study adapter, research-only tests. **No main, Studio, Tauri command registration or shared runtime/database edits.**

## What was implemented

- `crates/scriptotar-research/src/study_intelligence.rs`: deterministic, no-network `analyze_study_sources(project_id, study_id, analyzed_at, selected_material)`. Uses the existing observational `MetricSnapshot` and creator/age-matched peer comparison logic and the existing typed transcript feature/creative-beat vocabulary. Returns `StudyResearchReport` with **separate** `ObservedMetrics` and `StudyPatternObservation`.
- Source IDs are **canonical `study_sources.id`**. Original research/transcript UUIDs, project/Study UUIDs, locator, attachment `collected_at`, research `observed_at`, revision, access status, exact segment IDs and real timecode ranges are separate fields.
- The extractor quotes only opening/middle/closing literal text with bounded excerpts. CTA-like terms, spoken transition cues, questions and measured per-segment word/time spans are labeled **transcript-only interpretations**, never verified causal or creative claims. Missing segments, missing stats, missing peer groups, zero views, partial/untimed material and deleted sources are explicit limitations. No visual, retention or virality claims.
- `apps/desktop/src-tauri/src/services/study_research.rs`: a scoped SQLite reader that accepts only already attached real research/transcript IDs, caps source count at eight, caps text and segment JSON reads at 128 KB each, verifies project/source/study/active state, and never calls yt-dlp, remote providers or arbitrary files.
- An approval-only `AnalyzeEvidence` policy gate and `ToolExecutor` adapter use the **existing v8 effects ledger**, reserve before work, serialize a bounded, immutable local JSON report, and record a receipt only after durable write. `probe` can recover a persisted local report after an interrupted receipt, but never re-executes a missing/ambiguous effect. Retrieval rechecks active Study, revision, original source IDs and source availability. A previously available source that was subsequently revoked/deleted cannot be read from the report API.
- No new schema, credentials, platform scraping, frame analysis or provider egress.

## Exact shared contract for Agent 00

**Integration registration is deliberately NOT part of this PR.** Agent 00 must add `mod study_research;` in `apps/desktop/src-tauri/src/services.rs` and wire any Tauri/TypeScript commands in the shared owner-controlled files, with corresponding parity and EN/AR tests.

Construct `StudyResearchGate::new(services.clone(), project_uuid, study_uuid)` and `StudyResearchDriver::new(gate.clone())`; register **only** `ToolKind::AnalyzeEvidence` for a local, zero-provider-cost, explicit-approval `Engine` plan with `StepInput::AnalyzeEvidence { source_ids: canonical_study_source_uuids }`. Enforce <=8 unique IDs, `Estimate { tokens: 0, cost_microusd: 0, max_output_bytes: <=96_000, ... }`, `max_attempts: 1`, `requires_approval: true`, and the existing run/project/Study checkpoint and idempotent effect lifecycle. The engine reports a UUID `StepOutput.artifact_id` after a durable receipt; expose `read_study_research_artifact(&services, project, study, artifact_uuid)` through an **authorized read-only** API. Report JSON is an observable artifact, not an AI prompt.

**Run/step/approval ownership stays with Agent 00 and Agent 02.** No silently running analysis from UI selection; only create a proposed plan until an authorized approval transitions it. Do not implement an additional effect ledger or override a preexisting claim. Do not auto-register this tool in an MCP server.

## Contracts to Agents 04/05

- **Agent 04 (writing/claim review):** consume `StudyResearchReport.sources[].metrics` only as observed provider counters with dates, `sources[].observations` only as *attributed, unverified transcript possibilities*. Each `StudyEvidenceAnchor` contains `study_source_id`, `reference_id`, `transcript_id`, `transcript_revision`, and segment/times/excerpt. Never reclassify an observation as a verified factual claim or silently promote a guessed CTA/hook to a fact.
- **Agent 05 (orchestration/Studio integration):** show missing counters and unavailable/access-denied statuses. `feature_support` counts **distinct selected sources with a hypothesis**; it is not a ranking, trend, forecast or virality measure. Treat `source_text_is_untrusted_data = true` as a mandatory instruction-isolation boundary if converting findings to provider prompts. Require separate provider consent and budget authorization for any subsequent model call.

## Acceptance and evidence

Pure-crate tests authored: exact Study/source project ID refusal, duplicate IDs, unavailable source, missing counters vs real zero, multi-source feature counting, deterministic results and source-instruction quarantine. Adapter tests authored (need owner module registration before compilation): unattached/wrong-project refusal and durable report/reopen followed by archived Study refusal.

**Verification limitation:** no local `cargo`, `rustfmt`, or `rustc` is available in the editing environment and direct GitHub clone is blocked. CI workflow results on the draft PR head are required; this handoff does NOT claim compilation, runnable Tauri tools, installed-app success, or release readiness. The host adapter file will not be compiled until Agent 00 registers it. Add full native restart, effect claim/probe, source deletion, EN/AR/RTL and installed-app integration acceptance after registration.

## Risks / follow-ups for owner review

1. Source metadata can be refreshed under the same research UUID: findings are historical `scanned_at` snapshots, not a current guarantee. Report reuse must identify the original collection date.
2. The local JSON report store lives next to the SQLite database under `study-research-reports/`. Before shipping, confirm backup, deletion/retention and profile migration policy; no P2 scheduler or background collection is enabled.
3. Exact research URL equivalences are not guessed. Transcript-only sources are represented by their real transcript UUID, never by a fabricated research item ID.
4. No frame detection, platform bypass, unsupported source type, automatic posting or paid-provider execution. The main integration PR remains **DRAFT / NO-GO** until independent review, nine exact-head workflows and installed native QA.


## 2026-11-16 follow-up hardening

- Newly generated reports include `StudySourceFinding.origin_revision` derived from the original `research_items.scanned_at` or `transcripts.updated_at`. Reading a saved report now fails closed if its canonical origin was edited, deleted, detached, or its recorded revision no longer matches; this prevents silently presenting stale excerpts as current. **These timestamps are revision indicators, not cryptographic source-content attestations**; Agent 04 still needs authenticated content hashes and independent claims review.
- The exported `read_study_research_artifact` now requires an **Applied** v8 `AnalyzeEvidence` receipt with the matching run, step, idempotency key and exact artifact UUID. A standalone JSON file or an unsettled claim is not user-visible success. The driver-only reconciliation path may read a reserved already-persisted file without executing another tool.
- The preapproval gate requires an active project and Study with the selected exact StudySource IDs of `research_item` or `transcript`; manual/unattached/foreign sources are refused. Retrieved platform counts are explicitly labeled **stored metadata only**, not a fresh availability or live performance check.
- Additional tests: wrong-Study / access-denied / revoked / unsupported statuses, mixed research+transcript sources, historical snapshot invalidation and missing ledger receipt. A host-level approval → v8 receipt → local JSON → restart/reopen test is authored in the unregistered adapter.
- **Verification:** the previous head `56ee14b7` passed seven available GitHub Actions workflows on the same SHA; newer hardening commits require their own exact-head CI. Even a green workspace build does **not** compile `services/study_research.rs` until Agent 00 registers it. No native GUI or external platform acceptance is claimed.

### Additional Agent 00 integration requirement

After adding `mod study_research;` and wiring an approved `AnalyzeEvidence` run, execute the adapter's new host tests and confirm that the persisted effect receipt is required for user-facing report reads; the `probe` path must reconcile only already-present local artifacts, never rerun a missing reserved effect. Ensure canonical transcript revisions change on every text/segment edit; provider source snapshots and future authenticated hashes remain distinct. Confirm local report file retention, backup and deletion behavior before release.
