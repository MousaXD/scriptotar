# Agent 01 — Content Study data foundation handoff

**Status:** Draft, awaiting exact-head Rust validation and Agent 00 contract review. **Do not merge or release.**
**Repository:** https://github.com/MousaXD/scriptotar  
**Branch:** `agent-01/defragmentation-data-foundation`  
**Base:** `plan/scriptotar-defragmentation` (initial SHA `2948918865c1960260a8c602e4dfe261c82cd0d0`)  
**Draft PR:** https://github.com/MousaXD/scriptotar/pull/108  
**Last implementation commit before this handoff:** `7d5bddd98cc4d0f5c49174ef135326767a41817b`
**Exact latest branch SHA / CI:** obtain from PR #108 after this handoff commit; **do not** infer that older-head green checks certify it.

## Audited baseline / already fixed

- Main baseline for Scriptotar Next 1.4.1: `69e60f91c0b00560332dac06b732fc2259c1d632` as documented by planning branch.
- Existing **core schema v5** and separate **integration schema v5**, with protected migration ledgers; v5 already has project archive, source/creator guards and parent-lineage reassignment guards.
- Existing job_transcript_links makes completed job -> transcript lineage explicit; not reinvented.
- Existing transcript full-detail lazy API and summarized bootstrap are already present; not modified.
- The older `docs/TECHNICAL_FUTURES_AUDIT.md` contains version-specific assumptions (including bootstrap behavior), not treated as current truth.
- All existing crates, data, core migration versions 1–5, integration migration versions 1–5, Tauri commands, UI and AI adapters were preserved.

## Files / migration

- `crates/scriptotar-core/src/studies.rs`: typed `ContentStudy`, `StudySource`, `EvidenceReference`, `CreativeBrief`, `CreativeDraft` and `DraftVersion`, plus validated enum/input types.
- `crates/scriptotar-core/src/root.rs`: additive module exports.
- `crates/scriptotar-db/src/studies_schema.rs`: **core migration 6** `content_studies_and_drafts`, appended to the existing immutable core ledger; no modifications to old migrations.
- `crates/scriptotar-db/src/studies.rs`: project-scoped study CRUD/search/archive, exact source attachment, origin fingerprints, evidence and source-job associations.
- `crates/scriptotar-db/src/studies_drafts.rs`: creative brief CAS, draft/edit/rename/duplicate/archive/restore with transactionally written immutable versions, draft-source associations.
- `crates/scriptotar-db/src/lib.rs` + `root.rs`: additive migration/module registration and v6 rollback regression.
- `crates/scriptotar-db/tests/content_studies.rs`: restart/durability, v5 upgrade, lineage, deletion, CAS races and cross-project guards.
- `crates/scriptotar-db/tests/project_lifecycle.rs`: old v4 fixture now removes v6 additions before reconstructing v4; validates upgrade to v6.
- Contract: `docs/defragmentation/proposals/agent-01-data.md`; published for Agent 00 review in planning PR #102.

## API inventory — project_id required

Studies: `create_study`, `get_study`, `search_studies`, `update_study` (expected revision), `archive_study`, `restore_study`.

Sources: `add_study_source`, `get_study_source`, `list_study_sources`, `attach_job_to_study_source`, `list_study_source_job_ids`.

Evidence: `add_evidence_reference`, `get_evidence_reference`, `list_evidence_references`.

Brief: `save_creative_brief` (CAS; expected_revision=0 for create), `get_creative_brief`.

Drafts: `create_draft`, `get_draft`, `update_draft` (CAS), `rename_draft`, `duplicate_draft`, `search_drafts`, `list_draft_versions`, `archive_draft`, `restore_draft`, `attach_source_to_draft`, `list_draft_source_ids`.

These are new **Rust crate methods**, not yet public Tauri/TS APIs. Agent 02/00 must adapt both sides of the application boundary together.

## Security / integrity notes

- No all-project or ID-only study/draft getters; wrong-project IDs return `NotFound`.
- SQLite foreign keys and triggers reject cross-project referenced origins, evidence, drafts, job links and source associations, including direct SQL bypassing Rust methods.
- Existing origin deletion uses FK `ON DELETE SET NULL` plus retained original reference UUID, locator and `available=false` snapshot; never imply deleted content can still be opened.
- Study and draft archiving is reversible and blocks new writes. Parent project hard deletion cascades to study/brief/evidence/draft/version rows, preserving FK integrity.
- Fingerprints are deterministic UUIDv5 identity/content keys for **deduplication**, not cryptographic attestations, similarity classification or factual verification.
- Revision CAS uses guarded SQL/IMMEDIATE transactions; every saved edit and archive/restore generates a version record. No API keys or provider credentials are persisted.

## CI / QA evidence

- Earlier CI on PR #108: first Rust foundation failed only at **Rustfmt**, corrected in follow-up commits. Subsequent run at `be5af601...`: Security hygiene succeeded, and headless Rust-to-sidecar migration/recovery flows succeeded; these cannot certify later commits.
- Latest changed code includes a **v6 transactional rollback** regression. Exact-head Rustfmt, check, strict Clippy, full workspace tests, full CI, installed packaging and security results **must be checked on the final SHA**. No local `cargo` available in this session; never imply compilation was run locally.
- Core acceptance tests committed: v5->v6 upgrade with retained project/job, v4->v6 upgrade, multiple source types + draft survives reopen, deleted source tombstone, duplicate fingerprint, wrong-project typed/API + raw SQL, stale edit rejection, concurrent editors, source/job/draft links, immutable draft history, project cascade and migration rollback.

## Open integration / review requirements (Agent 00)

1. Review/freeze contract and validate semantics of source tombstones and draft attachments; the proposal was sent before changing any **shared cross-layer** interface.
2. Agent 02: add project-scoped Tauri service/DTO endpoints; avoid raw unscoped DB access and preserve existing API contract checks.
3. Agent 03: wire exact study/source IDs, editor autosave with expected revision/dirty/error handling and create/open/export flows; do not silently persist BYOK prompts or secrets.
4. Independent QA: certify v5 upgrade on an actual backed-up 1.4.1 database, full project/migration regressions and restart/installed-app acceptance. Fixture tests are not a substitute for a production-user database backup.
5. Confirm exact-head Rust foundation, CI, security and relevant packaged gates before making PR ready. Do not merge or release Agent 01's draft PR without Agent 00 review.

## Known limitations / follow-up

- Typed Rust crate API is not wired to frontend, Tauri DTOs, Studio navigation or exports by design (other agents own those files).
- Source locator and source_revision are provenance snapshots, not independent proof of authenticity; no platform permissions or data-use rights inferred.
- Source fingerprints are exact identity/content dedupe, not normalization across social-provider copies. More sophisticated dedupe/paged full-text indexing belongs in P2.
- Hard deletion of an origin deliberately retains a detached reference; callers must surface `available=false` and uncertainty before using it as evidence.
