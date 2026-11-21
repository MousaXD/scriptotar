//! Additive v13: durable cross-connection freeze for the final paid-send scope.
//!
//! An active barrier makes canonical authorization inputs immutable across
//! independent SQLite connections, including connections that do not hold the
//! desktop's in-process active-project mutex. It is global intentionally: a
//! writer to another project must not be allowed to modify a shared origin.
//!
//! The barrier is committed BEFORE the host recomputes the final consent scope.
//! No SQL write transaction is held while external provider I/O is in flight.
//! A process crash can strand the barrier: this is deliberate fail-closed
//! behavior. Recovery must reconcile the paid effect before any audited unlock.
//!
//! This is an integrity barrier against normal application/database writers;
//! it is NOT a security boundary against a hostile process that can execute
//! arbitrary DDL or delete this barrier table itself.
use rusqlite::{params, OptionalExtension, Transaction, TransactionBehavior};
use scriptotar_core::{now_rfc3339, RepositoryError, RepositoryResult};
use uuid::Uuid;

use super::{storage_error, SqliteStore};

/// Immutable identity that v13 cross-validates against the RESERVED v10/v12
/// attempt in the SAME SQLite IMMEDIATE transaction as barrier admission.
#[derive(Debug, Clone, Copy)]
pub struct ProviderDispatchEffect<'a> {
    pub run_id: &'a str,
    pub step_id: &'a str,
    pub idempotency_key: &'a str,
    pub attempt: i64,
    pub provider: &'a str,
    pub wire_sha256: &'a str,
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

/// Canonical project, Study, transcript, evidence, research and writing inputs.
/// Never protect agent_external_effects here: its pre-send CAS and receipt
/// reconciliation must be able to commit while the provider barrier is active.
/// Future schema changes must audit and extend this exact list.
const FREEZE_TABLES: &[&str] = &[
    "projects",
    "creators",
    "sources",
    "media",
    "transcripts",
    "research_items",
    "analyses",
    "ai_runs",
    "jobs",
    "application_settings",
    "content_studies",
    "study_sources",
    "study_evidence",
    "study_briefs",
    "creative_drafts",
    "draft_versions",
    "study_source_jobs",
    "draft_source_links",
    "evidence_reviews",
    "study_writing_batches",
    "study_writing_revisions",
    "agent_runs",
];

pub(crate) fn migration_13(tx: &Transaction<'_>) -> rusqlite::Result<()> {
    tx.execute_batch(
        "CREATE TABLE provider_dispatch_barrier (
            singleton INTEGER PRIMARY KEY CHECK(singleton=1),
            token TEXT NOT NULL UNIQUE CHECK(length(token)=36),
            project_id TEXT NOT NULL REFERENCES projects(id),
            study_id TEXT NOT NULL REFERENCES content_studies(id),
            scope_digest TEXT NOT NULL CHECK(length(scope_digest)=64),
            acquired_at TEXT NOT NULL,
            effect_run_id TEXT,
            effect_step_id TEXT,
            effect_idempotency_key TEXT REFERENCES agent_external_effects(idempotency_key),
            effect_attempt INTEGER,
            effect_provider TEXT,
            effect_wire_sha256 TEXT,
            CHECK(
                (effect_run_id IS NULL AND effect_step_id IS NULL
                 AND effect_idempotency_key IS NULL AND effect_attempt IS NULL
                 AND effect_provider IS NULL AND effect_wire_sha256 IS NULL)
                OR
                (effect_run_id IS NOT NULL AND effect_step_id IS NOT NULL
                 AND effect_idempotency_key IS NOT NULL AND effect_attempt IS NOT NULL
                 AND effect_provider IS NOT NULL AND effect_wire_sha256 IS NOT NULL
                 AND length(effect_run_id) BETWEEN 1 AND 128
                 AND length(effect_step_id) BETWEEN 1 AND 128
                 AND length(effect_idempotency_key) BETWEEN 1 AND 256
                 AND effect_attempt BETWEEN 1 AND 3
                 AND length(effect_provider) BETWEEN 1 AND 64
                 AND length(effect_wire_sha256)=64)
            )
        );
        CREATE TRIGGER provider_dispatch_barrier_immutable
        BEFORE UPDATE ON provider_dispatch_barrier
        BEGIN SELECT RAISE(ABORT, 'dispatch barrier cannot be updated'); END;
        -- v10 receipt and provider-ack status transitions must remain writable.
        -- Only cancellation intent is serialized against the final provider
        -- dispatch gate. A pre-admission cancellation wins via v10 CAS;
        -- after admission it must not race the network point of no return.
        CREATE TRIGGER provider_dispatch_barrier_effect_cancel
        BEFORE UPDATE OF cancel_requested ON agent_external_effects
        WHEN EXISTS (SELECT 1 FROM provider_dispatch_barrier)
        BEGIN SELECT RAISE(ABORT, 'provider dispatch cancellation is frozen'); END;
        -- A separate connection must not bypass the cancellation gate by
        -- changing status directly while leaving cancel_requested untouched.
        -- Authenticated successful/failed/uncertain receipt transitions remain
        -- writable; remote cancellation requires the reviewed recovery path.
        CREATE TRIGGER provider_dispatch_barrier_effect_cancel_status
        BEFORE UPDATE OF status ON agent_external_effects
        WHEN EXISTS (
            SELECT 1 FROM provider_dispatch_barrier b
            WHERE b.effect_idempotency_key=OLD.idempotency_key
              AND b.effect_attempt=OLD.attempt
        )
         AND NEW.status IN ('cancelled','not_executed')
        BEGIN SELECT RAISE(ABORT, 'provider dispatch cancellation status is frozen'); END;",
    )?;
    for table in FREEZE_TABLES {
        for (verb, suffix) in [
            ("INSERT", "insert"),
            ("UPDATE", "update"),
            ("DELETE", "delete"),
        ] {
            // Identifiers are static reviewed code, never user-controlled.
            tx.execute_batch(&format!(
                "CREATE TRIGGER provider_dispatch_barrier_{table}_{suffix}
                 BEFORE {verb} ON {table}
                 WHEN EXISTS (SELECT 1 FROM provider_dispatch_barrier)
                 BEGIN SELECT RAISE(ABORT, 'provider dispatch scope is frozen'); END;"
            ))?;
        }
    }
    Ok(())
}

/// Owns one globally exclusive, persisted authorization freeze.
/// Drop only releases this object's token. An unsuccessful release leaves the
/// barrier in place, rather than reauthorizing a stale paid request.
pub struct ProviderDispatchBarrier {
    store: SqliteStore,
    token: String,
}

impl ProviderDispatchBarrier {
    /// Called on each final send-boundary time check. If another process
    /// removed or replaced the barrier, fail closed before any provider I/O.
    pub fn verify(&self) -> RepositoryResult<()> {
        let connection = self.store.connection()?;
        let current: Option<String> = connection
            .query_row(
                "SELECT token FROM provider_dispatch_barrier WHERE singleton=1",
                [],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage_error)?;
        if current.as_deref() != Some(self.token.as_str()) {
            return Err(RepositoryError::Conflict(
                "provider dispatch barrier is missing or changed".into(),
            ));
        }
        Ok(())
    }
}

impl Drop for ProviderDispatchBarrier {
    fn drop(&mut self) {
        // A failed release strands the barrier and blocks further canonical
        // writes. Never auto-expire a potentially in-flight paid operation.
        let _ = self.store.release_provider_dispatch_barrier(&self.token);
    }
}

impl SqliteStore {
    /// Unbound freeze for offline host-scope and database regression tests.
    /// This is not accepted by the production sealed dispatch authority.
    pub fn acquire_provider_dispatch_barrier(
        &self,
        project_id: Uuid,
        study_id: Uuid,
        scope_digest: &str,
    ) -> RepositoryResult<ProviderDispatchBarrier> {
        self.acquire_provider_dispatch_barrier_inner(project_id, study_id, scope_digest, None)
    }

    /// Production admission binds the freeze to one previously RESERVED effect.
    /// This validation and the freeze INSERT run in one IMMEDIATE transaction:
    /// no independent cancellation or source writer can slip between them.
    pub fn acquire_provider_dispatch_barrier_for_effect(
        &self,
        project_id: Uuid,
        study_id: Uuid,
        scope_digest: &str,
        effect: ProviderDispatchEffect<'_>,
    ) -> RepositoryResult<ProviderDispatchBarrier> {
        self.acquire_provider_dispatch_barrier_inner(
            project_id,
            study_id,
            scope_digest,
            Some(effect),
        )
    }

    fn acquire_provider_dispatch_barrier_inner(
        &self,
        project_id: Uuid,
        study_id: Uuid,
        scope_digest: &str,
        effect: Option<ProviderDispatchEffect<'_>>,
    ) -> RepositoryResult<ProviderDispatchBarrier> {
        if !valid_digest(scope_digest) {
            return Err(RepositoryError::Validation(
                "invalid provider scope digest".into(),
            ));
        }
        if let Some(bound) = effect {
            if bound.run_id.is_empty()
                || bound.run_id.len() > 128
                || bound.step_id.is_empty()
                || bound.step_id.len() > 128
                || bound.idempotency_key.is_empty()
                || bound.idempotency_key.len() > 256
                || !(1..=3).contains(&bound.attempt)
                || bound.provider.is_empty()
                || bound.provider.len() > 64
                || !valid_digest(bound.wire_sha256)
            {
                return Err(RepositoryError::Validation(
                    "invalid bound provider effect identity".into(),
                ));
            }
        }
        let token = Uuid::new_v4().to_string();
        let mut connection = self.connection()?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(storage_error)?;
        if let Some(bound) = effect {
            let valid_effect: i64 = tx
                .query_row(
                    "SELECT COUNT(*) FROM agent_external_effects
                     WHERE idempotency_key=?1 AND project_id=?2 AND study_id=?3
                       AND run_id=?4 AND step_id=?5 AND attempt=?6
                       AND provider=?7 AND fingerprint=?8
                       AND status='reserved' AND cancel_requested=0",
                    params![
                        bound.idempotency_key,
                        project_id.to_string(),
                        study_id.to_string(),
                        bound.run_id,
                        bound.step_id,
                        bound.attempt,
                        bound.provider,
                        bound.wire_sha256,
                    ],
                    |row| row.get(0),
                )
                .map_err(storage_error)?;
            if valid_effect != 1 {
                return Err(RepositoryError::Conflict(
                    "provider barrier requires exact uncancelled durable reservation".into(),
                ));
            }
        }
        // Primary key singleton=1 forbids a second concurrent paid send.
        // SQLite IMMEDIATE serializes barrier admission against every writer.
        tx.execute(
            "INSERT INTO provider_dispatch_barrier(
                 singleton, token, project_id, study_id, scope_digest, acquired_at,
                 effect_run_id, effect_step_id, effect_idempotency_key, effect_attempt,
                 effect_provider, effect_wire_sha256
             ) VALUES(1, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                token,
                project_id.to_string(),
                study_id.to_string(),
                scope_digest,
                now_rfc3339(),
                effect.map(|item| item.run_id),
                effect.map(|item| item.step_id),
                effect.map(|item| item.idempotency_key),
                effect.map(|item| item.attempt),
                effect.map(|item| item.provider),
                effect.map(|item| item.wire_sha256),
            ],
        )
        .map_err(storage_error)?;
        tx.commit().map_err(storage_error)?;
        Ok(ProviderDispatchBarrier {
            store: self.clone(),
            token,
        })
    }

    fn release_provider_dispatch_barrier(&self, token: &str) -> RepositoryResult<()> {
        let mut connection = self.connection()?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(storage_error)?;
        let owner: Option<(
            Option<String>,
            Option<String>,
            Option<String>,
            Option<i64>,
            Option<String>,
            Option<String>,
        )> = tx
            .query_row(
                "SELECT effect_run_id,effect_step_id,effect_idempotency_key,
                        effect_attempt,effect_provider,effect_wire_sha256
                 FROM provider_dispatch_barrier WHERE singleton=1 AND token=?1",
                params![token],
                |row| Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                )),
            )
            .optional()
            .map_err(storage_error)?;
        let (run, step, key, attempt, provider, wire) = owner.ok_or_else(|| {
            RepositoryError::Conflict("provider dispatch barrier token is not current".into())
        })?;
        match (run, step, key, attempt, provider, wire) {
            (Some(run), Some(step), Some(key), Some(attempt), Some(provider), Some(wire)) => {
                let outcome: Option<(String, Option<String>, Option<String>)> = tx
                    .query_row(
                        "SELECT status,receipt_id,reconciliation_id
                         FROM agent_external_effects
                         WHERE idempotency_key=?1 AND run_id=?2 AND step_id=?3
                           AND attempt=?4 AND provider=?5 AND fingerprint=?6",
                        params![key, run, step, attempt, provider, wire],
                        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                    )
                    .optional()
                    .map_err(storage_error)?;
                let safe = match outcome {
                    // No successful DISPATCHED CAS: this bound attempt cannot
                    // have made a provider request through the certified path.
                    Some((status, _, _)) if status == "reserved" => true,
                    // A terminal charge has a durable provider receipt; a
                    // certified nonexecution/cancellation has provider evidence.
                    Some((status, receipt, _))
                        if matches!(status.as_str(), "completed" | "failed") =>
                    {
                        receipt.is_some()
                    }
                    Some((status, _, evidence))
                        if matches!(status.as_str(), "not_executed" | "cancelled") =>
                    {
                        evidence.is_some()
                    }
                    // DISPATCHED/UNCERTAIN, changed attempts or missing owner
                    // remain globally frozen for independently audited recovery.
                    _ => false,
                };
                if !safe {
                    return Err(RepositoryError::Conflict(
                        "unresolved or unauthenticated provider attempt retains scope freeze".into(),
                    ));
                }
            }
            (None, None, None, None, None, None) => {}
            _ => {
                return Err(RepositoryError::Conflict(
                    "partially missing provider effect ownership cannot be released".into(),
                ));
            }
        }
        let released = tx
            .execute(
                "DELETE FROM provider_dispatch_barrier WHERE singleton=1 AND token=?1",
                params![token],
            )
            .map_err(storage_error)?;
        if released != 1 {
            return Err(RepositoryError::Conflict(
                "provider dispatch barrier did not match owner token".into(),
            ));
        }
        tx.commit().map_err(storage_error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup() -> (TempDir, SqliteStore, Uuid, Uuid) {
        let dir = TempDir::new().unwrap();
        let db = SqliteStore::open(dir.path().join("scope.sqlite3")).unwrap();
        let project = Uuid::new_v4();
        let study = Uuid::new_v4();
        let connection = db.connection().unwrap();
        connection
            .execute(
                "INSERT INTO projects(id,name,created_at) VALUES(?1,'Freeze fixture','2026-11-21')",
                params![project.to_string()],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO content_studies(
                     id,project_id,title,goal,status,revision,created_at,updated_at
                 ) VALUES(?1,?2,'Freeze fixture','Source-bound','active',1,'2026-11-21','2026-11-21')",
                params![study.to_string(), project.to_string()],
            )
            .unwrap();
        (dir, db, project, study)
    }

    #[test]
    fn independent_connection_cannot_change_canonical_scope_until_barrier_releases() {
        let (_dir, db, project, study) = setup();
        let barrier = db
            .acquire_provider_dispatch_barrier(project, study, &"a".repeat(64))
            .unwrap();
        barrier.verify().unwrap();
        let other = rusqlite::Connection::open(db.path()).unwrap();
        for sql in [
            "UPDATE content_studies SET goal='changed' WHERE title='Freeze fixture'",
            "DELETE FROM content_studies WHERE title='Freeze fixture'",
            "INSERT INTO projects(id,name,created_at) VALUES('new-project','Other','2026-11-21')",
        ] {
            assert!(
                other.execute_batch(sql).is_err(),
                "scope writer escaped barrier: {sql}"
            );
        }
        assert!(db
            .acquire_provider_dispatch_barrier(project, study, &"b".repeat(64))
            .is_err());
        drop(barrier);
        assert_eq!(
            other
                .execute(
                    "UPDATE content_studies SET goal='changed' WHERE id=?1",
                    params![study.to_string()],
                )
                .unwrap(),
            1
        );
    }

    #[test]
    fn interrupted_owner_does_not_automatically_unlock_in_flight_paid_send() {
        let (_dir, db, project, study) = setup();
        let barrier = db
            .acquire_provider_dispatch_barrier(project, study, &"a".repeat(64))
            .unwrap();
        std::mem::forget(barrier); // Simulate a process killed without Drop.
        let reopened = SqliteStore::open(db.path()).unwrap();
        assert!(reopened
            .acquire_provider_dispatch_barrier(project, study, &"a".repeat(64))
            .is_err());
        let other = rusqlite::Connection::open(db.path()).unwrap();
        assert!(other
            .execute(
                "UPDATE content_studies SET goal='unapproved' WHERE id=?1",
                params![study.to_string()],
            )
            .is_err());
    }
}
