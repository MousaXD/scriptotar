//// Agent 10 independent, native v11 SQLite integrity assertions.
//// Appended to the real desktop integration test fixture, not a fake store.

#[test]
fn security_v11_receipt_is_immutable_and_survives_restart() {
    use study_writing_persistence::CanonicalSqliteCandidateStore;
    let (temp, db, plan, current, batch) = canonical_batch_fixture();
    let path = db.path().to_path_buf();
    let mut sink = CanonicalSqliteCandidateStore { store: &db, current: &current };
    let ids = sink.commit_three(&batch).unwrap();
    drop(sink);
    let conn = Connection::open(&path).unwrap();
    conn.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
    assert!(conn.execute(
        "UPDATE study_writing_batches SET payload_json='{}' WHERE run_id=?1",
        params![batch.run_id],
    ).is_err(), "receipt payload must be immutable");
    assert!(conn.execute(
        "DELETE FROM study_writing_batches WHERE run_id=?1",
        params![batch.run_id],
    ).is_err(), "a live Study receipt must not be deletable");
    drop(conn);
    drop(db);
    let reopened = scriptotar_db::SqliteStore::open(&path).unwrap();
    assert_eq!(reopened.schema_version().unwrap(), scriptotar_db::LATEST_SCHEMA_VERSION);
    assert!(reopened.get_study(plan.context.project_id, plan.context.study_id).is_ok());
    for id in ids {
        assert_eq!(reopened.get_draft(plan.context.project_id, id).unwrap().revision, 1);
    }
    let conn = Connection::open(reopened.path()).unwrap();
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM study_writing_batches WHERE run_id=?1",
        params![batch.run_id], |row| row.get(0),
    ).unwrap();
    assert_eq!(count, 1);
    drop(temp);
}

#[test]
fn security_v11_structured_first_version_cannot_be_rewritten_or_deleted() {
    use study_writing_persistence::CanonicalSqliteCandidateStore;
    let (_temp, db, _plan, current, batch) = canonical_batch_fixture();
    let mut sink = CanonicalSqliteCandidateStore { store: &db, current: &current };
    let ids = sink.commit_three(&batch).unwrap();
    let conn = Connection::open(db.path()).unwrap();
    conn.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
    let draft_id = ids[0].to_string();
    let existing: String = conn.query_row(
        "SELECT structured_json FROM study_writing_revisions WHERE draft_id=?1 AND version=1",
        params![draft_id], |row| row.get(0),
    ).unwrap();
    assert!(conn.execute(
        "UPDATE study_writing_revisions SET structured_json='{}' WHERE draft_id=?1 AND version=1",
        params![draft_id],
    ).is_err(), "structured version-1 must be immutable");
    assert!(conn.execute(
        "DELETE FROM study_writing_revisions WHERE draft_id=?1 AND version=1",
        params![draft_id],
    ).is_err(), "cannot erase version-1 while the draft exists");
    let after: String = conn.query_row(
        "SELECT structured_json FROM study_writing_revisions WHERE draft_id=?1 AND version=1",
        params![draft_id], |row| row.get(0),
    ).unwrap();
    assert_eq!(after, existing);
}

#[test]
fn security_v11_foreign_project_cannot_inject_a_valid_shape_receipt() {
    use scriptotar_core::{Project, ProjectRepository};
    let (_temp, db, plan, _current, _batch) = canonical_batch_fixture();
    let foreign = Project::new("Unrelated project");
    db.create_project(&foreign).unwrap();
    let conn = Connection::open(db.path()).unwrap();
    conn.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
    let forged = conn.execute(
        "INSERT INTO study_writing_batches(run_id,project_id,study_id,payload_json,draft_ids_json,committed_at)
         VALUES('foreign-batch',?1,?2,'{}','[\"a\",\"b\",\"c\"]','2026-11-16T12:00:00Z')",
        params![foreign.id.to_string(), plan.context.study_id.to_string()],
    );
    assert!(forged.is_err(), "v11 must reject foreign-project Study receipt");
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM study_writing_batches WHERE run_id='foreign-batch'",
        [], |row| row.get(0),
    ).unwrap();
    assert_eq!(count, 0);
}
