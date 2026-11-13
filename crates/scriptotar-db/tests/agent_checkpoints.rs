//! Real SQLite adapter certification: scoped runs, crash/restart, concurrent
//! revisions, immutable audits and late-failure transaction rollback.
use std::sync::{Arc, Barrier};
use std::thread;

use rusqlite::{params, Connection};
use scriptotar_agent::{
    AuditKind, CheckpointStore, Estimate, Limits, Plan, RunSnapshot, RunState, StepInput,
    StepPlan, StepRecord, StepState, StoreError, Usage,
};
use scriptotar_core::{Project, ProjectRepository};
use scriptotar_db::SqliteStore;
use tempfile::TempDir;
use uuid::Uuid;

fn fixture() -> (TempDir, SqliteStore, Project, Uuid) {
    let dir = TempDir::new().unwrap();
    let store = SqliteStore::open(dir.path().join("agent.sqlite3")).unwrap();
    let project = Project::new("Agent tests");
    store.create_project(&project).unwrap();
    let study = store.create_study(project.id, "Safe agent", "Original script").unwrap();
    (dir, store, project, study.id)
}

fn plan(project: Uuid, study: Uuid) -> Plan {
    Plan {
        id: Uuid::new_v4().to_string(),
        project_id: project.to_string(),
        study_id: study.to_string(),
        steps: vec![StepPlan {
            id: "read_1".into(),
            input: StepInput::ReadTranscript {
                job_id: "job_1".into(),
            },
            estimate: Estimate {
                tokens: 0,
                cost_microusd: 0,
                max_output_bytes: 1000,
                cpu_ms: 20,
                peak_memory_mib: 1,
            },
            max_attempts: 2,
            requires_approval: false,
        }],
        limits: Limits {
            max_tool_calls: 3,
            max_tokens: 100,
            max_cost_microusd: 500,
            max_elapsed_ms: 10000,
            max_output_bytes: 10000,
            max_cpu_ms: 1000,
            max_peak_memory_mib: 128,
        },
    }
}

fn initial(project: Uuid, study: Uuid) -> RunSnapshot {
    let plan = plan(project, study);
    let mut snapshot = RunSnapshot {
        schema_version: 1,
        steps: vec![StepRecord {
            id: "read_1".into(),
            state: StepState::Pending,
            attempts: 0,
            approved: false,
            idempotency_key: format!("{}:0:read_1", plan.id),
            pending_effect_id: None,
            output: None,
            failure_code: None,
        }],
        plan,
        revision: 0,
        state: RunState::Planned,
        current_step: 0,
        usage: Usage::default(),
        started_ms: 100,
        cancel_requested: false,
        pause_requested: false,
        events: vec![],
    };
    snapshot.event(100, AuditKind::Created);
    snapshot
}

fn later(old: &RunSnapshot) -> RunSnapshot {
    let mut snapshot = old.clone();
    snapshot.revision += 1;
    snapshot.state = RunState::Paused;
    snapshot.event(120 + snapshot.revision, AuditKind::Paused);
    snapshot
}

#[test]
fn scoped_checkpoint_survives_restart_with_exact_append_only_history() {
    let (temp, store, project, study) = fixture();
    let scoped = store.scoped_agent_checkpoints(project.id, study).unwrap();
    let run = initial(project.id, study);
    scoped.create(run.clone()).unwrap();
    let saved = later(&run);
    scoped.compare_and_swap(0, saved.clone()).unwrap();
    assert_eq!(scoped.load(&run.plan.id).unwrap(), Some(saved.clone()));
    drop(store);
    drop(scoped);
    let reopened = SqliteStore::open(temp.path().join("agent.sqlite3")).unwrap();
    assert_eq!(reopened.schema_version().unwrap(), 7);
    let same_scope = reopened.scoped_agent_checkpoints(project.id, study).unwrap();
    assert_eq!(same_scope.load(&run.plan.id).unwrap(), Some(saved.clone()));
    assert_eq!(same_scope.create(run), Err(StoreError::Exists));
    let conn = Connection::open(reopened.path()).unwrap();
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM agent_audit_events WHERE run_id=?1",
            params![saved.plan.id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, saved.events.len() as i64);
}

#[test]
fn wrong_project_and_study_scope_cannot_read_or_mutate_run() {
    let (_temp, store, project, study) = fixture();
    let other = Project::new("Other project");
    store.create_project(&other).unwrap();
    let other_study = store.create_study(other.id, "Other", "").unwrap();
    assert!(store.scoped_agent_checkpoints(other.id, study).is_err());
    let owner = store.scoped_agent_checkpoints(project.id, study).unwrap();
    let foreign = store
        .scoped_agent_checkpoints(other.id, other_study.id)
        .unwrap();
    let run = initial(project.id, study);
    owner.create(run.clone()).unwrap();
    assert_eq!(foreign.load(&run.plan.id).unwrap(), None);
    assert_eq!(foreign.create(run.clone()), Err(StoreError::Conflict));
    assert_eq!(
        foreign.compare_and_swap(0, later(&run)),
        Err(StoreError::Conflict)
    );
    assert_eq!(owner.load(&run.plan.id).unwrap(), Some(run));
}

#[test]
fn stale_or_tampered_checkpoint_cannot_rewrite_audit_history() {
    let (_temp, store, project, study) = fixture();
    let scoped = store.scoped_agent_checkpoints(project.id, study).unwrap();
    let run = initial(project.id, study);
    scoped.create(run.clone()).unwrap();
    let updated = later(&run);
    scoped.compare_and_swap(0, updated.clone()).unwrap();
    assert_eq!(
        scoped.compare_and_swap(0, later(&run)),
        Err(StoreError::Conflict)
    );
    let mut rewritten = later(&updated);
    rewritten.events[0].at_ms += 1;
    assert_eq!(
        scoped.compare_and_swap(1, rewritten),
        Err(StoreError::Conflict)
    );
    assert_eq!(scoped.load(&run.plan.id).unwrap(), Some(updated));
}

#[test]
fn late_audit_insert_failure_rolls_back_checkpoint_revision_and_state() {
    let (_temp, store, project, study) = fixture();
    let scoped = store.scoped_agent_checkpoints(project.id, study).unwrap();
    let run = initial(project.id, study);
    scoped.create(run.clone()).unwrap();
    let conn = Connection::open(store.path()).unwrap();
    conn.execute_batch(
        "CREATE TRIGGER agent_test_fail_audit BEFORE INSERT ON agent_audit_events
         BEGIN SELECT RAISE(ABORT, 'simulated audit write failure'); END;",
    )
    .unwrap();
    assert_eq!(
        scoped.compare_and_swap(0, later(&run)),
        Err(StoreError::Unavailable)
    );
    assert_eq!(scoped.load(&run.plan.id).unwrap(), Some(run.clone()));
    conn.execute_batch("DROP TRIGGER agent_test_fail_audit;")
        .unwrap();
    let saved = later(&run);
    scoped.compare_and_swap(0, saved.clone()).unwrap();
    assert_eq!(scoped.load(&run.plan.id).unwrap(), Some(saved));
}

#[test]
fn direct_sql_audit_edits_are_denied_and_corrupt_snapshot_fails_closed() {
    let (_temp, store, project, study) = fixture();
    let scoped = store.scoped_agent_checkpoints(project.id, study).unwrap();
    let run = initial(project.id, study);
    scoped.create(run.clone()).unwrap();
    let conn = Connection::open(store.path()).unwrap();
    assert!(conn
        .execute(
            "UPDATE agent_audit_events SET event_json='{}' WHERE run_id=?1",
            params![run.plan.id],
        )
        .is_err());
    conn.execute(
        "UPDATE agent_runs SET snapshot_json='{}' WHERE id=?1",
        params![run.plan.id],
    )
    .unwrap();
    assert_eq!(scoped.load(&run.plan.id), Err(StoreError::Unavailable));
}

#[test]
fn archived_study_blocks_checkpoint_creation_and_update() {
    let (_temp, store, project, study) = fixture();
    let scoped = store.scoped_agent_checkpoints(project.id, study).unwrap();
    let run = initial(project.id, study);
    scoped.create(run.clone()).unwrap();
    store.archive_study(project.id, study).unwrap();
    assert_eq!(
        scoped.compare_and_swap(0, later(&run)),
        Err(StoreError::Conflict)
    );
    let second = initial(project.id, study);
    assert_eq!(scoped.create(second), Err(StoreError::Conflict));
    assert_eq!(scoped.load(&run.plan.id).unwrap(), Some(run));
}

#[test]
fn simultaneous_writers_get_one_checkpoint_commit() {
    let (_temp, store, project, study) = fixture();
    let scoped = store.scoped_agent_checkpoints(project.id, study).unwrap();
    let run = initial(project.id, study);
    scoped.create(run.clone()).unwrap();
    let barrier = Arc::new(Barrier::new(3));
    let workers = (0..2)
        .map(|_| {
            let scoped = scoped.clone();
            let barrier = barrier.clone();
            let next = later(&run);
            thread::spawn(move || {
                barrier.wait();
                scoped.compare_and_swap(0, next)
            })
        })
        .collect::<Vec<_>>();
    barrier.wait();
    let results = workers
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(results.iter().filter(|result| result == &&Err(StoreError::Conflict)).count(), 1);
    assert_eq!(scoped.load(&run.plan.id).unwrap().unwrap().revision, 1);
}
