use std::fs;

use rusqlite::Connection;
use scriptotar_core::{
    now_rfc3339, ContentRepository, Job, JobInput, JobRepository, JobState, Media, Project,
    ProjectRepository, Source, SourceType, Transcript,
};
use scriptotar_db::SqliteStore;
use tempfile::TempDir;
use uuid::Uuid;

#[test]
fn completed_transcript_survives_database_reopen() {
    let temp = TempDir::new().unwrap();
    let database = temp.path().join("scriptotar.sqlite3");
    let project_id;
    let transcript_id;

    {
        let store = SqliteStore::open(&database).unwrap();
        store.run_integration_migrations().unwrap();
        let project = Project::new("Durability");
        project_id = project.id;
        store.create_project(&project).unwrap();

        let job = Job::new(
            project.id,
            JobInput::LocalFile(temp.path().join("clip.mp4").to_string_lossy().into_owned()),
        );
        store.insert_job(&job).unwrap();
        store.transition_job(job.id, JobState::Preparing).unwrap();
        store
            .transition_job(job.id, JobState::Transcribing)
            .unwrap();
        store.transition_job(job.id, JobState::Processing).unwrap();

        let now = now_rfc3339();
        let source = Source {
            id: Uuid::new_v4(),
            project_id: project.id,
            creator_id: None,
            source_type: SourceType::LocalFile,
            locator: "/tmp/clip.mp4".to_owned(),
            title: Some("Durable transcript".to_owned()),
            created_at: now.clone(),
        };
        let media = Media {
            id: Uuid::new_v4(),
            source_id: source.id,
            local_path: "/tmp/clip.mp4".to_owned(),
            duration_seconds: Some(2.0),
            mime_type: Some("video/mp4".to_owned()),
            created_at: now.clone(),
        };
        let transcript = Transcript {
            id: Uuid::new_v4(),
            media_id: media.id,
            language: Some("en".to_owned()),
            text: "survives reopen".to_owned(),
            segments_json: Some("[]".to_owned()),
            words_json: Some("[]".to_owned()),
            created_at: now.clone(),
            updated_at: now,
        };
        transcript_id = transcript.id;
        let completed = store
            .persist_transcription(job.id, &source, &media, &transcript)
            .unwrap();
        assert_eq!(completed.state, JobState::Completed);
    }

    let reopened = SqliteStore::open(&database).unwrap();
    reopened.run_integration_migrations().unwrap();
    let transcripts = reopened.list_transcripts(Some(project_id)).unwrap();
    assert_eq!(transcripts.len(), 1);
    assert_eq!(transcripts[0].transcript.id, transcript_id);
    assert_eq!(transcripts[0].transcript.text, "survives reopen");
}

#[test]
fn legacy_import_preserves_source_bytes_and_is_repeatable() {
    let temp = TempDir::new().unwrap();
    let legacy_path = temp.path().join("history.sqlite3");
    let legacy = Connection::open(&legacy_path).unwrap();
    legacy
        .execute_batch(
            "CREATE TABLE projects(name TEXT PRIMARY KEY, created_at TEXT NOT NULL);
             CREATE TABLE jobs(
                id TEXT PRIMARY KEY, created_at TEXT NOT NULL, source TEXT NOT NULL,
                input_type TEXT NOT NULL, title TEXT, status TEXT NOT NULL, language TEXT,
                output_dir TEXT, transcript TEXT, error TEXT, project TEXT NOT NULL
             );
             INSERT INTO projects VALUES('Inbox','2026-08-09 00:00:00');
             INSERT INTO jobs VALUES(
                'legacy-job','2026-08-09 00:00:00','/tmp/legacy.mp4','file',
                'Legacy','Done','en','/tmp/out','legacy transcript',NULL,'Inbox'
             );",
        )
        .unwrap();
    drop(legacy);

    let before = fs::read(&legacy_path).unwrap();
    let store = SqliteStore::open(temp.path().join("scriptotar.sqlite3")).unwrap();
    store.run_integration_migrations().unwrap();

    let first = store.import_legacy_database(&legacy_path).unwrap();
    assert!(!first.skipped);
    let backup_path = first.backup_path.expect("legacy backup path");
    assert_eq!(fs::read(&legacy_path).unwrap(), before);
    assert_eq!(fs::read(&backup_path).unwrap(), before);

    let second = store.import_legacy_database(&legacy_path).unwrap();
    assert!(second.skipped);
    assert_eq!(fs::read(&legacy_path).unwrap(), before);
    assert_eq!(store.list_transcripts(None).unwrap().len(), 1);
}

#[test]
fn integration_migrations_support_fresh_create_and_reopen() {
    let temp = TempDir::new().unwrap();
    let database = temp.path().join("scriptotar.sqlite3");

    {
        let store = SqliteStore::open(&database).unwrap();
        store.run_integration_migrations().unwrap();
        store.run_integration_migrations().unwrap();
    }

    let reopened = SqliteStore::open(&database).unwrap();
    reopened.run_integration_migrations().unwrap();
    assert!(reopened.schema_version().unwrap() >= 1);
}
