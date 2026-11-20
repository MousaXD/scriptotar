# Agent 01 — Content Study domain/database contract (P0-01, P0-03)

**Base:** plan/scriptotar-defragmentation @ `2948918865c1960260a8c602e4dfe261c82cd0d0`
**Contract state:** proposed to Agent 00 for cross-layer integration; additive Rust models and database storage can be implemented in Agent 01's exclusive crates without changing Tauri/TypeScript interfaces.

## Existing facts checked
- Core SQLite ledger is at version 5; integration/legacy ledger is independently at version 5. Do not edit versions 1–5.
- Existing tables: projects (archived_at), sources -> media -> transcripts, research_items, jobs, ai_runs and job_transcript_links. Project lineage/foreign-key guards already exist.
- Project state, settings persistence and job/transcript linkage already have implementations. New data objects must reuse their IDs, not match titles/URLs.

## Proposed typed entities and relationships

1. `ContentStudy`: id, project_id, title, goal, status (active/paused/completed), revision, created_at, updated_at, archived_at. Project is a mandatory FK. All reads and mutations require the caller's project ID; archive is reversible.
2. `StudySource`: id, project_id, study_id, source kind (research_item/source/media/transcript/job/ai_run/manual), optional typed FK to its origin, original reference identity snapshot, locator, manual_text (manual only), stable source fingerprint, origin revision, collected_at, uncertainty, created_at. Source availability is computed; deletion of linked legacy entities detaches the FK but retains a **clearly unavailable** provenance snapshot, not silently fabricated data.
3. `EvidenceReference`: id, project_id, study_id, study_source_id, claim, exact locator (segment/page or quote selection), optional timestamp seconds, uncertainty/verification state, created_at. Identity must refer to a selected source in that project/study.
4. `CreativeBrief`: one-to-one per study, audience, goals, language, region, platform, tone, duration_seconds, objective, CTA, constraints, revision, updated_at. Updates use expected_revision (compare-and-swap).
5. `CreativeDraft`: id, project_id, study_id, title, editable body, revision, created_at, updated_at, archived_at. Each persisted edit is a transactionally committed immutable `DraftVersion` (draft_id, version, title/body snapshot, saved_at). Updates require expected_revision; no lost updates.
6. Existing `job_transcript_links` is the authoritative job->transcript mapping; attached job and transcript origins use their IDs, not text.

## Storage contract
- New forward-only core schema migration `6: content_studies_and_drafts`, in the existing validated `schema_migrations` + `PRAGMA user_version` ledger.
- New tables with FKs, CHECKs, scope triggers and indexes; explicitly fail on project mismatch, fake reference IDs, archived project writes, stale revisions and duplicate fingerprints.
- Fingerprints are deterministic UUID-v5 digests of source-kind + durable normalized identity/text (not a security hash); never infer equivalence across projects; prevent duplicate insertion within a single study.
- ON DELETE SET NULL on origin FKs retains a provenance/tombstone row with source_available=false. Deleting the owning project cascades its own study objects; study/draft archival is non-destructive.
- No root Cargo, Tauri, Svelte, AI-provider, or existing migration edits. No credentials or provider API keys in these records.

## Proposed Rust store operations
`create_study(project_id,title,goal)`, `get_study(project_id,id)`, `search_studies(project_id, query, include_archived, limit)`, `update_study(project_id,id,expected_revision,...)`, `archive_study(project_id,id)`, `restore_study(project_id,id)`, `add_study_source(project_id,study_id,...)`, `list_study_sources(project_id,study_id)`, `add_evidence_reference(project_id,study_id,...)`, `list_evidence_references(project_id,study_id)`, `save_creative_brief(project_id,study_id,expected_revision,...)`, `get_creative_brief(project_id,study_id)`, `create_draft(project_id,study_id,...)`, `get_draft(project_id,id)`, `update_draft(project_id,id,expected_revision,...)`, `list_drafts(project_id,study_id)`, `list_draft_versions(project_id,draft_id)`, `archive_draft(project_id,id)`, `duplicate_draft(project_id,id)`.

APIs accept/return Rust typed structs from `scriptotar-core`, and return `RepositoryError::NotFound` for wrong-project IDs (no existence oracle), `Conflict` for stale CAS, `Validation` for invalid inputs. Search is bounded/paged later without copying full transcript bodies.

## Integration requests to Agent 00 (not touched by Agent 01)
- Publish freeze/adjustments for TypeScript `ScriptotarApi`, Tauri commands/DTO/services, event and autosave/error handling. Do not treat this proposal as already wired.
- Reuse project-scoped methods; never expose unscoped source attach/read APIs.
- Hook study/draft export, exact deep-links and manual sources into Studio only after DB/API contract tests.
- Review legacy entity deletion behavior (tombstone vs hard deny) and independent QA before merging.

## Acceptance / quality
Upgrade existing v5 DB without data loss; create/reopen multi-source studies + versioned drafts; reject cross-project IDs and raw SQL cross-scope associations; reject stale edits; retain source identity with unavailable status after legacy deletion; verify intact existing jobs/transcripts/migration tests; test version ledger tamper refusal.

**This is a proposed contract, not Agent 00 approval.** No shared cross-layer interfaces will be changed by Agent 01.