// External, potentially billable effects. Existing v8 local-read ledger is not changed.
// Reserve and persist DISPATCHED before I/O; a DISPATCHED/UNCERTAIN request
// is NEVER an invitation to resend it. Provider reconciliation is mandatory.
use rusqlite::{params, OptionalExtension, Row, Transaction, TransactionBehavior};
use scriptotar_agent::{valid_id, RunState, StepInput, StepState};
use scriptotar_core::{now_rfc3339, RepositoryError, RepositoryResult};
use uuid::Uuid;
use crate::{studies::{database, db_error}, SqliteStore};
use super::{verified_scope, invalid};

const MAX_ATTEMPTS: i64 = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExternalEffectStatus {
    Reserved, Dispatched, Completed, Failed, Cancelled, Uncertain, NotExecuted,
}
impl ExternalEffectStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Reserved => "reserved", Self::Dispatched => "dispatched",
            Self::Completed => "completed", Self::Failed => "failed",
            Self::Cancelled => "cancelled", Self::Uncertain => "uncertain",
            Self::NotExecuted => "not_executed",
        }
    }
    fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "reserved" => Self::Reserved, "dispatched" => Self::Dispatched,
            "completed" => Self::Completed, "failed" => Self::Failed,
            "cancelled" => Self::Cancelled, "uncertain" => Self::Uncertain,
            "not_executed" => Self::NotExecuted, _ => return None,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalEffect {
    pub idempotency_key: String,
    pub project_id: Uuid,
    pub study_id: Uuid,
    pub run_id: String,
    pub step_id: String,
    pub fingerprint: String,
    pub provider: String,
    pub status: ExternalEffectStatus,
    pub attempt: i64,
    pub token_limit: u64,
    pub cost_limit_microusd: u64,
    pub actual_tokens: u64,
    pub actual_cost_microusd: u64,
    pub provider_request_id: Option<String>,
    pub receipt_id: Option<String>,
    pub reconciliation_id: Option<String>,
    pub cancel_requested: bool,
    pub revision: i64,
}

#[derive(Debug, Clone, Copy)]
pub struct ExternalEffectKey<'a> {
    pub project: Uuid,
    pub study: Uuid,
    pub run_id: &'a str,
    pub step_id: &'a str,
    pub idempotency_key: &'a str,
}

#[derive(Debug, Clone, Copy)]
pub struct ExternalEffectLimits {
    pub tokens: u64,
    pub cost_microusd: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExternalReservation {
    Fresh(ExternalEffect),
    Existing(ExternalEffect),
}

/// Only trusted provider adapters can provide these verified receipts. Do NOT
/// expose this enum as a user-controlled IPC input. A failed HTTP connection
/// is Uncertain, never proof of NotExecuted.
#[derive(Debug, Clone)]
pub enum ExternalResolution<'a> {
    Completed {
        provider_request_id: Option<&'a str>, receipt_id: &'a str,
        tokens: u64, cost_microusd: u64,
    },
    Failed {
        provider_request_id: Option<&'a str>, receipt_id: &'a str,
        tokens: u64, cost_microusd: u64,
    },
    ProviderCancelled { evidence_id: &'a str },
    VerifiedNotExecuted { evidence_id: &'a str },
    Uncertain,
}

fn bounded_id(id: &str) -> bool {
    (1..=128).contains(&id.len())
        && id.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.' | b':' | b'/'))
}
fn fingerprint_ok(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
}
fn to_i64(n: u64) -> RepositoryResult<i64> {
    i64::try_from(n).map_err(|_| invalid("numeric bound"))
}
fn to_u64(n: i64) -> rusqlite::Result<u64> {
    u64::try_from(n).map_err(|_| rusqlite::Error::InvalidQuery)
}
fn conflict(msg: &str) -> RepositoryError { RepositoryError::Conflict(msg.into()) }

/// Additive schema, intentionally independent of historic v8 transitions.
/// Agent 00 owns registration in the versioned central migration ledger.
pub(crate) fn migration_external(tx: &Transaction<'_>) -> rusqlite::Result<()> {
    tx.execute_batch(r#"
CREATE TABLE IF NOT EXISTS agent_external_effects (
  idempotency_key TEXT PRIMARY KEY REFERENCES agent_effects(idempotency_key) ON DELETE CASCADE,
  project_id TEXT NOT NULL, study_id TEXT NOT NULL, run_id TEXT NOT NULL,
  step_id TEXT NOT NULL, fingerprint TEXT NOT NULL CHECK(length(fingerprint)=64),
  provider TEXT NOT NULL CHECK(length(provider) BETWEEN 1 AND 64),
  status TEXT NOT NULL CHECK(status IN
    ('reserved','dispatched','completed','failed','cancelled','uncertain','not_executed')),
  attempt INTEGER NOT NULL DEFAULT 1 CHECK(attempt BETWEEN 1 AND 3),
  token_limit INTEGER NOT NULL CHECK(token_limit>=0),
  cost_limit_microusd INTEGER NOT NULL CHECK(cost_limit_microusd>=0),
  actual_tokens INTEGER NOT NULL DEFAULT 0 CHECK(actual_tokens>=0 AND actual_tokens<=token_limit),
  actual_cost_microusd INTEGER NOT NULL DEFAULT 0 CHECK(actual_cost_microusd>=0 AND actual_cost_microusd<=cost_limit_microusd),
  provider_request_id TEXT CHECK(provider_request_id IS NULL OR length(provider_request_id)<=128),
  receipt_id TEXT CHECK(receipt_id IS NULL OR length(receipt_id)<=128),
  reconciliation_id TEXT CHECK(reconciliation_id IS NULL OR length(reconciliation_id)<=128),
  cancel_requested INTEGER NOT NULL DEFAULT 0 CHECK(cancel_requested IN (0,1)),
  revision INTEGER NOT NULL DEFAULT 1 CHECK(revision>=1),
  created_at TEXT NOT NULL, updated_at TEXT NOT NULL,
  UNIQUE(project_id,study_id,run_id,step_id,fingerprint),
  CHECK(status NOT IN ('completed','failed') OR receipt_id IS NOT NULL),
  CHECK(status!='not_executed' OR reconciliation_id IS NOT NULL)
);
CREATE TABLE IF NOT EXISTS agent_external_attempt_history (
  idempotency_key TEXT NOT NULL REFERENCES agent_external_effects(idempotency_key) ON DELETE CASCADE,
  attempt INTEGER NOT NULL CHECK(attempt BETWEEN 1 AND 3),
  evidence_id TEXT NOT NULL CHECK(length(evidence_id) BETWEEN 1 AND 128),
  settled_at TEXT NOT NULL,
  PRIMARY KEY(idempotency_key,attempt)
);
CREATE TRIGGER IF NOT EXISTS external_attempt_history_immutable
 BEFORE UPDATE ON agent_external_attempt_history
 BEGIN SELECT RAISE(ABORT,'external attempt history immutable'); END;
CREATE INDEX IF NOT EXISTS idx_external_effects_scope
 ON agent_external_effects(project_id,study_id,run_id,status);
CREATE TRIGGER IF NOT EXISTS external_effects_owner_insert
 BEFORE INSERT ON agent_external_effects
 WHEN NOT EXISTS (SELECT 1 FROM agent_effects e WHERE
  e.idempotency_key=NEW.idempotency_key AND e.project_id=NEW.project_id
  AND e.study_id=NEW.study_id AND e.run_id=NEW.run_id AND e.step_id=NEW.step_id
  AND e.status='reserved')
 BEGIN SELECT RAISE(ABORT,'external effect must match reserved scoped v8 claim'); END;
CREATE TRIGGER IF NOT EXISTS external_effects_immutable_identity
 BEFORE UPDATE OF idempotency_key,project_id,study_id,run_id,step_id,fingerprint,provider,
                  token_limit,cost_limit_microusd ON agent_external_effects
 BEGIN SELECT RAISE(ABORT,'external effect identity and limits immutable'); END;
CREATE TRIGGER IF NOT EXISTS external_effects_transitions
 BEFORE UPDATE OF status ON agent_external_effects
 WHEN NOT (
 (OLD.status='reserved' AND NEW.status IN ('dispatched','cancelled'))
 OR (OLD.status='dispatched' AND NEW.status IN
     ('completed','failed','cancelled','uncertain','not_executed'))
 OR (OLD.status='uncertain' AND NEW.status IN
     ('completed','failed','cancelled','not_executed'))
 OR (OLD.status='not_executed' AND NEW.status='reserved'
     AND NEW.attempt=OLD.attempt+1 AND OLD.attempt<3)
 )
 BEGIN SELECT RAISE(ABORT,'external effect transition forbidden'); END;
CREATE TRIGGER IF NOT EXISTS external_effects_terminal_guard
 BEFORE UPDATE ON agent_external_effects
 WHEN OLD.status IN ('completed','failed','cancelled')
 BEGIN SELECT RAISE(ABORT,'external effect terminal receipt immutable'); END;
"#)
}

/// v12: bind one provider-issued response ID to one *effect attempt* across both
/// acknowledgement and receipt roles. The same authenticated response ID may
/// appear in both columns of a single attempt; no other effect or attempt may
/// claim that identity. This extends v10 without altering historical migrations.
///
/// v11 retry rows lost the previous attempt's provider-request ID when the
/// current fields were cleared. Since these old identities cannot be recovered
/// from the v11 attempt-history table, refuse their migration, preserving all
/// data so a trusted reconciliation tool can be designed separately. No paid
/// dispatch is permitted through a fabricated or silently migrated tombstone.
pub(crate) fn migration_provider_identities(tx: &Transaction<'_>) -> rusqlite::Result<()> {
    let legacy_retry: Option<String> = tx.query_row(
        "SELECT idempotency_key FROM agent_external_effects WHERE attempt > 1 LIMIT 1",
        [],
        |row| row.get(0),
    ).optional()?;
    if legacy_retry.is_some() {
        return Err(rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_CONSTRAINT),
            Some("v12 migration refused: historical retry has unrecoverable prior provider identity; database unchanged".into()),
        ));
    }
    tx.execute_batch(
        r#"
CREATE TABLE agent_external_provider_identities (
    provider TEXT NOT NULL COLLATE NOCASE CHECK(length(provider) BETWEEN 1 AND 64),
    -- Origin role is diagnostic metadata, never a uniqueness namespace.
    kind TEXT NOT NULL CHECK(kind IN ('request','receipt')),
    identity TEXT NOT NULL CHECK(length(identity) BETWEEN 1 AND 128),
    idempotency_key TEXT NOT NULL,
    attempt INTEGER NOT NULL CHECK(attempt BETWEEN 1 AND 3),
    claimed_at TEXT NOT NULL,
    PRIMARY KEY(provider, identity)
);
INSERT INTO agent_external_provider_identities(provider, kind, identity, idempotency_key, attempt, claimed_at)
    SELECT provider, MIN(kind), identity, idempotency_key, attempt, MAX(claimed_at)
    FROM (
        SELECT provider, 'request' AS kind, provider_request_id AS identity, idempotency_key, attempt,
               updated_at AS claimed_at
        FROM agent_external_effects WHERE provider_request_id IS NOT NULL
        UNION ALL
        SELECT provider, 'receipt' AS kind, receipt_id AS identity, idempotency_key, attempt,
               updated_at AS claimed_at
        FROM agent_external_effects WHERE receipt_id IS NOT NULL
    )
    GROUP BY provider COLLATE NOCASE, identity, idempotency_key, attempt;
CREATE UNIQUE INDEX idx_external_provider_request_unique
    ON agent_external_effects(provider COLLATE NOCASE, provider_request_id)
    WHERE provider_request_id IS NOT NULL;
CREATE UNIQUE INDEX idx_external_provider_receipt_unique
    ON agent_external_effects(provider COLLATE NOCASE, receipt_id)
    WHERE receipt_id IS NOT NULL;
-- The table is an append-only anti-replay tombstone, not an editable cache.
-- Dropping the owning effect and retrying cannot release an identity claim.
CREATE TRIGGER external_provider_identity_claims_immutable_update
    BEFORE UPDATE ON agent_external_provider_identities
    BEGIN SELECT RAISE(ABORT,'provider identity claim immutable'); END;
CREATE TRIGGER external_provider_identity_claims_immutable_delete
    BEFORE DELETE ON agent_external_provider_identities
    BEGIN SELECT RAISE(ABORT,'provider identity claim immutable'); END;
CREATE TRIGGER external_provider_identity_on_insert
AFTER INSERT ON agent_external_effects
BEGIN
    INSERT OR IGNORE INTO agent_external_provider_identities
        (provider, kind, identity, idempotency_key, attempt, claimed_at)
        SELECT NEW.provider, 'request', NEW.provider_request_id, NEW.idempotency_key, NEW.attempt, NEW.updated_at
        WHERE NEW.provider_request_id IS NOT NULL;
    SELECT CASE WHEN NEW.provider_request_id IS NOT NULL
        AND NOT EXISTS (
            SELECT 1 FROM agent_external_provider_identities
            WHERE provider=NEW.provider AND identity=NEW.provider_request_id
              AND idempotency_key=NEW.idempotency_key AND attempt=NEW.attempt)
        THEN RAISE(ABORT, 'provider acknowledgement identity already claimed') END;
    INSERT OR IGNORE INTO agent_external_provider_identities
        (provider, kind, identity, idempotency_key, attempt, claimed_at)
        SELECT NEW.provider, 'receipt', NEW.receipt_id, NEW.idempotency_key, NEW.attempt, NEW.updated_at
        WHERE NEW.receipt_id IS NOT NULL;
    SELECT CASE WHEN NEW.receipt_id IS NOT NULL
        AND NOT EXISTS (
            SELECT 1 FROM agent_external_provider_identities
            WHERE provider=NEW.provider AND identity=NEW.receipt_id
              AND idempotency_key=NEW.idempotency_key AND attempt=NEW.attempt)
        THEN RAISE(ABORT, 'provider receipt identity already claimed') END;
END;
CREATE TRIGGER external_provider_identity_on_update
AFTER UPDATE OF attempt, provider_request_id, receipt_id ON agent_external_effects
BEGIN
    INSERT OR IGNORE INTO agent_external_provider_identities
        (provider, kind, identity, idempotency_key, attempt, claimed_at)
        SELECT NEW.provider, 'request', NEW.provider_request_id, NEW.idempotency_key, NEW.attempt, NEW.updated_at
        WHERE NEW.provider_request_id IS NOT NULL;
    SELECT CASE WHEN NEW.provider_request_id IS NOT NULL
        AND NOT EXISTS (
            SELECT 1 FROM agent_external_provider_identities
            WHERE provider=NEW.provider AND identity=NEW.provider_request_id
              AND idempotency_key=NEW.idempotency_key AND attempt=NEW.attempt)
        THEN RAISE(ABORT, 'provider acknowledgement identity already claimed') END;
    INSERT OR IGNORE INTO agent_external_provider_identities
        (provider, kind, identity, idempotency_key, attempt, claimed_at)
        SELECT NEW.provider, 'receipt', NEW.receipt_id, NEW.idempotency_key, NEW.attempt, NEW.updated_at
        WHERE NEW.receipt_id IS NOT NULL;
    SELECT CASE WHEN NEW.receipt_id IS NOT NULL
        AND NOT EXISTS (
            SELECT 1 FROM agent_external_provider_identities
            WHERE provider=NEW.provider AND identity=NEW.receipt_id
              AND idempotency_key=NEW.idempotency_key AND attempt=NEW.attempt)
        THEN RAISE(ABORT, 'provider receipt identity already claimed') END;
END;
"#,
    )
}

const SELECT: &str = "SELECT idempotency_key,project_id,study_id,run_id,step_id,
 fingerprint,provider,status,attempt,token_limit,cost_limit_microusd,
 actual_tokens,actual_cost_microusd,provider_request_id,receipt_id,
 reconciliation_id,cancel_requested,revision FROM agent_external_effects";

fn row(row: &Row<'_>) -> rusqlite::Result<ExternalEffect> {
    let project: String = row.get(1)?;
    let study: String = row.get(2)?;
    let status: String = row.get(7)?;
    Ok(ExternalEffect {
        idempotency_key: row.get(0)?,
        project_id: Uuid::parse_str(&project).map_err(|_| rusqlite::Error::InvalidQuery)?,
        study_id: Uuid::parse_str(&study).map_err(|_| rusqlite::Error::InvalidQuery)?,
        run_id: row.get(3)?,
        step_id: row.get(4)?,
        fingerprint: row.get(5)?,
        provider: row.get(6)?,
        status: ExternalEffectStatus::parse(&status).ok_or(rusqlite::Error::InvalidQuery)?,
        attempt: row.get(8)?,
        token_limit: to_u64(row.get(9)?)?,
        cost_limit_microusd: to_u64(row.get(10)?)?,
        actual_tokens: to_u64(row.get(11)?)?,
        actual_cost_microusd: to_u64(row.get(12)?)?,
        provider_request_id: row.get(13)?,
        receipt_id: row.get(14)?,
        reconciliation_id: row.get(15)?,
        cancel_requested: row.get::<_, i64>(16)? == 1,
        revision: row.get(17)?,
    })
}
fn scoped(
    db: &rusqlite::Connection, key: ExternalEffectKey<'_>,
) -> RepositoryResult<Option<ExternalEffect>> {
    db.query_row(
        &format!("{SELECT} WHERE idempotency_key=?1 AND project_id=?2
         AND study_id=?3 AND run_id=?4 AND step_id=?5"),
        params![key.idempotency_key, key.project.to_string(), key.study.to_string(),
                key.run_id, key.step_id], row,
    ).optional().map_err(db_error)
}
fn checked(db: &rusqlite::Connection, key: ExternalEffectKey<'_>)
    -> RepositoryResult<ExternalEffect> {
    verified_scope(db, key.project, key.study, key.run_id)?;
    scoped(db, key)?.ok_or_else(|| RepositoryError::NotFound("scoped external effect".into()))
}
fn may_execute(db: &rusqlite::Connection, key: ExternalEffectKey<'_>)
    -> RepositoryResult<()> {
    let run = verified_scope(db, key.project, key.study, key.run_id)?;
    let step = run.plan.steps.get(run.current_step).ok_or_else(|| conflict("run already ended"))?;
    let record = run.steps.get(run.current_step).ok_or_else(|| conflict("run checkpoint invalid"))?;
    if run.state != RunState::Running || run.cancel_requested || run.pause_requested
        || record.state != StepState::Running || record.attempts == 0
        || record.id != key.step_id || step.id != key.step_id
        || record.idempotency_key != key.idempotency_key
        || !record.approved || !step.approval_required()
    {
        return Err(conflict("external effect requires active, approved running step"));
    }
    Ok(())
}

impl SqliteStore {
    /// Atomic v8 claim + scoped external reservation under one IMMEDIATE write
    /// transaction. Never call an external provider for Existing.
    pub fn reserve_external_effect(
        &self, key: ExternalEffectKey<'_>, fingerprint: &str,
        provider: &str, limits: ExternalEffectLimits,
    ) -> RepositoryResult<ExternalReservation> {
        if !valid_id(key.run_id) || !valid_id(key.step_id)
            || key.idempotency_key.is_empty() || key.idempotency_key.len()>256
            || !fingerprint_ok(fingerprint) || !bounded_id(provider) || provider.len()>64
        { return Err(invalid("external identity")); }
        let tokens = to_i64(limits.tokens)?;
        let cost = to_i64(limits.cost_microusd)?;
        let mut db = database(self)?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate).map_err(db_error)?;
        // Schema v10 is installed atomically when SqliteStore opens. No per-request DDL.
        // An existing key from a local tool is not an external reservation.
        if let Some(existing) = scoped(&tx, key)? {
            if existing.fingerprint != fingerprint || existing.provider != provider
                || existing.token_limit != limits.tokens || existing.cost_limit_microusd != limits.cost_microusd
            { return Err(conflict("external effect request fingerprint or limits changed")); }
            tx.commit().map_err(db_error)?;
            return Ok(ExternalReservation::Existing(existing));
        }
        let duplicate: Option<String> = tx.query_row(
            "SELECT idempotency_key FROM agent_effects WHERE idempotency_key=?1",
            params![key.idempotency_key], |r| r.get(0)
        ).optional().map_err(db_error)?;
        if duplicate.is_some() { return Err(conflict("v8 effect key already claimed without external receipt")); }
        may_execute(&tx, key)?;
        let run = verified_scope(&tx, key.project, key.study, key.run_id)?;
        let step = &run.plan.steps[run.current_step];
        if matches!(step.input, StepInput::ReadStudyTranscript { .. }) {
            return Err(conflict("local transcript read must use its unchanged local adapter"));
        }
        if limits.tokens > step.estimate.tokens || limits.cost_microusd > step.estimate.cost_microusd
            || limits.tokens > run.plan.limits.max_tokens
            || limits.cost_microusd > run.plan.limits.max_cost_microusd
        { return Err(conflict("external provider budget exceeds approved checkpoint estimate")); }
        let kind = format!("{:?}", step.input.kind());
        let now = now_rfc3339();
        tx.execute(
            "INSERT INTO agent_effects(idempotency_key,run_id,project_id,study_id,step_id,tool_kind,
             status,revision,created_at,updated_at) VALUES (?1,?2,?3,?4,?5,?6,'reserved',1,?7,?7)",
            params![key.idempotency_key,key.run_id,key.project.to_string(),key.study.to_string(),
                    key.step_id,kind,now],
        ).map_err(db_error)?;
        tx.execute(
            "INSERT INTO agent_external_effects(idempotency_key,project_id,study_id,run_id,
             step_id,fingerprint,provider,status,token_limit,cost_limit_microusd,
             created_at,updated_at) VALUES (?1,?2,?3,?4,?5,?6,?7,'reserved',?8,?9,?10,?10)",
            params![key.idempotency_key,key.project.to_string(),key.study.to_string(),
                    key.run_id,key.step_id,fingerprint,provider,tokens,cost,now],
        ).map_err(db_error)?;
        let fresh = scoped(&tx,key)?.ok_or_else(|| RepositoryError::Storage("external reservation absent".into()))?;
        tx.commit().map_err(db_error)?;
        Ok(ExternalReservation::Fresh(fresh))
    }

    pub fn get_external_effect(&self, key: ExternalEffectKey<'_>)
        -> RepositoryResult<Option<ExternalEffect>> {
        let db = database(self)?;
        verified_scope(&db,key.project,key.study,key.run_id)?;
        // A missing v10 table is corruption, not an empty ledger.
        scoped(&db,key)
    }

    /// A successful CAS is the single dispatch permit. COMMIT before network I/O.
    /// After a crash, never call this method for an already Dispatched record.
    pub fn mark_external_dispatched(
        &self, key: ExternalEffectKey<'_>, expected_revision: i64, attempt: i64,
    ) -> RepositoryResult<ExternalEffect> {
        let mut db=database(self)?;
        let tx=db.transaction_with_behavior(TransactionBehavior::Immediate).map_err(db_error)?;
        may_execute(&tx,key)?;
        let previous=checked(&tx,key)?;
        // Recheck the original v8 claim under the SAME write transaction as
        // DISPATCHED CAS. A cross-process recovery/cancel may have changed the
        // legacy fence before v13 barrier admission; no caller may send when
        // that authoritative claim is already UNKNOWN or terminal.
        let legacy_status: Option<String> = tx
            .query_row(
                "SELECT status FROM agent_effects
                 WHERE idempotency_key=?1 AND project_id=?2 AND study_id=?3
                   AND run_id=?4 AND step_id=?5",
                params![
                    key.idempotency_key,
                    key.project.to_string(),
                    key.study.to_string(),
                    key.run_id,
                    key.step_id
                ],
                |row| row.get(0),
            )
            .optional()
            .map_err(db_error)?;
        if legacy_status.as_deref() != Some("reserved") {
            return Err(conflict("external dispatch requires exact reserved v8 claim"));
        }
        if previous.status != ExternalEffectStatus::Reserved || previous.revision != expected_revision
            || previous.attempt != attempt || previous.cancel_requested {
            return Err(conflict("external effect cannot be dispatched again"));
        }
        let changed=tx.execute(
            "UPDATE agent_external_effects SET status='dispatched',revision=revision+1,updated_at=?1
             WHERE idempotency_key=?2 AND project_id=?3 AND study_id=?4 AND run_id=?5 AND step_id=?6
               AND revision=?7 AND attempt=?8 AND status='reserved' AND cancel_requested=0",
            params![now_rfc3339(),key.idempotency_key,key.project.to_string(),key.study.to_string(),
                key.run_id,key.step_id,expected_revision,attempt],
        ).map_err(db_error)?;
        if changed!=1 { return Err(conflict("external dispatch CAS lost")); }
        let after=checked(&tx,key)?;
        tx.commit().map_err(db_error)?;
        Ok(after)
    }

    /// Store trusted provider evidence. Duplicate identical terminal callbacks
    /// are idempotent; contradictory or older-attempt callbacks are rejected.
    /// Persist an authoritative provider acceptance ID before any follow-up
    /// status polling. This is NOT a completion or replay permit. A crash
    /// after acknowledgement can be reconciled by the exact provider ID.
    pub fn record_external_ack(
        &self,key:ExternalEffectKey<'_>,attempt:i64,request_id:&str,
    ) -> RepositoryResult<ExternalEffect> {
        if !bounded_id(request_id) {
            return Err(invalid("provider acknowledgement"));
        }
        let mut db=database(self)?;
        let tx=db.transaction_with_behavior(TransactionBehavior::Immediate).map_err(db_error)?;
        let old=checked(&tx,key)?;
        if old.attempt!=attempt { return Err(conflict("stale provider acknowledgement")); }
        // Identical delayed callbacks are harmless even after a final receipt.
        if let Some(id)=old.provider_request_id.as_deref() {
            if id==request_id { return Ok(old); }
            return Err(conflict("conflicting provider request ID"));
        }
        if !matches!(old.status,ExternalEffectStatus::Dispatched | ExternalEffectStatus::Uncertain) {
            return Err(conflict("acknowledgement is not for an active dispatched attempt"));
        }
        let updated=tx.execute(
            "UPDATE agent_external_effects SET provider_request_id=?1,
             revision=revision+1,updated_at=?2
             WHERE idempotency_key=?3 AND project_id=?4 AND study_id=?5
               AND run_id=?6 AND step_id=?7 AND attempt=?8
               AND revision=?9 AND status IN ('dispatched','uncertain')
               AND provider_request_id IS NULL",
            params![request_id,now_rfc3339(),key.idempotency_key,
                key.project.to_string(),key.study.to_string(),key.run_id,key.step_id,
                attempt,old.revision],
        ).map_err(db_error)?;
        if updated!=1 { return Err(conflict("provider acknowledgement CAS lost")); }
        let after=checked(&tx,key)?;
        tx.commit().map_err(db_error)?;
        Ok(after)
    }

    pub fn reconcile_external_effect(
        &self, key: ExternalEffectKey<'_>, attempt: i64, resolution: ExternalResolution<'_>,
    ) -> RepositoryResult<ExternalEffect> {
        let mut db=database(self)?;
        let tx=db.transaction_with_behavior(TransactionBehavior::Immediate).map_err(db_error)?;
        let old=checked(&tx,key)?;
        if old.attempt!=attempt { return Err(conflict("stale provider attempt")); }
        let (status, provider_id, receipt_id, evidence_id, tokens, cost) = match resolution {
            ExternalResolution::Completed {provider_request_id,receipt_id,tokens,cost_microusd} =>
                (ExternalEffectStatus::Completed,provider_request_id,Some(receipt_id),None,tokens,cost_microusd),
            ExternalResolution::Failed {provider_request_id,receipt_id,tokens,cost_microusd} =>
                (ExternalEffectStatus::Failed,provider_request_id,Some(receipt_id),None,tokens,cost_microusd),
            ExternalResolution::ProviderCancelled {evidence_id} =>
                (ExternalEffectStatus::Cancelled,None,None,Some(evidence_id),0,0),
            ExternalResolution::VerifiedNotExecuted {evidence_id} =>
                (ExternalEffectStatus::NotExecuted,None,None,Some(evidence_id),0,0),
            ExternalResolution::Uncertain =>
                (ExternalEffectStatus::Uncertain,None,None,None,0,0),
        };
        if provider_id.is_some_and(|v| !bounded_id(v))
            || receipt_id.is_some_and(|v| !bounded_id(v))
            || evidence_id.is_some_and(|v| !bounded_id(v))
            || tokens>old.token_limit || cost>old.cost_limit_microusd {
            return Err(invalid("bounded provider receipt"));
        }
        if let (Some(existing),Some(incoming))=(old.provider_request_id.as_deref(),provider_id) {
            if existing!=incoming { return Err(conflict("provider request identity changed")); }
        }
        if old.status==status && matches!(status,ExternalEffectStatus::Completed
            | ExternalEffectStatus::Failed | ExternalEffectStatus::Cancelled
            | ExternalEffectStatus::NotExecuted)
            && (provider_id.is_none() || old.provider_request_id.as_deref()==provider_id)
            && old.receipt_id.as_deref()==receipt_id
            && old.reconciliation_id.as_deref()==evidence_id
            && old.actual_tokens==tokens && old.actual_cost_microusd==cost {
            return Ok(old);
        }
        if !matches!(old.status,ExternalEffectStatus::Dispatched | ExternalEffectStatus::Uncertain)
            || (old.status==ExternalEffectStatus::Uncertain && status==ExternalEffectStatus::Uncertain) {
            return Err(conflict("effect must be dispatched or uncertain to reconcile"));
        }
        let changed=tx.execute(
            "UPDATE agent_external_effects
             SET status=?1,provider_request_id=COALESCE(?2,provider_request_id),
                 receipt_id=?3,reconciliation_id=?4,actual_tokens=?5,actual_cost_microusd=?6,
                 revision=revision+1,updated_at=?7
             WHERE idempotency_key=?8 AND project_id=?9 AND study_id=?10 AND run_id=?11
               AND step_id=?12 AND revision=?13 AND attempt=?14",
            params![status.as_str(),provider_id,receipt_id,evidence_id,to_i64(tokens)?,to_i64(cost)?,
                now_rfc3339(),key.idempotency_key,key.project.to_string(),key.study.to_string(),
                key.run_id,key.step_id,old.revision,attempt],
        ).map_err(db_error)?;
        if changed!=1 { return Err(conflict("external receipt CAS lost")); }
        // Existing v8 receipt remains the authoritative fence for legacy runtime.
        // This update is in the SAME SQLite transaction as the external receipt.
        let (v8_status, artifact) = match status {
            ExternalEffectStatus::Completed => (Some("applied"),receipt_id),
            // Keep the v8 claim blocked, not terminal, while an explicit
            // proof-only bounded retry may still be authorized.
            ExternalEffectStatus::NotExecuted => (Some("unknown"),None),
            ExternalEffectStatus::Uncertain => (Some("unknown"),None),
            ExternalEffectStatus::Failed | ExternalEffectStatus::Cancelled => (Some("unknown"),None),
            _ => (None,None),
        };
        if let Some(v8_status)=v8_status {
            // Unknown in v8 is a blocked effect; it is never safe to redispatch
            // without a fresh, separately approved key after proven nonexecution.
            // A later positive receipt may still reconcile v8 Unknown -> Applied.
            let current: String=tx.query_row(
                "SELECT status FROM agent_effects WHERE idempotency_key=?1",
                params![key.idempotency_key], |r| r.get(0)
            ).map_err(db_error)?;
            let next = if current=="unknown" && v8_status=="unknown" {
                None
            } else if current=="reserved" || (current=="unknown" && v8_status!="unknown") {
                Some(v8_status)
            } else { return Err(conflict("v8 effect transition conflicts with receipt")); };
            if let Some(next)=next {
                tx.execute(
                    "UPDATE agent_effects SET status=?1,external_id=COALESCE(?2,external_id),
                     artifact_id=?3,revision=revision+1,updated_at=?4 WHERE idempotency_key=?5",
                    params![next,provider_id.or(old.provider_request_id.as_deref()),
                        artifact,now_rfc3339(),key.idempotency_key],
                ).map_err(db_error)?;
            }
        }
        let after=checked(&tx,key)?;
        tx.commit().map_err(db_error)?;
        Ok(after)
    }

    /// Stops unstarted work immediately. For dispatched work records ONLY a
    /// cancellation request: actual cancellation requires provider evidence.
    pub fn request_external_cancel(&self,key:ExternalEffectKey<'_>)
        -> RepositoryResult<ExternalEffect> {
        let mut db=database(self)?;
        let tx=db.transaction_with_behavior(TransactionBehavior::Immediate).map_err(db_error)?;
        let old=checked(&tx,key)?;
        if old.status==ExternalEffectStatus::Reserved {
            tx.execute(
                "UPDATE agent_external_effects SET status='cancelled',cancel_requested=1,
                 revision=revision+1,updated_at=?1 WHERE idempotency_key=?2 AND revision=?3",
                params![now_rfc3339(),key.idempotency_key,old.revision],
            ).map_err(db_error)?;
            // A first-attempt reservation has never gone to the provider.
            // A prepared retry has *earlier* dispatched history and its v8
            // Unknown fence must remain blocking even though the retry itself
            // can be stopped before sending any additional request.
            if old.attempt==1 {
                let settled=tx.execute(
                    "UPDATE agent_effects SET status='not_applied',revision=revision+1,
                     updated_at=?1 WHERE idempotency_key=?2 AND project_id=?3
                     AND study_id=?4 AND run_id=?5 AND status='reserved'",
                    params![now_rfc3339(),key.idempotency_key,key.project.to_string(),
                        key.study.to_string(),key.run_id],
                ).map_err(db_error)?;
                if settled!=1 { return Err(conflict("legacy effect fence changed before cancellation")); }
            } else {
                let fence:String=tx.query_row(
                    "SELECT status FROM agent_effects WHERE idempotency_key=?1
                     AND project_id=?2 AND study_id=?3 AND run_id=?4",
                    params![key.idempotency_key,key.project.to_string(),
                        key.study.to_string(),key.run_id],
                    |r|r.get(0),
                ).map_err(db_error)?;
                if fence!="unknown" { return Err(conflict("retry cancellation requires blocked v8 fence")); }
            }
        } else if matches!(old.status,ExternalEffectStatus::Dispatched | ExternalEffectStatus::Uncertain)
            && !old.cancel_requested {
            tx.execute(
                "UPDATE agent_external_effects SET cancel_requested=1,revision=revision+1,
                 updated_at=?1 WHERE idempotency_key=?2 AND revision=?3",
                params![now_rfc3339(),key.idempotency_key,old.revision],
            ).map_err(db_error)?;
            // A dispatched cancellation request does not prove a provider stop.
            // Preserve the pending/uncertain fence until a verified receipt.
            tx.execute(
                "UPDATE agent_effects SET status='unknown',revision=revision+1,
                 updated_at=?1 WHERE idempotency_key=?2 AND project_id=?3
                 AND study_id=?4 AND run_id=?5 AND status='reserved'",
                params![now_rfc3339(),key.idempotency_key,key.project.to_string(),
                    key.study.to_string(),key.run_id],
            ).map_err(db_error)?;
        } else if matches!(old.status,ExternalEffectStatus::Completed
            | ExternalEffectStatus::Failed | ExternalEffectStatus::NotExecuted) {
            return Err(conflict("completed or resolved external request cannot be cancelled"));
        }
        let after=checked(&tx,key)?;
        tx.commit().map_err(db_error)?;
        Ok(after)
    }

    /// Converts outstanding dispatched requests to durable, blocking uncertain
    /// states after restart. Never resubmits them; provider probe is mandatory.
    pub fn recover_external_effects(&self,project:Uuid,study:Uuid,run_id:&str)
        -> RepositoryResult<usize> {
        let mut db=database(self)?;
        let tx=db.transaction_with_behavior(TransactionBehavior::Immediate).map_err(db_error)?;
        verified_scope(&tx,project,study,run_id)?;
        let changed=tx.execute(
            "UPDATE agent_external_effects SET status='uncertain',revision=revision+1,
             updated_at=?1 WHERE project_id=?2 AND study_id=?3 AND run_id=?4 AND status='dispatched'",
            params![now_rfc3339(),project.to_string(),study.to_string(),run_id],
        ).map_err(db_error)?;
        tx.execute(
            "UPDATE agent_effects SET status='unknown',revision=revision+1,updated_at=?1
             WHERE project_id=?2 AND study_id=?3 AND run_id=?4 AND status='reserved'
             AND idempotency_key IN (SELECT idempotency_key FROM agent_external_effects
                 WHERE status='uncertain' AND project_id=?2 AND study_id=?3 AND run_id=?4)",
            params![now_rfc3339(),project.to_string(),study.to_string(),run_id],
        ).map_err(db_error)?;
        tx.commit().map_err(db_error)?;
        Ok(changed)
    }

    /// Only a trusted adapter holding a provider-confirmed nonexecution receipt
    /// may explicitly prepare one more attempt. Never invoked by recovery.
    /// It does not execute or dispatch anything and is bounded to three total.
    pub fn prepare_external_retry(
        &self,key:ExternalEffectKey<'_>, expected_revision:i64,
    ) -> RepositoryResult<ExternalEffect> {
        let mut db=database(self)?;
        let tx=db.transaction_with_behavior(TransactionBehavior::Immediate).map_err(db_error)?;
        may_execute(&tx,key)?; // repeated policy approval is also required in the service driver
        let run=verified_scope(&tx,key.project,key.study,key.run_id)?;
        let max_by_plan=i64::from(run.plan.steps[run.current_step].max_attempts);
        let old=checked(&tx,key)?;
        if old.revision!=expected_revision || old.status!=ExternalEffectStatus::NotExecuted
           || old.cancel_requested || old.attempt>=MAX_ATTEMPTS || old.attempt>=max_by_plan {
            return Err(conflict("retry requires fresh proven-nonexecution evidence"));
        }
        let evidence=old.reconciliation_id.as_deref()
            .ok_or_else(|| conflict("missing provider nonexecution proof"))?;
        tx.execute(
            "INSERT INTO agent_external_attempt_history(idempotency_key,attempt,evidence_id,settled_at)
             VALUES(?1,?2,?3,?4)",
            params![key.idempotency_key,old.attempt,evidence,now_rfc3339()],
        ).map_err(db_error)?;
        let changed=tx.execute(
            "UPDATE agent_external_effects SET status='reserved',attempt=attempt+1,
             actual_tokens=0,actual_cost_microusd=0,provider_request_id=NULL,
             receipt_id=NULL,reconciliation_id=NULL,revision=revision+1,updated_at=?1
             WHERE idempotency_key=?2 AND revision=?3 AND status='not_executed'
               AND attempt<3 AND cancel_requested=0",
            params![now_rfc3339(),key.idempotency_key,expected_revision],
        ).map_err(db_error)?;
        if changed!=1 { return Err(conflict("retry CAS lost")); }
        let after=checked(&tx,key)?;
        tx.commit().map_err(db_error)?;
        Ok(after)
    }

}


#[cfg(test)]
mod tests {
    use super::*;
    use scriptotar_agent::{AuditKind,CheckpointStore,Estimate,Limits,Plan,RunSnapshot,
        StepInput,StepPlan,StepRecord,Usage};
    use scriptotar_core::{Project,ProjectRepository};
    use std::sync::{Arc,Barrier};
    use std::thread;
    use tempfile::TempDir;

    fn setup() -> (TempDir,SqliteStore,Uuid,Uuid,RunSnapshot) {
        let dir=TempDir::new().unwrap();
        let db=SqliteStore::open(dir.path().join("external.sqlite3")).unwrap();
        let project=Project::new("External tests");
        db.create_project(&project).unwrap();
        let study=db.create_study(project.id,"Approved Study","").unwrap().id;
        let id=Uuid::new_v4().to_string();
        let plan=Plan {
            id:id.clone(),project_id:project.id.to_string(),study_id:study.to_string(),
            steps:vec![StepPlan {
                id:"draft".into(),input:StepInput::DraftScript {
                    brief_id:Uuid::new_v4().to_string()
                },
                estimate:Estimate {tokens:500,cost_microusd:2500,
                    max_output_bytes:1000,cpu_ms:1,peak_memory_mib:1},
                max_attempts:3,requires_approval:true,
            }],
            limits:Limits {max_tool_calls:1,max_tokens:500,max_cost_microusd:2500,
                max_elapsed_ms:60000,max_output_bytes:1000,max_cpu_ms:100,
                max_peak_memory_mib:4},
        };
        let mut snap=RunSnapshot {
            schema_version:1,plan,revision:0,state:RunState::Planned,
            current_step:0,
            steps:vec![StepRecord {id:"draft".into(),state:StepState::Pending,
                attempts:0,approved:false,idempotency_key:format!("{id}:0:draft"),
                pending_effect_id:None,output:None,failure_code:None}],
            usage:Usage::default(),started_ms:100,cancel_requested:false,
            pause_requested:false,events:vec![],
        };
        snap.event(100,AuditKind::Created);
        let checkpoints=db.scoped_agent_checkpoints(project.id,study).unwrap();
        checkpoints.create(snap.clone()).unwrap();
        snap.revision=1; snap.state=RunState::Running;
        snap.steps[0].state=StepState::Running;
        snap.steps[0].attempts=1;snap.steps[0].approved=true;
        snap.usage.tool_calls=1;snap.event(101,AuditKind::Dispatched);
        checkpoints.compare_and_swap(0,snap.clone()).unwrap();
        (dir,db,project.id,study,snap)
    }
    // Legacy upgrade fixtures deliberately reconstruct pre-v12 databases.
    // First remove the independently additive v13 barrier and its triggers;
    // do not alter how the production v12 identity migration is tested.
    fn remove_v13_from_legacy_fixture(conn: &rusqlite::Connection) {
        let mut statement = conn
            .prepare(
                "SELECT name FROM sqlite_master WHERE type='trigger'
                 AND name LIKE 'provider_dispatch_barrier_%'",
            )
            .unwrap();
        let names = statement
            .query_map([], |row| row.get::<_, String>(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        drop(statement);
        for name in names {
            assert!(name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_'));
            conn.execute_batch(&format!("DROP TRIGGER {name};")).unwrap();
        }
        conn.execute_batch(
            "DROP TABLE provider_dispatch_barrier;
             DELETE FROM schema_migrations WHERE version = 13;",
        )
        .unwrap();
    }

    fn identity(snap:&RunSnapshot,project:Uuid,study:Uuid)->ExternalEffectKey<'_> {
        ExternalEffectKey {project,study,run_id:&snap.plan.id,step_id:"draft",
            idempotency_key:&snap.steps[0].idempotency_key}
    }
    fn limits()->ExternalEffectLimits {
        ExternalEffectLimits {tokens:500,cost_microusd:2500}
    }
    fn fp()->&'static str {
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
    }
    fn reserve(db:&SqliteStore,snap:&RunSnapshot,project:Uuid,study:Uuid)
        -> ExternalEffect {
        match db.reserve_external_effect(identity(snap,project,study),fp(),"openai",limits()).unwrap() {
            ExternalReservation::Fresh(v)=>v,ExternalReservation::Existing(_)=>panic!("duplicate")
        }
    }

    #[test]
    fn v13_first_attempt_aborted_before_send_is_atomically_cancelled_at_guard_release() {
        let (_dir, db, project, study, snapshot) = setup();
        let key = identity(&snapshot, project, study);
        let reserved = reserve(&db, &snapshot, project, study);
        let guard = db
            .acquire_provider_dispatch_barrier_for_effect(
                project,
                study,
                &"a".repeat(64),
                crate::ProviderDispatchEffect {
                    run_id: key.run_id,
                    step_id: key.step_id,
                    idempotency_key: key.idempotency_key,
                    attempt: reserved.attempt,
                    provider: "openai",
                    wire_sha256: fp(),
                },
            )
            .unwrap();
        let external = rusqlite::Connection::open(db.path()).unwrap();
        assert!(external
            .execute(
                "UPDATE agent_external_effects SET cancel_requested=1
                 WHERE idempotency_key=?1",
                params![key.idempotency_key],
            )
            .is_err());
        // The host rejects the request before the durable DISPATCHED CAS.
        // A token-owned guard drop must settle both ledgers and release the
        // gate in one IMMEDIATE transaction, not leave an orphan reservation.
        drop(guard);
        let effect = db.get_external_effect(key).unwrap().unwrap();
        assert_eq!(effect.status, ExternalEffectStatus::Cancelled);
        assert!(effect.cancel_requested);
        assert_eq!(effect.revision, reserved.revision + 1);
        let legacy: String = external
            .query_row(
                "SELECT status FROM agent_effects WHERE idempotency_key=?1",
                params![key.idempotency_key],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(legacy, "not_applied");
        let still_frozen: i64 = external
            .query_row(
                "SELECT COUNT(*) FROM provider_dispatch_barrier",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(still_frozen, 0);
        assert!(db
            .mark_external_dispatched(key, effect.revision, effect.attempt)
            .is_err());
        assert!(matches!(
            db.reserve_external_effect(key, fp(), "openai", limits()).unwrap(),
            ExternalReservation::Existing(_)
        ));
        assert_eq!(
            external
                .execute(
                    "UPDATE content_studies SET goal='safe after no-send' WHERE id=?1",
                    params![study.to_string()],
                )
                .unwrap(),
            1
        );
    }

    #[test]
    fn v13_admission_and_v10_cas_reject_changed_v8_single_send_claim() {
        let (_dir, db, project, study, snapshot) = setup();
        let key = identity(&snapshot, project, study);
        let reserved = reserve(&db, &snapshot, project, study);
        let other = rusqlite::Connection::open(db.path()).unwrap();
        // A separate process can resolve the old effect BEFORE the v13 gate
        // commits. A still-RESERVED v10 row is not independent authorization
        // to send once the canonical v8 fence has become UNKNOWN.
        assert_eq!(
            other
                .execute(
                    "UPDATE agent_effects SET status='unknown',
                     revision=revision+1 WHERE idempotency_key=?1
                     AND status='reserved'",
                    params![key.idempotency_key],
                )
                .unwrap(),
            1
        );
        let result = db.acquire_provider_dispatch_barrier_for_effect(
            project,
            study,
            &"a".repeat(64),
            crate::ProviderDispatchEffect {
                run_id: key.run_id,
                step_id: key.step_id,
                idempotency_key: key.idempotency_key,
                attempt: reserved.attempt,
                provider: "openai",
                wire_sha256: fp(),
            },
        );
        assert!(result.is_err(), "v13 admitted a no-longer-reserved v8 claim");
        assert!(
            db.mark_external_dispatched(key, reserved.revision, reserved.attempt)
                .is_err(),
            "v10 CAS ignored changed v8 single-send state"
        );
        let effect = db.get_external_effect(key).unwrap().unwrap();
        assert_eq!(effect.status, ExternalEffectStatus::Reserved);
        assert_eq!(effect.revision, reserved.revision);
        let barrier_count: i64 = other
            .query_row(
                "SELECT COUNT(*) FROM provider_dispatch_barrier",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(barrier_count, 0);
    }

    #[test]
    fn v13_pre_send_cleanup_v8_write_failure_rolls_back_v10_and_preserves_barrier() {
        let (_dir, db, project, study, snapshot) = setup();
        let key = identity(&snapshot, project, study);
        let original = reserve(&db, &snapshot, project, study);
        let guard = db
            .acquire_provider_dispatch_barrier_for_effect(
                project,
                study,
                &"a".repeat(64),
                crate::ProviderDispatchEffect {
                    run_id: key.run_id,
                    step_id: key.step_id,
                    idempotency_key: key.idempotency_key,
                    attempt: original.attempt,
                    provider: "openai",
                    wire_sha256: fp(),
                },
            )
            .unwrap();
        let other = rusqlite::Connection::open(db.path()).unwrap();

        // Even status-only and DELETE bypasses against the v8 single-send
        // ledger must be rejected from a separate SQLite connection.
        let bypassed = other
            .execute(
                "UPDATE agent_effects
                 SET status='unknown',revision=revision+1
                 WHERE idempotency_key=?1 AND status='reserved'",
                params![key.idempotency_key],
            )
            .unwrap_err();
        assert!(
            bypassed
                .to_string()
                .contains("provider dispatch v8 fence is frozen"),
            "v8 mutation failed for unrelated reason: {bypassed}"
        );
        let deleted = other
            .execute(
                "DELETE FROM agent_effects WHERE idempotency_key=?1",
                params![key.idempotency_key],
            )
            .unwrap_err();
        assert!(
            deleted
                .to_string()
                .contains("provider dispatch v8 fence cannot be deleted"),
            "v8 deletion failed for unrelated reason: {deleted}"
        );

        // Inject a deterministic v8 write fault at the exact second half
        // of the owned two-ledger cleanup. A transaction must restore both
        // the v10 RESERVED row AND the already-deleted barrier on rollback.
        other
            .execute_batch(
                "CREATE TRIGGER test_block_v8_pre_send_close
                 BEFORE UPDATE OF status ON agent_effects
                 WHEN NEW.status='not_applied'
                 BEGIN SELECT RAISE(ABORT, 'injected v8 close fault'); END;",
            )
            .unwrap();

        drop(guard); // Release must fail closed: no half-cancelled claim.
        let after = db.get_external_effect(key).unwrap().unwrap();
        assert_eq!(after.status, ExternalEffectStatus::Reserved);
        assert!(!after.cancel_requested);
        assert_eq!(after.revision, original.revision);
        let v8_status: String = other
            .query_row(
                "SELECT status FROM agent_effects WHERE idempotency_key=?1",
                params![key.idempotency_key],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(v8_status, "reserved");
        let retained: i64 = other
            .query_row(
                "SELECT COUNT(*) FROM provider_dispatch_barrier
                 WHERE effect_idempotency_key=?1",
                params![key.idempotency_key],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(retained, 1, "failed release erased the original lock");
        assert!(other
            .execute(
                "UPDATE content_studies SET goal='not authorized' WHERE id=?1",
                params![study.to_string()],
            )
            .is_err());
        assert!(db
            .acquire_provider_dispatch_barrier_for_effect(
                project,
                study,
                &"a".repeat(64),
                crate::ProviderDispatchEffect {
                    run_id: key.run_id,
                    step_id: key.step_id,
                    idempotency_key: key.idempotency_key,
                    attempt: original.attempt,
                    provider: "openai",
                    wire_sha256: fp(),
                },
            )
            .is_err());
    }

    #[test]
    fn v13_cancellation_committed_before_barrier_admission_denies_paid_send() {
        let (_dir, db, project, study, snapshot) = setup();
        let key = identity(&snapshot, project, study);
        let reserved = reserve(&db, &snapshot, project, study);
        let other = rusqlite::Connection::open(db.path()).unwrap();
        assert_eq!(
            other
                .execute(
                    "UPDATE agent_external_effects
                     SET cancel_requested=1 WHERE idempotency_key=?1",
                    params![key.idempotency_key],
                )
                .unwrap(),
            1
        );
        let refused = db.acquire_provider_dispatch_barrier_for_effect(
            project,
            study,
            &"a".repeat(64),
            crate::ProviderDispatchEffect {
                run_id: key.run_id,
                step_id: key.step_id,
                idempotency_key: key.idempotency_key,
                attempt: reserved.attempt,
                provider: "openai",
                wire_sha256: fp(),
            },
        );
        assert!(refused.is_err());
        let freeze_count: i64 = other
            .query_row("SELECT COUNT(*) FROM provider_dispatch_barrier", [], |row| row.get(0))
            .unwrap();
        assert_eq!(freeze_count, 0, "failed consent cannot strand an unowned scope lock");
        assert!(db
            .mark_external_dispatched(key, reserved.revision, reserved.attempt)
            .is_err());
    }

    #[test]
    fn v13_dispatch_freeze_blocks_external_cancel_but_allows_v10_cas_and_receipt() {
        let (_dir, db, project, study, snapshot) = setup();
        let key = identity(&snapshot, project, study);
        let reserved = reserve(&db, &snapshot, project, study);
        let mut identity = crate::ProviderDispatchEffect {
            run_id: key.run_id,
            step_id: key.step_id,
            idempotency_key: key.idempotency_key,
            attempt: reserved.attempt,
            provider: "openai",
            wire_sha256: fp(),
        };
        // A forged or changed serialized payload cannot acquire this freeze
        // even if the Study and all unrelated run fields are correct.
        identity.wire_sha256 =
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
        assert!(db
            .acquire_provider_dispatch_barrier_for_effect(
                project,
                study,
                &"a".repeat(64),
                identity,
            )
            .is_err());
        identity.wire_sha256 = fp();
        let guard = db
            .acquire_provider_dispatch_barrier_for_effect(
                project,
                study,
                &"a".repeat(64),
                identity,
            )
            .unwrap();
        // Prove the recovery record is joined to this exact reserved attempt.
        let inspect = rusqlite::Connection::open(db.path()).unwrap();
        let recorded: (String, String, String, i64, String, String) = inspect
            .query_row(
                "SELECT effect_run_id,effect_step_id,effect_idempotency_key,
                        effect_attempt,effect_provider,effect_wire_sha256
                 FROM provider_dispatch_barrier WHERE singleton=1",
                [],
                |row| Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                )),
            )
            .unwrap();
        assert_eq!(recorded, (
            key.run_id.to_owned(),
            key.step_id.to_owned(),
            key.idempotency_key.to_owned(),
            reserved.attempt,
            "openai".to_owned(),
            fp().to_owned(),
        ));
        // Effect CAS must commit before the network send even with the
        // committed cross-connection canonical-scope barrier in place.
        let dispatched = db
            .mark_external_dispatched(key, reserved.revision, 1)
            .unwrap();
        assert_eq!(dispatched.status, ExternalEffectStatus::Dispatched);
        let external = rusqlite::Connection::open(db.path()).unwrap();
        let cancelled = external
            .execute(
                "UPDATE agent_external_effects
                 SET cancel_requested=1 WHERE idempotency_key=?1",
                params![key.idempotency_key],
            )
            .unwrap_err();
        assert!(
            cancelled
                .to_string()
                .contains("provider dispatch cancellation is frozen"),
            "external cancellation was rejected for unrelated reason: {cancelled}"
        );
        for terminal in ["cancelled", "not_executed"] {
            let forbidden = external
                .execute(
                    "UPDATE agent_external_effects SET status=?1
                     WHERE idempotency_key=?2",
                    params![terminal, key.idempotency_key],
                )
                .unwrap_err();
            assert!(
                forbidden
                    .to_string()
                    .contains("provider dispatch cancellation status is frozen"),
                "status-only bypass should be rejected by the gate: {forbidden}"
            );
        }
        // v10/v12 receipt reconciliation is independent of protected
        // canonical context writes and must remain operable.
        let settled = db
            .reconcile_external_effect(
                key,
                1,
                ExternalResolution::Completed {
                    provider_request_id: Some("request_under_gate_v13"),
                    receipt_id: "receipt_under_gate_v13",
                    tokens: 42,
                    cost_microusd: 100,
                },
            )
            .unwrap();
        assert_eq!(settled.status, ExternalEffectStatus::Completed);
        guard.verify().unwrap();
        drop(guard);
        // A committed, authenticated terminal receipt permits ordinary
        // scoped edits again; it did not authorize a second network send.
        assert_eq!(
            external
                .execute(
                    "UPDATE content_studies SET goal='after receipt' WHERE id=?1",
                    params![study.to_string()],
                )
                .unwrap(),
            1
        );
    }

    #[test]
    fn v13_ambiguous_sent_attempt_keeps_freeze_after_guard_drop_and_restart() {
        let (dir, db, project, study, snapshot) = setup();
        let key = identity(&snapshot, project, study);
        let reserved = reserve(&db, &snapshot, project, study);
        let guard = db
            .acquire_provider_dispatch_barrier_for_effect(
                project,
                study,
                &"a".repeat(64),
                crate::ProviderDispatchEffect {
                    run_id: key.run_id,
                    step_id: key.step_id,
                    idempotency_key: key.idempotency_key,
                    attempt: reserved.attempt,
                    provider: "openai",
                    wire_sha256: fp(),
                },
            )
            .unwrap();
        db.mark_external_dispatched(key, reserved.revision, 1)
            .unwrap();
        // Sending may have charged a provider. Dropping the normal RAII
        // guard is NOT permission to erase its ownership record.
        drop(guard);
        drop(db);
        let reopened = SqliteStore::open(dir.path().join("external.sqlite3")).unwrap();
        let independent = rusqlite::Connection::open(reopened.path()).unwrap();
        let persisted: (String, String, String, i64) = independent
            .query_row(
                "SELECT effect_run_id,effect_step_id,effect_idempotency_key,
                        effect_attempt FROM provider_dispatch_barrier WHERE singleton=1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .unwrap();
        assert_eq!(persisted, (
            key.run_id.to_owned(),
            key.step_id.to_owned(),
            key.idempotency_key.to_owned(),
            1,
        ));
        assert!(independent
            .execute(
                "UPDATE content_studies SET goal='unapproved restart edit' WHERE id=?1",
                params![study.to_string()],
            )
            .is_err());
        assert!(reopened
            .acquire_provider_dispatch_barrier_for_effect(
                project,
                study,
                &"a".repeat(64),
                crate::ProviderDispatchEffect {
                    run_id: key.run_id,
                    step_id: key.step_id,
                    idempotency_key: key.idempotency_key,
                    attempt: 1,
                    provider: "openai",
                    wire_sha256: fp(),
                },
            )
            .is_err());
        // Even if a late authenticated terminal receipt is persisted, a
        // crashed token is not silently unlocked on DB reopen. Agent 02
        // owns the future independently reviewed identity-bound release.
        reopened
            .reconcile_external_effect(
                key,
                1,
                ExternalResolution::Completed {
                    provider_request_id: Some("request_after_crash_v13"),
                    receipt_id: "receipt_after_crash_v13",
                    tokens: 42,
                    cost_microusd: 100,
                },
            )
            .unwrap();
        let still_locked: i64 = independent
            .query_row("SELECT COUNT(*) FROM provider_dispatch_barrier", [], |row| row.get(0))
            .unwrap();
        assert_eq!(still_locked, 1);
    }

    #[test]
    fn crash_intervals_restart_and_ambiguous_dispatch_never_replay() {
        let (dir,db,project,study,snap)=setup();
        let key=identity(&snap,project,study);
        assert!(db.get_external_effect(key).unwrap().is_none()); // crash before reservation
        let fresh=reserve(&db,&snap,project,study);
        drop(db); // crash after reservation (before network)
        let db=SqliteStore::open(dir.path().join("external.sqlite3")).unwrap();
        assert_eq!(db.get_external_effect(key).unwrap().unwrap().status,ExternalEffectStatus::Reserved);
        assert!(matches!(db.reserve_external_effect(key,fp(),"openai",limits()).unwrap(),
            ExternalReservation::Existing(_)));
        let dispatched=db.mark_external_dispatched(key,fresh.revision,1).unwrap();
        assert_eq!(dispatched.status,ExternalEffectStatus::Dispatched);
        drop(db); // crash after dispatch, before response
        let db=SqliteStore::open(dir.path().join("external.sqlite3")).unwrap();
        assert_eq!(db.recover_external_effects(project,study,&snap.plan.id).unwrap(),1);
        let uncertain=db.get_external_effect(key).unwrap().unwrap();
        assert_eq!(uncertain.status,ExternalEffectStatus::Uncertain);
        assert!(db.mark_external_dispatched(key,uncertain.revision,1).is_err());
        assert!(matches!(db.reserve_external_effect(key,fp(),"openai",limits()).unwrap(),
            ExternalReservation::Existing(_)));
        let success=db.reconcile_external_effect(key,1,ExternalResolution::Completed {
            provider_request_id:Some("req_123"),receipt_id:"receipt_123",
            tokens:123,cost_microusd:1400,
        }).unwrap();
        assert_eq!(success.actual_cost_microusd,1400);
        drop(db); // crash after receipt, before response reaches UI
        let db=SqliteStore::open(dir.path().join("external.sqlite3")).unwrap();
        assert_eq!(db.get_external_effect(key).unwrap().unwrap().status,ExternalEffectStatus::Completed);
        assert_eq!(db.recover_external_effects(project,study,&snap.plan.id).unwrap(),0);
        assert!(db.mark_external_dispatched(key,success.revision,1).is_err());
    }

    #[test]
    fn concurrent_claimers_only_one_fresh_and_fingerprint_cannot_change() {
        let (_dir,db,project,study,snap)=setup();
        let barrier=Arc::new(Barrier::new(3));
        let workers=(0..2).map(|_|{
            let db=db.clone();let snap=snap.clone();let barrier=barrier.clone();
            thread::spawn(move || {
                barrier.wait();
                db.reserve_external_effect(identity(&snap,project,study),
                    fp(),"openai",limits()).unwrap()
            })
        }).collect::<Vec<_>>();
        barrier.wait();
        let result=workers.into_iter().map(|w|w.join().unwrap()).collect::<Vec<_>>();
        assert_eq!(result.iter().filter(|v|matches!(v,ExternalReservation::Fresh(_))).count(),1);
        assert_eq!(result.iter().filter(|v|matches!(v,ExternalReservation::Existing(_))).count(),1);
        let other="ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";
        assert!(db.reserve_external_effect(identity(&snap,project,study),other,"openai",limits()).is_err());
    }

    #[test]
    fn duplicate_receipts_late_response_and_lost_ack() {
        let (_dir,db,project,study,snap)=setup();
        let key=identity(&snap,project,study);
        let reservation=reserve(&db,&snap,project,study);
        db.mark_external_dispatched(key,reservation.revision,1).unwrap();
        db.reconcile_external_effect(key,1,ExternalResolution::Uncertain).unwrap();
        let receive=||ExternalResolution::Completed {
            provider_request_id:Some("req_late"),receipt_id:"receipt_late",
            tokens:100,cost_microusd:900,
        };
        let first=db.reconcile_external_effect(key,1,receive()).unwrap();
        assert_eq!(db.reconcile_external_effect(key,1,receive()).unwrap(),first);
        assert!(db.reconcile_external_effect(key,1,ExternalResolution::Completed {
            provider_request_id:Some("req_late"),receipt_id:"other",
            tokens:100,cost_microusd:900,
        }).is_err());
        assert!(db.reconcile_external_effect(key,1,ExternalResolution::Completed {
            provider_request_id:Some("req_late"),receipt_id:"receipt_late",
            tokens:501,cost_microusd:900,
        }).is_err());
    }

    #[test]
    fn cancellation_pre_dispatch_vs_in_flight_and_manual_reconciliation() {
        let (_dir,db,project,study,snap)=setup();
        let key=identity(&snap,project,study);
        reserve(&db,&snap,project,study);
        let cancelled=db.request_external_cancel(key).unwrap();
        assert_eq!(cancelled.status,ExternalEffectStatus::Cancelled);
        assert_eq!(db.get_agent_effect(project,study,&snap.plan.id,key.idempotency_key)
            .unwrap().unwrap().status,super::super::EffectStatus::NotApplied);
        assert!(db.mark_external_dispatched(key,cancelled.revision,1).is_err());
        // A separate run models cancellation after a dispatch marker.
        let (_dir2,db2,p2,s2,run2)=setup();
        let key2=identity(&run2,p2,s2);
        let fresh=reserve(&db2,&run2,p2,s2);
        db2.mark_external_dispatched(key2,fresh.revision,1).unwrap();
        let pending=db2.request_external_cancel(key2).unwrap();
        assert_eq!(pending.status,ExternalEffectStatus::Dispatched);
        assert!(pending.cancel_requested);
        assert_eq!(db2.get_agent_effect(p2,s2,&run2.plan.id,key2.idempotency_key)
            .unwrap().unwrap().status,super::super::EffectStatus::Unknown);
        assert!(db2.mark_external_dispatched(key2,pending.revision,1).is_err());
        let settled=db2.reconcile_external_effect(key2,1,
            ExternalResolution::ProviderCancelled {evidence_id:"provider_cancel_1"}).unwrap();
        assert_eq!(settled.status,ExternalEffectStatus::Cancelled);
    }

    #[test]
    fn provider_proven_nonexecution_only_bounded_manual_retry() {
        let (_dir,db,project,study,snap)=setup();
        let key=identity(&snap,project,study);
        let mut current=reserve(&db,&snap,project,study);
        for attempt in 1..=3 {
            current=db.mark_external_dispatched(key,current.revision,attempt).unwrap();
            assert_eq!(current.attempt,attempt);
            current=db.reconcile_external_effect(key,attempt,
                ExternalResolution::VerifiedNotExecuted {evidence_id:"provider_never_ran"}).unwrap();
            assert_eq!(current.status,ExternalEffectStatus::NotExecuted);
            if attempt < 3 {
                current=db.prepare_external_retry(key,current.revision).unwrap();
                assert_eq!(current.attempt,attempt+1);
            }
        }
        assert!(db.prepare_external_retry(key,current.revision).is_err());
        assert!(db.mark_external_dispatched(key,current.revision,3).is_err());
        assert!(db.reconcile_external_effect(key,1,ExternalResolution::Completed {
            provider_request_id:Some("late_old"),receipt_id:"old",
            tokens:1,cost_microusd:1
        }).is_err());
    }

    #[test]
    fn different_project_overbudget_and_corrupt_checkpoint_fail_closed() {
        let (_dir,db,project,study,snap)=setup();
        let key=identity(&snap,project,study);
        let foreign=Project::new("Foreign");
        db.create_project(&foreign).unwrap();
        let other=db.create_study(foreign.id,"Foreign Study","").unwrap();
        let forged=ExternalEffectKey {project:foreign.id,study:other.id,..key};
        assert!(db.reserve_external_effect(forged,fp(),"openai",limits()).is_err());
        assert!(db.reserve_external_effect(key,fp(),"openai",
            ExternalEffectLimits {tokens:501,cost_microusd:2500}).is_err());
        let conn=rusqlite::Connection::open(db.path()).unwrap();
        conn.execute("UPDATE agent_runs SET snapshot_json='{}' WHERE id=?1",
                     params![snap.plan.id]).unwrap();
        assert!(db.reserve_external_effect(key,fp(),"openai",limits()).is_err());
        assert!(db.recover_external_effects(project,study,&snap.plan.id).is_err());
    }
    #[test]
    fn cancelling_prepared_retry_preserves_prior_dispatched_effect_fence() {
        let (_dir,db,project,study,snap)=setup();
        let key=identity(&snap,project,study);
        let first=reserve(&db,&snap,project,study);
        db.mark_external_dispatched(key,first.revision,1).unwrap();
        let proven=db.reconcile_external_effect(key,1,ExternalResolution::VerifiedNotExecuted {
            evidence_id:"provider_confirmed_no_charge",
        }).unwrap();
        let retry=db.prepare_external_retry(key,proven.revision).unwrap();
        assert_eq!(retry.status,ExternalEffectStatus::Reserved);
        assert_eq!(retry.attempt,2);
        let cancelled=db.request_external_cancel(key).unwrap();
        assert_eq!(cancelled.status,ExternalEffectStatus::Cancelled);
        assert!(cancelled.cancel_requested);
        assert_eq!(db.get_agent_effect(project,study,&snap.plan.id,key.idempotency_key)
            .unwrap().unwrap().status,super::super::EffectStatus::Unknown);
        assert!(db.mark_external_dispatched(key,cancelled.revision,2).is_err());
        assert!(db.prepare_external_retry(key,cancelled.revision).is_err());
    }

    #[test]
    fn provider_ack_is_durable_unique_and_reconciles_after_restart() {
        let (dir,db,project,study,snap)=setup();
        let key=identity(&snap,project,study);
        let first=reserve(&db,&snap,project,study);
        db.mark_external_dispatched(key,first.revision,1).unwrap();
        let ack=db.record_external_ack(key,1,"provider_req_71").unwrap();
        assert_eq!(ack.status,ExternalEffectStatus::Dispatched);
        assert_eq!(ack.provider_request_id.as_deref(),Some("provider_req_71"));
        assert_eq!(db.record_external_ack(key,1,"provider_req_71").unwrap(),ack);
        assert!(db.record_external_ack(key,1,"different_request").is_err());
        drop(db);
        let reopened=SqliteStore::open(dir.path().join("external.sqlite3")).unwrap();
        assert_eq!(reopened.recover_external_effects(project,study,&snap.plan.id).unwrap(),1);
        let recovered=reopened.get_external_effect(key).unwrap().unwrap();
        assert_eq!(recovered.status,ExternalEffectStatus::Uncertain);
        assert_eq!(recovered.provider_request_id.as_deref(),Some("provider_req_71"));
        assert!(reopened.mark_external_dispatched(key,recovered.revision,1).is_err());
        assert!(reopened.reconcile_external_effect(key,1,ExternalResolution::Completed {
            provider_request_id:Some("wrong_req"),receipt_id:"receipt_77",
            tokens:25,cost_microusd:400,
        }).is_err());
        let settled=reopened.reconcile_external_effect(key,1,ExternalResolution::Completed {
            provider_request_id:None,receipt_id:"receipt_77",
            tokens:25,cost_microusd:400,
        }).unwrap();
        assert_eq!(settled.provider_request_id.as_deref(),Some("provider_req_71"));
        assert_eq!(reopened.record_external_ack(key,1,"provider_req_71").unwrap(),settled);
        assert!(reopened.record_external_ack(key,1,"conflicting_late_id").is_err());
        assert_eq!(reopened.reconcile_external_effect(key,1,ExternalResolution::Completed {
            provider_request_id:None,receipt_id:"receipt_77",
            tokens:25,cost_microusd:400,
        }).unwrap(),settled);
        let legacy=reopened.get_agent_effect(project,study,&snap.plan.id,key.idempotency_key)
            .unwrap().unwrap();
        assert_eq!(legacy.external_id.as_deref(),Some("provider_req_71"));
        assert_eq!(legacy.status,super::super::EffectStatus::Applied);
    }

    #[test]
    fn revoked_running_intent_blocks_external_dispatch() {
        let (_dir,db,project,study,snap)=setup();
        let key=identity(&snap,project,study);
        let reserved=reserve(&db,&snap,project,study);
        let conn=rusqlite::Connection::open(db.path()).unwrap();
        conn.execute(
            "UPDATE agent_runs SET snapshot_json=json_set(snapshot_json,
             '$.cancel_requested',json('true')) WHERE id=?1",
            params![snap.plan.id],
        ).unwrap();
        assert!(db.mark_external_dispatched(key,reserved.revision,1).is_err());
        assert_eq!(db.get_external_effect(key).unwrap().unwrap().status,
                   ExternalEffectStatus::Reserved);
    }

    #[test]
    fn two_processes_cannot_both_claim_dispatch_marker() {
        let (_dir,db,project,study,snap)=setup();
        let reserved=reserve(&db,&snap,project,study);
        let barrier=Arc::new(Barrier::new(3));
        let workers=(0..2).map(|_|{
            let db=db.clone();
            let snap=snap.clone();
            let barrier=barrier.clone();
            thread::spawn(move || {
                barrier.wait();
                db.mark_external_dispatched(identity(&snap,project,study),reserved.revision,1)
            })
        }).collect::<Vec<_>>();
        barrier.wait();
        let result=workers.into_iter().map(|w|w.join().unwrap()).collect::<Vec<_>>();
        assert_eq!(result.iter().filter(|r|r.is_ok()).count(),1);
        assert_eq!(result.iter().filter(|r|r.is_err()).count(),1);
        assert_eq!(db.get_external_effect(identity(&snap,project,study))
            .unwrap().unwrap().status,ExternalEffectStatus::Dispatched);
    }

    #[test]
    fn injected_receipt_write_failure_rolls_back_dispatch_transition() {
        let (_dir,db,project,study,snap)=setup();
        let key=identity(&snap,project,study);
        let reserved=reserve(&db,&snap,project,study);
        let conn=rusqlite::Connection::open(db.path()).unwrap();
        conn.execute_batch(
            "CREATE TRIGGER external_test_rollback BEFORE UPDATE ON agent_external_effects
             BEGIN SELECT RAISE(ABORT,'injected database write fault'); END;"
        ).unwrap();
        assert!(db.mark_external_dispatched(key,reserved.revision,1).is_err());
        assert_eq!(db.get_external_effect(key).unwrap().unwrap().status,
                   ExternalEffectStatus::Reserved);
        conn.execute_batch("DROP TRIGGER external_test_rollback;").unwrap();
        assert_eq!(db.mark_external_dispatched(key,reserved.revision,1).unwrap().status,
                   ExternalEffectStatus::Dispatched);
    }

    #[test]
    fn schema_v10_upgrade_from_v9_is_versioned_and_repeatable() {
        let (dir, db, project, study, snap) = setup();
        let path = dir.path().join("external.sqlite3");
        {
            let conn = rusqlite::Connection::open(&path).unwrap();
            remove_v13_from_legacy_fixture(&conn);
            conn.execute_batch(
                "DROP TRIGGER external_provider_identity_on_insert;
                 DROP TRIGGER external_provider_identity_on_update;
                 DROP INDEX idx_external_provider_request_unique;
                 DROP INDEX idx_external_provider_receipt_unique;
                 DROP TABLE agent_external_provider_identities;
                 DELETE FROM schema_migrations WHERE version = 12;
                 DROP TRIGGER study_writing_batches_scope_insert;
                 DROP TRIGGER study_writing_batches_immutable_delete;
                 DROP TRIGGER study_writing_batches_immutable_update;
                 DROP TRIGGER study_writing_revisions_immutable_delete;
                 DROP TRIGGER study_writing_revisions_immutable_update;
                 DROP TABLE study_writing_revisions;
                 DROP TABLE study_writing_batches;
                 DELETE FROM schema_migrations WHERE version = 11;
                 DROP TABLE agent_external_attempt_history;
                 DROP TABLE agent_external_effects;
                 DELETE FROM schema_migrations WHERE version = 10;
                 PRAGMA user_version = 9;",
            ).unwrap();
        }
        drop(db);
        let upgraded = SqliteStore::open(&path).unwrap();
        assert_eq!(upgraded.schema_version().unwrap(), crate::LATEST_SCHEMA_VERSION);
        let receipt = reserve(&upgraded, &snap, project, study);
        assert_eq!(receipt.status, ExternalEffectStatus::Reserved);
        drop(upgraded);
        let reopened = SqliteStore::open(&path).unwrap();
        assert_eq!(reopened.schema_version().unwrap(), crate::LATEST_SCHEMA_VERSION);
        assert!(matches!(
            reopened.reserve_external_effect(identity(&snap, project, study), fp(), "openai", limits()).unwrap(),
            ExternalReservation::Existing(_)
        ));
        let conn = rusqlite::Connection::open(path).unwrap();
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM schema_migrations WHERE version = 10
             AND name = 'agent_external_provider_effects'",
            [],
            |r| r.get(0),
        ).unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn malformed_preexisting_v10_table_rolls_back_without_migration_ledger_change() {
        let (dir, db, project, study, _snap) = setup();
        let path = dir.path().join("external.sqlite3");
        {
            let conn = rusqlite::Connection::open(&path).unwrap();
            remove_v13_from_legacy_fixture(&conn);
            conn.execute_batch(
                "DROP TRIGGER external_provider_identity_on_insert;
                 DROP TRIGGER external_provider_identity_on_update;
                 DROP INDEX idx_external_provider_request_unique;
                 DROP INDEX idx_external_provider_receipt_unique;
                 DROP TABLE agent_external_provider_identities;
                 DELETE FROM schema_migrations WHERE version = 12;
                 DROP TRIGGER study_writing_batches_scope_insert;
                 DROP TRIGGER study_writing_batches_immutable_delete;
                 DROP TRIGGER study_writing_batches_immutable_update;
                 DROP TRIGGER study_writing_revisions_immutable_delete;
                 DROP TRIGGER study_writing_revisions_immutable_update;
                 DROP TABLE study_writing_revisions;
                 DROP TABLE study_writing_batches;
                 DELETE FROM schema_migrations WHERE version = 11;
                 DROP TABLE agent_external_attempt_history;
                 DROP TABLE agent_external_effects;
                 DELETE FROM schema_migrations WHERE version = 10;
                 PRAGMA user_version = 9;
                 CREATE TABLE agent_external_effects (sentinel INTEGER);",
            ).unwrap();
        }
        drop(db);
        assert!(SqliteStore::open(&path).is_err());
        let conn = rusqlite::Connection::open(&path).unwrap();
        let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0)).unwrap();
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM schema_migrations", [], |r| r.get(0),
        ).unwrap();
        let history_exists: i64 = conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master
             WHERE name = 'agent_external_attempt_history'",
            [],
            |r| r.get(0),
        ).unwrap();
        let project_exists: i64 = conn.query_row(
            "SELECT COUNT(*) FROM projects WHERE id = ?1",
            params![project.to_string()],
            |r| r.get(0),
        ).unwrap();
        let study_exists: i64 = conn.query_row(
            "SELECT COUNT(*) FROM content_studies WHERE id = ?1",
            params![study.to_string()],
            |r| r.get(0),
        ).unwrap();
        assert_eq!((version, count, history_exists, project_exists, study_exists), (9, 9, 0, 1, 1));
    }

    #[test]
    fn independently_opened_connections_compete_for_single_external_dispatch() {
        let (dir, db, project, study, snap) = setup();
        let reserved = reserve(&db, &snap, project, study);
        let path = dir.path().join("external.sqlite3");
        let barrier = Arc::new(Barrier::new(3));
        let workers = (0..2).map(|_| {
            let worker = SqliteStore::open(&path).unwrap();
            let snapshot = snap.clone();
            let barrier = barrier.clone();
            thread::spawn(move || {
                barrier.wait();
                worker.mark_external_dispatched(
                    identity(&snapshot, project, study),
                    reserved.revision, reserved.attempt,
                )
            })
        }).collect::<Vec<_>>();
        barrier.wait();
        let outcomes = workers.into_iter().map(|worker| worker.join().unwrap()).collect::<Vec<_>>();
        assert_eq!(outcomes.iter().filter(|outcome| outcome.is_ok()).count(), 1);
        assert_eq!(outcomes.iter().filter(|outcome| outcome.is_err()).count(), 1);
        let got = db.get_external_effect(identity(&snap, project, study)).unwrap().unwrap();
        assert_eq!(got.status, ExternalEffectStatus::Dispatched);
    }


    fn second_approved_run(
        db: &SqliteStore,
        original: &RunSnapshot,
        project: Uuid,
        study: Uuid,
    ) -> RunSnapshot {
        let mut second = original.clone();
        second.plan.id = Uuid::new_v4().to_string();
        second.plan.project_id = project.to_string();
        second.plan.study_id = study.to_string();
        second.steps[0].idempotency_key = format!("{}:0:draft", second.plan.id);
        second.revision = 0;
        second.state = RunState::Planned;
        second.steps[0].state = StepState::Pending;
        second.steps[0].attempts = 0;
        second.steps[0].approved = false;
        second.usage = Usage::default();
        second.events.clear();
        second.event(100, AuditKind::Created);
        let checkpoints = db.scoped_agent_checkpoints(project, study).unwrap();
        checkpoints.create(second.clone()).unwrap();
        second.revision = 1;
        second.state = RunState::Running;
        second.steps[0].state = StepState::Running;
        second.steps[0].attempts = 1;
        second.steps[0].approved = true;
        second.usage.tool_calls = 1;
        second.event(101, AuditKind::Dispatched);
        checkpoints.compare_and_swap(0, second.clone()).unwrap();
        second
    }

    #[test]
    fn provider_ack_tombstone_survives_retry_and_rejects_another_run() {
        let (_dir, db, project, study, first) = setup();
        let peer = second_approved_run(&db, &first, project, study);
        let a = identity(&first, project, study);
        let b = identity(&peer, project, study);
        let ar = reserve(&db, &first, project, study);
        let br = reserve(&db, &peer, project, study);
        db.mark_external_dispatched(a, ar.revision, 1).unwrap();
        db.mark_external_dispatched(b, br.revision, 1).unwrap();

        let ack = db.record_external_ack(a, 1, "ack_forever_12").unwrap();
        assert_eq!(db.record_external_ack(a, 1, "ack_forever_12").unwrap(), ack);
        assert!(db.record_external_ack(b, 1, "ack_forever_12").is_err());
        assert_eq!(db.get_external_effect(b).unwrap().unwrap().status, ExternalEffectStatus::Dispatched);
        let proven = db.reconcile_external_effect(
            a,
            1,
            ExternalResolution::VerifiedNotExecuted { evidence_id: "nonexecution_proof_12" },
        ).unwrap();
        let retried = db.prepare_external_retry(a, proven.revision).unwrap();
        assert_eq!(retried.attempt, 2);
        assert!(retried.provider_request_id.is_none());
        // Clearing a per-attempt column must never release an authentic provider identity.
        assert!(db.record_external_ack(b, 1, "ack_forever_12").is_err());
        assert!(db.get_external_effect(b).unwrap().unwrap().provider_request_id.is_none());
    }

    #[test]
    fn provider_receipt_collision_rolls_back_settlement_across_connections() {
        let (dir, db, project, study, first) = setup();
        let peer = second_approved_run(&db, &first, project, study);
        let a = identity(&first, project, study);
        let b = identity(&peer, project, study);
        let ar = reserve(&db, &first, project, study);
        let br = reserve(&db, &peer, project, study);
        db.mark_external_dispatched(a, ar.revision, 1).unwrap();
        db.mark_external_dispatched(b, br.revision, 1).unwrap();
        db.reconcile_external_effect(
            a, 1, ExternalResolution::Completed {
                provider_request_id: Some("request_unique_A"),
                receipt_id: "same_provider_receipt_12",
                tokens: 100, cost_microusd: 250,
            },
        ).unwrap();
        drop(db);
        let reopened = SqliteStore::open(dir.path().join("external.sqlite3")).unwrap();
        assert!(reopened.reconcile_external_effect(
            b, 1, ExternalResolution::Completed {
                provider_request_id: Some("request_unique_B"),
                receipt_id: "same_provider_receipt_12",
                tokens: 100, cost_microusd: 250,
            },
        ).is_err());
        let unchanged = reopened.get_external_effect(b).unwrap().unwrap();
        assert_eq!(unchanged.status, ExternalEffectStatus::Dispatched);
        assert_eq!(unchanged.actual_cost_microusd, 0);
        assert!(unchanged.provider_request_id.is_none());
    }

    #[test]
    fn distinct_trusted_provider_can_use_same_response_identifier() {
        let (_dir, db, project, study, first) = setup();
        let peer = second_approved_run(&db, &first, project, study);
        let a = identity(&first, project, study);
        let b = identity(&peer, project, study);
        let ar = reserve(&db, &first, project, study);
        let br = match db.reserve_external_effect(b, fp(), "anthropic", limits()).unwrap() {
            ExternalReservation::Fresh(value) => value,
            ExternalReservation::Existing(_) => panic!("expected new provider reservation"),
        };
        db.mark_external_dispatched(a, ar.revision, 1).unwrap();
        db.mark_external_dispatched(b, br.revision, 1).unwrap();
        for key in [a, b] {
            db.record_external_ack(key, 1, "same_scoped_request").unwrap();
            db.reconcile_external_effect(
                key, 1, ExternalResolution::Completed {
                    provider_request_id: Some("same_scoped_request"),
                    receipt_id: "same_scoped_receipt",
                    tokens: 100, cost_microusd: 250,
                },
            ).unwrap();
        }
        assert_eq!(db.get_external_effect(a).unwrap().unwrap().status, ExternalEffectStatus::Completed);
        assert_eq!(db.get_external_effect(b).unwrap().unwrap().status, ExternalEffectStatus::Completed);
    }

    #[test]
    fn racing_sqlite_connections_have_exactly_one_receipt_owner() {
        let (dir, db, project, study, first) = setup();
        let peer = second_approved_run(&db, &first, project, study);
        for snap in [&first, &peer] {
            let key = identity(snap, project, study);
            let r = reserve(&db, snap, project, study);
            db.mark_external_dispatched(key, r.revision, 1).unwrap();
        }
        drop(db);
        let gate = Arc::new(Barrier::new(3));
        let jobs = [first, peer].into_iter().map(|snap| {
            let path = dir.path().join("external.sqlite3");
            let gate = gate.clone();
            thread::spawn(move || {
                let worker = SqliteStore::open(path).unwrap();
                gate.wait();
                worker.reconcile_external_effect(
                    identity(&snap, project, study), 1, ExternalResolution::Completed {
                        provider_request_id: Some("race_ack_12"),
                        receipt_id: "race_receipt_12",
                        tokens: 100, cost_microusd: 250,
                    },
                )
            })
        }).collect::<Vec<_>>();
        gate.wait();
        let results = jobs.into_iter().map(|job| job.join().unwrap()).collect::<Vec<_>>();
        assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
        assert_eq!(results.iter().filter(|r| r.is_err()).count(), 1);
    }

    #[test]
    fn historical_duplicate_receipts_fail_v12_upgrade_without_data_loss() {
        let (dir, db, project, study, first) = setup();
        let peer = second_approved_run(&db, &first, project, study);
        reserve(&db, &first, project, study);
        reserve(&db, &peer, project, study);
        drop(db);
        let path = dir.path().join("external.sqlite3");
        // Construct a valid legacy v11 file with duplicate final receipt IDs.
        // Only the v12 infrastructure is removed; historical v10/v11 migrations stay intact.
        let connection = rusqlite::Connection::open(&path).unwrap();
        remove_v13_from_legacy_fixture(&connection);
        connection.execute_batch(
            "DROP TRIGGER external_provider_identity_on_insert;
             DROP TRIGGER external_provider_identity_on_update;
             DROP INDEX idx_external_provider_request_unique;
             DROP INDEX idx_external_provider_receipt_unique;
             DROP TABLE agent_external_provider_identities;
             DELETE FROM schema_migrations WHERE version = 12;
             PRAGMA user_version = 11;",
        ).unwrap();
        for key in [
            first.steps[0].idempotency_key.as_str(),
            peer.steps[0].idempotency_key.as_str(),
        ] {
            connection.execute(
                "UPDATE agent_external_effects SET receipt_id='historical_duplicate_12'
                 WHERE idempotency_key=?1",
                params![key],
            ).unwrap();
        }
        drop(connection);
        assert!(SqliteStore::open(&path).is_err(), "ambiguous historical identity must block upgrade");
        let connection = rusqlite::Connection::open(&path).unwrap();
        let version: i64 = connection.pragma_query_value(None, "user_version", |row| row.get(0)).unwrap();
        let preserved: i64 = connection.query_row(
            "SELECT COUNT(*) FROM agent_external_effects WHERE receipt_id='historical_duplicate_12'",
            [], |row| row.get(0),
        ).unwrap();
        assert_eq!(version, 11);
        assert_eq!(preserved, 2);
        let no_partial_table: i64 = connection.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='agent_external_provider_identities'",
            [], |row| row.get(0),
        ).unwrap();
        assert_eq!(no_partial_table, 0);
    }


    #[test]
    fn provider_receipt_unique_across_projects_even_after_restart() {
        let (dir, db, project, study, first) = setup();
        let foreign = Project::new("Other billing project");
        db.create_project(&foreign).unwrap();
        let other_study = db.create_study(foreign.id, "Other Study", "").unwrap().id;
        let peer = second_approved_run(&db, &first, foreign.id, other_study);
        let a = identity(&first, project, study);
        let b = identity(&peer, foreign.id, other_study);
        let ar = reserve(&db, &first, project, study);
        let br = reserve(&db, &peer, foreign.id, other_study);
        db.mark_external_dispatched(a, ar.revision, 1).unwrap();
        db.mark_external_dispatched(b, br.revision, 1).unwrap();
        db.reconcile_external_effect(a, 1, ExternalResolution::Completed {
            provider_request_id: Some("cross_project_response_12"),
            receipt_id: "cross_project_response_12",
            tokens: 100, cost_microusd: 250,
        }).unwrap();
        drop(db);
        let reopened = SqliteStore::open(dir.path().join("external.sqlite3")).unwrap();
        assert!(reopened.reconcile_external_effect(b, 1, ExternalResolution::Completed {
            provider_request_id: Some("cross_project_response_12"),
            receipt_id: "cross_project_response_12",
            tokens: 100, cost_microusd: 250,
        }).is_err());
        assert_eq!(
            reopened.get_external_effect(b).unwrap().unwrap().status,
            ExternalEffectStatus::Dispatched
        );
    }

    #[test]
    fn raw_sqlite_ack_assignment_cannot_bypass_provider_identity_owner() {
        let (dir, db, project, study, first) = setup();
        let peer = second_approved_run(&db, &first, project, study);
        reserve(&db, &first, project, study);
        reserve(&db, &peer, project, study);
        let connection = rusqlite::Connection::open(dir.path().join("external.sqlite3")).unwrap();
        connection.execute(
            "UPDATE agent_external_effects SET provider_request_id='raw_identity_12'
             WHERE idempotency_key=?1",
            params![first.steps[0].idempotency_key],
        ).unwrap();
        let conflict = connection.execute(
            "UPDATE agent_external_effects SET provider_request_id='raw_identity_12'
             WHERE idempotency_key=?1",
            params![peer.steps[0].idempotency_key],
        );
        assert!(conflict.is_err(), "raw SQL must be subject to the same durable identity fence");
        let second = db.get_external_effect(identity(&peer, project, study)).unwrap().unwrap();
        assert!(second.provider_request_id.is_none());
    }


    #[test]
    fn openai_response_identity_cannot_cross_ack_and_receipt_roles() {
        let (_dir, db, project, study, first) = setup();
        let peer = second_approved_run(&db, &first, project, study);
        let a = identity(&first, project, study);
        let b = identity(&peer, project, study);
        let ar = reserve(&db, &first, project, study);
        let br = reserve(&db, &peer, project, study);
        db.mark_external_dispatched(a, ar.revision, 1).unwrap();
        db.mark_external_dispatched(b, br.revision, 1).unwrap();
        db.record_external_ack(a, 1, "resp_shared_between_roles_12").unwrap();
        // The first effect owns the real response ID already. The second effect
        // must not claim that response under the other column's role.
        assert!(db.reconcile_external_effect(b, 1, ExternalResolution::Completed {
            provider_request_id: Some("resp_unrelated_request_12"),
            receipt_id: "resp_shared_between_roles_12",
            tokens: 100, cost_microusd: 250,
        }).is_err());
        let second = db.get_external_effect(b).unwrap().unwrap();
        assert_eq!(second.status, ExternalEffectStatus::Dispatched);
        assert!(second.provider_request_id.is_none());
        assert!(second.receipt_id.is_none());
    }

    #[test]
    fn valid_legacy_same_effect_ack_and_receipt_id_backfills_once() {
        let (dir, db, project, study, first) = setup();
        let key = identity(&first, project, study);
        let r = reserve(&db, &first, project, study);
        db.mark_external_dispatched(key, r.revision, 1).unwrap();
        let original = db.reconcile_external_effect(key, 1, ExternalResolution::Completed {
            provider_request_id: Some("resp_same_role_identity_12"),
            receipt_id: "resp_same_role_identity_12",
            tokens: 100, cost_microusd: 250,
        }).unwrap();
        let path = dir.path().join("external.sqlite3");
        drop(db);
        let conn = rusqlite::Connection::open(&path).unwrap();
        remove_v13_from_legacy_fixture(&conn);
        conn.execute_batch(
            "DROP TRIGGER external_provider_identity_on_insert;
             DROP TRIGGER external_provider_identity_on_update;
             DROP INDEX idx_external_provider_request_unique;
             DROP INDEX idx_external_provider_receipt_unique;
             DROP TABLE agent_external_provider_identities;
             DELETE FROM schema_migrations WHERE version=12;
             PRAGMA user_version=11;",
        ).unwrap();
        drop(conn);
        let upgraded = SqliteStore::open(&path).unwrap();
        assert_eq!(upgraded.schema_version().unwrap(), crate::LATEST_SCHEMA_VERSION);
        assert_eq!(upgraded.get_external_effect(key).unwrap().unwrap(), original);
        let conn = rusqlite::Connection::open(&path).unwrap();
        let claims: i64 = conn.query_row(
            "SELECT COUNT(*) FROM agent_external_provider_identities
             WHERE provider='openai' AND identity='resp_same_role_identity_12'",
            [], |row| row.get(0),
        ).unwrap();
        assert_eq!(claims, 1);
        drop(conn);
        // Reopening the same effect's authenticated final receipt is idempotent.
        assert_eq!(upgraded.reconcile_external_effect(key, 1, ExternalResolution::Completed {
            provider_request_id: Some("resp_same_role_identity_12"),
            receipt_id: "resp_same_role_identity_12",
            tokens: 100, cost_microusd: 250,
        }).unwrap(), original);
    }


    #[test]
    fn v12_identity_claims_are_immutable_under_raw_sqlite() {
        let (dir, db, project, study, snap) = setup();
        let key = identity(&snap, project, study);
        let first = reserve(&db, &snap, project, study);
        db.mark_external_dispatched(key, first.revision, 1).unwrap();
        db.record_external_ack(key, 1, "claimed_forever_12").unwrap();
        let conn = rusqlite::Connection::open(dir.path().join("external.sqlite3")).unwrap();
        assert!(conn.execute(
            "UPDATE agent_external_effects SET attempt=2 WHERE idempotency_key=?1",
            params![key.idempotency_key],
        ).is_err(), "changing an attempt cannot launder an existing ACK ownership claim");
        for attempted_sql in [
            "DELETE FROM agent_external_provider_identities WHERE identity='claimed_forever_12'",
            "UPDATE agent_external_provider_identities SET idempotency_key='stolen' WHERE identity='claimed_forever_12'",
            "UPDATE agent_external_provider_identities SET attempt=3 WHERE identity='claimed_forever_12'",
        ] {
            assert!(conn.execute_batch(attempted_sql).is_err(), "v12 tombstone was mutable");
        }
        let (owner, attempt): (String, i64) = conn.query_row(
            "SELECT idempotency_key, attempt FROM agent_external_provider_identities
             WHERE provider='openai' AND identity='claimed_forever_12'",
            [], |row| Ok((row.get(0)?, row.get(1)?)),
        ).unwrap();
        assert_eq!(owner, key.idempotency_key);
        assert_eq!(attempt, 1);
    }

    #[test]
    fn v12_same_effect_cannot_reuse_old_ack_as_new_attempt_receipt() {
        let (_dir, db, project, study, snap) = setup();
        let key = identity(&snap, project, study);
        let first = reserve(&db, &snap, project, study);
        db.mark_external_dispatched(key, first.revision, 1).unwrap();
        db.record_external_ack(key, 1, "attempt_one_ack_12").unwrap();
        let proof = db.reconcile_external_effect(
            key, 1, ExternalResolution::VerifiedNotExecuted {
                evidence_id: "attempt_one_proof_12",
            },
        ).unwrap();
        let retry = db.prepare_external_retry(key, proof.revision).unwrap();
        db.mark_external_dispatched(key, retry.revision, 2).unwrap();
        assert!(db.record_external_ack(key, 2, "attempt_one_ack_12").is_err());
        assert!(db.reconcile_external_effect(key, 2, ExternalResolution::Completed {
            provider_request_id: Some("fresh_attempt_two_request_12"),
            receipt_id: "attempt_one_ack_12",
            tokens: 35, cost_microusd: 110,
        }).is_err());
        let unchanged = db.get_external_effect(key).unwrap().unwrap();
        assert_eq!(unchanged.attempt, 2);
        assert_eq!(unchanged.status, ExternalEffectStatus::Dispatched);
        assert!(unchanged.provider_request_id.is_none());
        assert!(unchanged.receipt_id.is_none());
    }

    #[test]
    fn v12_legacy_v11_retry_with_lost_ack_fails_upgrade_atomically() {
        let (dir, db, project, study, snap) = setup();
        let key = identity(&snap, project, study);
        let first = reserve(&db, &snap, project, study);
        db.mark_external_dispatched(key, first.revision, 1).unwrap();
        db.record_external_ack(key, 1, "legacy_ack_erased_by_retry_12").unwrap();
        let proof = db.reconcile_external_effect(
            key, 1, ExternalResolution::VerifiedNotExecuted {
                evidence_id: "legacy_nonexecution_proof_12",
            },
        ).unwrap();
        let retry = db.prepare_external_retry(key, proof.revision).unwrap();
        assert_eq!(retry.attempt, 2);
        assert!(retry.provider_request_id.is_none());
        let path = dir.path().join("external.sqlite3");
        drop(db);
        let conn = rusqlite::Connection::open(&path).unwrap();
        remove_v13_from_legacy_fixture(&conn);
        conn.execute_batch(
            "DROP TRIGGER external_provider_identity_on_insert;
             DROP TRIGGER external_provider_identity_on_update;
             DROP INDEX idx_external_provider_request_unique;
             DROP INDEX idx_external_provider_receipt_unique;
             DROP TABLE agent_external_provider_identities;
             DELETE FROM schema_migrations WHERE version = 12;
             PRAGMA user_version = 11;",
        ).unwrap();
        drop(conn);
        assert!(SqliteStore::open(&path).is_err(),
            "v11 retry with unprovable previous identity must not be dispatchable");
        let conn = rusqlite::Connection::open(&path).unwrap();
        let (attempt, status, ack): (i64, String, Option<String>) = conn.query_row(
            "SELECT attempt, status, provider_request_id FROM agent_external_effects
             WHERE idempotency_key=?1",
            params![key.idempotency_key],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        ).unwrap();
        assert_eq!((attempt, status.as_str(), ack), (2, "reserved", None));
        let version: i64 = conn.pragma_query_value(None, "user_version", |row| row.get(0)).unwrap();
        assert_eq!(version, 11);
        let v12_rows: i64 = conn.query_row(
            "SELECT COUNT(*) FROM schema_migrations WHERE version=12",
            [], |row| row.get(0),
        ).unwrap();
        assert_eq!(v12_rows, 0);
    }

}
