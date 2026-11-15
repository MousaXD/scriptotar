//! Schema v9: append-only human evidence dispute/reopening history.
//! Earlier review reasons cannot be synthesized retroactively.
use rusqlite::Transaction;

pub(crate) fn migration_9(tx: &Transaction<'_>) -> rusqlite::Result<()> {
    tx.execute_batch(
        r#"
CREATE TABLE evidence_reviews (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    study_id TEXT NOT NULL REFERENCES content_studies(id) ON DELETE CASCADE,
    evidence_id TEXT NOT NULL REFERENCES study_evidence(id) ON DELETE CASCADE,
    previous_certainty TEXT NOT NULL CHECK(previous_certainty IN ('unverified','disputed')),
    next_certainty TEXT NOT NULL CHECK(next_certainty IN ('unverified','disputed')),
    reason TEXT NOT NULL CHECK(length(trim(reason)) BETWEEN 1 AND 4000),
    reviewed_at TEXT NOT NULL,
    CHECK(previous_certainty <> next_certainty)
);
CREATE INDEX idx_evidence_reviews_scope ON evidence_reviews(project_id,study_id,evidence_id);
CREATE TRIGGER evidence_review_scope_guard BEFORE INSERT ON evidence_reviews
WHEN NOT EXISTS(
    SELECT 1 FROM study_evidence ev
    JOIN content_studies st ON st.id=ev.study_id AND st.project_id=ev.project_id
    JOIN projects p ON p.id=st.project_id
    WHERE ev.id=NEW.evidence_id AND ev.study_id=NEW.study_id
      AND ev.project_id=NEW.project_id AND st.archived_at IS NULL
      AND p.archived_at IS NULL AND ev.certainty=NEW.next_certainty
)
BEGIN SELECT RAISE(ABORT,'evidence review must match active Study and saved status'); END;
CREATE TRIGGER evidence_review_immutable_update BEFORE UPDATE ON evidence_reviews
BEGIN SELECT RAISE(ABORT,'evidence review event is immutable'); END;
CREATE TRIGGER evidence_review_immutable_delete BEFORE DELETE ON evidence_reviews
WHEN EXISTS(SELECT 1 FROM study_evidence WHERE id=OLD.evidence_id)
BEGIN SELECT RAISE(ABORT,'evidence review event is immutable'); END;
"#,
    )
}
