// Agent 10 independent v10 regression. Inject ONLY inside the real
// agent_effects_external.rs existing #[cfg(test)] mod tests fixture.
// No production source, database fixture or network implementation is changed.
#[test]
fn independent_v10_provider_receipt_must_not_settle_two_distinct_runs() {
    let (_dir, db, project, study, first_run) = setup();
    // Create a separate, approved RUNNING checkpoint using the same fixture's
    // validated shape; a new run gets its own permitted v8/v10 effect key.
    let mut second_run = first_run.clone();
    second_run.plan.id = Uuid::new_v4().to_string();
    second_run.steps[0].idempotency_key =
        format!("{}:0:draft", second_run.plan.id);
    second_run.revision = 0;
    second_run.state = RunState::Planned;
    second_run.steps[0].state = StepState::Pending;
    second_run.steps[0].attempts = 0;
    second_run.steps[0].approved = false;
    second_run.usage = Usage::default();
    second_run.events.clear();
    second_run.event(100, AuditKind::Created);
    let checkpoints = db.scoped_agent_checkpoints(project, study).unwrap();
    checkpoints.create(second_run.clone()).unwrap();
    second_run.revision = 1;
    second_run.state = RunState::Running;
    second_run.steps[0].state = StepState::Running;
    second_run.steps[0].attempts = 1;
    second_run.steps[0].approved = true;
    second_run.usage.tool_calls = 1;
    second_run.event(101, AuditKind::Dispatched);
    checkpoints.compare_and_swap(0, second_run.clone()).unwrap();

    let first_key = identity(&first_run, project, study);
    let second_key = identity(&second_run, project, study);
    assert_ne!(first_key.idempotency_key, second_key.idempotency_key);
    let first_reservation = reserve(&db, &first_run, project, study);
    let second_reservation = reserve(&db, &second_run, project, study);
    db.mark_external_dispatched(first_key, first_reservation.revision, 1)
        .unwrap();
    db.mark_external_dispatched(second_key, second_reservation.revision, 1)
        .unwrap();

    let first_receipt = db.reconcile_external_effect(
        first_key,
        1,
        ExternalResolution::Completed {
            provider_request_id: Some("resp_same_provider_9123"),
            receipt_id: "resp_same_provider_9123",
            tokens: 100,
            cost_microusd: 450,
        },
    ).unwrap();
    assert_eq!(first_receipt.status, ExternalEffectStatus::Completed);

    // The same real provider response cannot independently authenticate two
    // chargeable runs. This must fail *even though each run has a unique
    // idempotency claim*. A repeated receipt is not a second provider invoice.
    let second = db.reconcile_external_effect(
        second_key,
        1,
        ExternalResolution::Completed {
            provider_request_id: Some("resp_same_provider_9123"),
            receipt_id: "resp_same_provider_9123",
            tokens: 100,
            cost_microusd: 450,
        },
    );
    assert!(
        second.is_err(),
        "v10 ledger accepted the identical provider receipt for two distinct paid runs"
    );
    assert_eq!(
        db.get_external_effect(second_key).unwrap().unwrap().status,
        ExternalEffectStatus::Dispatched,
        "rejected receipt must not silently complete or replay a second paid run"
    );
}

#[test]
fn independent_v10_provider_receipt_must_not_settle_across_project_boundary() {
    let (dir, db, first_project, first_study, first_run) = setup();
    let foreign_project = Project::new("Different billing project");
    db.create_project(&foreign_project).unwrap();
    let foreign_study = db
        .create_study(foreign_project.id, "Different scoped Study", "")
        .unwrap().id;

    // The foreign job is a *legitimate separate approved checkpoint*, not an
    // illegal cross-project claim. Both effects may execute independently,
    // but provider-issued response identity must never be double-credited.
    let mut foreign_run = first_run.clone();
    foreign_run.plan.id = Uuid::new_v4().to_string();
    foreign_run.plan.project_id = foreign_project.id.to_string();
    foreign_run.plan.study_id = foreign_study.to_string();
    foreign_run.steps[0].idempotency_key =
        format!("{}:0:draft", foreign_run.plan.id);
    foreign_run.revision = 0;
    foreign_run.state = RunState::Planned;
    foreign_run.steps[0].state = StepState::Pending;
    foreign_run.steps[0].attempts = 0;
    foreign_run.steps[0].approved = false;
    foreign_run.usage = Usage::default();
    foreign_run.events.clear();
    foreign_run.event(100, AuditKind::Created);
    let checkpoints = db
        .scoped_agent_checkpoints(foreign_project.id, foreign_study)
        .unwrap();
    checkpoints.create(foreign_run.clone()).unwrap();
    foreign_run.revision = 1;
    foreign_run.state = RunState::Running;
    foreign_run.steps[0].state = StepState::Running;
    foreign_run.steps[0].attempts = 1;
    foreign_run.steps[0].approved = true;
    foreign_run.usage.tool_calls = 1;
    foreign_run.event(101, AuditKind::Dispatched);
    checkpoints.compare_and_swap(0, foreign_run.clone()).unwrap();

    let first_key = identity(&first_run, first_project, first_study);
    let foreign_key = identity(&foreign_run, foreign_project.id, foreign_study);
    let first_reservation = reserve(&db, &first_run, first_project, first_study);
    let foreign_reservation = reserve(&db, &foreign_run, foreign_project.id, foreign_study);
    db.mark_external_dispatched(first_key, first_reservation.revision, 1)
        .unwrap();
    db.mark_external_dispatched(foreign_key, foreign_reservation.revision, 1)
        .unwrap();
    db.reconcile_external_effect(
        first_key, 1, ExternalResolution::Completed {
            provider_request_id: Some("resp_cross_project_9123"),
            receipt_id: "resp_cross_project_9123",
            tokens: 100,
            cost_microusd: 450,
        },
    ).unwrap();

    // A different process/host may reopen the same database after the first
    // provider receipt settles. Uniqueness cannot depend on process-local
    // memory, a per-instance HashSet or the original SQLite connection.
    drop(checkpoints);
    drop(db);
    let reopened = SqliteStore::open(dir.path().join("external.sqlite3")).unwrap();
    let reused = reopened.reconcile_external_effect(
        foreign_key, 1, ExternalResolution::Completed {
            provider_request_id: Some("resp_cross_project_9123"),
            receipt_id: "resp_cross_project_9123",
            tokens: 100,
            cost_microusd: 450,
        },
    );
    assert!(
        reused.is_err(),
        "provider receipt reused across projects and credited as two separate paid effects"
    );
    assert_eq!(
        reopened.get_external_effect(foreign_key).unwrap().unwrap().status,
        ExternalEffectStatus::Dispatched,
        "other project's in-flight payment must not be completed by a reused receipt"
    );
}

// Extra approved run uses an independent checkpoint and effect claim.
// Every test calls the real v8/v10 SQLite code, not an in-memory effect fake.
fn independent_v10_approved_peer(
    db: &SqliteStore,
    original: &RunSnapshot,
    project: Uuid,
    study: Uuid,
) -> RunSnapshot {
    let mut peer = original.clone();
    peer.plan.id = Uuid::new_v4().to_string();
    peer.plan.project_id = project.to_string();
    peer.plan.study_id = study.to_string();
    peer.steps[0].idempotency_key = format!("{}:0:draft", peer.plan.id);
    peer.revision = 0;
    peer.state = RunState::Planned;
    peer.steps[0].state = StepState::Pending;
    peer.steps[0].attempts = 0;
    peer.steps[0].approved = false;
    peer.usage = Usage::default();
    peer.events.clear();
    peer.event(100, AuditKind::Created);
    let checkpoints = db.scoped_agent_checkpoints(project, study).unwrap();
    checkpoints.create(peer.clone()).unwrap();
    peer.revision = 1;
    peer.state = RunState::Running;
    peer.steps[0].state = StepState::Running;
    peer.steps[0].attempts = 1;
    peer.steps[0].approved = true;
    peer.usage.tool_calls = 1;
    peer.event(101, AuditKind::Dispatched);
    checkpoints.compare_and_swap(0, peer.clone()).unwrap();
    peer
}

#[test]
fn independent_v10_acknowledgement_id_is_unique_before_final_settlement() {
    let (_dir, db, project, study, first_run) = setup();
    let peer = independent_v10_approved_peer(&db, &first_run, project, study);
    let original_key = identity(&first_run, project, study);
    let peer_key = identity(&peer, project, study);
    let first_reservation = reserve(&db, &first_run, project, study);
    let peer_reservation = reserve(&db, &peer, project, study);
    db.mark_external_dispatched(original_key, first_reservation.revision, 1)
        .unwrap();
    db.mark_external_dispatched(peer_key, peer_reservation.revision, 1)
        .unwrap();
    let original_ack = db
        .record_external_ack(original_key, 1, "resp_provider_ack_global_1001")
        .unwrap();
    assert_eq!(
        original_ack.provider_request_id.as_deref(),
        Some("resp_provider_ack_global_1001")
    );
    // An ACK can precede the metered body, a timeout or a process crash.
    // It cannot be claimed as evidence for an unrelated effect.
    let duplicate = db.record_external_ack(
        peer_key,
        1,
        "resp_provider_ack_global_1001",
    );
    assert!(
        duplicate.is_err(),
        "v10 accepted one provider acknowledgement for two different paid effects"
    );
    assert!(
        db.get_external_effect(peer_key)
            .unwrap().unwrap().provider_request_id.is_none(),
        "conflicting ACK must leave the second effect unchanged"
    );
}

#[test]
fn independent_v10_receipt_identity_is_unique_even_with_distinct_request_ids() {
    let (_dir, db, project, study, first_run) = setup();
    let peer = independent_v10_approved_peer(&db, &first_run, project, study);
    let original_key = identity(&first_run, project, study);
    let peer_key = identity(&peer, project, study);
    let first_reservation = reserve(&db, &first_run, project, study);
    let peer_reservation = reserve(&db, &peer, project, study);
    db.mark_external_dispatched(original_key, first_reservation.revision, 1)
        .unwrap();
    db.mark_external_dispatched(peer_key, peer_reservation.revision, 1)
        .unwrap();
    db.reconcile_external_effect(
        original_key, 1, ExternalResolution::Completed {
            provider_request_id: Some("resp_provider_unique_1001"),
            receipt_id: "receipt_same_2001",
            tokens: 100,
            cost_microusd: 450,
        },
    ).unwrap();

    let duplicate = db.reconcile_external_effect(
        peer_key, 1, ExternalResolution::Completed {
            provider_request_id: Some("resp_provider_unique_1002"),
            receipt_id: "receipt_same_2001",
            tokens: 100,
            cost_microusd: 450,
        },
    );
    assert!(
        duplicate.is_err(),
        "v10 allowed the same provider receipt ID under different request IDs"
    );
    assert_eq!(
        db.get_external_effect(peer_key).unwrap().unwrap().status,
        ExternalEffectStatus::Dispatched
    );
}

#[test]
fn independent_v10_concurrent_cross_run_receipt_settlement_has_one_winner() {
    let (dir, db, project, study, first_run) = setup();
    let peer = independent_v10_approved_peer(&db, &first_run, project, study);
    let original_key = identity(&first_run, project, study);
    let peer_key = identity(&peer, project, study);
    let first_reservation = reserve(&db, &first_run, project, study);
    let peer_reservation = reserve(&db, &peer, project, study);
    db.mark_external_dispatched(original_key, first_reservation.revision, 1)
        .unwrap();
    db.mark_external_dispatched(peer_key, peer_reservation.revision, 1)
        .unwrap();
    drop(db);
    let barrier = Arc::new(Barrier::new(3));
    let workers = [first_run, peer]
        .into_iter()
        .map(|run| {
            let path = dir.path().join("external.sqlite3");
            let gate = barrier.clone();
            thread::spawn(move || {
                let reopened = SqliteStore::open(&path).unwrap();
                gate.wait();
                reopened.reconcile_external_effect(
                    identity(&run, project, study),
                    1,
                    ExternalResolution::Completed {
                        provider_request_id: Some("resp_concurrent_shared_9012"),
                        receipt_id: "receipt_concurrent_shared_9012",
                        tokens: 100,
                        cost_microusd: 450,
                    },
                )
            })
        })
        .collect::<Vec<_>>();
    barrier.wait();
    let results = workers
        .into_iter()
        .map(|worker| worker.join().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        results.iter().filter(|result| result.is_ok()).count(), 1,
        "two concurrent independent SQLite connections credited one receipt to two runs"
    );
    assert_eq!(results.iter().filter(|result| result.is_err()).count(), 1);
}
