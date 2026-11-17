// Agent 10 independent *upgrade* security checks. Inject only when target
// uses schema v12. Runs against real production SqliteStore migrations,
// scoped effect rows and a recreated historical v11 SQLite file.
// No production source or actual external/provider transport is modified.

fn independent_v12_recreate_v11_schema(path: &std::path::Path) {
    let connection = rusqlite::Connection::open(path).unwrap();
    connection.execute_batch(
        "DROP TRIGGER external_provider_identity_on_insert;
         DROP TRIGGER external_provider_identity_on_update;
         DROP INDEX idx_external_provider_request_unique;
         DROP INDEX idx_external_provider_receipt_unique;
         DROP TABLE agent_external_provider_identities;
         DELETE FROM schema_migrations WHERE version = 12;
         PRAGMA user_version = 11;",
    ).unwrap();
    let v: i64 = connection.pragma_query_value(None, "user_version", |row| row.get(0)).unwrap();
    assert_eq!(v, 11);
}

#[test]
fn independent_v12_valid_historical_v11_receipt_backfills_and_replays_without_double_credit() {
    let (dir, db, project, study, first) = setup();
    let key = identity(&first, project, study);
    let reserved = reserve(&db, &first, project, study);
    db.mark_external_dispatched(key, reserved.revision, 1).unwrap();
    db.record_external_ack(key, 1, "req_historical_valid_112").unwrap();
    let terminal = db.reconcile_external_effect(
        key,
        1,
        ExternalResolution::Completed {
            provider_request_id: Some("req_historical_valid_112"),
            receipt_id: "receipt_historical_valid_112",
            tokens: 100,
            cost_microusd: 450,
        },
    ).unwrap();
    assert_eq!(terminal.status, ExternalEffectStatus::Completed);
    drop(db);
    let path = dir.path().join("external.sqlite3");
    independent_v12_recreate_v11_schema(&path);

    let migrated = SqliteStore::open(&path).unwrap();
    assert_eq!(migrated.schema_version().unwrap(), 12);
    let preserved = migrated.get_external_effect(key).unwrap().unwrap();
    assert_eq!(preserved, terminal, "v11→v12 migration must not modify charged receipt");
    assert_eq!(
        migrated.record_external_ack(key, 1, "req_historical_valid_112").unwrap(),
        preserved,
    );
    assert_eq!(
        migrated.reconcile_external_effect(
            key,
            1,
            ExternalResolution::Completed {
                provider_request_id: Some("req_historical_valid_112"),
                receipt_id: "receipt_historical_valid_112",
                tokens: 100,
                cost_microusd: 450,
            },
        ).unwrap(),
        preserved,
    );
    let connection = rusqlite::Connection::open(&path).unwrap();
    for kind in ["request", "receipt"] {
        let claimed: i64 = connection.query_row(
            "SELECT COUNT(*) FROM agent_external_provider_identities
             WHERE provider='openai' AND kind=?1 AND idempotency_key=?2",
            rusqlite::params![kind, key.idempotency_key],
            |row| row.get(0),
        ).unwrap();
        assert_eq!(claimed, 1, "valid v11 ID must be durably backfilled once");
    }
}

#[test]
fn independent_v12_cross_project_legacy_receipt_collision_rollback_is_atomic() {
    let (dir, db, first_project, first_study, first) = setup();
    let second_project = Project::new("Historical provider collision project");
    db.create_project(&second_project).unwrap();
    let second_study = db
        .create_study(second_project.id, "Historical second Study", "")
        .unwrap().id;
    let second = independent_v10_approved_peer(
        &db, &first, second_project.id, second_study,
    );
    let first_key = identity(&first, first_project, first_study);
    let second_key = identity(&second, second_project.id, second_study);
    let ar = reserve(&db, &first, first_project, first_study);
    let br = reserve(&db, &second, second_project.id, second_study);
    db.mark_external_dispatched(first_key, ar.revision, 1).unwrap();
    db.mark_external_dispatched(second_key, br.revision, 1).unwrap();
    drop(db);
    let path = dir.path().join("external.sqlite3");
    independent_v12_recreate_v11_schema(&path);
    let connection = rusqlite::Connection::open(&path).unwrap();
    for (key, request) in [
        (first_key.idempotency_key, "req_legacy_two_projects_A"),
        (second_key.idempotency_key, "req_legacy_two_projects_B"),
    ] {
        // Both historical v10 rows are legitimate independent billable effects,
        // but a malformed old v11 DB has attributed ONE receipt to both.
        connection.execute(
            "UPDATE agent_external_effects
             SET status='completed', provider_request_id=?1,
                 receipt_id='receipt_duplicate_cross_project',
                 actual_tokens=100, actual_cost_microusd=450
             WHERE idempotency_key=?2",
            rusqlite::params![request, key],
        ).unwrap();
        connection.execute(
            "UPDATE agent_effects
             SET status='applied', external_id=?1,
                 artifact_id='receipt_duplicate_cross_project'
             WHERE idempotency_key=?2",
            rusqlite::params![request, key],
        ).unwrap();
    }
    drop(connection);

    assert!(
        SqliteStore::open(&path).is_err(),
        "v12 upgrade silently chose a winner for ambiguous historical billing receipts"
    );
    let connection = rusqlite::Connection::open(&path).unwrap();
    let version: i64 = connection.pragma_query_value(None, "user_version", |row| row.get(0)).unwrap();
    assert_eq!(version, 11, "failed migration must not alter user_version");
    let migration_count: i64 = connection.query_row(
        "SELECT COUNT(*) FROM schema_migrations WHERE version=12",
        [],
        |row| row.get(0),
    ).unwrap();
    assert_eq!(migration_count, 0, "failed migration must not commit ledger v12");
    let preserved: i64 = connection.query_row(
        "SELECT COUNT(*) FROM agent_external_effects
         WHERE provider='openai' AND receipt_id='receipt_duplicate_cross_project'
           AND status='completed'",
        [],
        |row| row.get(0),
    ).unwrap();
    assert_eq!(preserved, 2, "reject malformed history without deleting charge records");
    let partial: i64 = connection.query_row(
        "SELECT COUNT(*) FROM sqlite_master
         WHERE type='table' AND name='agent_external_provider_identities'",
        [],
        |row| row.get(0),
    ).unwrap();
    assert_eq!(partial, 0, "partial v12 table must roll back with failed migration");
}
