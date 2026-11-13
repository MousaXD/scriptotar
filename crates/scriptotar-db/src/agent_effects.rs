//! Durable effect claim fence for future permissioned external tools.
//! Claim transaction MUST finish before invoking any side effect. An existing
//! reservation is NOT permission to dispatch again: probe the real service.
use rusqlite::{params, OptionalExtension, Row, TransactionBehavior};
use scriptotar_agent::{valid_id, RunSnapshot, RunState, StepState};
use scriptotar_core::{now_rfc3339, RepositoryError, RepositoryResult};
use uuid::Uuid;

use crate::{studies::{database, db_error}, SqliteStore};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectStatus {
    Reserved,
    Accepted,
    Applied,
    NotApplied,
    Unknown,
}
impl EffectStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Reserved => "reserved",
            Self::Accepted => "accepted",
            Self::Applied => "applied",
            Self::NotApplied => "not_applied",
            Self::Unknown => "unknown",
        }
    }
    fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "reserved" => Self::Reserved,
            "accepted" => Self::Accepted,
            "applied" => Self::Applied,
            "not_applied" => Self::NotApplied,
            "unknown" => Self::Unknown,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentEffect {
    pub idempotency_key: String,
    pub run_id: String,
    pub project_id: Uuid,
    pub study_id: Uuid,
    pub step_id: String,
    pub tool_kind: String,
    pub status: EffectStatus,
    pub external_id: Option<String>,
    pub artifact_id: Option<String>,
    pub revision: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EffectClaim {
    /// Exactly one caller receives the first durable claim. The claim is
    /// committed before any real external tool adapter may be invoked.
    Claimed(AgentEffect),
    /// No dispatch is allowed. Reconcile the existing effect instead.
    Existing(AgentEffect),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EffectUpdate {
    /// Service supplied a durable operation ID; acceptance is NOT completion.
    Accepted { effect_id: String },
    /// External service confirmed final success, not merely queued.
    Applied { artifact_id: String },
    /// Service proved the effect did not happen. No automatic replay.
    NotApplied,
    /// Outcome cannot be proven; manual reconciliation required.
    Unknown,
}
impl EffectUpdate {
    fn status(&self) -> EffectStatus {
        match self {
            Self::Accepted { .. } => EffectStatus::Accepted,
            Self::Applied { .. } => EffectStatus::Applied,
            Self::NotApplied => EffectStatus::NotApplied,
            Self::Unknown => EffectStatus::Unknown,
        }
    }
}

fn invalid(value: &str) -> RepositoryError {
    RepositoryError::Validation(format!("invalid agent effect {value}"))
}

fn effect_row(row: &Row<'_>) -> rusqlite::Result<AgentEffect> {
    let status = row.get::<_, String>(6)?;
    Ok(AgentEffect {
        idempotency_key: row.get(0)?,
        run_id: row.get(1)?,
        project_id: Uuid::parse_str(&row.get::<_, String>(2)?)
            .map_err(|_| rusqlite::Error::InvalidQuery)?,
        study_id: Uuid::parse_str(&row.get::<_, String>(3)?)
            .map_err(|_| rusqlite::Error::InvalidQuery)?,
        step_id: row.get(4)?,
        tool_kind: row.get(5)?,
        status: EffectStatus::parse(&status).ok_or(rusqlite::Error::InvalidQuery)?,
        external_id: row.get(7)?,
        artifact_id: row.get(8)?,
        revision: row.get(9)?,
    })
}

const EFFECT_SELECT: &str = "SELECT idempotency_key,run_id,project_id,study_id,step_id,\
 tool_kind,status,external_id,artifact_id,revision FROM agent_effects";

fn scoped_effect(
    connection: &rusqlite::Connection,
    project: Uuid,
    study: Uuid,
    run_id: &str,
    idempotency_key: &str,
) -> RepositoryResult<Option<AgentEffect>> {
    connection
        .query_row(
            &format!("{EFFECT_SELECT} WHERE idempotency_key=?1 AND project_id=?2 AND study_id=?3 AND run_id=?4"),
            params![idempotency_key, project.to_string(), study.to_string(), run_id],
            effect_row,
        )
        .optional()
        .map_err(db_error)
}

fn verified_scope(
    connection: &rusqlite::Connection,
    project: Uuid,
    study: Uuid,
    run_id: &str,
) -> RepositoryResult<RunSnapshot> {
    let raw: String = connection
        .query_row(
            "SELECT r.snapshot_json FROM agent_runs r
             JOIN content_studies s ON s.id=r.study_id AND s.project_id=r.project_id
             JOIN projects p ON p.id=r.project_id
             WHERE r.id=?1 AND r.project_id=?2 AND r.study_id=?3
               AND s.archived_at IS NULL AND p.archived_at IS NULL",
            params![run_id, project.to_string(), study.to_string()],
            |row| row.get(0),
        )
        .optional()
        .map_err(db_error)?
        .ok_or_else(|| RepositoryError::NotFound("active agent run".into()))?;
    let snapshot: RunSnapshot = serde_json::from_str(&raw)
        .map_err(|_| RepositoryError::Storage("agent checkpoint is corrupt".into()))?;
    if snapshot.plan.id != run_id
        || snapshot.plan.project_id != project.to_string()
        || snapshot.plan.study_id != study.to_string()
        || !snapshot.plan.validate()
        || snapshot.current_step >= snapshot.plan.steps.len()
        || snapshot.steps.len() != snapshot.plan.steps.len()
    {
        return Err(RepositoryError::Storage("agent checkpoint is invalid".into()));
    }
    Ok(snapshot)
}

impl SqliteStore {
    /// A scoped, immutable claim made ONLY after the engine persisted its
    /// Running intent and (for mutating/paid tools) explicit approval.
    /// Existing, unknown or terminal claims must never cause re-execution.
    pub fn reserve_agent_effect(
        &self,
        project: Uuid,
        study: Uuid,
        run_id: &str,
        step_id: &str,
        idempotency_key: &str,
    ) -> RepositoryResult<EffectClaim> {
        if !valid_id(run_id) || !valid_id(step_id)
            || idempotency_key.is_empty() || idempotency_key.len() > 256
        {
            return Err(invalid("identity"));
        }
        let mut db = database(self)?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(db_error)?;
        let snapshot = verified_scope(&tx, project, study, run_id)?;
        // Check existing claim before inspecting step state: after a restart
        // the engine may already have advanced, but the external effect may
        // still be pending and must not be dispatched again.
        let exists: Option<AgentEffect> = tx
            .query_row(
                &format!("{EFFECT_SELECT} WHERE idempotency_key=?1"),
                params![idempotency_key], effect_row
            )
            .optional().map_err(db_error)?;
        if let Some(effect) = exists {
            if effect.run_id != run_id || effect.study_id != study
                || effect.project_id != project || effect.step_id != step_id
            {
                return Err(RepositoryError::Conflict(
                    "agent effect identity belongs to a different scope".into()
                ));
            }
            tx.commit().map_err(db_error)?;
            return Ok(EffectClaim::Existing(effect));
        }
        let step = &snapshot.plan.steps[snapshot.current_step];
        let record = &snapshot.steps[snapshot.current_step];
        if snapshot.state != RunState::Running
            || record.state != StepState::Running
            || record.id != step.id
            || step.id != step_id
            || record.idempotency_key != idempotency_key
            || record.attempts == 0
            || (step.approval_required() && !record.approved)
        {
            return Err(RepositoryError::Conflict(
                "agent step was not durably approved and dispatched".into()
            ));
        }
        let tool_kind = format!("{:?}", step.input.kind());
        let now = now_rfc3339();
        tx.execute(
            "INSERT INTO agent_effects(
                idempotency_key,run_id,project_id,study_id,step_id,tool_kind,
                status,revision,created_at,updated_at
             ) VALUES(?1,?2,?3,?4,?5,?6,'reserved',1,?7,?7)",
            params![
                idempotency_key, run_id, project.to_string(), study.to_string(),
                step_id, tool_kind, now
            ],
        ).map_err(db_error)?;
        let claimed = scoped_effect(&tx, project, study, run_id, idempotency_key)?
            .ok_or_else(|| RepositoryError::Storage("agent effect reservation missing".into()))?;
        tx.commit().map_err(db_error)?;
        Ok(EffectClaim::Claimed(claimed))
    }

    /// Read-only probe. No evidence of a claim is NOT proof of nonapplication.
    /// Provider adapters must query the actual service before deciding retry.
    pub fn get_agent_effect(
        &self,
        project: Uuid,
        study: Uuid,
        run_id: &str,
        idempotency_key: &str,
    ) -> RepositoryResult<Option<AgentEffect>> {
        let db = database(self)?;
        verified_scope(&db, project, study, run_id)?;
        scoped_effect(&db, project, study, run_id, idempotency_key)
    }

    /// Only the actual external tool/service adapter should call this after
    /// verifying the external operation's receipt or a proven nonapplication.
    pub fn record_agent_effect(
        &self,
        project: Uuid,
        study: Uuid,
        run_id: &str,
        idempotency_key: &str,
        update: EffectUpdate,
    ) -> RepositoryResult<AgentEffect> {
        let external = match &update {
            EffectUpdate::Accepted { effect_id } if valid_id(effect_id) => Some(effect_id.as_str()),
            EffectUpdate::Accepted { .. } => return Err(invalid("external receipt")),
            _ => None,
        };
        let artifact = match &update {
            EffectUpdate::Applied { artifact_id } if valid_id(artifact_id) => Some(artifact_id.as_str()),
            EffectUpdate::Applied { .. } => return Err(invalid("artifact receipt")),
            _ => None,
        };
        let mut db = database(self)?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(db_error)?;
        verified_scope(&tx, project, study, run_id)?;
        let old = scoped_effect(&tx, project, study, run_id, idempotency_key)?
            .ok_or_else(|| RepositoryError::NotFound("agent effect reservation".into()))?;
        if !matches!(
            (old.status, update.status()),
            (EffectStatus::Reserved, EffectStatus::Accepted | EffectStatus::Applied | EffectStatus::NotApplied | EffectStatus::Unknown)
                | (EffectStatus::Accepted, EffectStatus::Applied | EffectStatus::NotApplied | EffectStatus::Unknown)
                | (EffectStatus::Unknown, EffectStatus::Applied | EffectStatus::NotApplied)
        ) {
            return Err(RepositoryError::Conflict("agent effect transition forbidden".into()));
        }
        let changed = tx.execute(
            "UPDATE agent_effects SET status=?1,external_id=?2,artifact_id=?3,
                revision=revision+1,updated_at=?4
             WHERE idempotency_key=?5 AND run_id=?6 AND project_id=?7
               AND study_id=?8 AND revision=?9
               AND EXISTS (
                 SELECT 1 FROM content_studies s JOIN projects p ON p.id=s.project_id
                 WHERE s.id=?8 AND s.project_id=?7
                 AND s.archived_at IS NULL AND p.archived_at IS NULL
               )",
            params![
                update.status().as_str(), external.or(old.external_id.as_deref()), artifact,
                now_rfc3339(), idempotency_key, run_id, project.to_string(),
                study.to_string(), old.revision,
            ],
        ).map_err(db_error)?;
        if changed != 1 {
            return Err(RepositoryError::Conflict("agent effect changed concurrently".into()));
        }
        let after = scoped_effect(&tx, project, study, run_id, idempotency_key)?
            .ok_or_else(|| RepositoryError::Storage("effect result missing".into()))?;
        tx.commit().map_err(db_error)?;
        Ok(after)
    }
}
