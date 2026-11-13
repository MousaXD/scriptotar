use std::{
    sync::{Arc, Barrier},
    thread,
};

use rusqlite::{params, Connection, OptionalExtension};
use scriptotar_core::{
    CreativeBriefInput, EvidenceCertainty, EvidenceInput, Job, JobInput, JobRepository, Project,
    ProjectRepository, RepositoryError, StudyEdit, StudySourceInput, StudySourceKind, StudyStatus,
};
use scriptotar_db::SqliteStore;
use tempfile::TempDir;
use uuid::Uuid;

fn fixture() -> (TempDir, SqliteStore, Project, Project) {
    let dir = TempDir::new().unwrap();
    let db = SqliteStore::open(dir.path().join("scriptotar.sqlite")).unwrap();
    db.run_integration_migrations().unwrap();
    let a = Project::new("Studio A");
    let b = Project::new("Studio B");
    db.create_project(&a).unwrap();
    db.create_project(&b).unwrap();
    (dir, db, a, b)
}

fn manual(text: &str) -> StudySourceInput {
    StudySourceInput {
        kind: StudySourceKind::Manual,
        reference_id: None,
        locator: Some("Creator notes page 1".into()),
        manual_text: Some(text.into()),
        source_revision: None,
        uncertainty: Some("unverified user note".into()),
    }
}

fn brief() -> CreativeBriefInput {
    CreativeBriefInput {
        audience: "UAE creator".into(),
        goal: "Three original Arabic hooks".into(),
        language: "ar".into(),
        region: Some("UAE".into()),
        platform: "reels".into(),
        tone: "informative".into(),
        duration_seconds: Some(40),
        objective: "educate".into(),
        cta: Some("learn more".into()),
        constraints: vec!["do not claim guaranteed reach".into()],
    }
}

fn seed_existing_origins(db: &SqliteStore, project: &Project) -> (Uuid, Uuid, Uuid) {
    let research = Uuid::new_v4();
    let source = Uuid::new_v4();
    let media = Uuid::new_v4();
    let transcript = Uuid::new_v4();
    let conn = Connection::open(db.path()).unwrap();
    conn.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
    conn.execute(
        "INSERT INTO research_items (id,project_id,source_url,platform,scanned_at)
         VALUES (?1,?2,'https://example.org/reel/1','sample','2026-11-13T10:00:00Z')",
        params![research.to_string(), project.id.to_string()],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO sources (id,project_id,creator_id,source_type,locator,created_at)
         VALUES (?1,?2,NULL,'url','https://example.org/reel/1','2026-11-13T10:00:00Z')",
        params![source.to_string(), project.id.to_string()],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO media(id,source_id,local_path,created_at)
         VALUES(?1,?2,'/tmp/study-reel.mp4','2026-11-13T10:00:00Z')",
        params![media.to_string(), source.to_string()],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO transcripts(id,media_id,text,created_at,updated_at)
         VALUES (?1,?2,'An example transcript','2026-11-13T10:00:00Z','2026-11-13T10:00:00Z')",
        params![transcript.to_string(), media.to_string()],
    )
    .unwrap();
    (research, media, transcript)
}

#[test]
fn study_multi_source_brief_evidence_drafts_survive_reopen_and_origin_deletion() {
    let (temp, db, project, _) = fixture();
    let (research_id, media_id, transcript_id) = seed_existing_origins(&db, &project);
    let study = db
        .create_study(project.id, "Arabic Reels", "Study three examples")
        .unwrap();
    let note = db
        .add_study_source(
            project.id,
            study.id,
            &manual("A manual creative observation"),
        )
        .unwrap();
    let research = db
        .add_study_source(
            project.id,
            study.id,
            &StudySourceInput {
                kind: StudySourceKind::ResearchItem,
                reference_id: Some(research_id),
                locator: None,
                manual_text: None,
                source_revision: Some("captured 2026-11-13".into()),
                uncertainty: None,
            },
        )
        .unwrap();
    let transcript = db
        .add_study_source(
            project.id,
            study.id,
            &StudySourceInput {
                kind: StudySourceKind::Transcript,
                reference_id: Some(transcript_id),
                locator: None,
                manual_text: None,
                source_revision: None,
                uncertainty: None,
            },
        )
        .unwrap();
    let media = db
        .add_study_source(
            project.id,
            study.id,
            &StudySourceInput {
                kind: StudySourceKind::Media,
                reference_id: Some(media_id),
                locator: None,
                manual_text: None,
                source_revision: None,
                uncertainty: None,
            },
        )
        .unwrap();
    assert_eq!(
        db.list_study_sources(project.id, study.id).unwrap().len(),
        4
    );
    assert_eq!(media.reference_id, Some(media_id));

    let evidence = db
        .add_evidence_reference(
            project.id,
            study.id,
            &EvidenceInput {
                study_source_id: transcript.id,
                claim: "The opening line contains a question".into(),
                locator: "transcript:0-3.5s".into(),
                timestamp_seconds: Some(1.2),
                certainty: EvidenceCertainty::Unverified,
                uncertainty_note: Some("transcript-only; no video frames reviewed".into()),
            },
        )
        .unwrap();
    assert_eq!(evidence.input.study_source_id, transcript.id);

    let draft = db
        .create_draft(project.id, study.id, "Angle 1", "First script")
        .unwrap();
    db.attach_source_to_draft(project.id, draft.id, research.id)
        .unwrap();
    db.attach_source_to_draft(project.id, draft.id, transcript.id)
        .unwrap();
    let updated = db
        .update_draft(project.id, draft.id, 1, "Angle 1 revised", "Second script")
        .unwrap();
    assert_eq!(updated.revision, 2);
    assert!(matches!(
        db.update_draft(project.id, draft.id, 1, "stale", "overwrite"),
        Err(RepositoryError::Conflict(_))
    ));
    let saved = db
        .save_creative_brief(project.id, study.id, 0, &brief())
        .unwrap();
    assert_eq!(saved.revision, 1);
    assert!(matches!(
        db.save_creative_brief(project.id, study.id, 0, &brief()),
        Err(RepositoryError::Conflict(_))
    ));
    let v2 = db
        .save_creative_brief(project.id, study.id, 1, &brief())
        .unwrap();
    assert_eq!(v2.revision, 2);

    drop(db);
    let reopened = SqliteStore::open(temp.path().join("scriptotar.sqlite")).unwrap();
    reopened.run_integration_migrations().unwrap();
    assert_eq!(reopened.schema_version().unwrap(), 8);
    assert_eq!(
        reopened.get_study(project.id, study.id).unwrap().title,
        "Arabic Reels"
    );
    assert_eq!(
        reopened
            .list_study_sources(project.id, study.id)
            .unwrap()
            .len(),
        4
    );
    assert_eq!(
        reopened
            .list_evidence_references(project.id, study.id)
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        reopened
            .get_creative_brief(project.id, study.id)
            .unwrap()
            .unwrap()
            .input
            .language,
        "ar"
    );
    assert_eq!(
        reopened.get_draft(project.id, draft.id).unwrap().body,
        "Second script"
    );
    assert_eq!(
        reopened
            .list_draft_versions(project.id, draft.id)
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        reopened
            .list_draft_source_ids(project.id, draft.id)
            .unwrap()
            .len(),
        2
    );

    let conn = Connection::open(reopened.path()).unwrap();
    conn.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
    conn.execute(
        "DELETE FROM transcripts WHERE id=?1",
        [transcript_id.to_string()],
    )
    .unwrap();
    let detached = reopened
        .get_study_source(project.id, study.id, transcript.id)
        .unwrap();
    assert!(!detached.available);
    assert_eq!(detached.reference_id, Some(transcript_id));
    assert_eq!(detached.locator, "https://example.org/reel/1");
    assert_eq!(
        reopened
            .get_evidence_reference(project.id, study.id, evidence.id)
            .unwrap()
            .input
            .certainty,
        EvidenceCertainty::Unverified
    );
    assert!(
        reopened
            .get_study_source(project.id, study.id, note.id)
            .unwrap()
            .available
    );
}

#[test]
fn scope_and_duplicate_guards_cover_apis_and_direct_sql() {
    let (_temp, db, a, b) = fixture();
    let (research_id, _, transcript_id) = seed_existing_origins(&db, &a);
    let study_a = db.create_study(a.id, "A-only", "protected").unwrap();
    let study_b = db.create_study(b.id, "B-only", "protected").unwrap();
    assert!(matches!(
        db.get_study(b.id, study_a.id),
        Err(RepositoryError::NotFound(_))
    ));
    assert!(matches!(
        db.update_study(
            b.id,
            study_a.id,
            1,
            &StudyEdit {
                title: "steal".into(),
                goal: "unsafe".into(),
                status: StudyStatus::Completed
            }
        ),
        Err(RepositoryError::NotFound(_))
    ));

    let s = db
        .add_study_source(a.id, study_a.id, &manual("a unique source"))
        .unwrap();
    assert!(matches!(
        db.add_study_source(a.id, study_a.id, &manual("a unique source")),
        Err(RepositoryError::Conflict(_))
    ));
    let other = StudySourceInput {
        kind: StudySourceKind::ResearchItem,
        reference_id: Some(research_id),
        locator: None,
        manual_text: None,
        source_revision: None,
        uncertainty: None,
    };
    assert!(matches!(
        db.add_study_source(b.id, study_b.id, &other),
        Err(RepositoryError::NotFound(_))
    ));
    assert!(matches!(
        db.get_study_source(b.id, study_b.id, s.id),
        Err(RepositoryError::NotFound(_))
    ));
    assert!(matches!(
        db.add_evidence_reference(
            b.id,
            study_b.id,
            &EvidenceInput {
                study_source_id: s.id,
                claim: "false".into(),
                locator: "line 1".into(),
                timestamp_seconds: None,
                certainty: EvidenceCertainty::Unverified,
                uncertainty_note: None,
            }
        ),
        Err(RepositoryError::NotFound(_))
    ));
    let d = db
        .create_draft(a.id, study_a.id, "secret", "not yours")
        .unwrap();
    assert!(matches!(
        db.get_draft(b.id, d.id),
        Err(RepositoryError::NotFound(_))
    ));
    assert!(matches!(
        db.attach_source_to_draft(b.id, d.id, s.id),
        Err(RepositoryError::NotFound(_))
    ));
    assert!(matches!(
        db.get_creative_brief(b.id, study_a.id),
        Err(RepositoryError::NotFound(_))
    ));

    // Even bypassing typed APIs, SQLite must reject attaching a B study to
    // an A origin or changing its project lineage after association.
    let conn = Connection::open(db.path()).unwrap();
    conn.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
    assert!(
        conn.execute(
            "UPDATE content_studies SET project_id=?1 WHERE id=?2",
            params![b.id.to_string(), study_a.id.to_string()],
        )
        .is_err(),
        "a study must never silently change its owning project"
    );
    let id = Uuid::new_v4();
    let cross_insert = conn.execute(
        "INSERT INTO study_sources (
             id,project_id,study_id,kind,reference_id,transcript_id,locator,
             fingerprint,collected_at,created_at
         ) VALUES (?1,?2,?3,'transcript',?4,?4,'invalid','x','now','now')",
        params![
            id.to_string(),
            b.id.to_string(),
            study_b.id.to_string(),
            transcript_id.to_string()
        ],
    );
    assert!(cross_insert.is_err());
    let a_source = db
        .add_study_source(
            a.id,
            study_a.id,
            &StudySourceInput {
                kind: StudySourceKind::Transcript,
                reference_id: Some(transcript_id),
                locator: None,
                manual_text: None,
                source_revision: None,
                uncertainty: None,
            },
        )
        .unwrap();
    assert!(a_source.available);
    assert!(conn
        .execute(
            "UPDATE sources SET project_id=?1 WHERE id=(
           SELECT source_id FROM media WHERE id=(SELECT media_id FROM transcripts WHERE id=?2)
         )",
            params![b.id.to_string(), transcript_id.to_string()],
        )
        .is_err());
}

#[test]
fn job_and_draft_attachment_scopes_and_immutability() {
    let (_temp, db, a, b) = fixture();
    let study = db.create_study(a.id, "Pipeline", "Job lineage").unwrap();
    let source = db
        .add_study_source(a.id, study.id, &manual("Linked source"))
        .unwrap();
    let job_a = Job::new(a.id, JobInput::Url("https://example.org/reel/3".into()));
    let job_b = Job::new(b.id, JobInput::Url("https://example.org/reel/4".into()));
    db.insert_job(&job_a).unwrap();
    db.insert_job(&job_b).unwrap();
    db.attach_job_to_study_source(a.id, study.id, source.id, job_a.id)
        .unwrap();
    assert!(matches!(
        db.attach_job_to_study_source(a.id, study.id, source.id, job_b.id),
        Err(RepositoryError::NotFound(_))
    ));
    assert_eq!(
        db.list_study_source_job_ids(a.id, study.id, source.id)
            .unwrap(),
        vec![job_a.id]
    );

    let draft = db.create_draft(a.id, study.id, "Ready", "v1").unwrap();
    db.attach_source_to_draft(a.id, draft.id, source.id)
        .unwrap();
    let duplicate = db.duplicate_draft(a.id, draft.id).unwrap();
    assert_eq!(duplicate.body, "v1");
    assert_eq!(
        db.list_draft_source_ids(a.id, duplicate.id).unwrap(),
        vec![source.id]
    );
    let version = db.list_draft_versions(a.id, draft.id).unwrap().remove(0);
    let conn = Connection::open(db.path()).unwrap();
    conn.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
    assert!(conn
        .execute(
            "UPDATE draft_versions SET body='overwritten' WHERE id=?1",
            [version.id.to_string()],
        )
        .is_err());
    assert!(conn
        .execute(
            "DELETE FROM draft_versions WHERE id=?1",
            [version.id.to_string()],
        )
        .is_err());
    assert!(conn
        .execute(
            "INSERT INTO draft_source_links(project_id,draft_id,study_source_id,attached_at)
         VALUES(?1,?2,?3,'now')",
            params![
                b.id.to_string(),
                duplicate.id.to_string(),
                source.id.to_string()
            ],
        )
        .is_err());

    db.archive_draft(a.id, draft.id).unwrap();
    assert!(matches!(
        db.update_draft(a.id, draft.id, 2, "stale", "bad"),
        Err(RepositoryError::Conflict(_))
    ));
    db.restore_draft(a.id, draft.id).unwrap();
    assert_eq!(db.list_draft_versions(a.id, draft.id).unwrap().len(), 3);
    db.archive_study(a.id, study.id).unwrap();
    assert_eq!(db.search_studies(a.id, "", false, 20).unwrap().len(), 0);
    assert_eq!(db.search_studies(a.id, "", true, 20).unwrap().len(), 1);
    assert!(db.create_draft(a.id, study.id, "bad", "").is_err());
    db.restore_study(a.id, study.id).unwrap();
    assert_eq!(db.get_study(a.id, study.id).unwrap().archived_at, None);
}

#[test]
fn simultaneous_draft_saves_keep_one_winner_and_full_history() {
    let (_temp, db, project, _) = fixture();
    let study = db.create_study(project.id, "Race", "edit").unwrap();
    let draft = db
        .create_draft(project.id, study.id, "Initial", "")
        .unwrap();
    let barrier = Arc::new(Barrier::new(3));
    let mut tasks = Vec::new();
    for n in 0..2 {
        let store = db.clone();
        let barrier = barrier.clone();
        tasks.push(thread::spawn(move || {
            barrier.wait();
            store.update_draft(project.id, draft.id, 1, &format!("Draft {n}"), "content")
        }));
    }
    barrier.wait();
    let results = tasks
        .into_iter()
        .map(|task| task.join().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter(|r| matches!(r, Err(RepositoryError::Conflict(_))))
            .count(),
        1
    );
    assert_eq!(
        db.list_draft_versions(project.id, draft.id).unwrap().len(),
        2
    );
}

#[test]
fn deleting_project_cascades_study_history_without_orphans() {
    let (_temp, db, project, _) = fixture();
    let study = db.create_study(project.id, "Delete", "clean").unwrap();
    let src = db
        .add_study_source(project.id, study.id, &manual("source"))
        .unwrap();
    let draft = db
        .create_draft(project.id, study.id, "script", "body")
        .unwrap();
    db.attach_source_to_draft(project.id, draft.id, src.id)
        .unwrap();
    db.add_evidence_reference(
        project.id,
        study.id,
        &EvidenceInput {
            study_source_id: src.id,
            claim: "claim".into(),
            locator: "page:1".into(),
            timestamp_seconds: None,
            certainty: EvidenceCertainty::Unverified,
            uncertainty_note: None,
        },
    )
    .unwrap();
    let conn = Connection::open(db.path()).unwrap();
    conn.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
    conn.execute("DELETE FROM projects WHERE id=?1", [project.id.to_string()])
        .unwrap();
    let check: String = conn
        .query_row("PRAGMA foreign_key_check", [], |row| row.get(0))
        .optional()
        .unwrap_or(None)
        .unwrap_or_default();
    assert!(check.is_empty());
    for table in [
        "content_studies",
        "study_sources",
        "study_evidence",
        "creative_drafts",
        "draft_versions",
        "draft_source_links",
    ] {
        let count: i64 = conn
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(count, 0, "orphaned records in {table}");
    }
}

#[test]
fn scriptotar_141_core_v5_database_upgrades_without_losing_projects_or_jobs() {
    let (temp, store, project, _) = fixture();
    let job = Job::new(project.id, JobInput::LocalFile("/tmp/legacy-v5.mp4".into()));
    store.insert_job(&job).unwrap();
    drop(store);

    // Strip only the unmerged v6 additions to model a genuine 1.4.1 v5 file.
    // A fake v5 user_version over an existing v6 schema is not a valid fixture.
    let path = temp.path().join("scriptotar.sqlite");
    let conn = Connection::open(&path).unwrap();
    let mut stmt = conn
        .prepare(
            "SELECT name FROM sqlite_master WHERE type='trigger'
         AND (name LIKE 'study_%' OR name LIKE 'creative_%' OR name LIKE 'draft_%')",
        )
        .unwrap();
    let names = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    drop(stmt);
    for trigger in names {
        conn.execute_batch(&format!("DROP TRIGGER \"{trigger}\";"))
            .unwrap();
    }
    conn.execute_batch(
        "DROP TABLE agent_effects;
         DELETE FROM schema_migrations WHERE version=8;
         DROP TABLE agent_audit_events;
         DROP TABLE agent_runs;
         DELETE FROM schema_migrations WHERE version=7;
         DROP TABLE draft_source_links;
         DROP TABLE study_source_jobs;
         DROP TABLE draft_versions;
         DROP TABLE creative_drafts;
         DROP TABLE study_briefs;
         DROP TABLE study_evidence;
         DROP TABLE study_sources;
         DROP TABLE content_studies;
         DELETE FROM schema_migrations WHERE version = 6;
         PRAGMA user_version = 5;",
    )
    .unwrap();
    assert_eq!(
        conn.query_row("PRAGMA user_version", [], |row| row.get::<_, i64>(0))
            .unwrap(),
        5
    );
    drop(conn);

    let upgraded = SqliteStore::open(&path).unwrap();
    upgraded.run_integration_migrations().unwrap();
    assert_eq!(upgraded.schema_version().unwrap(), 8);
    assert_eq!(upgraded.get_project(project.id).unwrap(), project);
    assert_eq!(upgraded.get_job(job.id).unwrap().project_id, project.id);
    let study = upgraded
        .create_study(project.id, "Fresh", "after upgrade")
        .unwrap();
    assert_eq!(
        upgraded.get_study(project.id, study.id).unwrap().title,
        "Fresh"
    );
    let conn = Connection::open(&path).unwrap();
    assert_eq!(
        conn.query_row(
            "SELECT COUNT(*) FROM schema_migrations WHERE version=6",
            [],
            |row| row.get::<_, i64>(0)
        )
        .unwrap(),
        1
    );
}
