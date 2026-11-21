use std::{
    collections::BTreeMap,
    sync::{Arc, Barrier},
    thread,
};

use rusqlite::{params, types::Value, Connection, OpenFlags, OptionalExtension};
use scriptotar_core::{
    CreativeBriefInput, EvidenceCertainty, EvidenceInput, Job, JobInput, JobRepository, Project,
    ProjectRepository, RepositoryError, StudyEdit, StudySourceInput, StudySourceKind, StudyStatus,
};
use scriptotar_db::{SqliteStore, StudioBundleUpdate, LATEST_SCHEMA_VERSION};
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

/// Entire database row-content snapshot through a read-only SQLite connection.
/// This includes provider approvals, effects, attempts, receipts and audit tables.
fn whole_database_snapshot(db: &SqliteStore) -> BTreeMap<String, Vec<String>> {
    let connection = Connection::open_with_flags(db.path(), OpenFlags::SQLITE_OPEN_READ_ONLY)
        .expect("snapshot must never open the database for writing");
    let tables = {
        let mut statement = connection
            .prepare(
                "SELECT name FROM sqlite_master WHERE type='table'
             AND name NOT GLOB 'sqlite_*' ORDER BY name",
            )
            .unwrap();
        let rows = statement
            .query_map([], |row| row.get::<_, String>(0))
            .unwrap();
        rows.collect::<rusqlite::Result<Vec<_>>>().unwrap()
    };
    let mut snapshot = BTreeMap::new();
    for name in tables {
        let sql = format!("SELECT * FROM \"{}\"", name.replace('"', "\"\""));
        let mut statement = connection.prepare(&sql).unwrap();
        let rows = statement
            .query_map([], |row| {
                (0..row.as_ref().column_count())
                    .map(|column| row.get::<_, Value>(column))
                    .collect::<rusqlite::Result<Vec<_>>>()
            })
            .unwrap();
        let mut contents = rows
            .map(|row| format!("{:?}", row.unwrap()))
            .collect::<Vec<_>>();
        contents.sort();
        snapshot.insert(name, contents);
    }
    snapshot
}

#[test]
fn revoke_study_source_access_preserves_provenance_and_is_atomic_on_denial() {
    let (_directory, db, project_a, project_b) = fixture();
    let (_research, _media, transcript_id) = seed_existing_origins(&db, &project_a);
    let study = db
        .create_study(project_a.id, "Protected source", "Read provenance")
        .unwrap();
    let source = db
        .add_study_source(
            project_a.id,
            study.id,
            &StudySourceInput {
                kind: StudySourceKind::Transcript,
                reference_id: Some(transcript_id),
                locator: None,
                manual_text: None,
                source_revision: Some("original-transcript-v1".into()),
                uncertainty: None,
            },
        )
        .unwrap();
    let evidence = db
        .add_evidence_reference(
            project_a.id,
            study.id,
            &EvidenceInput {
                study_source_id: source.id,
                claim: "Historical observation remains unverified".into(),
                locator: "0:03".into(),
                timestamp_seconds: Some(3.0),
                certainty: EvidenceCertainty::Unverified,
                uncertainty_note: None,
            },
        )
        .unwrap();
    let draft = db
        .create_draft(project_a.id, study.id, "Original", "Immutable body")
        .unwrap();
    db.attach_source_to_draft(project_a.id, draft.id, source.id)
        .unwrap();
    let before = db.get_study(project_a.id, study.id).unwrap();
    let before_source = db
        .get_study_source(project_a.id, study.id, source.id)
        .unwrap();
    let before_draft = db.get_draft(project_a.id, draft.id).unwrap();
    let all_rows_before = whole_database_snapshot(&db);

    // Wrong-project, forged-reference and stale revision attempts do not
    // consume the CAS token or change any persisted row.
    assert!(db
        .revoke_study_source_access(
            project_b.id,
            study.id,
            before.revision,
            StudySourceKind::Transcript,
            transcript_id
        )
        .is_err());
    assert!(db
        .revoke_study_source_access(
            project_a.id,
            study.id,
            before.revision,
            StudySourceKind::Transcript,
            Uuid::new_v4()
        )
        .is_err());
    assert!(matches!(
        db.revoke_study_source_access(
            project_a.id,
            study.id,
            before.revision + 5,
            StudySourceKind::Transcript,
            transcript_id
        ),
        Err(RepositoryError::Conflict(_))
    ));
    assert_eq!(db.get_study(project_a.id, study.id).unwrap(), before);
    assert_eq!(
        db.get_study_source(project_a.id, study.id, source.id)
            .unwrap(),
        before_source
    );
    assert_eq!(db.get_draft(project_a.id, draft.id).unwrap(), before_draft);
    assert_eq!(
        whole_database_snapshot(&db),
        all_rows_before,
        "denied operations must not change any canonical approval/effect/audit row"
    );

    db.revoke_study_source_access(
        project_a.id,
        study.id,
        before.revision,
        StudySourceKind::Transcript,
        transcript_id,
    )
    .unwrap();
    let revoked = db
        .get_study_source(project_a.id, study.id, source.id)
        .unwrap();
    assert!(!revoked.available);
    assert_eq!(revoked.id, source.id);
    assert_eq!(revoked.reference_id, Some(transcript_id));
    assert_eq!(revoked.locator, source.locator);
    assert_eq!(revoked.fingerprint, source.fingerprint);
    assert_eq!(revoked.source_revision, source.source_revision);
    assert_eq!(
        db.get_study(project_a.id, study.id).unwrap().revision,
        before.revision + 1
    );
    assert!(db
        .attached_transcript_byte_length(project_a.id, study.id, transcript_id)
        .is_err());
    assert_eq!(db.get_draft(project_a.id, draft.id).unwrap(), before_draft);
    assert_eq!(
        db.list_evidence_references(project_a.id, study.id).unwrap()[0].id,
        evidence.id
    );
    let connection = Connection::open(db.path()).unwrap();
    let original_origin: i64 = connection
        .query_row(
            "SELECT count(*) FROM transcripts WHERE id=?1",
            params![transcript_id.to_string()],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        original_origin, 1,
        "project-owned original must not be deleted"
    );
    let all_rows_revoked = whole_database_snapshot(&db);
    for (table, previous) in &all_rows_before {
        if table != "content_studies" && table != "study_sources" {
            assert_eq!(
                all_rows_revoked.get(table),
                Some(previous),
                "authorized source revocation must preserve all historical {table} rows"
            );
        }
    }
    // Even a current-revision editor write cannot reattach the tombstoned
    // origin; the unique provenance fingerprint makes resurrection fail closed.
    let desired = [(StudySourceKind::Transcript, transcript_id)];
    let edit = StudyEdit {
        title: "Protected source".into(),
        goal: "Read provenance".into(),
        status: StudyStatus::Active,
    };
    let saved_brief = brief();
    assert!(db
        .save_studio_bundle(
            project_a.id,
            study.id,
            StudioBundleUpdate {
                expected_revision: before.revision + 1,
                edit: &edit,
                brief: &saved_brief,
                brief_changed: false,
                sources: &desired,
                draft: &before_draft.body,
                draft_id: Some(draft.id),
                expected_draft_revision: Some(before_draft.revision),
            },
        )
        .is_err());
    assert_eq!(
        whole_database_snapshot(&db),
        all_rows_revoked,
        "a failed source reattachment must roll back every canonical row"
    );

    // Revocation is not reversible by a replay or a stale new Studio save.
    assert!(db
        .revoke_study_source_access(
            project_a.id,
            study.id,
            before.revision + 1,
            StudySourceKind::Transcript,
            transcript_id
        )
        .is_err());
    assert_eq!(
        db.get_study(project_a.id, study.id).unwrap().revision,
        before.revision + 1
    );
    assert_eq!(
        db.get_study_source(project_a.id, study.id, source.id)
            .unwrap(),
        revoked
    );
    assert_eq!(whole_database_snapshot(&db), all_rows_revoked);

    // A revoked historical source ID must never authorize NEW evidence or
    // source links, including links on a newly created draft. Keep existing
    // evidence and the old draft's historical links fully readable.
    let fresh_draft = db
        .create_draft(
            project_a.id,
            study.id,
            "Post-revocation draft",
            "Independent new draft",
        )
        .unwrap();
    let before_denials = whole_database_snapshot(&db);
    let denied_evidence = db.add_evidence_reference(
        project_a.id,
        study.id,
        &EvidenceInput {
            study_source_id: source.id,
            claim: "No new evidence after permission withdrawal".into(),
            locator: "0:08".into(),
            timestamp_seconds: Some(8.0),
            certainty: EvidenceCertainty::Unverified,
            uncertainty_note: None,
        },
    );
    assert!(
        denied_evidence.is_err(),
        "revoked origin gained new evidence"
    );
    assert!(
        db.attach_source_to_draft(project_a.id, fresh_draft.id, source.id)
            .is_err(),
        "revoked origin gained new draft source link"
    );
    assert_eq!(
        whole_database_snapshot(&db),
        before_denials,
        "rejected source use changed canonical SQLite row contents"
    );
    assert_eq!(
        db.list_evidence_references(project_a.id, study.id)
            .unwrap()
            .len(),
        1
    );
    assert!(db
        .list_draft_source_ids(project_a.id, fresh_draft.id)
        .unwrap()
        .is_empty());

    // This is an independent attack surface: a real job link, an accepted
    // reviewed-script candidate, or cloning historical immutable draft links
    // must not revive access through the same tombstone.
    let job = Job::new(
        project_a.id,
        JobInput::Url("https://example.com/after-revoke".into()),
    );
    db.insert_job(&job).unwrap();
    let before_other_denials = whole_database_snapshot(&db);
    assert!(
        db.attach_job_to_study_source(project_a.id, study.id, source.id, job.id)
            .is_err(),
        "revoked source gained a new job link"
    );
    assert!(
        db.create_reviewed_study_draft(
            project_a.id,
            study.id,
            before.revision + 1,
            "Forbidden new candidate",
            "Must not save",
            &[source.id]
        )
        .is_err(),
        "revoked source entered a new script candidate"
    );
    assert!(
        db.duplicate_draft(project_a.id, draft.id).is_err(),
        "legacy draft duplication copied a revoked historical link"
    );
    assert!(
        db.duplicate_study_draft_checked(project_a.id, study.id, draft.id, before_draft.revision)
            .is_err(),
        "Studio draft duplication copied revoked provenance"
    );
    assert_eq!(
        whole_database_snapshot(&db),
        before_other_denials,
        "revoked source gained new jobs, draft links or candidate rows"
    );
    assert_eq!(db.get_draft(project_a.id, draft.id).unwrap(), before_draft);
    assert_eq!(
        db.get_study_source(project_a.id, study.id, source.id)
            .unwrap(),
        revoked
    );
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
    assert_eq!(reopened.schema_version().unwrap(), LATEST_SCHEMA_VERSION);
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

// This test module reconstructs old databases from a current schema. New
// additive migrations must be physically removed, not merely hidden with
// PRAGMA user_version, or the version ledger correctly rejects the fixture.
fn remove_v13_from_legacy_fixture(conn: &Connection) {
    let mut statement = conn
        .prepare(
            "SELECT name FROM sqlite_master WHERE type='trigger'
             AND name LIKE 'provider_dispatch_barrier_%'",
        )
        .unwrap();
    let names = statement
        .query_map([], |row| row.get::<_, String>(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    drop(statement);
    for name in names {
        assert!(name.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'_'));
        conn.execute_batch(&format!("DROP TRIGGER {name};"))
            .unwrap();
    }
    conn.execute_batch(
        "DROP TABLE provider_dispatch_barrier;
         DELETE FROM schema_migrations WHERE version=13;",
    )
    .unwrap();
}

#[test]
fn scriptotar_141_core_v5_database_upgrades_without_losing_projects_or_jobs() {
    let (temp, store, project, _) = fixture();
    let job = Job::new(project.id, JobInput::LocalFile("/tmp/legacy-v5.mp4".into()));
    store.insert_job(&job).unwrap();
    drop(store);

    // Strip all post-v5 schema objects to model a genuine 1.4.1 v5 file.
    // A fake v5 user_version over an existing v6 schema is not a valid fixture.
    let path = temp.path().join("scriptotar.sqlite");
    let conn = Connection::open(&path).unwrap();
    remove_v13_from_legacy_fixture(&conn);
    let mut stmt = conn
        .prepare(
            "SELECT name FROM sqlite_master WHERE type='trigger'
         AND (name LIKE 'study_%' OR name LIKE 'creative_%'
           OR name LIKE 'draft_%' OR name LIKE 'evidence_review_%')",
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
        "DROP TRIGGER external_provider_identity_on_insert;
         DROP TRIGGER external_provider_identity_on_update;
         DROP INDEX idx_external_provider_request_unique;
         DROP INDEX idx_external_provider_receipt_unique;
         DROP TABLE agent_external_provider_identities;
         DELETE FROM schema_migrations WHERE version=12;
         DROP TABLE study_writing_revisions;
         DROP TABLE study_writing_batches;
         DELETE FROM schema_migrations WHERE version=11;
         DROP TABLE agent_external_attempt_history;
         DROP TABLE agent_external_effects;
         DELETE FROM schema_migrations WHERE version=10;
         DROP TABLE evidence_reviews;
         DELETE FROM schema_migrations WHERE version=9;
         DROP TABLE agent_effects;
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
    assert_eq!(upgraded.schema_version().unwrap(), LATEST_SCHEMA_VERSION);
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

#[test]
fn draft_candidate_index_loads_only_metadata_and_denies_foreign_project() {
    let (_dir, db, owner, foreign) = fixture();
    let study = db.create_study(owner.id, "Candidate index", "").unwrap();
    let large_body = "An original studio script. ".repeat(30_000);
    let first = db
        .create_draft(owner.id, study.id, "Original", &large_body)
        .unwrap();
    let second = db
        .duplicate_study_draft_checked(owner.id, study.id, first.id, first.revision)
        .unwrap();
    let summaries = db.list_draft_summaries(owner.id, study.id, 200).unwrap();
    assert_eq!(summaries.len(), 2);
    assert!(summaries
        .iter()
        .any(|value| value.id == first.id && value.title == "Original"));
    assert!(summaries
        .iter()
        .any(|value| value.id == second.id && value.title == "Copy of Original"));
    assert_eq!(
        db.list_draft_summaries(owner.id, study.id, 1)
            .unwrap()
            .len(),
        1
    );
    assert!(db.list_draft_summaries(foreign.id, study.id, 200).is_err());
    assert_eq!(db.get_draft(owner.id, second.id).unwrap().body, large_body);
    let renamed = db
        .rename_draft(owner.id, second.id, second.revision, "Original angle B")
        .unwrap();
    assert_eq!(renamed.body, large_body);
    assert_eq!(renamed.revision, second.revision + 1);
    assert_eq!(
        db.list_draft_versions(owner.id, second.id).unwrap().len(),
        2
    );
    assert!(matches!(
        db.rename_draft(owner.id, second.id, second.revision, "Stale"),
        Err(RepositoryError::Conflict(_))
    ));
    assert!(db
        .rename_draft(foreign.id, second.id, renamed.revision, "Foreign")
        .is_err());
    assert_eq!(db.get_draft(owner.id, first.id).unwrap().title, "Original");
    let archived = db
        .set_study_draft_archived_checked(owner.id, study.id, second.id, renamed.revision, true)
        .unwrap();
    assert!(archived.archived_at.is_some());
    assert_eq!(
        db.list_draft_summaries(owner.id, study.id, 200)
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        db.list_archived_draft_summaries(owner.id, study.id, 200)
            .unwrap()
            .len(),
        1
    );
    assert!(matches!(
        db.set_study_draft_archived_checked(owner.id, study.id, first.id, first.revision, true),
        Err(RepositoryError::Conflict(_))
    ));
    assert!(matches!(
        db.set_study_draft_archived_checked(owner.id, study.id, second.id, renamed.revision, false),
        Err(RepositoryError::Conflict(_))
    ));
    assert!(db
        .set_study_draft_archived_checked(foreign.id, study.id, second.id, archived.revision, false)
        .is_err());
    let restored = db
        .set_study_draft_archived_checked(owner.id, study.id, second.id, archived.revision, false)
        .unwrap();
    assert!(restored.archived_at.is_none());
    assert_eq!(restored.body, large_body);
    assert_eq!(
        db.list_draft_versions(owner.id, second.id).unwrap().len(),
        4
    );
    assert!(db
        .list_archived_draft_summaries(owner.id, study.id, 200)
        .unwrap()
        .is_empty());
}

// v11 is an additive transaction. A corrupted existing table must fail closed
// without advancing the migration ledger or losing historical content.
#[test]
fn writing_v10_upgrade_and_schema_collision_rollback_preserve_existing_studies() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("writing-migration.sqlite");
    let store = SqliteStore::open(&path).unwrap();
    store.run_integration_migrations().unwrap();
    let project = Project::new("Preserved writing project");
    store.create_project(&project).unwrap();
    let study = store
        .create_study(project.id, "Preserved Study", "Keep original")
        .unwrap();
    drop(store);

    let conn = Connection::open(&path).unwrap();
    remove_v13_from_legacy_fixture(&conn);
    conn.execute_batch(
        "PRAGMA foreign_keys=ON;
         DROP TRIGGER external_provider_identity_on_insert;
         DROP TRIGGER external_provider_identity_on_update;
         DROP INDEX idx_external_provider_request_unique;
         DROP INDEX idx_external_provider_receipt_unique;
         DROP TABLE agent_external_provider_identities;
         DELETE FROM schema_migrations WHERE version=12;
         DROP TRIGGER study_writing_batches_scope_insert;
         DROP TRIGGER study_writing_batches_immutable_delete;
         DROP TRIGGER study_writing_batches_immutable_update;
         DROP TRIGGER study_writing_revisions_immutable_delete;
         DROP TRIGGER study_writing_revisions_immutable_update;
         DROP TABLE study_writing_revisions;
         DROP TABLE study_writing_batches;
         DELETE FROM schema_migrations WHERE version=11;
         PRAGMA user_version=10;
         CREATE TABLE study_writing_batches(sentinel INTEGER);",
    )
    .unwrap();
    drop(conn);

    assert!(SqliteStore::open(&path).is_err());
    let conn = Connection::open(&path).unwrap();
    let version: i64 = conn
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .unwrap();
    assert_eq!(version, 10);
    let applied: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM schema_migrations WHERE version=11",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(applied, 0);
    let title: String = conn
        .query_row(
            "SELECT title FROM content_studies WHERE id=?1",
            params![study.id.to_string()],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(title, "Preserved Study");
    conn.execute_batch("DROP TABLE study_writing_batches;")
        .unwrap();
    drop(conn);

    let upgraded = SqliteStore::open(&path).unwrap();
    assert_eq!(upgraded.schema_version().unwrap(), LATEST_SCHEMA_VERSION);
    assert_eq!(
        upgraded.get_study(project.id, study.id).unwrap().title,
        title
    );
    drop(upgraded);
    let reopened = SqliteStore::open(&path).unwrap();
    assert_eq!(reopened.schema_version().unwrap(), LATEST_SCHEMA_VERSION);
    let second = Project::new("Foreign writing project");
    reopened.create_project(&second).unwrap();
    let conn = Connection::open(&path).unwrap();
    conn.execute_batch("PRAGMA foreign_keys=ON").unwrap();
    let mismatched = conn.execute(
        "INSERT INTO study_writing_batches(
             run_id,project_id,study_id,payload_json,draft_ids_json,committed_at)
         VALUES('foreign-claim',?1,?2,'{}','[1,2,3]','2026-11-16T00:00:00Z')",
        params![second.id.to_string(), study.id.to_string()],
    );
    assert!(mismatched.is_err());
}
