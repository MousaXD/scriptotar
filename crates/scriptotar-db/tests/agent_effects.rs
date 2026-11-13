//! Durable, project-scoped idempotency claims for future permissioned tools.
//! This tests the service-side fence only; it does not issue network requests.
use std::sync::{Arc, Barrier};
use std::thread;

use rusqlite::{params, Connection};
use scriptotar_agent::{
    AuditKind, CheckpointStore, Estimate, Limits, Plan, RunSnapshot, RunState, StepInput, StepPlan,
    StepRecord, StepState, Usage,
};
use scriptotar_core::{Project, ProjectRepository, RepositoryError};
use scriptotar_db::{EffectClaim, EffectStatus, EffectUpdate, SqliteStore};
use tempfile::TempDir;
use uuid::Uuid;

fn fixture() -> (TempDir, SqliteStore, Project, Uuid) {
    let dir = TempDir::new().unwrap();
    let db = SqliteStore::open(dir.path().join("effects.sqlite3")).unwrap();
    let project = Project::new("Run effects");
    db.create_project(&project).unwrap();
    let study = db.create_study(project.id, "Storyboard", "").unwrap();
    (dir, db, project, study.id)
}

fn running(db: &SqliteStore, project: Uuid, study: Uuid) -> RunSnapshot {
    let id = Uuid::new_v4().to_string();
    let step_id = "safe_step".to_owned();
    let plan = Plan {
        id: id.clone(),
        project_id: project.to_string(),
        study_id: study.to_string(),
        steps: vec![StepPlan {
            id: step_id.clone(),
            input: StepInput::ReadStudyTranscript {
                transcript_id: Uuid::new_v4().to_string(),
            },
            estimate: Estimate {
                tokens: 0,
                cost_microusd: 0,
                max_output_bytes: 64,
                cpu_ms: 1,
                peak_memory_mib: 1,
            },
            max_attempts: 1,
            requires_approval: true,
        }],
        limits: Limits {
            max_tool_calls: 1,
            max_tokens: 0,
            max_cost_microusd: 0,
            max_elapsed_ms: 60_000,
            max_output_bytes: 64,
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
            id: step_id.clone(),
            state: StepState::Pending,
            attempts: 0,
            approved: false,
            idempotency_key: format!("{id}:0:{step_id}"),
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
    let store = db.scoped_agent_checkpoints(project, study).unwrap();
    store.create(snapshot.clone()).unwrap();
    snapshot.revision = 1;
    snapshot.state = RunState::Running;
    snapshot.steps[0].state = StepState::Running;
    snapshot.steps[0].attempts = 1;
    snapshot.steps[0].approved = true;
    snapshot.usage.tool_calls = 1;
    snapshot.event(101, AuditKind::Dispatched);
    store.compare_and_swap(0, snapshot.clone()).unwrap();
    snapshot
}

#[test]
fn claim_is_unique_restart_safe_and_requires_persisted_intent() {
    let (dir, db, project, study) = fixture();
    let run = running(&db, project.id, study);
    let key = &run.steps[0].idempotency_key;
    let first = db
        .reserve_agent_effect(project.id, study, &run.plan.id, "safe_step", key)
        .unwrap();
    let EffectClaim::Claimed(claim) = first else {
        panic!("first claim was not unique");
    };
    assert_eq!(claim.status, EffectStatus::Reserved);
    assert_eq!(claim.project_id, project.id);
    assert_eq!(claim.revision, 1);
    assert!(matches!(
        db.reserve_agent_effect(project.id, study, &run.plan.id, "safe_step", key)
            .unwrap(),
        EffectClaim::Existing(_)
    ));
    drop(db);
    let reopened = SqliteStore::open(dir.path().join("effects.sqlite3")).unwrap();
    assert_eq!(reopened.schema_version().unwrap(), 8);
    assert_eq!(
        reopened
            .get_agent_effect(project.id, study, &run.plan.id, key)
            .unwrap(),
        Some(claim.clone())
    );
    assert_eq!(
        reopened
            .record_agent_effect(
                project.id,
                study,
                &run.plan.id,
                key,
                EffectUpdate::Applied {
                    artifact_id: "remote_receipt_1".into()
                }
            )
            .unwrap()
            .status,
        EffectStatus::Applied
    );
    assert!(matches!(
        reopened.reserve_agent_effect(project.id, study, &run.plan.id, "safe_step", key).unwrap(),
        EffectClaim::Existing(ref current) if current.status == EffectStatus::Applied
    ));
    assert!(matches!(
        reopened.record_agent_effect(
            project.id,
            study,
            &run.plan.id,
            key,
            EffectUpdate::Applied {
                artifact_id: "second".into()
            }
        ),
        Err(RepositoryError::Conflict(_))
    ));
}

#[test]
fn missing_approval_wrong_step_and_forged_keys_never_claim() {
    let (_dir, db, project, study) = fixture();
    let store = db.scoped_agent_checkpoints(project.id, study).unwrap();
    let planned = RunSnapshot {
        schema_version: 1,
        plan: Plan {
            id: Uuid::new_v4().to_string(),
            project_id: project.id.to_string(),
            study_id: study.to_string(),
            steps: vec![StepPlan {
                id: "step".into(),
                input: StepInput::ReadStudyTranscript {
                    transcript_id: Uuid::new_v4().to_string(),
                },
                estimate: Estimate {
                    tokens: 0,
                    cost_microusd: 0,
                    max_output_bytes: 20,
                    cpu_ms: 2,
                    peak_memory_mib: 1,
                },
                max_attempts: 1,
                requires_approval: true,
            }],
            limits: Limits {
                max_tool_calls: 1,
                max_tokens: 0,
                max_cost_microusd: 0,
                max_elapsed_ms: 5000,
                max_output_bytes: 20,
                max_cpu_ms: 20,
                max_peak_memory_mib: 4,
            },
        },
        revision: 0,
        state: RunState::Planned,
        current_step: 0,
        steps: vec![],
        usage: Usage::default(),
        started_ms: 100,
        cancel_requested: false,
        pause_requested: false,
        events: vec![],
    };
    let mut planned = planned;
    planned.steps.push(StepRecord {
        id: "step".into(),
        state: StepState::Pending,
        attempts: 0,
        approved: false,
        idempotency_key: format!("{}:0:step", planned.plan.id),
        pending_effect_id: None,
        output: None,
        failure_code: None,
    });
    planned.event(100, AuditKind::Created);
    store.create(planned.clone()).unwrap();
    for (step, key) in [
        ("step", planned.steps[0].idempotency_key.as_str()),
        ("forged", planned.steps[0].idempotency_key.as_str()),
        ("step", "forged:key"),
    ] {
        assert!(matches!(
            db.reserve_agent_effect(project.id, study, &planned.plan.id, step, key),
            Err(RepositoryError::Conflict(_))
        ));
    }
    assert!(db
        .get_agent_effect(
            project.id,
            study,
            &planned.plan.id,
            &planned.steps[0].idempotency_key
        )
        .unwrap()
        .is_none());
}

#[test]
fn foreign_projects_and_studies_cannot_probe_or_claim_other_run_keys() {
    let (_dir, db, project, study) = fixture();
    let run = running(&db, project.id, study);
    let key = &run.steps[0].idempotency_key;
    let other = Project::new("Other scope");
    db.create_project(&other).unwrap();
    let other_study = db.create_study(other.id, "Other storyboard", "").unwrap();
    assert!(db
        .get_agent_effect(other.id, other_study.id, &run.plan.id, key)
        .is_err());
    assert!(db
        .reserve_agent_effect(other.id, other_study.id, &run.plan.id, "safe_step", key)
        .is_err());
    let foreign_run = running(&db, other.id, other_study.id);
    assert!(matches!(
        db.reserve_agent_effect(
            other.id,
            other_study.id,
            &foreign_run.plan.id,
            "safe_step",
            key
        ),
        Err(RepositoryError::Conflict(_))
    ));
    assert!(db
        .reserve_agent_effect(project.id, other_study.id, &run.plan.id, "safe_step", key)
        .is_err());
}

#[test]
fn ambiguous_effect_cannot_replay_or_turn_back_into_accepted() {
    let (_dir, db, project, study) = fixture();
    let run = running(&db, project.id, study);
    let key = &run.steps[0].idempotency_key;
    db.reserve_agent_effect(project.id, study, &run.plan.id, "safe_step", key)
        .unwrap();
    let unknown = db
        .record_agent_effect(project.id, study, &run.plan.id, key, EffectUpdate::Unknown)
        .unwrap();
    assert_eq!(unknown.status, EffectStatus::Unknown);
    assert!(matches!(
        db.reserve_agent_effect(project.id, study, &run.plan.id, "safe_step", key),
        Ok(EffectClaim::Existing(_))
    ));
    assert!(matches!(
        db.record_agent_effect(
            project.id,
            study,
            &run.plan.id,
            key,
            EffectUpdate::Accepted {
                effect_id: "late_queue".into()
            }
        ),
        Err(RepositoryError::Conflict(_))
    ));
    // Only independent evidence of a settled effect may produce a terminal state.
    assert_eq!(
        db.record_agent_effect(
            project.id,
            study,
            &run.plan.id,
            key,
            EffectUpdate::NotApplied
        )
        .unwrap()
        .status,
        EffectStatus::NotApplied
    );
}

#[test]
fn effect_write_failure_rolls_back_old_state_and_retry_is_exactly_once() {
    let (_dir, db, project, study) = fixture();
    let run = running(&db, project.id, study);
    let key = &run.steps[0].idempotency_key;
    db.reserve_agent_effect(project.id, study, &run.plan.id, "safe_step", key)
        .unwrap();
    let conn = Connection::open(db.path()).unwrap();
    conn.execute_batch(
        "CREATE TRIGGER test_effect_write_failure BEFORE UPDATE ON agent_effects
         BEGIN SELECT RAISE(ABORT,'simulated service failure'); END;",
    )
    .unwrap();
    assert!(db
        .record_agent_effect(
            project.id,
            study,
            &run.plan.id,
            key,
            EffectUpdate::Accepted {
                effect_id: "job_1".into()
            }
        )
        .is_err());
    assert_eq!(
        db.get_agent_effect(project.id, study, &run.plan.id, key)
            .unwrap()
            .unwrap()
            .status,
        EffectStatus::Reserved
    );
    conn.execute_batch("DROP TRIGGER test_effect_write_failure;")
        .unwrap();
    let accepted = db
        .record_agent_effect(
            project.id,
            study,
            &run.plan.id,
            key,
            EffectUpdate::Accepted {
                effect_id: "job_1".into(),
            },
        )
        .unwrap();
    assert_eq!(accepted.status, EffectStatus::Accepted);
    assert_eq!(accepted.revision, 2);
    let settled = db
        .record_agent_effect(
            project.id,
            study,
            &run.plan.id,
            key,
            EffectUpdate::Applied {
                artifact_id: "artifact_1".into(),
            },
        )
        .unwrap();
    assert_eq!(settled.external_id.as_deref(), Some("job_1"));
    assert_eq!(settled.artifact_id.as_deref(), Some("artifact_1"));
    assert_eq!(settled.revision, 3);
    assert!(conn
        .execute(
            "UPDATE agent_effects SET status='reserved' WHERE idempotency_key=?1",
            params![key]
        )
        .is_err());
}

#[test]
fn two_claimers_share_one_durable_claim_and_no_other_dispatch() {
    let (_dir, db, project, study) = fixture();
    let run = running(&db, project.id, study);
    let key = run.steps[0].idempotency_key;
    let run_id = run.plan.id;
    let barrier = Arc::new(Barrier::new(3));
    let workers = (0..2)
        .map(|_| {
            let db = db.clone();
            let barrier = barrier.clone();
            let key = key.clone();
            let run_id = run_id.clone();
            thread::spawn(move || {
                barrier.wait();
                db.reserve_agent_effect(project.id, study, &run_id, "safe_step", &key)
                    .unwrap()
            })
        })
        .collect::<Vec<_>>();
    barrier.wait();
    let claims = workers
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        claims
            .iter()
            .filter(|value| matches!(value, EffectClaim::Claimed(_)))
            .count(),
        1
    );
    assert_eq!(
        claims
            .iter()
            .filter(|value| matches!(value, EffectClaim::Existing(_)))
            .count(),
        1
    );
}
