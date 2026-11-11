//! Regression coverage for the patched bundled SQLite dependency. These tests
//! intentionally exercise the same WAL and online-backup paths used by upgrades.
use std::time::Duration;

use rusqlite::{backup::Backup, Connection};
use scriptotar_core::{
    Job, JobInput, JobRepository, Project, ProjectRepository, SettingsRepository,
};
use scriptotar_db::{SqliteStore, LATEST_INTEGRATION_SCHEMA_VERSION, LATEST_SCHEMA_VERSION};
use tempfile::TempDir;

#[test]
fn patched_bundled_sqlite_keeps_wal_online_backup_and_user_data() {
    assert!(
        rusqlite::version_number() >= 3_054_000,
        "bundled SQLite must contain the fix for RUSTSEC-2026-0459 (SQLite 3.54.0+)"
    );

    let temp = TempDir::new().unwrap();
    let path = temp.path().join("source.sqlite3");
    let backup_path = temp.path().join("backup.sqlite3");
    let store = SqliteStore::open(&path).unwrap();
    store.run_integration_migrations().unwrap();

    let project = Project::new("Keep data across SQLite upgrade");
    store.create_project(&project).unwrap();
    let job = Job::new(
        project.id,
        JobInput::LocalFile("/tmp/dependency-upgrade-clip.mp4".to_owned()),
    );
    store.insert_job(&job).unwrap();
    let mut settings = store.load_settings().unwrap();
    settings.active_project_id = Some(project.id);
    store.save_settings(&settings).unwrap();

    let source = Connection::open(&path).unwrap();
    let journal: String = source
        .pragma_query_value(None, "journal_mode", |row| row.get(0))
        .unwrap();
    assert_eq!(journal.to_ascii_lowercase(), "wal");

    let mut destination = Connection::open(&backup_path).unwrap();
    {
        let backup = Backup::new(&source, &mut destination).unwrap();
        backup
            .run_to_completion(64, Duration::from_millis(5), None)
            .unwrap();
    }
    let integrity: String = destination
        .query_row("PRAGMA quick_check", [], |row| row.get(0))
        .unwrap();
    assert_eq!(integrity, "ok");
    drop(destination);

    let restored = SqliteStore::open(&backup_path).unwrap();
    restored.run_integration_migrations().unwrap();
    assert_eq!(restored.schema_version().unwrap(), LATEST_SCHEMA_VERSION);
    assert_eq!(restored.get_project(project.id).unwrap(), project);
    assert_eq!(restored.get_job(job.id).unwrap().id, job.id);
    assert_eq!(
        restored.load_settings().unwrap().active_project_id,
        Some(project.id)
    );
}

#[test]
fn reopened_schema_migrations_are_repeatable_and_tampering_fails_closed() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("upgraded.sqlite3");
    let project = Project::new("Must survive failed upgrade");

    {
        let store = SqliteStore::open(&path).unwrap();
        store.run_integration_migrations().unwrap();
        store.create_project(&project).unwrap();
        store.run_integration_migrations().unwrap();
    }
    let reopened = SqliteStore::open(&path).unwrap();
    reopened.run_integration_migrations().unwrap();
    assert_eq!(reopened.get_project(project.id).unwrap(), project);

    let connection = Connection::open(&path).unwrap();
    for (table, expected) in [
        ("schema_migrations", LATEST_SCHEMA_VERSION),
        (
            "integration_schema_migrations",
            LATEST_INTEGRATION_SCHEMA_VERSION,
        ),
    ] {
        let actual: u32 = connection
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(actual, expected, "{table} must not be re-applied");
    }

    connection
        .execute(
            "UPDATE schema_migrations SET name = 'unexpected' WHERE version = 1",
            [],
        )
        .unwrap();
    assert!(SqliteStore::open(&path).is_err(), "tampered ledger must fail closed");
    let preserved: String = connection
        .query_row(
            "SELECT name FROM projects WHERE id = ?1",
            [project.id.to_string()],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(preserved, project.name);
}
