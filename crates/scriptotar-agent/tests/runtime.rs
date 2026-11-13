use scriptotar_agent::{
    model::*, Certainty, CheckpointStore, Engine, ExecutionContext, MemoryStore, PolicyGate,
    ProbeResult, RuntimeError, ToolExecutor, ToolFailure, ToolReceipt, ToolRegistry,
};
use std::{
    collections::VecDeque,
    sync::{atomic::{AtomicBool, Ordering}, Arc, Barrier, Mutex},
    thread,
};

#[derive(Default)]
struct FakeState {
    receipts: VecDeque<Result<ToolReceipt, ToolFailure>>,
    probes: VecDeque<Result<ProbeResult, ToolFailure>>,
    keys: Vec<String>,
    executions: usize,
    probes_count: usize,
}

#[derive(Clone, Default)]
struct FakeDriver {
    state: Arc<Mutex<FakeState>>,
    barrier: Option<Arc<Barrier>>,
}

impl FakeDriver {
    fn receipt(&self, value: Result<ToolReceipt, ToolFailure>) {
        self.state.lock().unwrap().receipts.push_back(value);
    }
    fn probe_result(&self, value: Result<ProbeResult, ToolFailure>) {
        self.state.lock().unwrap().probes.push_back(value);
    }
    fn calls(&self) -> usize {
        self.state.lock().unwrap().executions
    }
    fn keys(&self) -> Vec<String> {
        self.state.lock().unwrap().keys.clone()
    }
}

impl ToolExecutor for FakeDriver {
    fn execute(&self, context: &ExecutionContext, _: &StepInput) -> Result<ToolReceipt, ToolFailure> {
        let result = {
            let mut state = self.state.lock().unwrap();
            state.executions += 1;
            state.keys.push(context.idempotency_key.clone());
            state.receipts.pop_front().unwrap_or_else(|| Ok(ToolReceipt::Applied(output("artifact_1"))))
        };
        if let Some(barrier) = &self.barrier {
            barrier.wait();
            barrier.wait();
        }
        result
    }

    fn probe(&self, _: &ExecutionContext, _: &StepInput, _: Option<&str>) -> Result<ProbeResult, ToolFailure> {
        let mut state = self.state.lock().unwrap();
        state.probes_count += 1;
        state.probes.pop_front().unwrap_or(Ok(ProbeResult::Unknown))
    }
}

#[derive(Clone)]
struct FakePolicy(Arc<AtomicBool>);

impl Default for FakePolicy {
    fn default() -> Self { Self(Arc::new(AtomicBool::new(true))) }
}
impl PolicyGate for FakePolicy {
    fn authorize(&self, _: &Plan, _: &StepPlan) -> Result<(), RuntimeError> {
        if self.0.load(Ordering::SeqCst) { Ok(()) } else { Err(RuntimeError::PolicyDenied) }
    }
}

type TestEngine = Engine<MemoryStore, FakeDriver, FakePolicy>;

fn output(id: &str) -> StepOutput {
    StepOutput { artifact_id: id.into(), tokens: 0, cost_microusd: 0, bytes: 1 }
}

fn step(id: &str) -> StepPlan {
    StepPlan {
        id: id.into(),
        input: StepInput::SelectResearch { source_ids: vec!["source_1".into()] },
        estimate: Estimate { tokens: 0, cost_microusd: 0, max_output_bytes: 1000, cpu_ms: 1, peak_memory_mib: 1 },
        max_attempts: 2,
        requires_approval: false,
    }
}

fn paid_step() -> StepPlan {
    StepPlan {
        id: "s1".into(),
        input: StepInput::DraftScript { brief_id: "brief_1".into() },
        estimate: Estimate { tokens: 10, cost_microusd: 50, max_output_bytes: 1000, cpu_ms: 1, peak_memory_mib: 1 },
        max_attempts: 2,
        requires_approval: true,
    }
}

fn plan(steps: Vec<StepPlan>) -> Plan {
    Plan {
        id: "run_1".into(),
        project_id: "project_1".into(),
        study_id: "study_1".into(),
        steps,
        limits: Limits {
            max_tool_calls: 3, max_tokens: 100, max_cost_microusd: 100,
            max_elapsed_ms: 1000, max_output_bytes: 1000, max_cpu_ms: 10, max_peak_memory_mib: 10,
        },
    }
}

fn engine(driver: FakeDriver) -> TestEngine {
    let mut registry = ToolRegistry::default();
    for kind in [
        ToolKind::DiscoverResearch, ToolKind::SelectResearch, ToolKind::QueueTranscript,
        ToolKind::ReadTranscript, ToolKind::AnalyzeEvidence, ToolKind::DraftScript,
        ToolKind::ReviseScript, ToolKind::SaveArtifact,
    ] { registry.allow(kind); }
    Engine::new(MemoryStore::default(), driver, FakePolicy::default(), registry)
}

fn not_applied(retryable: bool) -> ToolFailure {
    ToolFailure { code: "provider_unavailable".into(), retryable, certainty: Certainty::NotApplied }
}

#[test]
fn create_does_not_dispatch_and_plan_is_immutable() {
    let d = FakeDriver::default();
    let e = engine(d.clone());
    let r = e.create(plan(vec![step("s1")]), 0).unwrap();
    assert_eq!(r.state, RunState::Planned);
    assert_eq!(d.calls(), 0);
    assert_eq!(r.events.len(), 1);
    assert_eq!(e.create(plan(vec![step("s1")]), 0), Err(RuntimeError::Store(scriptotar_agent::StoreError::Exists)));
    assert_eq!(r.plan.project_id, "project_1");
}

#[test]
fn malformed_plans_and_duplicate_step_ids_are_rejected() {
    let e = engine(FakeDriver::default());
    for p in [
        plan(vec![]),
        plan(vec![step("same"), step("same")]),
        plan(vec![StepPlan { max_attempts: 0, ..step("s1") }]),
        plan(vec![StepPlan { input: StepInput::DiscoverResearch { query: "".into() }, ..step("s1") }]),
    ] { assert_eq!(e.create(p, 0), Err(RuntimeError::InvalidPlan)); }
}

#[test]
fn immediate_action_completes_only_after_applied_receipt() {
    let d = FakeDriver::default();
    let e = engine(d.clone());
    e.create(plan(vec![step("s1")]), 0).unwrap();
    let r = e.tick("run_1", 1).unwrap();
    assert_eq!(r.state, RunState::Completed);
    assert_eq!(r.steps[0].state, StepState::Completed);
    assert_eq!(r.steps[0].output.as_ref().unwrap().artifact_id, "artifact_1");
    assert_eq!(d.calls(), 1);
    e.tick("run_1", 2).unwrap();
    assert_eq!(d.calls(), 1);
}

#[test]
fn paid_action_requires_explicit_approval_and_exact_step() {
    let d = FakeDriver::default();
    let e = engine(d.clone());
    e.create(plan(vec![paid_step()]), 0).unwrap();
    assert_eq!(e.tick("run_1", 1).unwrap().state, RunState::AwaitingApproval);
    assert_eq!(d.calls(), 0);
    assert_eq!(e.approve("run_1", "wrong_step", 2), Err(RuntimeError::StepMismatch));
    e.approve("run_1", "s1", 3).unwrap();
    d.receipt(Ok(ToolReceipt::Applied(StepOutput {
        artifact_id: "script_1".into(), tokens: 8, cost_microusd: 40, bytes: 100,
    })));
    assert_eq!(e.tick("run_1", 4).unwrap().state, RunState::Completed);
    assert_eq!(d.calls(), 1);
}

#[test]
fn approval_denial_stops_all_effects() {
    let d = FakeDriver::default();
    let e = engine(d.clone());
    e.create(plan(vec![paid_step()]), 0).unwrap();
    e.tick("run_1", 1).unwrap();
    assert_eq!(e.deny("run_1", "s1", 2).unwrap().state, RunState::Cancelled);
    e.tick("run_1", 3).unwrap();
    assert_eq!(d.calls(), 0);
}

#[test]
fn queued_is_not_completed_until_service_proves_completion() {
    let d = FakeDriver::default();
    d.receipt(Ok(ToolReceipt::Accepted { effect_id: "job_1".into() }));
    d.probe_result(Ok(ProbeResult::Pending));
    d.probe_result(Ok(ProbeResult::Applied(output("transcript_1"))));
    let e = engine(d.clone());
    e.create(plan(vec![step("s1")]), 0).unwrap();
    let r = e.tick("run_1", 1).unwrap();
    assert_eq!(r.state, RunState::NeedsInput);
    assert_eq!(r.steps[0].state, StepState::AwaitingResult);
    assert!(r.steps[0].output.is_none());
    assert_eq!(e.reconcile("run_1", 2).unwrap().state, RunState::NeedsInput);
    let done = e.reconcile("run_1", 3).unwrap();
    assert_eq!(done.state, RunState::Completed);
    assert_eq!(done.steps[0].output.unwrap().artifact_id, "transcript_1");
    assert_eq!(d.calls(), 1);
}

#[test]
fn retry_requires_proof_of_no_effect_and_reuses_same_key() {
    let d = FakeDriver::default();
    d.receipt(Err(not_applied(true)));
    d.receipt(Ok(ToolReceipt::Applied(output("saved_1"))));
    let e = engine(d.clone());
    e.create(plan(vec![step("s1")]), 0).unwrap();
    assert_eq!(e.tick("run_1", 1).unwrap().state, RunState::Paused);
    assert_eq!(d.calls(), 1);
    assert_eq!(e.resume("run_1", 2).unwrap().state, RunState::Running);
    assert_eq!(e.tick("run_1", 3).unwrap().state, RunState::Completed);
    assert_eq!(d.keys(), vec!["run_1:0:s1", "run_1:0:s1"]);
}

#[test]
fn uncertain_provider_failure_is_blocked_without_retry() {
    let d = FakeDriver::default();
    d.receipt(Err(ToolFailure {
        code: "possible_duplicate_after_timeout".into(),
        retryable: true,
        certainty: Certainty::UnknownEffect,
    }));
    d.probe_result(Ok(ProbeResult::Unknown));
    let e = engine(d.clone());
    e.create(plan(vec![step("s1")]), 0).unwrap();
    assert_eq!(e.tick("run_1", 1).unwrap().state, RunState::Blocked);
    assert_eq!(e.reconcile("run_1", 2).unwrap().state, RunState::Blocked);
    assert_eq!(d.calls(), 1);
    assert_eq!(e.resume("run_1", 3), Err(RuntimeError::WrongState));
}

#[test]
fn permanent_provider_failure_is_failed_not_completed() {
    let d = FakeDriver::default();
    d.receipt(Err(not_applied(false)));
    let e = engine(d.clone());
    e.create(plan(vec![step("s1")]), 0).unwrap();
    let r = e.tick("run_1", 1).unwrap();
    assert_eq!(r.state, RunState::Failed);
    assert!(r.steps[0].output.is_none());
}

#[test]
fn policy_revocation_blocks_pending_dispatch() {
    let d = FakeDriver::default();
    let e = engine(d.clone());
    e.create(plan(vec![step("s1")]), 0).unwrap();
    e.policy.0.store(false, Ordering::SeqCst);
    assert_eq!(e.tick("run_1", 1).unwrap().state, RunState::Blocked);
    assert_eq!(d.calls(), 0);
}

#[test]
fn missing_tool_registration_is_fail_closed() {
    let d = FakeDriver::default();
    let e = Engine::new(MemoryStore::default(), d.clone(), FakePolicy::default(), ToolRegistry::default());
    e.create(plan(vec![step("s1")]), 0).unwrap();
    assert_eq!(e.tick("run_1", 1).unwrap().state, RunState::Blocked);
    assert_eq!(d.calls(), 0);
}

#[test]
fn call_token_cost_time_and_output_limits_block_safely() {
    for (limits, estimate, time) in [
        (Limits { max_tool_calls: 1, max_tokens: 100, max_cost_microusd: 100, max_elapsed_ms: 1000, max_output_bytes: 1000, max_cpu_ms: 10, max_peak_memory_mib: 10 },
            Estimate { tokens: 0, cost_microusd: 0, max_output_bytes: 1000, cpu_ms: 1, peak_memory_mib: 1 }, 1),
        (Limits { max_tool_calls: 3, max_tokens: 0, max_cost_microusd: 100, max_elapsed_ms: 1000, max_output_bytes: 1000, max_cpu_ms: 10, max_peak_memory_mib: 10 },
            Estimate { tokens: 1, cost_microusd: 0, max_output_bytes: 1000, cpu_ms: 1, peak_memory_mib: 1 }, 1),
        (Limits { max_tool_calls: 3, max_tokens: 100, max_cost_microusd: 0, max_elapsed_ms: 1000, max_output_bytes: 1000, max_cpu_ms: 10, max_peak_memory_mib: 10 },
            Estimate { tokens: 0, cost_microusd: 1, max_output_bytes: 1000, cpu_ms: 1, peak_memory_mib: 1 }, 1),
        (Limits { max_tool_calls: 3, max_tokens: 100, max_cost_microusd: 100, max_elapsed_ms: 1, max_output_bytes: 1000, max_cpu_ms: 10, max_peak_memory_mib: 10 },
            Estimate { tokens: 0, cost_microusd: 0, max_output_bytes: 1000, cpu_ms: 1, peak_memory_mib: 1 }, 2),
    ] {
        let d = FakeDriver::default();
        let e = engine(d.clone());
        let mut p = plan(vec![StepPlan { estimate, ..step("s1") }]);
        p.limits = limits;
        if limits.max_tool_calls == 1 {
            // First step completes; second step cannot dispatch.
            p.steps.push(step("s2"));
            e.create(p, 0).unwrap();
            assert_eq!(e.tick("run_1", 1).unwrap().state, RunState::Running);
            assert_eq!(e.tick("run_1", 2).unwrap().state, RunState::Blocked);
        } else {
            e.create(p, 0).unwrap();
            if estimate.cost_microusd > 0 {
                assert_eq!(e.tick("run_1", 1).unwrap().state, RunState::AwaitingApproval);
                e.approve("run_1", "s1", 1).unwrap();
            }
            assert_eq!(e.tick("run_1", time).unwrap().state, RunState::Blocked);
        }
    }
    let d = FakeDriver::default();
    d.receipt(Ok(ToolReceipt::Applied(StepOutput {
        bytes: 1001, ..output("artifact_1")
    })));
    let e = engine(d);
    e.create(plan(vec![step("s1")]), 0).unwrap();
    assert_eq!(e.tick("run_1", 1).unwrap().state, RunState::Blocked);
}

#[test]
fn cancellation_before_dispatch_cancels_step_immediately() {
    let d = FakeDriver::default();
    let e = engine(d.clone());
    e.create(plan(vec![step("s1")]), 0).unwrap();
    assert_eq!(e.cancel("run_1", 1).unwrap().state, RunState::Cancelled);
    e.tick("run_1", 2).unwrap();
    assert_eq!(d.calls(), 0);
}

#[test]
fn cancellation_race_records_late_real_effect_without_completing_run() {
    let barrier = Arc::new(Barrier::new(2));
    let d = FakeDriver { barrier: Some(barrier.clone()), ..Default::default() };
    let e = Arc::new(engine(d.clone()));
    e.create(plan(vec![step("s1"), step("s2")]), 0).unwrap();
    let e2 = e.clone();
    let task = thread::spawn(move || e2.tick("run_1", 1).unwrap());
    barrier.wait(); // Driver is running after checkpoint was committed.
    assert_eq!(e.cancel("run_1", 2).unwrap().state, RunState::Cancelling);
    barrier.wait();
    let returned = task.join().unwrap();
    assert_eq!(returned.state, RunState::Cancelled);
    assert_eq!(returned.steps[0].state, StepState::Completed);
    assert_eq!(returned.current_step, 1);
    assert_eq!(d.calls(), 1);
}

#[test]
fn pause_race_stops_following_steps_but_records_finished_one() {
    let barrier = Arc::new(Barrier::new(2));
    let d = FakeDriver { barrier: Some(barrier.clone()), ..Default::default() };
    let e = Arc::new(engine(d.clone()));
    e.create(plan(vec![step("s1"), step("s2")]), 0).unwrap();
    let e2 = e.clone();
    let task = thread::spawn(move || e2.tick("run_1", 1).unwrap());
    barrier.wait();
    e.pause("run_1", 2).unwrap();
    barrier.wait();
    assert_eq!(task.join().unwrap().state, RunState::Paused);
    assert_eq!(e.get("run_1").unwrap().current_step, 1);
    assert_eq!(d.calls(), 1);
}

#[test]
fn serializable_checkpoint_roundtrips_on_restart_without_reexecution() {
    let d = FakeDriver::default();
    d.receipt(Ok(ToolReceipt::Accepted { effect_id: "job_1".into() }));
    let e = engine(d.clone());
    e.create(plan(vec![step("s1")]), 0).unwrap();
    let pending = e.tick("run_1", 1).unwrap();
    let serialized = serde_json::to_string(&pending).unwrap();
    let persisted: RunSnapshot = serde_json::from_str(&serialized).unwrap();
    let restarted = Engine::new(MemoryStore::from_snapshots([persisted]), d.clone(), e.policy.clone(), e.registry.clone());
    assert_eq!(restarted.recover("run_1", 2).unwrap().state, RunState::NeedsInput);
    d.probe_result(Ok(ProbeResult::Applied(output("artifact_1"))));
    assert_eq!(restarted.reconcile("run_1", 3).unwrap().state, RunState::Completed);
    assert_eq!(d.calls(), 1);
}

#[test]
fn every_state_is_serializable_and_recovery_does_not_claim_false_success() {
    let d = FakeDriver::default();
    let e = engine(d.clone());
    let initial = e.create(plan(vec![step("s1")]), 0).unwrap();
    let variants = [
        RunState::Planned, RunState::AwaitingApproval, RunState::Running,
        RunState::Cancelling, RunState::Paused, RunState::NeedsInput,
        RunState::Blocked, RunState::Completed, RunState::Failed, RunState::Cancelled,
    ];
    for state in variants {
        let mut r = initial.clone();
        r.state = state;
        if matches!(state, RunState::Running | RunState::Cancelling | RunState::Blocked) {
            r.steps[0].state = StepState::Running;
        }
        if state == RunState::NeedsInput {
            r.steps[0].state = StepState::AwaitingResult;
            r.steps[0].pending_effect_id = Some("queued_1".into());
        }
        if state == RunState::Completed {
            r.steps[0].state = StepState::Completed;
            r.steps[0].output = Some(output("artifact_1"));
            r.current_step = 1;
        }
        if state == RunState::Failed { r.steps[0].state = StepState::Failed; }
        if state == RunState::Cancelled { r.steps[0].state = StepState::Cancelled; }
        let bytes = serde_json::to_vec(&r).unwrap();
        let decoded: RunSnapshot = serde_json::from_slice(&bytes).unwrap();
        let restarted = Engine::new(MemoryStore::from_snapshots([decoded]), d.clone(), e.policy.clone(), e.registry.clone());
        let recovered = restarted.recover("run_1", 5).unwrap();
        if matches!(state, RunState::Running | RunState::Cancelling) {
            assert_eq!(recovered.state, RunState::Blocked);
        } else {
            assert_eq!(recovered.state, state);
        }
    }
    assert_eq!(d.calls(), 0);
}

#[test]
fn partial_progress_is_preserved_before_later_failure() {
    let d = FakeDriver::default();
    d.receipt(Ok(ToolReceipt::Applied(output("first_1"))));
    d.receipt(Err(not_applied(false)));
    let e = engine(d.clone());
    e.create(plan(vec![step("s1"), step("s2")]), 0).unwrap();
    assert_eq!(e.tick("run_1", 1).unwrap().current_step, 1);
    let failed = e.tick("run_1", 2).unwrap();
    assert_eq!(failed.state, RunState::Failed);
    assert_eq!(failed.steps[0].state, StepState::Completed);
    assert_eq!(failed.steps[0].output.as_ref().unwrap().artifact_id, "first_1");
}

#[test]
fn persisted_failed_receipts_do_not_leak_tool_error_text() {
    let d = FakeDriver::default();
    d.receipt(Err(ToolFailure {
        code: "secret:key=ABC123 provider down".into(),
        retryable: false,
        certainty: Certainty::NotApplied,
    }));
    let e = engine(d);
    e.create(plan(vec![step("s1")]), 0).unwrap();
    let r = e.tick("run_1", 1).unwrap();
    assert_eq!(r.steps[0].failure_code.as_deref(), Some("tool_failure"));
    // Adapter contracts require standardized, secret-free codes; never persist the raw error.
    assert!(!serde_json::to_string(&r).unwrap().contains("ABC123"));
    assert!(serde_json::to_string(&r.events).unwrap().contains("failed"));
}

#[test]
fn duplicate_dispatch_cannot_cross_cas_claim() {
    let barrier = Arc::new(Barrier::new(2));
    let d = FakeDriver { barrier: Some(barrier.clone()), ..Default::default() };
    let e = Arc::new(engine(d.clone()));
    e.create(plan(vec![step("s1")]), 0).unwrap();
    let e2 = e.clone();
    let task = thread::spawn(move || e2.tick("run_1", 1).unwrap());
    barrier.wait();
    assert_eq!(e.tick("run_1", 2).unwrap().steps[0].state, StepState::Running);
    assert_eq!(d.calls(), 1);
    barrier.wait();
    assert_eq!(task.join().unwrap().state, RunState::Completed);
    assert_eq!(d.calls(), 1);
}

#[test]
fn ambiguous_crash_can_only_resume_after_safe_probe() {
    let d = FakeDriver::default();
    let e = engine(d.clone());
    let mut snapshot = e.create(plan(vec![step("s1")]), 0).unwrap();
    snapshot.state = RunState::Running;
    snapshot.steps[0].state = StepState::Running;
    let store = MemoryStore::from_snapshots([snapshot]);
    let isolated = Engine::new(store, d.clone(), e.policy.clone(), e.registry.clone());
    assert_eq!(isolated.recover("run_1", 1).unwrap().state, RunState::Blocked);
    assert_eq!(isolated.reconcile("run_1", 2).unwrap().state, RunState::Blocked);
    d.probe_result(Ok(ProbeResult::NotApplied));
    assert_eq!(isolated.reconcile("run_1", 3).unwrap().state, RunState::Paused);
    assert_eq!(d.calls(), 0);
}

#[test]
fn accepted_job_cancel_is_not_false_completion_and_reconciles_denial() {
    let d = FakeDriver::default();
    d.receipt(Ok(ToolReceipt::Accepted { effect_id: "job_1".into() }));
    d.probe_result(Ok(ProbeResult::NotApplied));
    let e = engine(d.clone());
    e.create(plan(vec![step("s1")]), 0).unwrap();
    assert_eq!(e.tick("run_1", 1).unwrap().state, RunState::NeedsInput);
    assert_eq!(e.cancel("run_1", 2).unwrap().state, RunState::Cancelling);
    assert_eq!(e.reconcile("run_1", 3).unwrap().state, RunState::Cancelled);
    assert_eq!(d.calls(), 1);
}

#[test]
fn resource_reservations_cap_cumulative_cpu_and_peak_memory() {
    let d = FakeDriver::default();
    let e = engine(d.clone());
    let mut p = plan(vec![step("s1"), step("s2")]);
    p.limits.max_cpu_ms = 1;
    e.create(p, 0).unwrap();
    assert_eq!(e.tick("run_1", 1).unwrap().state, RunState::Running);
    assert_eq!(e.tick("run_1", 2).unwrap().state, RunState::Blocked);
    assert_eq!(d.calls(), 1);

    let mut invalid = plan(vec![StepPlan {
        estimate: Estimate { peak_memory_mib: 11, ..step("s1").estimate },
        ..step("s1")
    }]);
    invalid.id = "other_run".into();
    assert_eq!(engine(FakeDriver::default()).create(invalid, 0), Err(RuntimeError::InvalidPlan));
}

#[test]
fn pending_service_step_can_pause_without_discarding_service_id() {
    let d = FakeDriver::default();
    d.receipt(Ok(ToolReceipt::Accepted { effect_id: "job_1".into() }));
    let e = engine(d.clone());
    e.create(plan(vec![step("s1")]), 0).unwrap();
    e.tick("run_1", 1).unwrap();
    assert_eq!(e.pause("run_1", 2).unwrap().state, RunState::Paused);
    assert_eq!(e.reconcile("run_1", 2), Err(RuntimeError::WrongState));
    assert_eq!(e.resume("run_1", 3).unwrap().state, RunState::NeedsInput);
    assert_eq!(e.get("run_1").unwrap().steps[0].pending_effect_id.as_deref(), Some("job_1"));
}

#[test]
fn corrupt_checkpoint_is_rejected_without_issuing_tools() {
    let d = FakeDriver::default();
    let e = engine(d.clone());
    let mut snapshot = e.create(plan(vec![step("s1")]), 0).unwrap();
    snapshot.current_step = usize::MAX;
    let restarted = Engine::new(MemoryStore::from_snapshots([snapshot]), d.clone(), e.policy.clone(), e.registry.clone());
    assert_eq!(restarted.recover("run_1", 1), Err(RuntimeError::InvalidPlan));
    assert_eq!(d.calls(), 0);
}

#[test]
fn queued_terminal_failure_uses_typed_service_evidence() {
    let d = FakeDriver::default();
    d.receipt(Ok(ToolReceipt::Accepted { effect_id: "job_1".into() }));
    d.probe_result(Ok(ProbeResult::Failed(not_applied(false))));
    let e = engine(d.clone());
    e.create(plan(vec![step("s1")]), 0).unwrap();
    assert_eq!(e.tick("run_1", 1).unwrap().state, RunState::NeedsInput);
    assert_eq!(e.reconcile("run_1", 2).unwrap().state, RunState::Failed);
    assert_eq!(d.calls(), 1);
}

#[test]
fn backwards_clock_movement_fails_closed_instead_of_resetting_time_limit() {
    let d = FakeDriver::default();
    let e = engine(d.clone());
    e.create(plan(vec![step("s1")]), 100).unwrap();
    assert_eq!(e.tick("run_1", 99).unwrap().state, RunState::Blocked);
    assert_eq!(d.calls(), 0);
}

#[test]
fn late_receipt_cannot_be_recorded_against_a_subsequent_step() {
    let barrier = Arc::new(Barrier::new(2));
    let d = FakeDriver { barrier: Some(barrier.clone()), ..Default::default() };
    d.probe_result(Ok(ProbeResult::Applied(output("first_from_probe"))));
    let e = Arc::new(engine(d.clone()));
    e.create(plan(vec![step("s1"), step("s2")]), 0).unwrap();

    let e2 = e.clone();
    let original = thread::spawn(move || e2.tick("run_1", 1));
    barrier.wait();
    let reconciled = e.reconcile("run_1", 2).unwrap();
    assert_eq!(reconciled.current_step, 1);
    assert_eq!(reconciled.steps[0].output.unwrap().artifact_id, "first_from_probe");
    barrier.wait();

    // The first attempt finished late; its receipt cannot mutate step two.
    assert_eq!(original.join().unwrap(), Err(RuntimeError::WrongState));
    let final_state = e.get("run_1").unwrap();
    assert_eq!(final_state.steps[1].state, StepState::Pending);
    assert!(final_state.steps[1].output.is_none());
    assert_eq!(d.calls(), 1);
}

#[test]
fn paused_in_flight_crash_never_restarts_tool_without_probe() {
    let d = FakeDriver::default();
    let e = engine(d.clone());
    let mut paused = e.create(plan(vec![step("s1")]), 0).unwrap();
    paused.state = RunState::Paused;
    paused.steps[0].state = StepState::Running;
    paused.steps[0].attempts = 1;
    let restored = Engine::new(MemoryStore::from_snapshots([paused]), d.clone(), e.policy.clone(), e.registry.clone());
    assert_eq!(restored.recover("run_1", 2).unwrap().state, RunState::Blocked);
    assert_eq!(d.calls(), 0);
}

#[test]
fn corrupted_exhausted_retry_count_fails_without_overflow_or_dispatch() {
    let d = FakeDriver::default();
    let e = engine(d.clone());
    let mut stalled = e.create(plan(vec![step("s1")]), 0).unwrap();
    stalled.steps[0].attempts = u8::MAX;
    let restored = Engine::new(MemoryStore::from_snapshots([stalled]), d.clone(), e.policy.clone(), e.registry.clone());
    assert_eq!(restored.tick("run_1", 1).unwrap().state, RunState::Failed);
    assert_eq!(d.calls(), 0);
}

#[test]
fn compound_id_separator_must_not_allow_colliding_idempotency_keys() {
    let d = FakeDriver::default();
    let e = engine(d);
    let mut unsafe_plan = plan(vec![step("s1")]);
    unsafe_plan.id = "a:0".into();
    assert_eq!(e.create(unsafe_plan, 0), Err(RuntimeError::InvalidPlan));
}

#[derive(Clone)]
struct RaceStore {
    inner: MemoryStore,
    ready: Arc<Barrier>,
}

impl CheckpointStore for RaceStore {
    fn create(&self, snapshot: RunSnapshot) -> Result<(), scriptotar_agent::StoreError> {
        self.inner.create(snapshot)
    }

    fn load(&self, run_id: &str) -> Result<Option<RunSnapshot>, scriptotar_agent::StoreError> {
        let snapshot = self.inner.load(run_id)?;
        if snapshot.as_ref().is_some_and(|run| run.revision == 0) {
            // Both competing ticks must claim the exact same persisted revision.
            self.ready.wait();
        }
        Ok(snapshot)
    }

    fn compare_and_swap(
        &self,
        expected_revision: u64,
        next: RunSnapshot,
    ) -> Result<(), scriptotar_agent::StoreError> {
        self.inner.compare_and_swap(expected_revision, next)
    }
}

#[test]
fn simultaneous_ticks_must_have_one_cas_winner_and_one_conflict() {
    let driver = FakeDriver::default();
    let template = engine(driver.clone());
    let store = RaceStore { inner: MemoryStore::default(), ready: Arc::new(Barrier::new(2)) };
    let e = Arc::new(Engine::new(store, driver.clone(), template.policy, template.registry));
    e.create(plan(vec![step("s1")]), 0).unwrap();

    let a = e.clone();
    let b = e.clone();
    let one = thread::spawn(move || a.tick("run_1", 1));
    let two = thread::spawn(move || b.tick("run_1", 1));
    let r1 = one.join().unwrap();
    let r2 = two.join().unwrap();
    let winners = [&r1, &r2].iter().filter(|r| matches!(r, Ok(s) if s.state == RunState::Completed)).count();
    let conflicts = [&r1, &r2].iter().filter(|r| matches!(r, Err(RuntimeError::Store(scriptotar_agent::StoreError::Conflict)))).count();
    assert_eq!(winners, 1);
    assert_eq!(conflicts, 1);
    assert_eq!(driver.calls(), 1);
}

#[test]
fn repeated_pause_and_cancel_are_idempotent() {
    let e = engine(FakeDriver::default());
    e.create(plan(vec![step("s1")]), 0).unwrap();
    let paused = e.pause("run_1", 1).unwrap();
    let paused_again = e.pause("run_1", 2).unwrap();
    assert_eq!(paused.revision, paused_again.revision);

    let canceled = e.cancel("run_1", 3).unwrap();
    let canceled_again = e.cancel("run_1", 4).unwrap();
    assert_eq!(canceled.revision, canceled_again.revision);
}

#[test]
fn forged_checkpoint_audit_sequence_and_missing_completed_artifact_are_rejected() {
    let d = FakeDriver::default();
    let e = engine(d.clone());
    let base = e.create(plan(vec![step("s1")]), 0).unwrap();
    let mut bad_audit = base.clone();
    bad_audit.events[0].sequence = 3;
    let store = MemoryStore::from_snapshots([bad_audit]);
    let reloaded = Engine::new(store, d.clone(), e.policy.clone(), e.registry.clone());
    assert_eq!(reloaded.get("run_1"), Err(RuntimeError::InvalidPlan));

    let mut missing_output = base;
    missing_output.state = RunState::Completed;
    missing_output.current_step = 1;
    missing_output.steps[0].state = StepState::Completed;
    let store = MemoryStore::from_snapshots([missing_output]);
    let reloaded = Engine::new(store, d.clone(), e.policy.clone(), e.registry.clone());
    assert_eq!(reloaded.get("run_1"), Err(RuntimeError::InvalidPlan));
    assert_eq!(d.calls(), 0);
}

#[test]
fn a_proven_failed_queued_attempt_clears_stale_job_locator_before_retry() {
    let d = FakeDriver::default();
    d.receipt(Ok(ToolReceipt::Accepted { effect_id: "old_job_1".into() }));
    d.probe_result(Ok(ProbeResult::Failed(not_applied(true))));
    let e = engine(d.clone());
    e.create(plan(vec![step("s1")]), 0).unwrap();
    assert_eq!(e.tick("run_1", 1).unwrap().state, RunState::NeedsInput);
    let paused = e.reconcile("run_1", 2).unwrap();
    assert_eq!(paused.state, RunState::Paused);
    assert_eq!(paused.steps[0].state, StepState::Pending);
    assert!(paused.steps[0].pending_effect_id.is_none());
    e.resume("run_1", 3).unwrap();
    d.receipt(Ok(ToolReceipt::Applied(output("committed_2"))));
    assert_eq!(e.tick("run_1", 4).unwrap().state, RunState::Completed);
    assert_eq!(d.calls(), 2);
}

#[test]
fn policy_restoration_can_reopen_never_dispatched_blocked_step() {
    let d = FakeDriver::default();
    let e = engine(d.clone());
    e.create(plan(vec![step("s1")]), 0).unwrap();
    e.policy.0.store(false, Ordering::SeqCst);
    assert_eq!(e.tick("run_1", 1).unwrap().state, RunState::Blocked);
    assert_eq!(e.recheck_blocked("run_1", 2), Err(RuntimeError::PolicyDenied));
    e.policy.0.store(true, Ordering::SeqCst);
    let opened = e.recheck_blocked("run_1", 3).unwrap();
    assert_eq!(opened.state, RunState::Running);
    assert_eq!(opened.steps[0].attempts, 0);
    assert_eq!(e.tick("run_1", 4).unwrap().state, RunState::Completed);
    assert_eq!(d.calls(), 1);
}

#[test]
fn uncertain_inflight_work_cannot_bypass_reconciliation_by_policy_recheck() {
    let d = FakeDriver::default();
    d.receipt(Err(ToolFailure {
        code: "provider_timeout".into(),
        retryable: true,
        certainty: Certainty::UnknownEffect,
    }));
    let e = engine(d.clone());
    e.create(plan(vec![step("s1")]), 0).unwrap();
    assert_eq!(e.tick("run_1", 1).unwrap().state, RunState::Blocked);
    assert_eq!(e.recheck_blocked("run_1", 2), Err(RuntimeError::WrongState));
    assert_eq!(d.calls(), 1);
}
