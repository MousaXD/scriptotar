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
