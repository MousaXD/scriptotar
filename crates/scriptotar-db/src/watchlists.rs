use std::time::Duration;

use rusqlite::{params, Connection, OptionalExtension};
use scriptotar_core::{ProjectRepository, RepositoryError, RepositoryResult, Watchlist};
use uuid::Uuid;

use crate::SqliteStore;

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

fn validate_watchlist_text<'a>(field: &str, value: &'a str) -> RepositoryResult<&'a str> {
    let value = value.trim();
    if value.is_empty() {
        return Err(RepositoryError::Validation(format!(
            "watchlist {field} cannot be empty"
        )));
    }
    if value.chars().count() > 2_048 {
        return Err(RepositoryError::Validation(format!(
            "watchlist {field} is too long"
        )));
    }
    Ok(value)
}

impl SqliteStore {
    pub fn upsert_watchlist(
        &self,
        project_id: Uuid,
        label: &str,
        profile_url: &str,
        limit_count: u32,
    ) -> RepositoryResult<Watchlist> {
        self.run_integration_migrations()?;
        self.get_project(project_id)?;
        let label = validate_watchlist_text("label", label)?;
        let profile_url = validate_watchlist_text("profile URL", profile_url)?;
        if !(1..=200).contains(&limit_count) {
            return Err(RepositoryError::Validation(
                "watchlist limit must be between 1 and 200".to_owned(),
            ));
        }

        let connection = open_connection(self)?;
        let existing = connection
            .query_row(
                "SELECT id, last_scan_at FROM watchlists WHERE project_id = ?1 AND profile_url = ?2",
                params![project_id.to_string(), profile_url],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?)),
            )
            .optional()
            .map_err(storage_error)?;

        let (id, last_scan_at) = if let Some((id, last_scan_at)) = existing {
            let id = Uuid::parse_str(&id)
                .map_err(|error| RepositoryError::Storage(error.to_string()))?;
            connection
                .execute(
                    "UPDATE watchlists SET label = ?1, limit_count = ?2 WHERE id = ?3",
                    params![label, i64::from(limit_count), id.to_string()],
                )
                .map_err(storage_error)?;
            (id, last_scan_at)
        } else {
            let id = Uuid::new_v4();
            connection
                .execute(
                    "INSERT INTO watchlists(id, project_id, label, profile_url, limit_count, last_scan_at)
                     VALUES(?1, ?2, ?3, ?4, ?5, NULL)",
                    params![
                        id.to_string(),
                        project_id.to_string(),
                        label,
                        profile_url,
                        i64::from(limit_count),
                    ],
                )
                .map_err(storage_error)?;
            (id, None)
        };

        Ok(Watchlist {
            id,
            project_id,
            label: label.to_owned(),
            profile_url: profile_url.to_owned(),
            limit_count,
            last_scan_at,
        })
    }

    pub fn mark_watchlist_scanned_by_profile(
        &self,
        project_id: Uuid,
        profile_url: &str,
        scanned_at: &str,
    ) -> RepositoryResult<bool> {
        self.run_integration_migrations()?;
        let profile_url = validate_watchlist_text("profile URL", profile_url)?;
        let scanned_at = validate_watchlist_text("scan timestamp", scanned_at)?;
        let connection = open_connection(self)?;
        let updated = connection
            .execute(
                "UPDATE watchlists SET last_scan_at = ?1 WHERE project_id = ?2 AND profile_url = ?3",
                params![scanned_at, project_id.to_string(), profile_url],
            )
            .map_err(storage_error)?;
        Ok(updated > 0)
    }
}

#[cfg(test)]
mod tests {
    use scriptotar_core::{now_rfc3339, Project, ProjectRepository, WatchlistRepository};
    use tempfile::tempdir;

    use super::SqliteStore;

    #[test]
    fn integration_watchlist_upsert_is_project_scoped_persistent_and_idempotent() {
        let temp = tempdir().unwrap();
        let store = SqliteStore::open(temp.path().join("scriptotar.sqlite3")).unwrap();
        store.run_integration_migrations().unwrap();
        let first_project = Project::new("First");
        let second_project = Project::new("Second");
        store.create_project(&first_project).unwrap();
        store.create_project(&second_project).unwrap();

        let first = store
            .upsert_watchlist(
                first_project.id,
                "Creator",
                "https://www.youtube.com/@creator",
                25,
            )
            .unwrap();
        let updated = store
            .upsert_watchlist(
                first_project.id,
                "Creator updated",
                "https://www.youtube.com/@creator",
                50,
            )
            .unwrap();
        store
            .upsert_watchlist(
                second_project.id,
                "Creator",
                "https://www.youtube.com/@creator",
                10,
            )
            .unwrap();

        assert_eq!(first.id, updated.id);
        let first_project_rows = store.list_watchlists(Some(first_project.id)).unwrap();
        assert_eq!(first_project_rows.len(), 1);
        assert_eq!(first_project_rows[0].label, "Creator updated");
        assert_eq!(first_project_rows[0].limit_count, 50);
        assert_eq!(first_project_rows[0].project_id, first_project.id);

        let scan_time = now_rfc3339();
        assert!(store
            .mark_watchlist_scanned_by_profile(
                first_project.id,
                "https://www.youtube.com/@creator",
                &scan_time,
            )
            .unwrap());
        let scanned = store.list_watchlists(Some(first_project.id)).unwrap();
        assert_eq!(scanned[0].last_scan_at.as_deref(), Some(scan_time.as_str()));

        drop(store);
        let reopened = SqliteStore::open(temp.path().join("scriptotar.sqlite3")).unwrap();
        reopened.run_integration_migrations().unwrap();
        let reopened_rows = reopened.list_watchlists(Some(first_project.id)).unwrap();
        assert_eq!(reopened_rows.len(), 1);
        assert_eq!(reopened_rows[0].id, first.id);
        assert_eq!(
            reopened_rows[0].profile_url,
            "https://www.youtube.com/@creator"
        );
        assert_eq!(
            reopened_rows[0].last_scan_at.as_deref(),
            Some(scan_time.as_str())
        );
    }
}
