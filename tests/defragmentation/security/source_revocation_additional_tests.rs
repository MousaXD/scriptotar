// Agent 10 independent security regressions against PR #137's REAL
// migration-backed SqliteStore. Appended ONLY to the disposable CI copy of
// crates/scriptotar-db/tests/content_studies.rs; production PR is unchanged.
// Source-scoped revocation is not merely an editor UI preference: a revoked
// source must not gain NEW evidence/draft links through other repository APIs.
// Preexisting immutable evidence and draft provenance must remain readable.

// Read the *actual* canonical SQLite file in query-only mode. Row counts alone
// miss unauthorized changes to private drafts, provider ledgers and audit text.
fn independent_revoked_database_snapshot(
    db: &SqliteStore,
) -> Vec<(String, Vec<Vec<String>>)> {
    use rusqlite::OpenFlags;
    let conn = Connection::open_with_flags(db.path(), OpenFlags::SQLITE_OPEN_READ_ONLY)
        .expect("canonical app SQLite must open read-only");
    conn.pragma_update(None, "query_only", "ON").unwrap();
    let mut index = conn.prepare(
        "SELECT name FROM sqlite_master WHERE type='table'          AND name NOT LIKE 'sqlite_%' ORDER BY name"
    ).unwrap();
    let names = index.query_map([], |row| row.get::<_, String>(0))
        .unwrap().map(|row| row.unwrap()).collect::<Vec<_>>();
    assert!(names.iter().any(|table| table == "study_sources"));
    assert!(names.iter().any(|table| table == "study_evidence"));
    assert!(names.iter().any(|table| table == "agent_audit_events"));
    names.into_iter().map(|table| {
        let quoted = table.replace('"', "\"\"");
        let mut stmt = conn.prepare(&format!("SELECT * FROM \"{quoted}\"")).unwrap();
        let columns = stmt.column_count();
        let mut rows: Vec<Vec<String>> = stmt.query_map([], |row| {
            (0..columns).map(|col| {
                let field = row.get_ref(col)?;
                Ok(format!("{field:?}"))
            }).collect::<rusqlite::Result<Vec<String>>>()
        }).unwrap().map(|row| row.unwrap()).collect();
        rows.sort();
        (table, rows)
    }).collect()
}

#[test]
fn independent_revoked_source_cannot_acquire_new_evidence() {
    let (_scratch, db, owner, _foreign) = fixture();
    let (_research, _media, transcript_id) = seed_existing_origins(&db, &owner);
    let study = db.create_study(owner.id, "Withhold source access", "Test").unwrap();
    let linked = db.add_study_source(
        owner.id, study.id,
        &StudySourceInput {
            kind: StudySourceKind::Transcript,
            reference_id: Some(transcript_id),
            locator: None,
            manual_text: None,
            source_revision: Some("original-source-revision".into()),
            uncertainty: None,
        },
    ).unwrap();
    // Establish legitimate history before denial; it MUST survive revocation.
    let historic_evidence = db.add_evidence_reference(
        owner.id, study.id,
        &EvidenceInput {
            study_source_id: linked.id,
            claim: "Historical pre-revocation claim".into(),
            locator: "0:03".into(),
            timestamp_seconds: Some(3.0),
            certainty: EvidenceCertainty::Unverified,
            uncertainty_note: None,
        },
    ).unwrap();
    let before_study = db.get_study(owner.id, study.id).unwrap();
    db.revoke_study_source_access(
        owner.id, study.id, before_study.revision,
        StudySourceKind::Transcript, transcript_id,
    ).unwrap();
    let after_study = db.get_study(owner.id, study.id).unwrap();
    let original = db.get_study_source(owner.id, study.id, linked.id).unwrap();
    assert!(!original.available, "source is still available after revocation");
    assert_eq!(original.reference_id, Some(transcript_id));
    assert_eq!(
        db.list_evidence_references(owner.id, study.id).unwrap()[0].id,
        historic_evidence.id,
        "revocation deleted historical evidence",
    );

    let snapshot_before_denial = independent_revoked_database_snapshot(&db);
    // The same revoked source ID is still an immutable historical identity.
    // This cannot confer fresh authorization on a future mutation.
    let rejected = db.add_evidence_reference(
        owner.id, study.id,
        &EvidenceInput {
            study_source_id: linked.id,
            claim: "NEW claim must be rejected after access withdrawal".into(),
            locator: "0:08".into(),
            timestamp_seconds: Some(8.0),
            certainty: EvidenceCertainty::Unverified,
            uncertainty_note: None,
        },
    );
    assert!(
        rejected.is_err(),
        "FAIL OPEN: canonical DB created new evidence on a revoked Study source"
    );
    assert_eq!(
        db.get_study(owner.id, study.id).unwrap(), after_study,
        "rejected operation changed the Study revision",
    );
    let persisted_evidence = db.list_evidence_references(owner.id, study.id).unwrap();
    assert_eq!(persisted_evidence.len(), 1);
    assert_eq!(persisted_evidence[0].id, historic_evidence.id);
    assert_eq!(independent_revoked_database_snapshot(&db), snapshot_before_denial,
        "denied evidence write modified canonical SQLite rows");
}

#[test]
fn independent_revoked_source_cannot_acquire_new_draft_link() {
    let (_scratch, db, owner, _foreign) = fixture();
    let (_research, _media, transcript_id) = seed_existing_origins(&db, &owner);
    let study = db.create_study(owner.id, "Withdraw draft source", "Test").unwrap();
    let source = db.add_study_source(
        owner.id, study.id,
        &StudySourceInput {
            kind: StudySourceKind::Transcript,
            reference_id: Some(transcript_id),
            locator: None,
            manual_text: None,
            source_revision: None,
            uncertainty: None,
        },
    ).unwrap();
    let historic = db.create_draft(owner.id, study.id, "Historic", "Original text").unwrap();
    let fresh = db.create_draft(owner.id, study.id, "New link must fail", "Original text").unwrap();
    db.attach_source_to_draft(owner.id, historic.id, source.id).unwrap();
    let before = db.get_study(owner.id, study.id).unwrap();
    db.revoke_study_source_access(
        owner.id, study.id, before.revision,
        StudySourceKind::Transcript, transcript_id,
    ).unwrap();
    assert!(!db.get_study_source(owner.id, study.id, source.id).unwrap().available);
    let revoked = db.get_study(owner.id, study.id).unwrap();
    assert_eq!(db.list_draft_source_ids(owner.id, historic.id).unwrap(), vec![source.id]);
    assert!(db.list_draft_source_ids(owner.id, fresh.id).unwrap().is_empty());

    // An immutable historic link remains readable; no new link may be created
    // after permission withdrawal even through the direct storage API.
    let snapshot_before_denial = independent_revoked_database_snapshot(&db);
    let attempted = db.attach_source_to_draft(owner.id, fresh.id, source.id);
    assert!(attempted.is_err(),
        "FAIL OPEN: canonical DB created new draft-source link after revocation");
    assert_eq!(db.get_study(owner.id, study.id).unwrap(), revoked);
    assert_eq!(db.list_draft_source_ids(owner.id, historic.id).unwrap(), vec![source.id]);
    assert!(db.list_draft_source_ids(owner.id, fresh.id).unwrap().is_empty());
    assert_eq!(independent_revoked_database_snapshot(&db), snapshot_before_denial,
        "denied draft link modified canonical SQLite rows");
}

#[test]
fn independent_revoked_origin_cannot_be_reselected_by_editor_bundle() {
    use scriptotar_db::StudioBundleUpdate;
    let (_scratch, db, owner, _foreign) = fixture();
    let (_research, _media, transcript_id) = seed_existing_origins(&db, &owner);
    let study = db.create_study(owner.id, "Revoked origin", "No revive").unwrap();
    let source = db.add_study_source(
        owner.id, study.id,
        &StudySourceInput {
            kind: StudySourceKind::Transcript,
            reference_id: Some(transcript_id),
            locator: None,
            manual_text: None,
            source_revision: None,
            uncertainty: None,
        },
    ).unwrap();
    let draft = db.create_draft(owner.id, study.id, "Persisted", "Original").unwrap();
    let current = db.get_study(owner.id, study.id).unwrap();
    db.revoke_study_source_access(
        owner.id, study.id, current.revision,
        StudySourceKind::Transcript, transcript_id,
    ).unwrap();
    let tombstone = db.get_study_source(owner.id, study.id, source.id).unwrap();
    let after = db.get_study(owner.id, study.id).unwrap();
    let original_draft = db.get_draft(owner.id, draft.id).unwrap();

    let snapshot_before_denial = independent_revoked_database_snapshot(&db);
    let proposed = StudyEdit {
        title: "Forged reattachment".into(),
        goal: "Never save".into(),
        status: StudyStatus::Active,
    };
    let attempted = db.save_studio_bundle(
        owner.id, study.id,
        StudioBundleUpdate {
            expected_revision: after.revision,
            edit: &proposed,
            brief: &brief(),
            brief_changed: false,
            sources: &[(StudySourceKind::Transcript, transcript_id)],
            draft: "Unauthorized change",
            draft_id: Some(draft.id),
            expected_draft_revision: Some(original_draft.revision),
        },
    );
    assert!(attempted.is_err(), "FAIL OPEN: editor save revived a revoked origin");
    assert_eq!(db.get_study(owner.id, study.id).unwrap(), after);
    assert_eq!(db.get_study_source(owner.id, study.id, source.id).unwrap(), tombstone);
    assert_eq!(db.get_draft(owner.id, draft.id).unwrap(), original_draft);
    assert!(db.list_draft_source_ids(owner.id, draft.id).unwrap().is_empty());
    assert_eq!(independent_revoked_database_snapshot(&db), snapshot_before_denial,
        "denied bundle reselect modified canonical SQLite rows");
}

#[test]
fn independent_revoked_source_cannot_gain_job_or_clone_lineage() {
    let (_scratch, db, owner, _foreign) = fixture();
    let (_research, _media, transcript_id) = seed_existing_origins(&db, &owner);
    let study = db.create_study(owner.id, "Revoked provenance", "Keep history").unwrap();
    let source = db.add_study_source(
        owner.id, study.id,
        &StudySourceInput {
            kind: StudySourceKind::Transcript,
            reference_id: Some(transcript_id),
            locator: None,
            manual_text: None,
            source_revision: Some("original source v1".into()),
            uncertainty: None,
        },
    ).unwrap();
    let historic = db.create_draft(owner.id, study.id, "Pre-revoke", "Immutable text").unwrap();
    db.attach_source_to_draft(owner.id, historic.id, source.id).unwrap();
    let job = Job::new(
        owner.id,
        JobInput::Url("https://example.org/original-owned-source".into()),
    );
    db.insert_job(&job).unwrap();
    let before = db.get_study(owner.id, study.id).unwrap();
    db.revoke_study_source_access(
        owner.id, study.id, before.revision,
        StudySourceKind::Transcript, transcript_id,
    ).unwrap();
    let revoked = db.get_study(owner.id, study.id).unwrap();
    let tombstone = db.get_study_source(owner.id, study.id, source.id).unwrap();
    assert!(!tombstone.available, "source authorization not withdrawn");
    assert_eq!(
        db.list_draft_source_ids(owner.id, historic.id).unwrap(),
        vec![source.id],
        "historical immutable source attachment was erased",
    );
    assert!(db.list_study_source_job_ids(owner.id, study.id, source.id).unwrap().is_empty());

    let snapshot_before_denial = independent_revoked_database_snapshot(&db);
    // Independent of the direct evidence and draft-link tests: both these
    // lower-level APIs can create new provenance through a revoked tombstone.
    assert!(
        db.attach_job_to_study_source(owner.id, study.id, source.id, job.id).is_err(),
        "FAIL OPEN: revoked Study source gained a NEW job association",
    );
    assert!(
        db.create_reviewed_study_draft(
            owner.id, study.id, revoked.revision,
            "New script must not cite revoked source",
            "Do not create candidate",
            &[source.id],
        ).is_err(),
        "FAIL OPEN: reviewed candidate gained revoked provenance",
    );
    assert!(
        db.duplicate_draft(owner.id, historic.id).is_err(),
        "FAIL OPEN: legacy draft clone copied a revoked historical source link",
    );
    assert!(
        db.duplicate_study_draft_checked(
            owner.id, study.id, historic.id, historic.revision
        ).is_err(),
        "FAIL OPEN: scoped draft clone copied a revoked historical source link",
    );
    assert_eq!(db.get_study(owner.id, study.id).unwrap(), revoked);
    assert_eq!(db.get_study_source(owner.id, study.id, source.id).unwrap(), tombstone);
    assert_eq!(
        db.list_draft_source_ids(owner.id, historic.id).unwrap(),
        vec![source.id],
        "denied new work corrupted historical provenance",
    );
    assert!(db.list_study_source_job_ids(owner.id, study.id, source.id).unwrap().is_empty());
    assert_eq!(independent_revoked_database_snapshot(&db), snapshot_before_denial,
        "denied job/candidate/clone operations modified any canonical SQLite row");
}
