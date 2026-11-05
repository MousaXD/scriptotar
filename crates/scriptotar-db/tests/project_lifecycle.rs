use rusqlite::{params, Connection};
use scriptotar_core::{
    now_rfc3339, Job, JobInput, JobRepository, Project, ProjectRepository, SettingsRepository,
};
use scriptotar_db::SqliteStore;
use tempfile::TempDir;
use uuid::Uuid;

#[test]
fn project_rename_archive_and_restore_preserve_jobs_and_active_selection() {
    let temp = TempDir::new().unwrap();
    let database = temp.path().join("scriptotar.sqlite3");
    let store = SqliteStore::open(&database).unwrap();
    let project = Project::new("Original");
    store.create_project(&project).unwrap();
    let job = Job::new(
        project.id,
        JobInput::LocalFile("/tmp/keep.mp4".to_owned()),
    );
    store.insert_job(&job).unwrap();
    let mut settings = store.load_settings().unwrap();
    settings.active_project_id = Some(project.id);
    store.save_settings(&settings).unwrap();

    let renamed = store.rename_project(project.id, "Renamed").unwrap();
    assert_eq!(renamed.id, project.id);
    assert_eq!(renamed.created_at, project.created_at);
    store.archive_project(project.id).unwrap();
    store.archive_project(project.id).unwrap();
    assert!(store.list_active_projects().unwrap().is_empty());
    assert_eq!(store.list_archived_projects().unwrap(), vec![renamed.clone()]);
    assert_eq!(store.get_job(job.id).unwrap().project_id, project.id);
    assert_eq!(store.load_settings().unwrap().active_project_id, Some(project.id));

    drop(store);
    let reopened = SqliteStore::open(&database).unwrap();
    assert_eq!(reopened.list_archived_projects().unwrap(), vec![renamed.clone()]);
    reopened.restore_project(project.id).unwrap();
    assert!(reopened.list_archived_projects().unwrap().is_empty());
    assert_eq!(reopened.list_active_projects().unwrap(), vec![renamed]);
    assert_eq!(reopened.get_job(job.id).unwrap().id, job.id);
}

#[test]
fn rename_rejects_existing_name_without_changing_either_id() {
    let temp = TempDir::new().unwrap();
    let store = SqliteStore::open(temp.path().join("scriptotar.sqlite3")).unwrap();
    let first = Project::new("First");
    let second = Project::new("Second");
    store.create_project(&first).unwrap();
    store.create_project(&second).unwrap();

    assert!(store.rename_project(first.id, "second").is_err());
    assert_eq!(store.get_project(first.id).unwrap(), first);
    assert_eq!(store.get_project(second.id).unwrap(), second);
}

#[test]
fn database_rejects_foreign_creator_and_analysis_links() {
    let temp = TempDir::new().unwrap();
    let store = SqliteStore::open(temp.path().join("scriptotar.sqlite3")).unwrap();
    let project_a = Project::new("A");
    let project_b = Project::new("B");
    store.create_project(&project_a).unwrap();
    store.create_project(&project_b).unwrap();

    let conn = Connection::open(store.path()).unwrap();
    conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
    let creator = Uuid::new_v4();
    let source = Uuid::new_v4();
    let media = Uuid::new_v4();
    let transcript = Uuid::new_v4();
    let now = now_rfc3339();

    conn.execute(
        "INSERT INTO creators(id, project_id, platform, profile_url, created_at)
         VALUES(?1, ?2, 'youtube', 'https://example.invalid/creator', ?3)",
        params![creator.to_string(), project_a.id.to_string(), now],
    )
    .unwrap();
    let bad_source = conn.execute(
        "INSERT INTO sources(id, project_id, creator_id, source_type, locator, created_at)
         VALUES(?1, ?2, ?3, 'url', 'https://example.invalid/video', ?4)",
        params![
            Uuid::new_v4().to_string(),
            project_b.id.to_string(),
            creator.to_string(),
            now
        ],
    );
    assert!(bad_source.is_err());

    let bad_research = conn.execute(
        "INSERT INTO research_items(
             id, project_id, creator_id, source_url, platform, scanned_at
         ) VALUES(?1, ?2, ?3, 'https://example.invalid/video', 'youtube', ?4)",
        params![
            Uuid::new_v4().to_string(),
            project_b.id.to_string(),
            creator.to_string(),
            now
        ],
    );
    assert!(bad_research.is_err());

    conn.execute(
        "INSERT INTO sources(id, project_id, source_type, locator, created_at)
         VALUES(?1, ?2, 'local_file', '/tmp/safe.mp4', ?3)",
        params![source.to_string(), project_a.id.to_string(), now],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO media(id, source_id, local_path, created_at)
         VALUES(?1, ?2, '/tmp/safe.mp4', ?3)",
        params![media.to_string(), source.to_string(), now],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO transcripts(id, media_id, text, created_at, updated_at)
         VALUES(?1, ?2, 'safe', ?3, ?3)",
        params![transcript.to_string(), media.to_string(), now],
    )
    .unwrap();
    let invalid_analysis = conn.execute(
        "INSERT INTO analyses(id, project_id, transcript_id, kind, content, created_at)
         VALUES(?1, ?2, ?3, 'summary', 'wrong project', ?4)",
        params![
            Uuid::new_v4().to_string(),
            project_b.id.to_string(),
            transcript.to_string(),
            now
        ],
    );
    assert!(invalid_analysis.is_err());
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM analyses", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 0);
}
