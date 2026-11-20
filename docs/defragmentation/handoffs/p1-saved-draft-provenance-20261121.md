# Agent 04 — immutable DraftVersion evidence / claim provenance (2026-11-21)

## Scope and exact baseline

- Base: integration PR #114, `b12843c75764601248a69c0dd89ff753af8df366`.
- Implementation: `apps/desktop/src-tauri/src/services/study_verification.rs` only; no schema migration, provider dispatch, public Tauri/TS command, manual review upgrade, or merge to main.
- Existing core: `crates/scriptotar-ai/src/writing/verification/mod.rs` and v9 evidence review; existing verification remains offline and fail closed.
- This PR does **not** authorize publishing, real-world truth claims, external fact checking or any independent `Verified` status.

## Exact evidence-to-saved-draft chain

1. Host holds the active-project lock; rejects foreign project, invalid draft UUID, archived or wrong-Study draft, and stale Study / draft revision.
2. Host loads the current `creative_drafts` row and a **matching immutable `draft_versions` row**. Title, body, revision and draft ID must match; versions are scoped by project.
3. Host loads `study_writing_revisions.structured_json` for this **same current version**, requiring a corresponding canonical saved draft. A supplied model/user `ScriptDraft.id` has no independent authority.
4. It parses the stored `GeneratedCandidate`, reproduces the canonical `export_candidate` text, requires byte-for-byte equality with the immutable version body and exact title, then compares **all typed script fields** (including beats, claim statements, claim IDs, status, evidence links and manual edit flags) against the caller proposal, substituting only the true canonical draft UUID.
5. Re-read current draft, immutable version, structured payload, Study, sources, evidence and origin-content hashes. Fail with conflict on any change. This is repeated-read consistency, **not a single SQLite snapshot**; independent verified status remains unavailable until stronger atomic/reviewer controls are implemented.
6. Reports expose a domain-separated SHA-256 fingerprint of project, Study, draft ID, immutable version UUID, version number, title and exact body. This is local content/version binding, **not proof of publisher identity or source accuracy**.
7. The core verifier resolves known evidence IDs only from attached, scoped Study rows; source-content digests are computed from real host bytes. Transcript citations require actual segment ID, exact segment quote and in-range timestamp. Manual citations require the creator-entered excerpt to be literally contained in stored manual text. Missing/forged excerpt bindings become untrusted, not source-supported.
8. Human observation flags and generated citations are never independent claim-level review; the v9 human observation dispute log is **not reused** as an approval ledger. `human_reviews=[]`, so this adapter cannot independently certify any claim.

## What can be assessed today

- **Host-saved, structured, exact-body-matched candidate:** deterministic per-claim review of cited evidence IDs/locators/modality/source access, missing evidence and unsupported or disputed assertions. Status stays unverified, blocked, disputed/contradicted as applicable; it cannot become independently Verified.
- **Unannotated narration:** remains unclassified and cannot be automatically certified. Exhaustive claim inventory is unavailable.
- **Creator observations:** unverified by default; dispute review is informative only, not external corroboration.
- **Exact stored transcript segment:** may support a *literal attribution* check; does not establish the speaker's factual correctness, editorial independence, visual facts or real-world truth.
- **Manually edited draft or restored version:** no matching authenticated structured manifest for the new version means typed review is **denied**. The human may continue editing/exporting normally; UI must show a new fact review is needed, never retain a previous Verified label.
- **Research metadata/AI output/ambiguous source:** cannot be promoted to independent fact checking. No new external-source authority is invented.

## Tests added / changed

- Real SQLite-backed saved revision bind, persisted version ID and deterministic digest.
- Forged typed claim statement, forged evidence citation, fake draft UUID and foreign project rejection.
- Manual editor revision update, old/new revision rejection and restart persistence.
- Missing structured provenance on an otherwise real Studio draft denied.
- Literal manual quote accepted only as **unreviewed**; invented quote/prompt-injection string rejected as untrusted.
- Existing canonical transcript segment/timecode/quote and host-computed source-content tests remain.

## Required next work / blocks (Agent 00 / 03 / 05 / 06)

- Agent 03: preserve current selected source bytes, transcript revisions, canonical locators and explicit source accessibility for one truly atomic verification snapshot.
- Agent 05: save/revise the authoritative structured claim manifest alongside **every** immutable `DraftVersion` and invalidate it on manual edits/restores. Targeted revision text replacement currently does not always serialize as `export_candidate`; this PR deliberately blocks such mismatches.
- Agent 06: add a human-readable review UI contract for per-claim findings, source navigation and *unverified/disputed/source-attributed/independently-verified* distinctions, with no false green badges. A production public IPC endpoint is not added here.
- Agent 00/security: design and independently review an authenticated append-only *claim-level* reviewer ledger with reviewer qualifications, source owners/origin independence, revocation, lifetime, and an atomic scoped snapshot. Do not reuse v9 observation decisions or caller-provided `VerifiedReview` JSON.
- Production external fact corroboration remains absent. Any real-world, high-stakes or source-contradicted statement requires an appropriate authoritative independent source and human review.
- Exact-SHA Rustfmt, strict Clippy, workspace/security tests and frontend/API parity must pass before integration. The installed desktop WebView QA and release checks are Agent 00/07 responsibilities. Until then this is DRAFT / NO-GO.

## 2026-11-21 continuation — attributed source navigation and bounded version read

- The read-only host result now carries `claim_sources`: the exact project-/Study-scoped evidence ID, StudySource UUID, canonical content SHA-256 (if available), exact locator/segment/timecode, bounded 480-character preview and explicit attribution state. This is additive to the existing offline `VerificationReport` and deliberately has **no public Tauri/TypeScript command**; Agent 06 must not display mock verification as production certification.
- Attribution states are `literal_excerpt_unreviewed`, `disputed_observation`, `untrusted_or_unmatched` and `source_unavailable`. **Literal attribution only states that quoted bytes exist in that attached source; it does not certify factual correctness.** The creator's disputed/reopened state is reflected on a fresh read, never promoted to independent Verified.
- Canonical manual-source locators must match the attached source's locator as well as a nonempty literal excerpt. Transcript segment IDs, timestamps and exact quote must match canonical stored segments.
- `scoped_current_saved_version` joins one current `draft_versions` row to the project, active Study and draft, instead of loading every historical version body. It is checked before and after host-source verification; stale editor version or restoration fails closed.
- The new persisted test exercises literal source navigation, a creator's dispute/reopen cycle, fake claim/citation/draft IDs, current immutable version fingerprint, ordinary editor invalidation and restart. This verifies local data integrity, **not independent real-world fact checking**.
- Do not describe this PR as a complete claim-verification UI, authenticated independent reviewer storage, atomic SQLite verification transaction or a release-ready source. Strict exact-head Rustfmt/Clippy/workspace/security/desktop contract CI still required.

## 2026-11-21 continuation — authoritative draft lineage

- Existing v11 `draft_source_links` is now checked against the exact structured `GeneratedCandidate.lineage`: every currently cited claim evidence ID must be present in the stored candidate lineage, and each lineage source must be linked to the saved draft. Historical links are allowed to remain a superset after a targeted revision removes a citation; those extra links do not themselves justify a current claim citation.
- Every stored lineage evidence ID is independently resolved against the current canonical `study_evidence.study_source_id` row. Evidence from a second source attached to the same Study **cannot** be cited under a different source's draft lineage. A new adversarial SQLite regression constructs this exact false relationship and requires denial.
- Exact current immutable versions are read by a single project/Study/active-draft/revision-scoped SQL join; the verifier does not load the full historical body timeline. A scoped-version test rejects wrong project, Study and stale versions after an edit.
- A restored old body is a *new* DraftVersion and cannot inherit the prior structured review simply because the text is byte-identical. Explicit restart regression checks that restoration remains denied. Direct SQLite mutation of manual source bytes is checked against the existing immutable-source trigger.
- **Critical gate:** no independent authenticated claim-review ledger exists. A source excerpt that can be found in saved bytes is `literal_excerpt_unreviewed`, not `verified`. Creator review can dispute/reopen but can never confer independent truth. The production UI command, atomic source snapshot and release/installed QA remain outside this focused Agent 04 PR.

## Production writing-ID compatibility and deliberate uncertainty

- Verified the real Agent 05 input contract in `study_gateway.rs`: evidence identifiers are often `observation:<StudyEvidenceUUID>`, `transcript:<originTranscriptUUID>:<segmentArrayIndex>` or `transcript:<originTranscriptUUID>:text`. They are **not** all plain UUIDs.
- Canonical lineage parsing now accepts these exact shapes, resolves observation IDs to real scoped `study_evidence` records, checks transcript origin UUID against the actual attached `StudySource.reference_id`, and refuses wrong sources or malformed segment selectors. Legacy direct evidence UUIDs remain supported.
- A writer's `observation:<uuid>` citation aliases its host-resolved observation/dispute record in the verifier; its creator-provided certainty cannot become independent Verified.
- A writer's `transcript:<origin>:<index|text>` citation receives read-only navigation and host-computed content SHA-256. The source is **literal-but-unreviewed only when** the indexed current stored canonical segment ID, bounded excerpt, section text, and timestamp all exactly agree; otherwise it is untrusted. An untimed `:text` reference cannot fabricate a segment ID. A revoked source remains unavailable. None of these cases independently verify the spoken claim.
- The additional unit tests cover canonical writing identifier formats, wrong transcript origins, malicious/malformed selectors and foreign source attribution. This deliberately refuses invented segment provenance, even when a structured saved script and source ID are genuine.
- **Still required before a user-visible independent 'Verified' verdict:** true canonical transcript segment cross-check (or explicitly attributed, not fact-checked status), authenticated independent reviewer ledger, atomic snapshot, exact-source frontend/Tauri command wiring and CI.

## Final source-attribution hardening checkpoint

- The v11 indexed transcript excerpt is checked against currently saved canonical segment ID, 960-character excerpt, 250-character section locator and exact recorded start second. A successful match earns only `literal_excerpt_unreviewed` (source attribution), never independent factual approval. A missing segment, mismatched quote/time/section, untimed `:text` or revoked source cannot receive the label.
- Exact written segment matching includes positive and forged-quote, wrong-timecode, wrong-section, untimed-text, and invalid-array-index negatives. The source/content version itself still requires the host-scoped SHA-256 binding and double-read consistency check.
- This candidate is deliberately limited to a private read-only host method. Agent 00/06 own public API and native Studio rendering; authoritative independent fact review and a single SQLite atomic snapshot remain blockers. Never report independent Verified or release readiness on this PR.
