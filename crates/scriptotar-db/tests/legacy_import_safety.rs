use std::path::Path;

use rusqlite::{params, Connection};
use scriptotar_core::{ContentRepository, Project, ProjectRepository, RepositoryError};
use scriptotar_db::SqliteStore;
use tempfile::TempDir;

fn classic_database(path: &Path, transcript: &str, input_type: &str) -> Connection {
    let connection = Connection::open(path).unwrap();
    connection
        .pragma_update(None, "journal_mode", "WAL")
        .unwrap();
    connection
        .execute_batch(
            "CREATE TABLE projects(name TEXT PRIMARY KEY, created_at TEXT NOT NULL);
             CREATE TABLE jobs(
                 id TEXT PRIMARY KEY, created_at TEXT NOT NULL, source TEXT NOT NULL,
                 input_type TEXT NOT NULL, title TEXT, status TEXT NOT NULL, language TEXT,
                 output_dir TEXT, transcript TEXT, error TEXT, project TEXT NOT NULL
             );
             INSERT INTO projects VALUES('Historical', '2026-01-01 00:00:00');",
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO jobs(
                 id, created_at, source, input_type, title, status,
                 language, output_dir, transcript, error, project
             ) VALUES(
                 'original-id', '2026-01-01 00:00:00', '/tmp/legacy.mp4',
                 ?1, 'Historical title', 'Done', 'en', '/tmp/out', ?2, NULL, 'Historical'
             )",
            params![input_type, transcript],
        )
        .unwrap();
    connection
}

#[test]
fn live_wal_import_reads_consistent_backup_and_detects_changed_content() {
    let temp = TempDir::new().unwrap();
    let legacy_path = temp.path().join("history.sqlite3");
    let legacy_writer = classic_database(&legacy_path, "original transcript", "file");
    let store = SqliteStore::open(temp.path().join("modern.sqlite3")).unwrap();

    let first = store.import_legacy_database(&legacy_path).unwrap();
    assert_eq!(first.jobs, 1);
    let backup_path = first.backup_path.unwrap();
    let backup = Connection::open(&backup_path).unwrap();
    let original_snapshot: String = backup
        .query_row(
            "SELECT transcript FROM jobs WHERE id = 'original-id'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(original_snapshot, "original transcript");
    assert_eq!(store.list_projects().unwrap().len(), 1);
    assert_eq!(
        store.list_transcripts(None).unwrap()[0].transcript.text,
        "original transcript"
    );

    let repeated = store.import_legacy_database(&legacy_path).unwrap();
    assert!(repeated.skipped);

    // The WAL writer remains open and changes the same row without changing its ID.
    legacy_writer
        .execute(
            "UPDATE jobs SET transcript = 'new content in WAL' WHERE id = 'original-id'",
            [],
        )
        .unwrap();
    let conflict = store.import_legacy_database(&legacy_path).unwrap_err();
    assert!(matches!(conflict, RepositoryError::Conflict(_)));
    assert_eq!(
        store.list_transcripts(None).unwrap()[0].transcript.text,
        "original transcript"
    );
    assert_eq!(store.list_transcripts(None).unwrap().len(), 1);

    // The first backup is immutable even after the legacy database changes.
    let unchanged_backup: String = backup
        .query_row(
            "SELECT transcript FROM jobs WHERE id = 'original-id'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(unchanged_backup, "original transcript");
}

#[test]
fn separate_legacy_databases_cannot_overwrite_shared_deterministic_ids() {
    let temp = TempDir::new().unwrap();
    let first_path = temp.path().join("old-a.sqlite3");
    let second_path = temp.path().join("old-b.sqlite3");
    let _first = classic_database(&first_path, "first source", "file");
    let _second = classic_database(&second_path, "second source", "file");
    let store = SqliteStore::open(temp.path().join("modern.sqlite3")).unwrap();

    store.import_legacy_database(&first_path).unwrap();
    let failed = store.import_legacy_database(&second_path).unwrap_err();
    assert!(matches!(failed, RepositoryError::Conflict(_)));
    let transcripts = store.list_transcripts(None).unwrap();
    assert_eq!(transcripts.len(), 1);
    assert_eq!(transcripts[0].transcript.text, "first source");
}

#[test]
fn ambiguous_legacy_input_type_rolls_back_every_record() {
    let temp = TempDir::new().unwrap();
    let legacy_path = temp.path().join("history.sqlite3");
    let _legacy = classic_database(&legacy_path, "never import", "unknown");
    let store = SqliteStore::open(temp.path().join("modern.sqlite3")).unwrap();

    let failed = store.import_legacy_database(&legacy_path).unwrap_err();
    assert!(matches!(failed, RepositoryError::Validation(_)));
    assert!(store.list_projects().unwrap().is_empty());
    assert!(store.list_transcripts(None).unwrap().is_empty());
}

#[test]
fn legacy_project_name_collision_does_not_join_unrelated_user_data() {
    let temp = TempDir::new().unwrap();
    let legacy_path = temp.path().join("history.sqlite3");
    let _legacy = classic_database(&legacy_path, "never join", "file");
    let store = SqliteStore::open(temp.path().join("modern.sqlite3")).unwrap();
    let native = Project::new("Historical");
    store.create_project(&native).unwrap();

    let failed = store.import_legacy_database(&legacy_path).unwrap_err();
    assert!(matches!(failed, RepositoryError::Conflict(_)));
    assert_eq!(store.list_projects().unwrap(), vec![native]);
    assert!(store.list_transcripts(None).unwrap().is_empty());
}

#[cfg(unix)]
#[test]
fn dangling_symlink_backup_is_never_replaced_by_snapshot_publish() {
    use std::fs;
    use std::os::unix::fs::symlink;

    let temp = TempDir::new().unwrap();
    let legacy_path = temp.path().join("history.sqlite3");
    let _legacy = classic_database(&legacy_path, "must stay intact", "file");
    let store = SqliteStore::open(temp.path().join("modern.sqlite3")).unwrap();

    let report = store.import_legacy_database(&legacy_path).unwrap();
    let backup_path = std::path::PathBuf::from(report.backup_path.unwrap());
    let retained_copy = temp.path().join("retained-original-snapshot.sqlite3");
    fs::rename(&backup_path, &retained_copy).unwrap();
    let nonexistent_target = temp.path().join("nonexistent-target.sqlite3");
    symlink(&nonexistent_target, &backup_path).unwrap();
    assert!(!backup_path.exists());
    assert!(fs::symlink_metadata(&backup_path)
        .unwrap()
        .file_type()
        .is_symlink());

    let result = store.import_legacy_database(&legacy_path);
    assert!(matches!(result, Err(RepositoryError::Conflict(_))));
    assert!(fs::symlink_metadata(&backup_path)
        .unwrap()
        .file_type()
        .is_symlink());
    assert!(!nonexistent_target.exists());
    assert!(retained_copy.exists());
    assert_eq!(
        store.list_transcripts(None).unwrap()[0].transcript.text,
        "must stay intact"
    );
}

#[test]
fn existing_backup_with_different_contents_is_never_overwritten() {
    let temp = TempDir::new().unwrap();
    let legacy_path = temp.path().join("history.sqlite3");
    let _legacy = classic_database(&legacy_path, "preserve imported transcript", "file");
    let store = SqliteStore::open(temp.path().join("modern.sqlite3")).unwrap();

    let report = store.import_legacy_database(&legacy_path).unwrap();
    let backup_path = report.backup_path.unwrap();
    std::fs::write(&backup_path, b"do not overwrite").unwrap();

    let result = store.import_legacy_database(&legacy_path);
    assert!(matches!(result, Err(RepositoryError::Conflict(_))));
    assert_eq!(std::fs::read(&backup_path).unwrap(), b"do not overwrite");
    assert_eq!(
        store.list_transcripts(None).unwrap()[0].transcript.text,
        "preserve imported transcript"
    );
}

#[cfg(unix)]
#[test]
fn symlink_to_identical_backup_is_rejected_without_following_it() {
    use std::fs;
    use std::os::unix::fs::symlink;

    let temp = TempDir::new().unwrap();
    let legacy_path = temp.path().join("history.sqlite3");
    let _legacy = classic_database(&legacy_path, "keep original data", "file");
    let store = SqliteStore::open(temp.path().join("modern.sqlite3")).unwrap();

    let report = store.import_legacy_database(&legacy_path).unwrap();
    let backup_path = std::path::PathBuf::from(report.backup_path.unwrap());
    let retained_copy = temp.path().join("original-backup.sqlite3");
    fs::rename(&backup_path, &retained_copy).unwrap();
    symlink(&retained_copy, &backup_path).unwrap();

    let result = store.import_legacy_database(&legacy_path);
    assert!(matches!(result, Err(RepositoryError::Conflict(_))));
    assert!(fs::symlink_metadata(&backup_path)
        .unwrap()
        .file_type()
        .is_symlink());
    assert!(retained_copy.is_file());
    assert_eq!(
        store.list_transcripts(None).unwrap()[0].transcript.text,
        "keep original data"
    );
}
