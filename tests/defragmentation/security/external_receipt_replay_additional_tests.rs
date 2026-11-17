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


#[test]
fn independent_v10_same_effect_receipt_replay_after_reopen_is_idempotent() {
    let (dir, db, project, study, run) = setup();
    let key = identity(&run, project, study);
    let first = reserve(&db, &run, project, study);
    db.mark_external_dispatched(key, first.revision, 1).unwrap();
    let acknowledged = db
        .record_external_ack(key, 1, "req_independent_reopen_1")
        .unwrap();
    assert_eq!(
        acknowledged.provider_request_id.as_deref(),
        Some("req_independent_reopen_1")
    );
    let receipt = || ExternalResolution::Completed {
        provider_request_id: Some("req_independent_reopen_1"),
        receipt_id: "receipt_independent_reopen_1",
        tokens: 100,
        cost_microusd: 450,
    };
    let settled = db.reconcile_external_effect(key, 1, receipt()).unwrap();
    assert_eq!(settled.status, ExternalEffectStatus::Completed);
    drop(db);

    let reopened = SqliteStore::open(dir.path().join("external.sqlite3")).unwrap();
    assert_eq!(
        reopened.record_external_ack(key, 1, "req_independent_reopen_1").unwrap(),
        settled,
        "a duplicate ACK for the original effect must not allocate new charge"
    );
    assert_eq!(
        reopened.reconcile_external_effect(key, 1, receipt()).unwrap(),
        settled,
        "identical late receipt for the original effect must remain idempotent"
    );
    assert!(reopened.mark_external_dispatched(key, settled.revision, 1).is_err());
    let connection = rusqlite::Connection::open(dir.path().join("external.sqlite3")).unwrap();
    let credited_rows: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM agent_external_effects
             WHERE provider = 'openai' AND provider_request_id = ?1
               AND receipt_id = ?2 AND status = 'completed'",
            rusqlite::params!["req_independent_reopen_1", "receipt_independent_reopen_1"],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(credited_rows, 1, "one effect, one durable billable receipt");
}

#[test]
fn independent_v10_provider_scoped_identities_do_not_block_distinct_providers() {
    let (_dir, db, project, study, run) = setup();
    let other = independent_v10_approved_peer(&db, &run, project, study);
    let first_key = identity(&run, project, study);
    let other_key = identity(&other, project, study);
    let first = reserve(&db, &run, project, study);
    let other_reserved = db
        .reserve_external_effect(other_key, fp(), "anthropic", limits())
        .unwrap();
    let ExternalReservation::Fresh(other_first) = other_reserved else {
        panic!("independent provider must receive a fresh effect");
    };
    db.mark_external_dispatched(first_key, first.revision, 1).unwrap();
    db.mark_external_dispatched(other_key, other_first.revision, 1).unwrap();
    let same_provider_local_request_id = "req_provider_local_101";
    let same_provider_local_receipt_id = "receipt_provider_local_202";
    for key in [first_key, other_key] {
        let outcome = db
            .reconcile_external_effect(
                key,
                1,
                ExternalResolution::Completed {
                    provider_request_id: Some(same_provider_local_request_id),
                    receipt_id: same_provider_local_receipt_id,
                    tokens: 40,
                    cost_microusd: 200,
                },
            )
            .unwrap();
        assert_eq!(outcome.status, ExternalEffectStatus::Completed);
    }
    assert_ne!(
        db.get_external_effect(first_key).unwrap().unwrap().provider,
        db.get_external_effect(other_key).unwrap().unwrap().provider,
        "uniqueness must be provider-scoped, not globally overbroad"
    );
}

#[test]
fn independent_v10_old_attempt_ack_cannot_be_reclaimed_after_proven_retry() {
    let (_dir, db, project, study, run) = setup();
    let key = identity(&run, project, study);
    let initial = reserve(&db, &run, project, study);
    db.mark_external_dispatched(key, initial.revision, 1).unwrap();
    db.record_external_ack(key, 1, "req_historical_attempt_identity_101")
        .unwrap();
    let not_executed = db
        .reconcile_external_effect(
            key,
            1,
            ExternalResolution::VerifiedNotExecuted {
                evidence_id: "proof_independent_nonexecution_1",
            },
        )
        .unwrap();
    let retry = db.prepare_external_retry(key, not_executed.revision).unwrap();
    assert_eq!(retry.attempt, 2);
    assert_eq!(retry.status, ExternalEffectStatus::Reserved);

    // The retry clears the current-row request field. It must not erase an
    // immutable identity claimed by attempt 1 before the nonexecution proof.
    let peer = independent_v10_approved_peer(&db, &run, project, study);
    let peer_key = identity(&peer, project, study);
    let peer_reserved = reserve(&db, &peer, project, study);
    db.mark_external_dispatched(peer_key, peer_reserved.revision, 1)
        .unwrap();
    assert!(
        db.record_external_ack(peer_key, 1, "req_historical_attempt_identity_101")
            .is_err(),
        "a provider request identity from attempt 1 was reassigned to another effect after retry"
    );
    let losing = db.get_external_effect(peer_key).unwrap().unwrap();
    assert_eq!(losing.status, ExternalEffectStatus::Dispatched);
    assert!(losing.provider_request_id.is_none());
}


#[test]
fn independent_v12_provider_case_alias_cannot_double_credit() {
    let (_dir, db, project, study, run) = setup();
    let peer = independent_v10_approved_peer(&db, &run, project, study);
    let first_key = identity(&run, project, study);
    let peer_key = identity(&peer, project, study);
    let first = reserve(&db, &run, project, study);
    // The production API presently accepts any bounded provider string.
    // "OpenAI" and "openai" are not two separate billing authorities.
    let alias = db.reserve_external_effect(peer_key, fp(), "OpenAI", limits());
    let peer_first = match alias {
        // Rejecting a noncanonical provider spelling is also a valid safe outcome.
        Err(_) => {
            assert!(db.get_external_effect(peer_key).unwrap().is_none());
            return;
        }
        Ok(ExternalReservation::Fresh(value)) => value,
        Ok(ExternalReservation::Existing(_)) => {
            panic!("different approved run must never reuse an old reservation");
        }
    };
    db.mark_external_dispatched(first_key, first.revision, 1).unwrap();
    db.mark_external_dispatched(peer_key, peer_first.revision, 1).unwrap();
    db.reconcile_external_effect(
        first_key,
        1,
        ExternalResolution::Completed {
            provider_request_id: Some("req_provider_alias_101"),
            receipt_id: "receipt_provider_alias_202",
            tokens: 30,
            cost_microusd: 200,
        },
    )
    .unwrap();

    assert!(
        db.reconcile_external_effect(
            peer_key,
            1,
            ExternalResolution::Completed {
                provider_request_id: Some("req_provider_alias_101"),
                receipt_id: "receipt_provider_alias_202",
                tokens: 30,
                cost_microusd: 200,
            },
        )
        .is_err(),
        "case-alias provider namespace allowed the same OpenAI receipt to bill a second effect"
    );
    let losing = db.get_external_effect(peer_key).unwrap().unwrap();
    assert_eq!(losing.status, ExternalEffectStatus::Dispatched);
    assert!(losing.receipt_id.is_none());
}

#[test]
fn independent_v12_retry_cannot_claim_same_provider_request_from_older_attempt() {
    let (_dir, db, project, study, run) = setup();
    let key = identity(&run, project, study);
    let first = reserve(&db, &run, project, study);
    db.mark_external_dispatched(key, first.revision, 1).unwrap();
    db.record_external_ack(key, 1, "req_previous_attempt_101").unwrap();
    let proof = db
        .reconcile_external_effect(
            key,
            1,
            ExternalResolution::VerifiedNotExecuted {
                evidence_id: "proof_previous_attempt_nonexecution",
            },
        )
        .unwrap();
    let next = db.prepare_external_retry(key, proof.revision).unwrap();
    assert_eq!(next.attempt, 2);
    db.mark_external_dispatched(key, next.revision, 2).unwrap();
    // Equal effect_id is not enough: request IDs are single-attempt proofs.
    // Reassigning attempt-1 ACK to attempt 2 could falsely reconcile a
    // different billable dispatch from a delayed prior provider callback.
    assert!(
        db.record_external_ack(key, 2, "req_previous_attempt_101")
            .is_err(),
        "stale provider request from attempt 1 was accepted as a new attempt-2 ACK"
    );
    let after = db.get_external_effect(key).unwrap().unwrap();
    assert_eq!(after.attempt, 2);
    assert_eq!(after.status, ExternalEffectStatus::Dispatched);
    assert!(after.provider_request_id.is_none());
}


#[test]
fn independent_v12_openai_response_id_cannot_cross_request_receipt_roles() {
    let (_dir, db, project, study, run) = setup();
    let peer = independent_v10_approved_peer(&db, &run, project, study);
    let first_key = identity(&run, project, study);
    let peer_key = identity(&peer, project, study);
    let first = reserve(&db, &run, project, study);
    let second = reserve(&db, &peer, project, study);
    db.mark_external_dispatched(first_key, first.revision, 1).unwrap();
    db.mark_external_dispatched(peer_key, second.revision, 1).unwrap();

    // The desktop OpenAI sealed adapter uses the SAME response ID as its
    // provider_request_id and receipt_id. Two different effects cannot claim
    // that same provider-issued OpenAI response under different column roles.
    db.record_external_ack(first_key, 1, "resp_shared_across_roles_101")
        .unwrap();
    let stolen = db.reconcile_external_effect(
        peer_key,
        1,
        ExternalResolution::Completed {
            provider_request_id: Some("resp_different_request_202"),
            receipt_id: "resp_shared_across_roles_101",
            tokens: 55,
            cost_microusd: 225,
        },
    );
    assert!(
        stolen.is_err(),
        "one OpenAI response ID was assigned as one effect's ACK and another effect's receipt"
    );
    let losing = db.get_external_effect(peer_key).unwrap().unwrap();
    assert_eq!(losing.status, ExternalEffectStatus::Dispatched);
    assert!(losing.receipt_id.is_none());
}


#[test]
fn independent_v12_provider_identity_claims_are_append_only_under_raw_sqlite() {
    let (dir, db, project, study, run) = setup();
    let key = identity(&run, project, study);
    let fresh = reserve(&db, &run, project, study);
    db.mark_external_dispatched(key, fresh.revision, 1).unwrap();
    db.record_external_ack(key, 1, "resp_claimed_forever_101").unwrap();

    // The v12 registry is the only surviving proof after a legitimate
    // verified-not-executed retry has cleared the current v10 ACK field.
    // It must not be possible to modify/delete a tombstone using ordinary
    // application-level SQLite statements.
    let connection = rusqlite::Connection::open(dir.path().join("external.sqlite3")).unwrap();
    let original: String = connection.query_row(
        "SELECT idempotency_key FROM agent_external_provider_identities
         WHERE provider='openai' AND identity='resp_claimed_forever_101'",
        [],
        |r| r.get(0),
    ).unwrap();
    assert_eq!(original, key.idempotency_key);
    let erase = connection.execute(
        "DELETE FROM agent_external_provider_identities
         WHERE provider='openai' AND identity='resp_claimed_forever_101'",
        [],
    );
    assert!(erase.is_err(), "a provider identity tombstone can be deleted");
    let steal = connection.execute(
        "UPDATE agent_external_provider_identities
         SET idempotency_key='forged_other_effect'
         WHERE provider='openai' AND identity='resp_claimed_forever_101'",
        [],
    );
    assert!(steal.is_err(), "a provider identity tombstone can be reassigned");
    let after: String = connection.query_row(
        "SELECT idempotency_key FROM agent_external_provider_identities
         WHERE provider='openai' AND identity='resp_claimed_forever_101'",
        [],
        |r| r.get(0),
    ).unwrap();
    assert_eq!(after, original, "failed tampering must preserve the original owner");
}
