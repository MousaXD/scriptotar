use std::time::Duration;

use rusqlite::{params, Connection, OptionalExtension};
use scriptotar_core::{RepositoryError, RepositoryResult};
use uuid::Uuid;

use crate::SqliteStore;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WatchlistRefreshState {
    Refreshing,
    RetryScheduled,
    Failed,
    Healthy,
}

impl WatchlistRefreshState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Refreshing => "refreshing",
            Self::RetryScheduled => "retry_scheduled",
            Self::Failed => "failed",
            Self::Healthy => "healthy",
        }
    }

    fn from_str(value: &str) -> RepositoryResult<Self> {
        match value {
            "refreshing" => Ok(Self::Refreshing),
            "retry_scheduled" => Ok(Self::RetryScheduled),
            "failed" => Ok(Self::Failed),
            "healthy" => Ok(Self::Healthy),
            other => Err(RepositoryError::Storage(format!(
                "unknown persisted watchlist refresh state: {other}"
            ))),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WatchlistRefreshStatus {
    pub watchlist_id: Uuid,
    pub state: WatchlistRefreshState,
    pub last_attempt_at: Option<String>,
    pub last_success_at: Option<String>,
    pub last_error: Option<String>,
    pub next_retry_at: Option<String>,
}

fn storage_error(error: rusqlite::Error) -> RepositoryError {
    RepositoryError::Storage(error.to_string())
}

fn open_connection(store: &SqliteStore) -> RepositoryResult<Connection> {
    let connection = Connection::open(store.path()).map_err(storage_error)?;
    connection
        .pragma_update(None, "foreign_keys", true)
        .map_err(storage_error)?;
    connection
        .busy_timeout(Duration::from_secs(5))
        .map_err(storage_error)?;
    Ok(connection)
}

fn ensure_schema(connection: &Connection) -> RepositoryResult<()> {
    connection
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS watchlist_refresh_status (
                watchlist_id TEXT PRIMARY KEY REFERENCES watchlists(id) ON DELETE CASCADE,
                state TEXT NOT NULL CHECK(state IN ('refreshing', 'retry_scheduled', 'failed', 'healthy')),
                last_attempt_at TEXT,
                last_success_at TEXT,
                last_error TEXT,
                next_retry_at TEXT
            );",
        )
        .map_err(storage_error)
}

impl SqliteStore {
    pub fn record_watchlist_refresh_attempt(
        &self,
        watchlist_id: Uuid,
        attempted_at: &str,
    ) -> RepositoryResult<()> {
        self.run_integration_migrations()?;
        let connection = open_connection(self)?;
        ensure_schema(&connection)?;
        connection
            .execute(
                "INSERT INTO watchlist_refresh_status(
                    watchlist_id, state, last_attempt_at, last_success_at, last_error, next_retry_at
                 ) VALUES(?1, 'refreshing', ?2, NULL, NULL, NULL)
                 ON CONFLICT(watchlist_id) DO UPDATE SET
                    state = 'refreshing',
                    last_attempt_at = excluded.last_attempt_at,
                    last_error = NULL,
                    next_retry_at = NULL",
                params![watchlist_id.to_string(), attempted_at],
            )
            .map_err(storage_error)?;
        Ok(())
    }

    pub fn record_watchlist_refresh_success(
        &self,
        watchlist_id: Uuid,
        succeeded_at: &str,
    ) -> RepositoryResult<()> {
        self.run_integration_migrations()?;
        let connection = open_connection(self)?;
        ensure_schema(&connection)?;
        connection
            .execute(
                "INSERT INTO watchlist_refresh_status(
                    watchlist_id, state, last_attempt_at, last_success_at, last_error, next_retry_at
                 ) VALUES(?1, 'healthy', ?2, ?2, NULL, NULL)
                 ON CONFLICT(watchlist_id) DO UPDATE SET
                    state = 'healthy',
                    last_attempt_at = COALESCE(watchlist_refresh_status.last_attempt_at, excluded.last_attempt_at),
                    last_success_at = excluded.last_success_at,
                    last_error = NULL,
                    next_retry_at = NULL",
                params![watchlist_id.to_string(), succeeded_at],
            )
            .map_err(storage_error)?;
        Ok(())
    }

    pub fn record_watchlist_refresh_failure(
        &self,
        watchlist_id: Uuid,
        failed_at: &str,
        safe_error: &str,
        next_retry_at: Option<&str>,
    ) -> RepositoryResult<()> {
        self.run_integration_migrations()?;
        let connection = open_connection(self)?;
        ensure_schema(&connection)?;
        let state = if next_retry_at.is_some() {
            WatchlistRefreshState::RetryScheduled
        } else {
            WatchlistRefreshState::Failed
        };
        connection
            .execute(
                "INSERT INTO watchlist_refresh_status(
                    watchlist_id, state, last_attempt_at, last_success_at, last_error, next_retry_at
                 ) VALUES(?1, ?2, ?3, NULL, ?4, ?5)
                 ON CONFLICT(watchlist_id) DO UPDATE SET
                    state = excluded.state,
                    last_attempt_at = COALESCE(watchlist_refresh_status.last_attempt_at, excluded.last_attempt_at),
                    last_error = excluded.last_error,
                    next_retry_at = excluded.next_retry_at",
                params![
                    watchlist_id.to_string(),
                    state.as_str(),
                    failed_at,
                    safe_error,
                    next_retry_at,
                ],
            )
            .map_err(storage_error)?;
        Ok(())
    }

    pub fn recover_interrupted_watchlist_refreshes(&self) -> RepositoryResult<usize> {
        self.run_integration_migrations()?;
        let connection = open_connection(self)?;
        ensure_schema(&connection)?;
        connection
            .execute(
                "UPDATE watchlist_refresh_status
                 SET state = 'failed',
                     last_error = 'The previous automatic refresh was interrupted before completion.',
                     next_retry_at = NULL
                 WHERE state = 'refreshing'",
                [],
            )
            .map_err(storage_error)
    }

    pub fn list_watchlist_refresh_status(
        &self,
        project_id: Option<Uuid>,
    ) -> RepositoryResult<Vec<WatchlistRefreshStatus>> {
        self.run_integration_migrations()?;
        let connection = open_connection(self)?;
        ensure_schema(&connection)?;
        let sql = if project_id.is_some() {
            "SELECT s.watchlist_id, s.state, s.last_attempt_at, s.last_success_at, s.last_error, s.next_retry_at
             FROM watchlist_refresh_status s
             JOIN watchlists w ON w.id = s.watchlist_id
             WHERE w.project_id = ?1
             ORDER BY w.label COLLATE NOCASE"
        } else {
            "SELECT watchlist_id, state, last_attempt_at, last_success_at, last_error, next_retry_at
             FROM watchlist_refresh_status
             ORDER BY watchlist_id"
        };
        let mut statement = connection.prepare(sql).map_err(storage_error)?;
        let map_row = |row: &rusqlite::Row<'_>| -> rusqlite::Result<(
            String,
            String,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
        )> {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
            ))
        };
        let rows = if let Some(project_id) = project_id {
            statement
                .query_map(params![project_id.to_string()], map_row)
                .map_err(storage_error)?
        } else {
            statement.query_map([], map_row).map_err(storage_error)?
        };
        rows.map(|row| {
            let (watchlist_id, state, last_attempt_at, last_success_at, last_error, next_retry_at) =
                row.map_err(storage_error)?;
            Ok(WatchlistRefreshStatus {
                watchlist_id: Uuid::parse_str(&watchlist_id)
                    .map_err(|error| RepositoryError::Storage(error.to_string()))?,
                state: WatchlistRefreshState::from_str(&state)?,
                last_attempt_at,
                last_success_at,
                last_error,
                next_retry_at,
            })
        })
        .collect()
    }

    pub fn watchlist_refresh_status(
        &self,
        watchlist_id: Uuid,
    ) -> RepositoryResult<Option<WatchlistRefreshStatus>> {
        self.run_integration_migrations()?;
        let connection = open_connection(self)?;
        ensure_schema(&connection)?;
        let row = connection
            .query_row(
                "SELECT watchlist_id, state, last_attempt_at, last_success_at, last_error, next_retry_at
                 FROM watchlist_refresh_status WHERE watchlist_id = ?1",
                params![watchlist_id.to_string()],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, Option<String>>(2)?,
                        row.get::<_, Option<String>>(3)?,
                        row.get::<_, Option<String>>(4)?,
                        row.get::<_, Option<String>>(5)?,
                    ))
                },
            )
            .optional()
            .map_err(storage_error)?;
        row.map(
            |(watchlist_id, state, last_attempt_at, last_success_at, last_error, next_retry_at)| {
                Ok(WatchlistRefreshStatus {
                    watchlist_id: Uuid::parse_str(&watchlist_id)
                        .map_err(|error| RepositoryError::Storage(error.to_string()))?,
                    state: WatchlistRefreshState::from_str(&state)?,
                    last_attempt_at,
                    last_success_at,
                    last_error,
                    next_retry_at,
                })
            },
        )
        .transpose()
    }
}

#[cfg(test)]
mod tests {
    use scriptotar_core::{Project, ProjectRepository};
    use tempfile::tempdir;

    use super::*;

    fn store_with_watchlist() -> (tempfile::TempDir, SqliteStore, Uuid) {
        let temp = tempdir().unwrap();
        let store = SqliteStore::open(temp.path().join("scriptotar.sqlite3")).unwrap();
        store.run_integration_migrations().unwrap();
        let project = Project::new("Inbox");
        store.create_project(&project).unwrap();
        let watchlist = store
            .upsert_watchlist(
                project.id,
                "Creator",
                "https://www.youtube.com/@creator",
                25,
            )
            .unwrap();
        (temp, store, watchlist.id)
    }

    #[test]
    fn refresh_failure_and_recovery_survive_restart() {
        let (temp, store, watchlist_id) = store_with_watchlist();
        store
            .record_watchlist_refresh_attempt(watchlist_id, "2026-08-10T10:00:00Z")
            .unwrap();
        store
            .record_watchlist_refresh_failure(
                watchlist_id,
                "2026-08-10T10:00:00Z",
                "Provider authentication is required.",
                Some("2026-08-10T11:00:00Z"),
            )
            .unwrap();
        drop(store);

        let reopened = SqliteStore::open(temp.path().join("scriptotar.sqlite3")).unwrap();
        let status = reopened
            .watchlist_refresh_status(watchlist_id)
            .unwrap()
            .unwrap();
        assert_eq!(status.state, WatchlistRefreshState::RetryScheduled);
        assert_eq!(
            status.last_error.as_deref(),
            Some("Provider authentication is required.")
        );
        assert_eq!(
            status.next_retry_at.as_deref(),
            Some("2026-08-10T11:00:00Z")
        );

        reopened
            .record_watchlist_refresh_success(watchlist_id, "2026-08-10T11:01:00Z")
            .unwrap();
        let recovered = reopened
            .watchlist_refresh_status(watchlist_id)
            .unwrap()
            .unwrap();
        assert_eq!(recovered.state, WatchlistRefreshState::Healthy);
        assert!(recovered.last_error.is_none());
        assert!(recovered.next_retry_at.is_none());
        assert_eq!(
            recovered.last_success_at.as_deref(),
            Some("2026-08-10T11:01:00Z")
        );
    }

    #[test]
    fn interrupted_refresh_is_marked_failed_on_restart_recovery() {
        let (_temp, store, watchlist_id) = store_with_watchlist();
        store
            .record_watchlist_refresh_attempt(watchlist_id, "2026-08-10T10:00:00Z")
            .unwrap();
        assert_eq!(store.recover_interrupted_watchlist_refreshes().unwrap(), 1);
        let status = store
            .watchlist_refresh_status(watchlist_id)
            .unwrap()
            .unwrap();
        assert_eq!(status.state, WatchlistRefreshState::Failed);
        assert_eq!(
            status.last_error.as_deref(),
            Some("The previous automatic refresh was interrupted before completion.")
        );
    }
}
