//! Isolated, fail-closed research-to-script coordinator. No network, Tauri, or SQLite here.
//! Driver implementations MUST call existing project-scoped, policy-checked AppServices.
pub mod model;
pub mod store;

pub use model::*;
pub use store::{CheckpointStore, MemoryStore, StoreError};

use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeError {
    InvalidPlan,
    NotFound,
    WrongState,
    StepMismatch,
    ToolNotRegistered,
    PolicyDenied,
    Store(StoreError),
}

impl From<StoreError> for RuntimeError {
    fn from(e: StoreError) -> Self {
        Self::Store(e)
    }
}

/// Tool registration is deny-by-default. This is NOT an MCP execute-command bridge.
#[derive(Debug, Clone, Default)]
pub struct ToolRegistry {
    allowed: BTreeSet<ToolKind>,
}

impl ToolRegistry {
    pub fn allow(&mut self, kind: ToolKind) {
        self.allowed.insert(kind);
    }

    pub fn contains(&self, kind: ToolKind) -> bool {
        self.allowed.contains(&kind)
    }
}

/// Must recheck project/study ownership, provider permissions and egress policy at dispatch.
pub trait PolicyGate: Send + Sync {
    fn authorize(&self, plan: &Plan, step: &StepPlan) -> Result<(), RuntimeError>;
}

/// Never persist credentials or raw provider output in a checkpoint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionContext {
    pub run_id: String,
    pub project_id: String,
    pub study_id: String,
    pub step_id: String,
    pub idempotency_key: String,
    /// AppServices must enforce this deadline on in-flight I/O and computation.
    pub deadline_ms: u64,
    pub estimate: Estimate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolReceipt {
    /// The underlying operation has finished, not merely been submitted.
    Applied(StepOutput),
    /// A durable service-side operation/job was accepted; it is NOT completed.
    Accepted { effect_id: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProbeResult {
    Applied(StepOutput),
    Pending,
    /// Terminal failure reported by the actual queued job or provider ledger.
    Failed(ToolFailure),
    /// Service proves that the operation did not happen; retry may be safe.
    NotApplied,
    /// Missing or ambiguous evidence; never automatically retry.
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Certainty {
    NotApplied,
    UnknownEffect,
}

/// A public error *code*, never an exception body, authorization header or model response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolFailure {
    pub code: String,
    pub retryable: bool,
    pub certainty: Certainty,
}

pub trait ToolExecutor: Send + Sync {
    fn execute(
        &self,
        context: &ExecutionContext,
        input: &StepInput,
    ) -> Result<ToolReceipt, ToolFailure>;

    /// Query the existing service's idempotency/job ledger. Never redo an effect here.
    fn probe(
        &self,
        context: &ExecutionContext,
        input: &StepInput,
        pending_effect_id: Option<&str>,
    ) -> Result<ProbeResult, ToolFailure>;

    /// Best-effort stop signal; a cancellation is not settled until probe confirms the effect.
    fn request_cancel(
        &self,
        _context: &ExecutionContext,
        _input: &StepInput,
        _pending_effect_id: Option<&str>,
    ) -> Result<(), ToolFailure> {
        Err(ToolFailure { code: "cancel_unavailable".into(), retryable: false, certainty: Certainty::UnknownEffect })
    }
}

pub struct Engine<S, D, P> {
    pub store: S,
    pub driver: D,
    pub policy: P,
    pub registry: ToolRegistry,
}

impl<S: CheckpointStore, D: ToolExecutor, P: PolicyGate> Engine<S, D, P> {
    pub fn new(store: S, driver: D, policy: P, registry: ToolRegistry) -> Self {
        Self { store, driver, policy, registry }
    }

    pub fn get(&self, id: &str) -> Result<RunSnapshot, RuntimeError> {
        let run = self.store.load(id)?.ok_or(RuntimeError::NotFound)?;
        if run.schema_version != 1
            || !run.plan.validate()
            || run.steps.len() != run.plan.steps.len()
            || run.current_step > run.steps.len()
            || run.steps.iter().zip(&run.plan.steps).any(|(record, step)| record.id != step.id)
            || run.steps[..run.current_step].iter().any(|step| step.state != StepState::Completed)
            || (run.state == RunState::Completed && run.current_step != run.steps.len())
            || run.usage.tool_calls > run.plan.limits.max_tool_calls
            || run.usage.reserved_tokens > run.plan.limits.max_tokens
            || run.usage.reserved_cost_microusd > run.plan.limits.max_cost_microusd
            || run.usage.reserved_cpu_ms > run.plan.limits.max_cpu_ms
        {
            return Err(RuntimeError::InvalidPlan);
        }
        Ok(run)
    }

    pub fn create(&self, plan: Plan, now_ms: u64) -> Result<RunSnapshot, RuntimeError> {
        if !plan.validate() {
            return Err(RuntimeError::InvalidPlan);
        }
        let steps = plan.steps.iter().enumerate().map(|(index, step)| StepRecord {
            id: step.id.clone(),
            state: StepState::Pending,
            attempts: 0,
            approved: false,
            // The stable key survives retry and process restart; service must deduplicate it.
            idempotency_key: format!("{}:{}:{}", plan.id, index, step.id),
            pending_effect_id: None,
            output: None,
            failure_code: None,
        }).collect();
        let mut run = RunSnapshot {
            schema_version: 1,
            plan,
            revision: 0,
            state: RunState::Planned,
            current_step: 0,
            steps,
            usage: Usage::default(),
            started_ms: now_ms,
            cancel_requested: false,
            pause_requested: false,
            events: Vec::new(),
        };
        run.event(now_ms, AuditKind::Created);
        self.store.create(run.clone())?;
        Ok(run)
    }

    pub fn approve(&self, run_id: &str, step_id: &str, now: u64) -> Result<RunSnapshot, RuntimeError> {
        let mut run = self.get(run_id)?;
        if run.state != RunState::AwaitingApproval {
            return Err(RuntimeError::WrongState);
        }
        let current = run.current_step;
        if run.steps[current].id != step_id {
            return Err(RuntimeError::StepMismatch);
        }
        run.steps[current].approved = true;
        run.state = RunState::Running;
        run.event(now, AuditKind::Approved);
        self.save(run)
    }

    pub fn deny(&self, run_id: &str, step_id: &str, now: u64) -> Result<RunSnapshot, RuntimeError> {
        let mut run = self.get(run_id)?;
        if run.state != RunState::AwaitingApproval {
            return Err(RuntimeError::WrongState);
        }
        let current = run.current_step;
        if run.steps[current].id != step_id {
            return Err(RuntimeError::StepMismatch);
        }
        run.steps[current].state = StepState::Cancelled;
        run.state = RunState::Cancelled;
        run.event(now, AuditKind::Denied);
        self.save(run)
    }

    pub fn pause(&self, run_id: &str, now: u64) -> Result<RunSnapshot, RuntimeError> {
        let mut run = self.get(run_id)?;
        if run.state.terminal() {
            return Err(RuntimeError::WrongState);
        }
        if matches!(run.state, RunState::Cancelling | RunState::Blocked) {
            return Err(RuntimeError::WrongState);
        }
        let executing = run.steps.get(run.current_step).is_some_and(|step| step.state == StepState::Running);
        if executing {
            run.pause_requested = true;
        } else {
            run.state = RunState::Paused;
        }
        run.event(now, AuditKind::Paused);
        self.save(run)
    }

    pub fn resume(&self, run_id: &str, now: u64) -> Result<RunSnapshot, RuntimeError> {
        let mut run = self.get(run_id)?;
        if run.state != RunState::Paused {
            return Err(RuntimeError::WrongState);
        }
        run.pause_requested = false;
        run.state = if run.steps.get(run.current_step).is_some_and(|s| s.state == StepState::AwaitingResult) {
            RunState::NeedsInput
        } else {
            RunState::Running
        };
        run.event(now, AuditKind::Resumed);
        self.save(run)
    }

    pub fn cancel(&self, run_id: &str, now: u64) -> Result<RunSnapshot, RuntimeError> {
        let mut run = self.get(run_id)?;
        if run.state.terminal() {
            return Ok(run);
        }
        let running = run.steps.get(run.current_step).is_some_and(|step| {
            matches!(step.state, StepState::Running | StepState::AwaitingResult)
        });
        run.cancel_requested = true;
        run.state = if running { RunState::Cancelling } else { RunState::Cancelled };
        if !running {
            if let Some(step) = run.steps.get_mut(run.current_step) {
                step.state = StepState::Cancelled;
            }
        }
        run.event(now, if running { AuditKind::CancelRequested } else { AuditKind::Cancelled });
        let run = self.save(run)?;
        if running {
            // Request cancellation only after recording intent. The adapter must check policy again.
            let ctx = context(&run);
            let step = &run.plan.steps[run.current_step];
            let effect = run.steps[run.current_step].pending_effect_id.as_deref();
            let _ = self.driver.request_cancel(&ctx, &step.input, effect);
        }
        Ok(run)
    }

    /// Must run at startup before issuing any new work; uncertain effects require reconciliation.
    pub fn recover(&self, run_id: &str, now: u64) -> Result<RunSnapshot, RuntimeError> {
        let mut run = self.get(run_id)?;
        if run.state.terminal() {
            return Ok(run);
        }
        // Even a paused run might have a tool in flight at shutdown.
        // The next session must probe, never issue a duplicate request.
        if run.steps.get(run.current_step).is_some_and(|s| s.state == StepState::Running)
            || (matches!(run.state, RunState::Running | RunState::Cancelling)
                && run.steps.get(run.current_step).is_some_and(|s| s.state == StepState::AwaitingResult))
        {
            run.state = RunState::Blocked;
            run.event(now, AuditKind::Interrupted);
            return self.save(run);
        }
        if run.cancel_requested {
            if run.steps.get(run.current_step).is_some_and(|s| s.state == StepState::AwaitingResult) {
                run.state = RunState::Blocked;
                run.event(now, AuditKind::Interrupted);
            } else {
                run.state = RunState::Cancelled;
                run.event(now, AuditKind::Cancelled);
            }
            return self.save(run);
        }
        Ok(run)
    }

    /// Dispatch at most one step. An intent CAS is committed before invoking the tool.
    pub fn tick(&self, run_id: &str, now: u64) -> Result<RunSnapshot, RuntimeError> {
        let mut run = self.get(run_id)?;
        if !matches!(run.state, RunState::Planned | RunState::Running) {
            return Ok(run);
        }
        if run.current_step >= run.steps.len() {
            run.state = RunState::Completed;
            run.event(now, AuditKind::Completed);
            return self.save(run);
        }
        let index = run.current_step;
        if run.steps[index].state != StepState::Pending {
            // A process may still be executing this step. Never dispatch it twice.
            return Ok(run);
        }
        if run.steps[index].attempts >= run.plan.steps[index].max_attempts {
            run.steps[index].state = StepState::Failed;
            run.state = RunState::Failed;
            run.event(now, AuditKind::Failed);
            return self.save(run);
        }
        let step = run.plan.steps[index].clone();
        if !self.registry.contains(step.input.kind()) {
            run.state = RunState::Blocked;
            run.event(now, AuditKind::Blocked);
            return self.save(run);
        }
        if step.approval_required() && !run.steps[index].approved {
            run.state = RunState::AwaitingApproval;
            run.event(now, AuditKind::AwaitingApproval);
            return self.save(run);
        }
        if self.policy.authorize(&run.plan, &step).is_err() {
            run.state = RunState::Blocked;
            run.event(now, AuditKind::Blocked);
            return self.save(run);
        }
        let estimate = step.estimate;
        let over_limit = now < run.started_ms
            || now.saturating_sub(run.started_ms) > run.plan.limits.max_elapsed_ms
            || run.usage.tool_calls.checked_add(1).is_none_or(|n| n > run.plan.limits.max_tool_calls)
            || run.usage.reserved_tokens.checked_add(estimate.tokens).is_none_or(|n| n > run.plan.limits.max_tokens)
            || run.usage.reserved_cost_microusd.checked_add(estimate.cost_microusd).is_none_or(|n| n > run.plan.limits.max_cost_microusd)
            || run.usage.reserved_cpu_ms.checked_add(estimate.cpu_ms).is_none_or(|n| n > run.plan.limits.max_cpu_ms)
            || estimate.peak_memory_mib > run.plan.limits.max_peak_memory_mib;
        if over_limit {
            run.state = RunState::Blocked;
            run.event(now, AuditKind::LimitReached);
            return self.save(run);
        }
        run.usage.tool_calls += 1;
        run.usage.reserved_tokens += estimate.tokens;
        run.usage.reserved_cost_microusd += estimate.cost_microusd;
        run.usage.reserved_cpu_ms += estimate.cpu_ms;
        run.steps[index].state = StepState::Running;
        run.steps[index].attempts += 1;
        run.state = RunState::Running;
        run.event(now, AuditKind::Dispatched);
        let run = self.save(run)?; // CAS, then effect. Conflict means no execution.
        let context = context(&run);
        let receipt = self.driver.execute(&context, &step.input);
        self.finish(run_id, index, run.steps[index].attempts, now, receipt)
    }

    /// Poll/query the service-side ledger after queued work or a crash.
    /// Unknown effects stay blocked. No "execute" call is made from this method.
    pub fn reconcile(&self, run_id: &str, now: u64) -> Result<RunSnapshot, RuntimeError> {
        let run = self.get(run_id)?;
        let Some(record) = run.steps.get(run.current_step) else {
            return Ok(run);
        };
        if !matches!(record.state, StepState::Running | StepState::AwaitingResult) {
            return Err(RuntimeError::WrongState);
        }
        if run.state == RunState::Paused || run.state.terminal() {
            return Err(RuntimeError::WrongState);
        }
        let step = &run.plan.steps[run.current_step];
        if !self.registry.contains(step.input.kind()) || self.policy.authorize(&run.plan, step).is_err() {
            return Err(RuntimeError::PolicyDenied);
        }
        let ctx = context(&run);
        let result = self.driver.probe(&ctx, &step.input, record.pending_effect_id.as_deref());
        match result {
            Ok(ProbeResult::Applied(output)) => self.finish(run_id, run.current_step, record.attempts, now, Ok(ToolReceipt::Applied(output))),
            Ok(ProbeResult::NotApplied) => self.mark_not_applied(run_id, run.current_step, record.attempts, now),
            Ok(ProbeResult::Failed(failure)) => self.finish(run_id, run.current_step, record.attempts, now, Err(failure)),
            Ok(ProbeResult::Pending) => self.mark_pending(run_id, run.current_step, record.attempts, now, false),
            Ok(ProbeResult::Unknown) | Err(_) => self.mark_pending(run_id, run.current_step, record.attempts, now, true),
        }
    }

    fn finish(&self, run_id: &str, expected_step: usize, expected_attempt: u8, now: u64, receipt: Result<ToolReceipt, ToolFailure>) -> Result<RunSnapshot, RuntimeError> {
        // A cancellation/pause can CAS while the external service is executing.
        for _ in 0..16 {
            let mut run = self.get(run_id)?;
            // The original attempt and an async status probe can settle concurrently.
            // Never record a late receipt against a newly advanced step.
            if run.current_step != expected_step || run.steps[expected_step].attempts != expected_attempt {
                return Err(RuntimeError::WrongState);
            }
            let i = expected_step;
            if run.steps.get(i).is_none_or(|s| !matches!(s.state, StepState::Running | StepState::AwaitingResult)) {
                return Err(RuntimeError::WrongState);
            }
            let estimate = run.plan.steps[i].estimate;
            match &receipt {
                Ok(ToolReceipt::Applied(output))
                    if valid_id(&output.artifact_id)
                        && output.bytes <= estimate.max_output_bytes
                        && output.tokens <= estimate.tokens
                        && output.cost_microusd <= estimate.cost_microusd =>
                {
                    run.steps[i].output = Some(output.clone());
                    run.steps[i].state = StepState::Completed;
                    run.steps[i].pending_effect_id = None;
                    run.event(now, AuditKind::Completed);
                    run.current_step += 1;
                    run.state = if run.cancel_requested {
                        RunState::Cancelled
                    } else if run.current_step == run.steps.len() {
                        RunState::Completed
                    } else if run.pause_requested {
                        RunState::Paused
                    } else {
                        RunState::Running
                    };
                    if run.cancel_requested { run.event(now, AuditKind::Cancelled); }
                }
                Ok(ToolReceipt::Accepted { effect_id }) if valid_id(effect_id) => {
                    run.steps[i].state = StepState::AwaitingResult;
                    run.steps[i].pending_effect_id = Some(effect_id.clone());
                    run.state = if run.cancel_requested { RunState::Cancelling } else { RunState::NeedsInput };
                    run.event(now, AuditKind::Accepted);
                }
                Ok(_) => {
                    // The effect may have happened; exceeding the adapter contract is NOT safe to retry.
                    run.steps[i].state = StepState::AwaitingResult;
                    run.steps[i].failure_code = Some("invalid_receipt_or_budget".into());
                    run.state = RunState::Blocked;
                    run.event(now, AuditKind::Blocked);
                }
                Err(failure) if failure.certainty == Certainty::NotApplied => {
                    run.steps[i].failure_code = Some(safe_code(&failure.code));
                    if run.cancel_requested {
                        run.steps[i].state = StepState::Cancelled;
                        run.state = RunState::Cancelled;
                        run.event(now, AuditKind::Cancelled);
                    } else if failure.retryable && run.steps[i].attempts < run.plan.steps[i].max_attempts {
                        run.steps[i].state = StepState::Pending;
                        run.state = RunState::Paused;
                        run.event(now, AuditKind::RetryReady);
                    } else {
                        run.steps[i].state = StepState::Failed;
                        run.state = RunState::Failed;
                        run.event(now, AuditKind::Failed);
                    }
                }
                Err(failure) => {
                    run.steps[i].failure_code = Some(safe_code(&failure.code));
                    run.steps[i].state = StepState::AwaitingResult;
                    run.state = RunState::Blocked;
                    run.event(now, AuditKind::Blocked);
                }
            }
            match self.save(run) {
                Err(RuntimeError::Store(StoreError::Conflict)) => continue,
                Ok(committed) => {
                    // A cancellation may have raced with a newly accepted background job.
                    // Now the durable job/effect ID is available for service-side cancellation.
                    if committed.cancel_requested
                        && committed.steps.get(i).is_some_and(|s| s.state == StepState::AwaitingResult)
                    {
                        let ctx = context(&committed);
                        let step = &committed.plan.steps[i];
                        let effect = committed.steps[i].pending_effect_id.as_deref();
                        let _ = self.driver.request_cancel(&ctx, &step.input, effect);
                    }
                    return Ok(committed);
                }
                Err(error) => return Err(error),
            }
        }
        Err(RuntimeError::Store(StoreError::Conflict))
    }

    fn mark_not_applied(&self, id: &str, expected_step: usize, expected_attempt: u8, now: u64) -> Result<RunSnapshot, RuntimeError> {
        let mut run = self.get(id)?;
        if run.current_step != expected_step || run.steps[expected_step].attempts != expected_attempt {
            return Err(RuntimeError::WrongState);
        }
        let i = expected_step;
        if !matches!(run.steps[i].state, StepState::Running | StepState::AwaitingResult) {
            return Err(RuntimeError::WrongState);
        }
        if run.cancel_requested {
            run.steps[i].state = StepState::Cancelled;
            run.state = RunState::Cancelled;
            run.event(now, AuditKind::Cancelled);
        } else if run.steps[i].attempts < run.plan.steps[i].max_attempts {
            run.steps[i].state = StepState::Pending;
            run.steps[i].pending_effect_id = None;
            run.state = RunState::Paused;
            run.event(now, AuditKind::RetryReady);
        } else {
            run.steps[i].state = StepState::Failed;
            run.state = RunState::Failed;
            run.event(now, AuditKind::Failed);
        }
        self.save(run)
    }

    fn mark_pending(&self, id: &str, expected_step: usize, expected_attempt: u8, now: u64, unknown: bool) -> Result<RunSnapshot, RuntimeError> {
        let mut run = self.get(id)?;
        if run.current_step != expected_step || run.steps[expected_step].attempts != expected_attempt {
            return Err(RuntimeError::WrongState);
        }
        if !matches!(run.steps[expected_step].state, StepState::Running | StepState::AwaitingResult) {
            return Err(RuntimeError::WrongState);
        }
        let next = if unknown || run.cancel_requested { RunState::Blocked } else { RunState::NeedsInput };
        if run.state == next && run.steps[run.current_step].state == StepState::AwaitingResult {
            return Ok(run); // Polling must not grow a limitless audit stream.
        }
        run.steps[run.current_step].state = StepState::AwaitingResult;
        run.state = next;
        run.event(now, if unknown { AuditKind::Blocked } else { AuditKind::Reconciled });
        self.save(run)
    }

    fn save(&self, mut run: RunSnapshot) -> Result<RunSnapshot, RuntimeError> {
        let previous = run.revision;
        run.revision = previous.checked_add(1).ok_or(RuntimeError::InvalidPlan)?;
        self.store.compare_and_swap(previous, run.clone())?;
        Ok(run)
    }
}

fn context(run: &RunSnapshot) -> ExecutionContext {
    let step = &run.plan.steps[run.current_step];
    let record = &run.steps[run.current_step];
    ExecutionContext {
        run_id: run.plan.id.clone(),
        project_id: run.plan.project_id.clone(),
        study_id: run.plan.study_id.clone(),
        step_id: step.id.clone(),
        idempotency_key: record.idempotency_key.clone(),
        deadline_ms: run.started_ms.saturating_add(run.plan.limits.max_elapsed_ms),
        estimate: step.estimate,
    }
}

fn safe_code(s: &str) -> String {
    // Never copy arbitrary provider exceptions, tokens, URLs or user content into a checkpoint.
    match s {
        "provider_unavailable" | "provider_timeout" | "source_denied"
        | "not_found" | "effect_unknown" | "request_rejected" => s.to_owned(),
        _ => "tool_failure".to_owned(),
    }
}
