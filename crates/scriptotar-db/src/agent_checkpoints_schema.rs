//! Migration 7: transactionally durable, project-bound agent checkpoints.
//! This is storage for the isolated runtime, NOT permission to execute tools.
use rusqlite::Transaction;

pub(crate) fn migration_7(tx: &Transaction<'_>) -> rusqlite::Result<()> {
    tx.execute_batch(
        r#"
CREATE TABLE agent_runs (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    study_id TEXT NOT NULL REFERENCES content_studies(id) ON DELETE CASCADE,
    revision INTEGER NOT NULL CHECK(revision >= 0),
    snapshot_json TEXT NOT NULL CHECK(json_valid(snapshot_json)),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE INDEX idx_agent_runs_project_study
    ON agent_runs(project_id, study_id, updated_at DESC, id);
CREATE TRIGGER agent_runs_owner_guard_insert BEFORE INSERT ON agent_runs
WHEN NOT EXISTS (
    SELECT 1 FROM content_studies st
    JOIN projects p ON p.id=st.project_id
    WHERE st.id=NEW.study_id AND st.project_id=NEW.project_id
      AND st.archived_at IS NULL AND p.archived_at IS NULL
)
BEGIN SELECT RAISE(ABORT, 'agent run requires an active study in its project'); END;
CREATE TRIGGER agent_runs_owner_guard_update
BEFORE UPDATE OF project_id,study_id ON agent_runs
BEGIN SELECT RAISE(ABORT, 'agent run project/study cannot change'); END;

-- Append-only metadata-level audit. The whole checkpoint and the newly
-- appended events must commit or roll back in a single SQLite transaction.
CREATE TABLE agent_audit_events (
    run_id TEXT NOT NULL REFERENCES agent_runs(id) ON DELETE CASCADE,
    sequence INTEGER NOT NULL CHECK(sequence >= 1),
    event_json TEXT NOT NULL CHECK(json_valid(event_json)),
    PRIMARY KEY(run_id,sequence)
);
CREATE INDEX idx_agent_audit_events_run ON agent_audit_events(run_id,sequence);
CREATE TRIGGER agent_audit_immutable_update
BEFORE UPDATE ON agent_audit_events
BEGIN SELECT RAISE(ABORT, 'agent audit events cannot be edited'); END;
"#,
    )
}
