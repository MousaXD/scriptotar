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
    let job = Job::new(project.id, JobInput::LocalFile("/tmp/keep.mp4".to_owned()));
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
    assert_eq!(
        store.list_archived_projects().unwrap(),
        vec![renamed.clone()]
    );
    assert_eq!(store.get_job(job.id).unwrap().project_id, project.id);
    assert_eq!(
        store.load_settings().unwrap().active_project_id,
        Some(project.id)
    );

    drop(store);
    let reopened = SqliteStore::open(&database).unwrap();
    assert_eq!(
        reopened.list_archived_projects().unwrap(),
        vec![renamed.clone()]
    );
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

#[test]
fn job_transcript_link_rejects_cross_project_lineage() {
    let temp = TempDir::new().unwrap();
    let store = SqliteStore::open(temp.path().join("scriptotar.sqlite3")).unwrap();
    store.run_integration_migrations().unwrap();
    let a = Project::new("A");
    let b = Project::new("B");
    store.create_project(&a).unwrap();
    store.create_project(&b).unwrap();
    let job = Job::new(b.id, JobInput::LocalFile("/tmp/unrelated.mp4".to_owned()));
    store.insert_job(&job).unwrap();

    let conn = Connection::open(store.path()).unwrap();
    conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
    let now = now_rfc3339();
    let source = Uuid::new_v4();
    let media = Uuid::new_v4();
    let transcript = Uuid::new_v4();
    conn.execute(
        "INSERT INTO sources(id, project_id, source_type, locator, created_at)
         VALUES(?1, ?2, 'local_file', '/tmp/owned.mp4', ?3)",
        params![source.to_string(), a.id.to_string(), now],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO media(id, source_id, local_path, created_at)
         VALUES(?1, ?2, '/tmp/owned.mp4', ?3)",
        params![media.to_string(), source.to_string(), now],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO transcripts(id, media_id, text, created_at, updated_at)
         VALUES(?1, ?2, 'owned', ?3, ?3)",
        params![transcript.to_string(), media.to_string(), now],
    )
    .unwrap();

    assert!(conn
        .execute(
            "INSERT INTO job_transcript_links(job_id, transcript_id, linked_at)
             VALUES(?1, ?2, ?3)",
            params![job.id.to_string(), transcript.to_string(), now],
        )
        .is_err());
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM job_transcript_links", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(count, 0);
}

#[test]
fn parent_reassignments_cannot_break_existing_project_lineage() {
    let temp = TempDir::new().unwrap();
    let store = SqliteStore::open(temp.path().join("scriptotar.sqlite3")).unwrap();
    store.run_integration_migrations().unwrap();
    let a = Project::new("Parent A");
    let b = Project::new("Parent B");
    store.create_project(&a).unwrap();
    store.create_project(&b).unwrap();
    let job = Job::new(a.id, JobInput::LocalFile("/tmp/owned.mp4".to_owned()));
    store.insert_job(&job).unwrap();

    let creator = Uuid::new_v4();
    let source_a = Uuid::new_v4();
    let source_b = Uuid::new_v4();
    let media_a = Uuid::new_v4();
    let media_b = Uuid::new_v4();
    let transcript_a = Uuid::new_v4();
    let transcript_b = Uuid::new_v4();
    let now = now_rfc3339();
    let conn = Connection::open(store.path()).unwrap();
    conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
    conn.execute(
        "INSERT INTO creators(id, project_id, platform, profile_url, created_at)
         VALUES(?1, ?2, 'youtube', 'https://example.invalid/owner', ?3)",
        params![creator.to_string(), a.id.to_string(), now],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO research_items(
             id, project_id, creator_id, source_url, platform, scanned_at
         ) VALUES(?1, ?2, ?3, 'https://example.invalid/owner/video', 'youtube', ?4)",
        params![
            Uuid::new_v4().to_string(),
            a.id.to_string(),
            creator.to_string(),
            now
        ],
    )
    .unwrap();
    for (id, project_id, locator) in [
        (source_a, a.id, "/tmp/owned.mp4"),
        (source_b, b.id, "/tmp/other.mp4"),
    ] {
        conn.execute(
            "INSERT INTO sources(id, project_id, source_type, locator, created_at)
             VALUES(?1, ?2, 'local_file', ?3, ?4)",
            params![id.to_string(), project_id.to_string(), locator, now],
        )
        .unwrap();
    }
    for (id, source_id, transcript_id) in [
        (media_a, source_a, transcript_a),
        (media_b, source_b, transcript_b),
    ] {
        conn.execute(
            "INSERT INTO media(id, source_id, local_path, created_at)
             VALUES(?1, ?2, '/tmp/owned.mp4', ?3)",
            params![id.to_string(), source_id.to_string(), now],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO transcripts(id, media_id, text, created_at, updated_at)
             VALUES(?1, ?2, 'owned', ?3, ?3)",
            params![transcript_id.to_string(), id.to_string(), now],
        )
        .unwrap();
    }
    conn.execute(
        "INSERT INTO analyses(id, project_id, transcript_id, kind, content, created_at)
         VALUES(?1, ?2, ?3, 'summary', 'keep same project', ?4)",
        params![
            Uuid::new_v4().to_string(),
            a.id.to_string(),
            transcript_a.to_string(),
            now
        ],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO job_transcript_links(job_id, transcript_id, linked_at)
         VALUES(?1, ?2, ?3)",
        params![job.id.to_string(), transcript_a.to_string(), now],
    )
    .unwrap();

    for (sql, row_id, new_value) in [
        (
            "UPDATE creators SET project_id = ?1 WHERE id = ?2",
            creator,
            b.id,
        ),
        (
            "UPDATE sources SET project_id = ?1 WHERE id = ?2",
            source_a,
            b.id,
        ),
        (
            "UPDATE media SET source_id = ?1 WHERE id = ?2",
            media_a,
            source_b,
        ),
        (
            "UPDATE transcripts SET media_id = ?1 WHERE id = ?2",
            transcript_a,
            media_b,
        ),
        (
            "UPDATE jobs SET project_id = ?1 WHERE id = ?2",
            job.id,
            b.id,
        ),
    ] {
        assert!(
            conn.execute(sql, params![new_value.to_string(), row_id.to_string()])
                .is_err(),
            "reassignment should fail without corrupting stored project links: {sql}"
        );
    }
    let project: String = conn
        .query_row(
            "SELECT project_id FROM sources WHERE id = ?1",
            params![source_a.to_string()],
            |row| row.get(0),
        )
        .unwrap();
    // Valid same-project rewiring must remain possible on a non-destructive v5 schema.
    let source_peer = Uuid::new_v4();
    conn.execute(
        "INSERT INTO sources(id, project_id, source_type, locator, created_at)
         VALUES(?1, ?2, 'local_file', '/tmp/peer.mp4', ?3)",
        params![source_peer.to_string(), a.id.to_string(), now],
    )
    .unwrap();
    conn.execute(
        "UPDATE media SET source_id = ?1 WHERE id = ?2",
        params![source_peer.to_string(), media_a.to_string()],
    )
    .unwrap();
    let media_peer = Uuid::new_v4();
    conn.execute(
        "INSERT INTO media(id, source_id, local_path, created_at)
         VALUES(?1, ?2, '/tmp/peer.mp4', ?3)",
        params![media_peer.to_string(), source_a.to_string(), now],
    )
    .unwrap();
    conn.execute(
        "UPDATE transcripts SET media_id = ?1 WHERE id = ?2",
        params![media_peer.to_string(), transcript_a.to_string()],
    )
    .unwrap();
    // An unlinked transcript can move independently without invalidating a child link.
    conn.execute(
        "UPDATE transcripts SET media_id = ?1 WHERE id = ?2",
        params![media_a.to_string(), transcript_b.to_string()],
    )
    .unwrap();

    // Unlinked records are free to move to another existing project.
    let free_creator = Uuid::new_v4();
    conn.execute(
        "INSERT INTO creators(id, project_id, platform, profile_url, created_at)
         VALUES(?1, ?2, 'youtube', 'https://example.invalid/unlinked', ?3)",
        params![free_creator.to_string(), a.id.to_string(), now],
    )
    .unwrap();
    conn.execute(
        "UPDATE creators SET project_id = ?1 WHERE id = ?2",
        params![b.id.to_string(), free_creator.to_string()],
    )
    .unwrap();
    let free_source = Uuid::new_v4();
    conn.execute(
        "INSERT INTO sources(id, project_id, source_type, locator, created_at)
         VALUES(?1, ?2, 'local_file', '/tmp/unlinked.mp4', ?3)",
        params![free_source.to_string(), a.id.to_string(), now],
    )
    .unwrap();
    conn.execute(
        "UPDATE sources SET project_id = ?1 WHERE id = ?2",
        params![b.id.to_string(), free_source.to_string()],
    )
    .unwrap();
    let free_job = Job::new(a.id, JobInput::LocalFile("/tmp/free.mp4".to_owned()));
    store.insert_job(&free_job).unwrap();
    conn.execute(
        "UPDATE jobs SET project_id = ?1 WHERE id = ?2",
        params![b.id.to_string(), free_job.id.to_string()],
    )
    .unwrap();
    assert_eq!(store.get_job(free_job.id).unwrap().project_id, b.id);

    assert_eq!(project, a.id.to_string());
    assert_eq!(store.get_job(job.id).unwrap().project_id, a.id);
    assert_eq!(
        store
            .list_job_transcript_links(Some(a.id))
            .unwrap()
            .get(&job.id),
        Some(&transcript_a)
    );
}

#[test]
fn historical_v4_to_v5_upgrade_preserves_project_jobs_and_settings() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("historical-v4.sqlite3");
    let store = SqliteStore::open(&path).unwrap();
    store.run_integration_migrations().unwrap();
    let project = Project::new("Before upgrade");
    store.create_project(&project).unwrap();
    let job = Job::new(
        project.id,
        JobInput::LocalFile("/tmp/keep-v4.mp4".to_owned()),
    );
    store.insert_job(&job).unwrap();
    let mut settings = store.load_settings().unwrap();
    settings.active_project_id = Some(project.id);
    store.save_settings(&settings).unwrap();

    // Reconstruct the exact v4 schema shape from v5 by undoing only v5's
    // additive triggers and ledger rows. Existing project/job data remains untouched.
    let connection = Connection::open(&path).unwrap();
    connection
        .execute_batch(
            "DROP TRIGGER creators_project_reassignment_guard;
             DROP TRIGGER sources_project_reassignment_guard;
             DROP TRIGGER media_source_reassignment_guard;
             DROP TRIGGER transcripts_media_reassignment_guard;
             DROP TRIGGER jobs_project_lineage_reassignment_guard;
             DROP TRIGGER sources_project_lineage_reassignment_guard;
             DROP TRIGGER media_source_lineage_reassignment_guard;
             DROP TRIGGER transcripts_media_lineage_reassignment_guard;
             DELETE FROM schema_migrations WHERE version = 5;
             DELETE FROM integration_schema_migrations WHERE version = 5;
             PRAGMA user_version = 4;",
        )
        .unwrap();
    drop(connection);
    drop(store);

    let upgraded = SqliteStore::open(&path).unwrap();
    upgraded.run_integration_migrations().unwrap();
    upgraded.run_integration_migrations().unwrap();
    assert_eq!(upgraded.schema_version().unwrap(), 5);
    assert_eq!(upgraded.get_project(project.id).unwrap(), project);
    assert_eq!(upgraded.get_job(job.id).unwrap().project_id, project.id);
    assert_eq!(
        upgraded.load_settings().unwrap().active_project_id,
        Some(project.id)
    );
    let connection = Connection::open(&path).unwrap();
    let count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM integration_schema_migrations WHERE version = 5",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 1);
}
