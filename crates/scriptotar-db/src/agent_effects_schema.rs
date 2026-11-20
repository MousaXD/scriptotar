//! Schema v8: claim-before-dispatch idempotency ledger for future permitted
//! external agent tools. No execution or permission is enabled by this table.
use rusqlite::Transaction;

pub(crate) fn migration_8(tx: &Transaction<'_>) -> rusqlite::Result<()> {
    tx.execute_batch(
        r#"
CREATE TABLE agent_effects (
    idempotency_key TEXT PRIMARY KEY
        CHECK(length(idempotency_key) BETWEEN 1 AND 256),
    run_id TEXT NOT NULL REFERENCES agent_runs(id) ON DELETE CASCADE,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    study_id TEXT NOT NULL REFERENCES content_studies(id) ON DELETE CASCADE,
    step_id TEXT NOT NULL CHECK(length(step_id) BETWEEN 1 AND 128),
    tool_kind TEXT NOT NULL CHECK(length(tool_kind) BETWEEN 1 AND 64),
    status TEXT NOT NULL CHECK(status IN
        ('reserved','accepted','applied','not_applied','unknown')),
    external_id TEXT,
    artifact_id TEXT,
    revision INTEGER NOT NULL DEFAULT 1 CHECK(revision >= 1),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CHECK(NOT (external_id IS NOT NULL AND length(external_id) > 128)),
    CHECK(NOT (artifact_id IS NOT NULL AND length(artifact_id) > 128)),
    CHECK((status='accepted' AND external_id IS NOT NULL)
       OR status<>'accepted'),
    CHECK((status='applied' AND artifact_id IS NOT NULL)
       OR status<>'applied'),
    CHECK((status<>'not_applied' AND status<>'reserved')
       OR (external_id IS NULL AND artifact_id IS NULL))
);
CREATE INDEX idx_agent_effects_run ON agent_effects(run_id, created_at, idempotency_key);
CREATE INDEX idx_agent_effects_scope ON agent_effects(project_id, study_id, status);

CREATE TRIGGER agent_effect_owner_insert BEFORE INSERT ON agent_effects
WHEN NOT EXISTS(
    SELECT 1 FROM agent_runs r
    JOIN content_studies s ON s.id=r.study_id AND s.project_id=r.project_id
    JOIN projects p ON p.id=r.project_id
    WHERE r.id=NEW.run_id AND r.project_id=NEW.project_id
      AND r.study_id=NEW.study_id AND s.archived_at IS NULL AND p.archived_at IS NULL
)
BEGIN SELECT RAISE(ABORT,'agent effect must belong to an active run and study'); END;
CREATE TRIGGER agent_effect_owner_immutable BEFORE UPDATE OF
    idempotency_key,run_id,project_id,study_id,step_id,tool_kind
ON agent_effects BEGIN SELECT RAISE(ABORT,'agent effect owner is immutable'); END;
CREATE TRIGGER agent_effect_transition_guard BEFORE UPDATE OF status ON agent_effects
WHEN NOT (
    (OLD.status='reserved' AND NEW.status IN
        ('accepted','applied','not_applied','unknown'))
    OR (OLD.status='accepted' AND NEW.status IN
        ('applied','not_applied','unknown'))
    OR (OLD.status='unknown' AND NEW.status IN ('applied','not_applied'))
)
BEGIN SELECT RAISE(ABORT,'agent effect transition requires external evidence'); END;
CREATE TRIGGER agent_effect_terminal_immutable BEFORE UPDATE ON agent_effects
WHEN OLD.status IN ('applied','not_applied')
BEGIN SELECT RAISE(ABORT,'terminal agent effect is immutable'); END;
"#,
    )
}
