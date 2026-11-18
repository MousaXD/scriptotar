
//// Independent Agent 10 tests appended to Agent 05's actual canonical
//// SQLite integration fixture by run_feature_gates.sh. Not a mock sink.

#[test]
fn security_source_fingerprint_revoked_after_generation_denies_atomic_acceptance() {
    use study_writing_persistence::CanonicalSqliteCandidateStore;
    let (_temp, db, plan, current, batch) = canonical_batch_fixture();
    current.0.borrow_mut().sources[0].fingerprint =
        "revoked-after-provider-generation".to_owned();
    let mut sink = CanonicalSqliteCandidateStore { store: &db, current: &current };
    assert!(matches!(sink.commit_three(&batch), Err(WritingWorkflowError::StaleContext)));
    let conn = Connection::open(db.path()).unwrap();
    for table in ["creative_drafts", "draft_versions", "study_writing_batches"] {
        let count: i64 = conn.query_row(
            &format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0)
        ).unwrap();
        assert_eq!(count, 0, "partial commit in {table}");
    }
    assert_eq!(
        db.get_study(plan.context.project_id, plan.context.study_id)
            .unwrap().revision,
        plan.context.study_revision
    );
}

#[test]
fn security_foreign_lineage_cannot_write_first_canonical_draft() {
    use study_writing_persistence::CanonicalSqliteCandidateStore;
    let (_temp, db, _plan, current, mut batch) = canonical_batch_fixture();
    batch.candidates[0].lineage[0].source_id = Uuid::new_v4();
    let mut sink = CanonicalSqliteCandidateStore { store: &db, current: &current };
    assert!(matches!(
        sink.commit_three(&batch),
        Err(WritingWorkflowError::InvalidContext(_))
    ));
    let count: i64 = Connection::open(db.path()).unwrap()
        .query_row("SELECT COUNT(*) FROM creative_drafts", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 0);
}

#[test]
fn security_model_supported_claims_are_saved_as_unverified() {
    use study_writing_persistence::CanonicalSqliteCandidateStore;
    use scriptotar_ai::writing::ClaimStatus;
    let (_temp, db, _plan, current, mut batch) = canonical_batch_fixture();
    assert!(!batch.candidates[0].draft.claims.is_empty());
    batch.candidates[0].draft.claims[0].status = ClaimStatus::Supported;
    let mut sink = CanonicalSqliteCandidateStore { store: &db, current: &current };
    let ids = sink.commit_three(&batch).unwrap();
    assert_eq!(ids.len(), 3);
    let raw: String = Connection::open(db.path()).unwrap()
        .query_row(
            "SELECT payload_json FROM study_writing_batches WHERE run_id=?1",
            params![batch.run_id], |r| r.get(0),
        ).unwrap();
    let saved: GeneratedBatch = serde_json::from_str(&raw).unwrap();
    assert_eq!(saved.candidates[0].draft.claims[0].status, ClaimStatus::Unverified);
    assert_eq!(
        db.list_draft_summaries(batch.context.project_id, batch.context.study_id, 200)
            .unwrap().len(),
        3
    );
}

#[test]
fn security_duplicate_receipt_with_tampered_payload_does_not_create_draft_four() {
    use study_writing_persistence::CanonicalSqliteCandidateStore;
    let (_temp, db, _plan, current, batch) = canonical_batch_fixture();
    let mut sink = CanonicalSqliteCandidateStore { store: &db, current: &current };
    let ids = sink.commit_three(&batch).unwrap();
    assert_eq!(sink.commit_three(&batch).unwrap(), ids);
    let mut forgery = batch.clone();
    forgery.candidates[0].draft.beats[0].narration =
        "Substituted body after human consent".into();
    assert!(matches!(
        sink.commit_three(&forgery),
        Err(WritingWorkflowError::StaleContext)
    ));
    let count: i64 = Connection::open(db.path()).unwrap().query_row(
        "SELECT COUNT(*) FROM creative_drafts WHERE study_id=?1",
        params![batch.context.study_id.to_string()], |r| r.get(0),
    ).unwrap();
    assert_eq!(count, 3);
}
