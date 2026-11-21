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

use crate::{storage_error, SqliteStore};

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
            acquired_at TEXT NOT NULL
        );
        CREATE TRIGGER provider_dispatch_barrier_immutable
        BEFORE UPDATE ON provider_dispatch_barrier
        BEGIN SELECT RAISE(ABORT, 'dispatch barrier cannot be updated'); END;",
    )?;
    for table in FREEZE_TABLES {
        for (verb, suffix) in [("INSERT", "insert"), ("UPDATE", "update"), ("DELETE", "delete")] {
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
    pub fn acquire_provider_dispatch_barrier(
        &self,
        project_id: Uuid,
        study_id: Uuid,
        scope_digest: &str,
    ) -> RepositoryResult<ProviderDispatchBarrier> {
        if scope_digest.len() != 64
            || !scope_digest
                .bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        {
            return Err(RepositoryError::Validation("invalid provider scope digest".into()));
        }
        let token = Uuid::new_v4().to_string();
        let mut connection = self.connection()?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(storage_error)?;
        // Primary key singleton=1 forbids a second concurrent paid send.
        // SQLite IMMEDIATE serializes barrier admission against every writer.
        tx.execute(
            "INSERT INTO provider_dispatch_barrier(
                 singleton, token, project_id, study_id, scope_digest, acquired_at
             ) VALUES(1, ?1, ?2, ?3, ?4, ?5)",
            params![
                token,
                project_id.to_string(),
                study_id.to_string(),
                scope_digest,
                now_rfc3339()
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
            assert!(other.execute_batch(sql).is_err(), "scope writer escaped barrier: {sql}");
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
