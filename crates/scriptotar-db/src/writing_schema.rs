//! Canonical additive SQLite v11 writing receipt and structured revision migration.
//! Historical v1-v10 schemas are never rewritten. Agent 00 owns this registry.
use rusqlite::Transaction;

pub const WRITING_RECEIPT_MIGRATION: &str = r#"
CREATE TABLE study_writing_batches (
    run_id TEXT PRIMARY KEY CHECK(length(run_id) BETWEEN 1 AND 128),
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    study_id TEXT NOT NULL REFERENCES content_studies(id) ON DELETE CASCADE,
    payload_json TEXT NOT NULL CHECK(json_valid(payload_json) AND length(payload_json) <= 2000000),
    draft_ids_json TEXT NOT NULL CHECK(json_valid(draft_ids_json) AND json_array_length(draft_ids_json)=3),
    committed_at TEXT NOT NULL
);
-- Each independently saved revision gets a matching structured copy.
-- If a manual Studio edit creates an unstructured draft version, targeted
-- AI revision MUST fail closed instead of quietly replacing that user's text.
CREATE TABLE study_writing_revisions (
    draft_id TEXT NOT NULL REFERENCES creative_drafts(id) ON DELETE CASCADE,
    version INTEGER NOT NULL CHECK(version >= 1),
    structured_json TEXT NOT NULL CHECK(json_valid(structured_json) AND length(structured_json) <= 1000000),
    PRIMARY KEY(draft_id, version)
);
CREATE TRIGGER study_writing_revisions_immutable_update
    BEFORE UPDATE ON study_writing_revisions
    BEGIN SELECT RAISE(ABORT, 'structured revision is immutable'); END;
CREATE TRIGGER study_writing_revisions_immutable_delete
    BEFORE DELETE ON study_writing_revisions
    WHEN EXISTS (SELECT 1 FROM creative_drafts WHERE id=OLD.draft_id)
    BEGIN SELECT RAISE(ABORT, 'structured revision is immutable'); END;

CREATE INDEX idx_study_writing_batches_scope
    ON study_writing_batches(project_id, study_id, committed_at);
CREATE TRIGGER study_writing_batches_immutable_update
    BEFORE UPDATE ON study_writing_batches
    BEGIN SELECT RAISE(ABORT, 'writing receipt is immutable'); END;
CREATE TRIGGER study_writing_batches_immutable_delete
    BEFORE DELETE ON study_writing_batches
    WHEN EXISTS (SELECT 1 FROM content_studies WHERE id=OLD.study_id)
    BEGIN SELECT RAISE(ABORT, 'writing receipt is immutable'); END;
CREATE TRIGGER study_writing_batches_scope_insert
    BEFORE INSERT ON study_writing_batches
    WHEN NOT EXISTS(
        SELECT 1 FROM content_studies st
        WHERE st.id=NEW.study_id AND st.project_id=NEW.project_id
    )
    BEGIN SELECT RAISE(ABORT, 'writing batch project and Study mismatch'); END;
"#;

pub(crate) fn migration_11(tx: &Transaction<'_>) -> rusqlite::Result<()> {
    tx.execute_batch(WRITING_RECEIPT_MIGRATION)
}
