//! Agent10 no-network independent OS-process negative / positive controls for v13.
 //! This is intentionally test-only. A green run proves exactly the tested
 //! SQLite trigger boundary, not provider transport, pricing or receipt safety.
use rusqlite::params;
use scriptotar_db::SqliteStore;
use scriptotar_agent::{
    AuditKind, CheckpointStore, Estimate, Limits, Plan, RunSnapshot, RunState,
    StepInput, StepPlan, StepRecord, StepState, Usage,
};
use scriptotar_core::{Project, ProjectRepository};
use scriptotar_db::{
    ExternalEffectKey, ExternalEffectLimits, ExternalEffectStatus, ExternalReservation,
    ExternalResolution, ProviderDispatchEffect,
};
use std::{
    path::Path,
    process::{Command, Stdio},
    thread,
    time::Duration,
};
use tempfile::TempDir;
use uuid::Uuid;

#[test]
fn agent10_os_process_writer_fixture() {
    let Ok(path) = std::env::var("SCRIPTOTAR_AGENT10_V13_DB") else {
        return; // Normal cargo test invocation never modifies arbitrary DBs.
    };
    let study = std::env::var("SCRIPTOTAR_AGENT10_V13_STUDY").unwrap();
    let expected_block = std::env::var("SCRIPTOTAR_AGENT10_V13_BLOCK")
        .unwrap() == "1";
    let connection = rusqlite::Connection::open(path).unwrap();
    let result = connection.execute(
        "UPDATE content_studies SET goal='written by independent OS process' WHERE id=?1",
        params![study],
    );
    if expected_block {
        let error = result.expect_err("independent OS process modified a frozen Study");
        assert!(
            error.to_string().contains("provider dispatch scope is frozen"),
            "unexpected failure, not a verified trigger rejection: {error}"
        );
        println!("AGENT10_V13_TRIGGER_REJECTED");
    } else {
        assert_eq!(result.unwrap(), 1);
        println!("AGENT10_V13_WRITE_AFTER_RELEASE");
    }
}

fn actual_os_process_probe(db_path: &Path, study: Uuid, blocked: bool) -> String {
    let mut child = Command::new(std::env::current_exe().unwrap())
        .arg("agent10_os_process_writer_fixture")
        .arg("--nocapture")
        .env("SCRIPTOTAR_AGENT10_V13_DB", db_path)
        .env("SCRIPTOTAR_AGENT10_V13_STUDY", study.to_string())
        .env("SCRIPTOTAR_AGENT10_V13_BLOCK", if blocked { "1" } else { "0" })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    let mut exited = false;
    for _ in 0..800 {
        if child.try_wait().unwrap().is_some() {
            exited = true;
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }
    if !exited {
        let _ = child.kill();
    }
    let output = child.wait_with_output().unwrap();
    assert!(
        exited && output.status.success(),
        "child process did not complete cleanly before guard release. stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    String::from_utf8(output.stdout).unwrap()
}

#[test]
fn agent10_v13_actual_second_os_process_is_denied_then_allowed_after_guard() {
    let directory = TempDir::new().unwrap();
    let db = SqliteStore::open(directory.path().join("v13.sqlite3")).unwrap();
    assert_eq!(db.schema_version().unwrap(), 13);
    let project = Uuid::new_v4();
    let study = Uuid::new_v4();
    let connection = rusqlite::Connection::open(db.path()).unwrap();
    connection
        .execute(
            "INSERT INTO projects(id,name,created_at) VALUES(?1,'Real process fixture','2026-11-21')",
            params![project.to_string()],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO content_studies(
                id,project_id,title,goal,status,revision,created_at,updated_at
            ) VALUES(?1,?2,'Live Study','Approved original','active',1,'2026-11-21','2026-11-21')",
            params![study.to_string(), project.to_string()],
        )
        .unwrap();
    drop(connection);

    let guard = db
        .acquire_provider_dispatch_barrier(project, study, &"a".repeat(64))
        .unwrap();
    guard.verify().unwrap();
    let rejected = actual_os_process_probe(db.path(), study, true);
    assert!(rejected.contains("AGENT10_V13_TRIGGER_REJECTED"));
    guard.verify().unwrap();
    let unchanged: String = rusqlite::Connection::open(db.path())
        .unwrap()
        .query_row(
            "SELECT goal FROM content_studies WHERE id=?1",
            params![study.to_string()],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(unchanged, "Approved original");
    drop(guard);

    let allowed = actual_os_process_probe(db.path(), study, false);
    assert!(allowed.contains("AGENT10_V13_WRITE_AFTER_RELEASE"));
    let after: String = rusqlite::Connection::open(db.path())
        .unwrap()
        .query_row(
            "SELECT goal FROM content_studies WHERE id=?1",
            params![study.to_string()],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(after, "written by independent OS process");
}

#[test]
fn agent10_v13_bound_reserved_effect_rejects_forged_identity_and_external_mutation() {
    let directory = TempDir::new().unwrap();
    let db = SqliteStore::open(directory.path().join("v13-bound.sqlite3")).unwrap();
    assert_eq!(db.schema_version().unwrap(), 13);
    let project = Project::new("Agent10 bound effect owner");
    db.create_project(&project).unwrap();
    let study = db
        .create_study(project.id, "Agent10 sealed Study", "")
        .unwrap()
        .id;
    let run_id = Uuid::new_v4().to_string();
    let plan = Plan {
        id: run_id.clone(),
        project_id: project.id.to_string(),
        study_id: study.to_string(),
        steps: vec![StepPlan {
            id: "draft".into(),
            input: StepInput::DraftScript {
                brief_id: Uuid::new_v4().to_string(),
            },
            estimate: Estimate {
                tokens: 500,
                cost_microusd: 2_500,
                max_output_bytes: 1_000,
                cpu_ms: 1,
                peak_memory_mib: 1,
            },
            max_attempts: 3,
            requires_approval: true,
        }],
        limits: Limits {
            max_tool_calls: 1,
            max_tokens: 500,
            max_cost_microusd: 2_500,
            max_elapsed_ms: 60_000,
            max_output_bytes: 1_000,
            max_cpu_ms: 100,
            max_peak_memory_mib: 4,
        },
    };
    let mut snapshot = RunSnapshot {
        schema_version: 1,
        plan,
        revision: 0,
        state: RunState::Planned,
        current_step: 0,
        steps: vec![StepRecord {
            id: "draft".into(),
            state: StepState::Pending,
            attempts: 0,
            approved: false,
            idempotency_key: format!("{run_id}:0:draft"),
            pending_effect_id: None,
            output: None,
            failure_code: None,
        }],
        usage: Usage::default(),
        started_ms: 100,
        cancel_requested: false,
        pause_requested: false,
        events: vec![],
    };
    snapshot.event(100, AuditKind::Created);
    let checkpoints = db.scoped_agent_checkpoints(project.id, study).unwrap();
    checkpoints.create(snapshot.clone()).unwrap();
    snapshot.revision = 1;
    snapshot.state = RunState::Running;
    snapshot.steps[0].state = StepState::Running;
    snapshot.steps[0].attempts = 1;
    snapshot.steps[0].approved = true;
    snapshot.usage.tool_calls = 1;
    snapshot.event(101, AuditKind::Dispatched);
    checkpoints.compare_and_swap(0, snapshot.clone()).unwrap();
    let key = ExternalEffectKey {
        project: project.id,
        study,
        run_id: &snapshot.plan.id,
        step_id: "draft",
        idempotency_key: &snapshot.steps[0].idempotency_key,
    };
    let digest = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    let reserved = match db.reserve_external_effect(
        key,
        digest,
        "openai",
        ExternalEffectLimits {
            tokens: 500,
            cost_microusd: 2_500,
        },
    ).unwrap() {
        ExternalReservation::Fresh(value) => value,
        ExternalReservation::Existing(_) => panic!("first claim was unexpectedly existing"),
    };
    let original_attempt = ProviderDispatchEffect {
        run_id: &snapshot.plan.id,
        step_id: "draft",
        idempotency_key: &snapshot.steps[0].idempotency_key,
        attempt: 1,
        provider: "openai",
        wire_sha256: digest,
    };
    let scope_digest = "a".repeat(64);

    // Invalid provider/wire/attempt must NOT acquire the exclusive barrier.
    for (provider, wire_sha256, attempt) in [
        ("another-provider", digest, 1),
        ("openai", "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff", 1),
        ("openai", digest, 2),
    ] {
        let check = db.acquire_provider_dispatch_barrier_for_effect(
            project.id,
            study,
            &scope_digest,
            ProviderDispatchEffect {
                provider,
                wire_sha256,
                attempt,
                ..original_attempt
            },
        );
        assert!(check.is_err(), "forged or stale effect gained the dispatch freeze");
    }
    let guard = db.acquire_provider_dispatch_barrier_for_effect(
        project.id,
        study,
        &scope_digest,
        original_attempt,
    ).unwrap();
    guard.verify().unwrap();
    let denied = actual_os_process_probe(db.path(), study, true);
    assert!(denied.contains("AGENT10_V13_TRIGGER_REJECTED"));
    // Cancellation must not race the final committed freeze / dispatch CAS.
    assert!(
        db.request_external_cancel(key).is_err(),
        "external cancellation raced the bound send freeze"
    );
    let dispatched = db
        .mark_external_dispatched(key, reserved.revision, 1)
        .unwrap();
    assert_eq!(dispatched.status, ExternalEffectStatus::Dispatched);
    assert!(db.mark_external_dispatched(key, dispatched.revision, 1).is_err());
    let settled = db.reconcile_external_effect(
        key,
        1,
        ExternalResolution::Completed {
            provider_request_id: Some("agent10_bound_ack"),
            receipt_id: "agent10_bound_receipt",
            tokens: 100,
            cost_microusd: 250,
        },
    ).unwrap();
    assert_eq!(settled.status, ExternalEffectStatus::Completed);
    guard.verify().unwrap();
    drop(guard);
    let allowed = actual_os_process_probe(db.path(), study, false);
    assert!(allowed.contains("AGENT10_V13_WRITE_AFTER_RELEASE"));
}
