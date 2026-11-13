//! SQLite-backed, project-scoped adapter for the isolated scriptotar-agent engine.
//!
//! The in-memory agent store is a fixture only. This adapter persists a CAS
//! checkpoint plus append-only event metadata in one IMMEDIATE transaction.
//! It NEVER executes tools, stores provider credentials, or authorizes egress.
use rusqlite::{params, OptionalExtension, Transaction, TransactionBehavior};
use scriptotar_agent::{CheckpointStore, RunSnapshot, StoreError};
use std::convert::TryFrom;
use uuid::Uuid;

use crate::{studies::database, SqliteStore};

const MAX_CHECKPOINT_BYTES: usize = 2_000_000;
const MAX_AUDIT_EVENTS: usize = 8192;

#[derive(Clone)]
pub struct ScopedAgentCheckpointStore {
    db: SqliteStore,
    project_id: Uuid,
    study_id: Uuid,
}

impl SqliteStore {
    /// Scope the runtime to an existing Content Study before issuing an agent
    /// command. AppServices must also authorize the active project per call.
    pub fn scoped_agent_checkpoints(
        &self,
        project_id: Uuid,
        study_id: Uuid,
    ) -> scriptotar_core::RepositoryResult<ScopedAgentCheckpointStore> {
        self.get_study(project_id, study_id)?;
        Ok(ScopedAgentCheckpointStore {
            db: self.clone(),
            project_id,
            study_id,
        })
    }
}

impl ScopedAgentCheckpointStore {
    fn verify_snapshot(&self, snapshot: &RunSnapshot) -> Result<(), StoreError> {
        if snapshot.schema_version != 1
            || !snapshot.plan.validate()
            || snapshot.plan.project_id != self.project_id.to_string()
            || snapshot.plan.study_id != self.study_id.to_string()
            || snapshot.steps.len() != snapshot.plan.steps.len()
            || snapshot.current_step > snapshot.steps.len()
            || snapshot.events.is_empty()
            || snapshot.events.len() > MAX_AUDIT_EVENTS
            || snapshot.events.iter().enumerate().any(|(i, event)| {
                event.sequence != i as u64 + 1
            })
            || snapshot
                .steps
                .iter()
                .zip(&snapshot.plan.steps)
                .any(|(record, step)| record.id != step.id)
        {
            return Err(StoreError::Conflict);
        }
        Ok(())
    }

    fn serialize(&self, snapshot: &RunSnapshot) -> Result<String, StoreError> {
        self.verify_snapshot(snapshot)?;
        let data = serde_json::to_string(snapshot).map_err(|_| StoreError::Unavailable)?;
        if data.len() > MAX_CHECKPOINT_BYTES {
            return Err(StoreError::Conflict);
        }
        Ok(data)
    }

    fn read_checkpoint(
        &self,
        tx: &Transaction<'_>,
        run_id: &str,
    ) -> Result<Option<(i64, RunSnapshot)>, StoreError> {
        let raw: Option<(i64, String)> = tx
            .query_row(
                "SELECT revision,snapshot_json FROM agent_runs
                 WHERE id=?1 AND project_id=?2 AND study_id=?3",
                params![run_id, self.project_id.to_string(), self.study_id.to_string()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(|_| StoreError::Unavailable)?;
        let Some((revision, raw)) = raw else {
            return Ok(None);
        };
        if revision < 0 || raw.len() > MAX_CHECKPOINT_BYTES {
            return Err(StoreError::Unavailable);
        }
        let snapshot: RunSnapshot =
            serde_json::from_str(&raw).map_err(|_| StoreError::Unavailable)?;
        self.verify_snapshot(&snapshot).map_err(|_| StoreError::Unavailable)?;
        if snapshot.plan.id != run_id || i64::try_from(snapshot.revision).ok() != Some(revision) {
            return Err(StoreError::Unavailable);
        }
        let mut stmt = tx
            .prepare(
                "SELECT sequence,event_json FROM agent_audit_events
                 WHERE run_id=?1 ORDER BY sequence",
            )
            .map_err(|_| StoreError::Unavailable)?;
        let rows = stmt
            .query_map(params![run_id], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|_| StoreError::Unavailable)?;
        let events = rows
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| StoreError::Unavailable)?;
        if events.len() != snapshot.events.len() {
            return Err(StoreError::Unavailable);
        }
        for ((sequence, json), event) in events.iter().zip(&snapshot.events) {
            if *sequence != i64::try_from(event.sequence).map_err(|_| StoreError::Unavailable)?
                || serde_json::from_str::<scriptotar_agent::AuditEvent>(json)
                    .map_err(|_| StoreError::Unavailable)?
                    != *event
            {
                return Err(StoreError::Unavailable);
            }
        }
        Ok(Some((revision, snapshot)))
    }

    fn append_events(
        tx: &Transaction<'_>,
        run_id: &str,
        events: &[scriptotar_agent::AuditEvent],
    ) -> Result<(), StoreError> {
        for event in events {
            let json = serde_json::to_string(event).map_err(|_| StoreError::Unavailable)?;
            let sequence = i64::try_from(event.sequence).map_err(|_| StoreError::Conflict)?;
            tx.execute(
                "INSERT INTO agent_audit_events(run_id,sequence,event_json) VALUES(?1,?2,?3)",
                params![run_id, sequence, json],
            )
            .map_err(|_| StoreError::Unavailable)?;
        }
        Ok(())
    }
}

impl CheckpointStore for ScopedAgentCheckpointStore {
    fn create(&self, snapshot: RunSnapshot) -> Result<(), StoreError> {
        if snapshot.revision != 0 {
            return Err(StoreError::Conflict);
        }
        let json = self.serialize(&snapshot)?;
        let mut db = database(&self.db).map_err(|_| StoreError::Unavailable)?;
        let tx = db
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| StoreError::Unavailable)?;
        let owned: i64 = tx
            .query_row(
                "SELECT EXISTS(
                   SELECT 1 FROM content_studies s
                   JOIN projects p ON p.id=s.project_id
                   WHERE s.id=?1 AND s.project_id=?2
                     AND s.archived_at IS NULL AND p.archived_at IS NULL
                 )",
                params![self.study_id.to_string(), self.project_id.to_string()],
                |row| row.get(0),
            )
            .map_err(|_| StoreError::Unavailable)?;
        if owned != 1 {
            return Err(StoreError::Conflict);
        }
        let exists: i64 = tx
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM agent_runs WHERE id=?1)",
                params![snapshot.plan.id],
                |row| row.get(0),
            )
            .map_err(|_| StoreError::Unavailable)?;
        if exists != 0 {
            return Err(StoreError::Exists);
        }
        tx.execute(
            "INSERT INTO agent_runs(
              id,project_id,study_id,revision,snapshot_json,created_at,updated_at
             ) VALUES(?1,?2,?3,0,?4,?5,?5)",
            params![
                snapshot.plan.id,
                self.project_id.to_string(),
                self.study_id.to_string(),
                json,
                scriptotar_core::now_rfc3339(),
            ],
        )
        .map_err(|_| StoreError::Unavailable)?;
        Self::append_events(&tx, &snapshot.plan.id, &snapshot.events)?;
        tx.commit().map_err(|_| StoreError::Unavailable)
    }

    fn load(&self, run_id: &str) -> Result<Option<RunSnapshot>, StoreError> {
        if !scriptotar_agent::valid_id(run_id) {
            return Err(StoreError::NotFound);
        }
        let mut db = database(&self.db).map_err(|_| StoreError::Unavailable)?;
        let tx = db
            .transaction_with_behavior(TransactionBehavior::Deferred)
            .map_err(|_| StoreError::Unavailable)?;
        let snapshot = self.read_checkpoint(&tx, run_id)?.map(|(_, run)| run);
        tx.commit().map_err(|_| StoreError::Unavailable)?;
        Ok(snapshot)
    }

    fn compare_and_swap(
        &self,
        expected_revision: u64,
        next: RunSnapshot,
    ) -> Result<(), StoreError> {
        let json = self.serialize(&next)?;
        let expected = i64::try_from(expected_revision).map_err(|_| StoreError::Conflict)?;
        if next.revision != expected_revision.checked_add(1).ok_or(StoreError::Conflict)? {
            return Err(StoreError::Conflict);
        }
        let mut db = database(&self.db).map_err(|_| StoreError::Unavailable)?;
        let tx = db
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| StoreError::Unavailable)?;
        let Some((current_revision, current)) =
            self.read_checkpoint(&tx, &next.plan.id)?
        else {
            return Err(StoreError::NotFound);
        };
        if current_revision != expected
            || current.plan != next.plan
            || current.schema_version != next.schema_version
            || current.steps.len() != next.steps.len()
            || current.started_ms != next.started_ms
            || next.events.len() <= current.events.len()
            || !next.events.starts_with(&current.events)
        {
            return Err(StoreError::Conflict);
        }
        let changed = tx
            .execute(
                "UPDATE agent_runs SET revision=?1,snapshot_json=?2,updated_at=?3
                 WHERE id=?4 AND project_id=?5 AND study_id=?6 AND revision=?7
                   AND EXISTS(
                     SELECT 1 FROM content_studies s JOIN projects p ON p.id=s.project_id
                     WHERE s.id=?6 AND s.project_id=?5
                       AND s.archived_at IS NULL AND p.archived_at IS NULL
                   )",
                params![
                    expected + 1,
                    json,
                    scriptotar_core::now_rfc3339(),
                    next.plan.id,
                    self.project_id.to_string(),
                    self.study_id.to_string(),
                    expected,
                ],
            )
            .map_err(|_| StoreError::Unavailable)?;
        if changed != 1 {
            return Err(StoreError::Conflict);
        }
        Self::append_events(&tx, &next.plan.id, &next.events[current.events.len()..])?;
        tx.commit().map_err(|_| StoreError::Unavailable)
    }
}
