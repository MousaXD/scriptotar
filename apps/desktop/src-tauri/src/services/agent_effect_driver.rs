// Adapter seam for explicit provider policy and Agent 05 orchestration.
// Agent 00 owns any public IPC/shared provider authorization wiring.
use super::AppServices;
use scriptotar_agent::ExecutionContext;
use scriptotar_core::{RepositoryError, RepositoryResult};
use scriptotar_db::{ExternalEffect, ExternalEffectKey, ExternalEffectLimits,
    ExternalEffectStatus, ExternalReservation, ExternalResolution};
use uuid::Uuid;

/// Agent 01: revalidate user approval, budgets, source access and egress on
/// every dispatch, including expiry and revocation; never persist secrets.
pub trait ExternalEffectPolicy: Send + Sync {
    fn authorize(&self, ctx:&ExecutionContext, provider:&str, fingerprint:&str,
                 limits:ExternalEffectLimits) -> RepositoryResult<()>;
}

pub enum ProviderEvidence {
    /// Accepted/queued remotely, but NOT completed. Persist ID and probe.
    Accepted { request_id:String },
    Completed { request_id:Option<String>, receipt_id:String, tokens:u64, cost_microusd:u64 },
    Failed { request_id:Option<String>, receipt_id:String, tokens:u64, cost_microusd:u64 },
    Cancelled { evidence_id:String },
    NotExecuted { evidence_id:String },
    Uncertain,
}
impl ProviderEvidence {
    fn resolution(&self)->Option<ExternalResolution<'_>> {
        Some(match self {
            Self::Accepted { .. } => return None,
            Self::Completed {request_id,receipt_id,tokens,cost_microusd}=>
                ExternalResolution::Completed {provider_request_id:request_id.as_deref(),
                    receipt_id,tokens:*tokens,cost_microusd:*cost_microusd},
            Self::Failed {request_id,receipt_id,tokens,cost_microusd}=>
                ExternalResolution::Failed {provider_request_id:request_id.as_deref(),
                    receipt_id,tokens:*tokens,cost_microusd:*cost_microusd},
            Self::Cancelled {evidence_id}=>ExternalResolution::ProviderCancelled {evidence_id},
            Self::NotExecuted {evidence_id}=>ExternalResolution::VerifiedNotExecuted {evidence_id},
            Self::Uncertain=>ExternalResolution::Uncertain,
        })
    }
}

/// The actual approved provider adapter owns network I/O. A timeout, 5xx,
/// lost acknowledgement, or eventually consistent absence is UNCERTAIN.
pub trait ExternalEffectProvider: Send + Sync {
    fn dispatch_once(&self,ctx:&ExecutionContext,fingerprint:&str,attempt:i64,
                     limits:ExternalEffectLimits)->ProviderEvidence;
    fn probe(&self,effect:&ExternalEffect)->ProviderEvidence;
}

fn effect_key(ctx:&ExecutionContext)->RepositoryResult<ExternalEffectKey<'_>> {
    let not_found=||RepositoryError::NotFound("agent effect scope".into());
    Ok(ExternalEffectKey {
        project:Uuid::parse_str(&ctx.project_id).map_err(|_|not_found())?,
        study:Uuid::parse_str(&ctx.study_id).map_err(|_|not_found())?,
        run_id:&ctx.run_id,step_id:&ctx.step_id,idempotency_key:&ctx.idempotency_key,
    })
}

impl AppServices {
    /// A committed dispatched marker precedes provider I/O. Never send an
    /// Existing reservation, even when the previous caller crashed.
    pub fn dispatch_authorized_external_effect(
        &self,ctx:&ExecutionContext,provider_name:&str,fingerprint:&str,
        limits:ExternalEffectLimits,policy:&dyn ExternalEffectPolicy,
        provider:&dyn ExternalEffectProvider,
    )->RepositoryResult<ExternalEffect> {
        let key=effect_key(ctx)?;
        self.require_active_project(key.project)?;
        policy.authorize(ctx,provider_name,fingerprint,limits)?;
        let claim=self.store.reserve_external_effect(key,fingerprint,provider_name,limits)?;
        let ExternalReservation::Fresh(fresh)=claim else {
            return Err(RepositoryError::Conflict("existing paid effect: reconcile, do not replay".into()));
        };
        self.execute_fenced_external_attempt(ctx,provider_name,fingerprint,limits,
            policy,provider,fresh)
    }

    /// Explicit second/third attempt only. An earlier provider-confirmed
    /// NotExecuted receipt and manual prepare_external_after_verified_nonexecution
    /// MUST already have durably prepared this attempt. The normal "first call"
    /// path never dispatches an Existing reservation.
    pub fn dispatch_prepared_external_retry(
        &self,ctx:&ExecutionContext,provider_name:&str,fingerprint:&str,
        limits:ExternalEffectLimits,policy:&dyn ExternalEffectPolicy,
        provider:&dyn ExternalEffectProvider,
    )->RepositoryResult<ExternalEffect> {
        let key=effect_key(ctx)?;
        self.require_active_project(key.project)?;
        policy.authorize(ctx,provider_name,fingerprint,limits)?;
        let ready=self.store.get_external_effect(key)?
            .ok_or_else(||RepositoryError::NotFound("prepared external retry".into()))?;
        if ready.status!=ExternalEffectStatus::Reserved || ready.attempt<=1
            || ready.provider!=provider_name || ready.fingerprint!=fingerprint
            || ready.token_limit!=limits.tokens
            || ready.cost_limit_microusd!=limits.cost_microusd
            || ready.cancel_requested {
            return Err(RepositoryError::Conflict(
                "retry is not proven, prepared and authorized".into()
            ));
        }
        self.execute_fenced_external_attempt(ctx,provider_name,fingerprint,limits,
            policy,provider,ready)
    }

    fn execute_fenced_external_attempt(
        &self,ctx:&ExecutionContext,provider_name:&str,fingerprint:&str,
        limits:ExternalEffectLimits,policy:&dyn ExternalEffectPolicy,
        provider:&dyn ExternalEffectProvider,ready:ExternalEffect,
    )->RepositoryResult<ExternalEffect> {
        let key=effect_key(ctx)?;
        self.require_active_project(key.project)?;
        policy.authorize(ctx,provider_name,fingerprint,limits)?;
        let dispatched=self.store.mark_external_dispatched(key,ready.revision,ready.attempt)?;
        // This transaction is committed before ANY external call. If the app
        // crashes now, a later caller must reconcile rather than replay.
        // A pre-I/O revocation means the service can prove nonexecution locally.
        if self.require_active_project(key.project).is_err()
            || policy.authorize(ctx,provider_name,fingerprint,limits).is_err() {
            return self.store.reconcile_external_effect(key,dispatched.attempt,
                ExternalResolution::VerifiedNotExecuted {evidence_id:"local_pre_send_revoked"});
        }
        let current=self.store.get_external_effect(key)?
            .ok_or_else(||RepositoryError::Storage("external dispatch missing".into()))?;
        if current.cancel_requested {
            return self.store.reconcile_external_effect(key,current.attempt,
                ExternalResolution::VerifiedNotExecuted {evidence_id:"local_pre_send_cancelled"});
        }
        let evidence=provider.dispatch_once(ctx,fingerprint,dispatched.attempt,limits);
        match evidence {
            ProviderEvidence::Accepted {request_id} =>
                self.store.record_external_ack(key,dispatched.attempt,&request_id),
            outcome => self.store.reconcile_external_effect(
                key,dispatched.attempt,outcome.resolution().ok_or_else(
                    ||RepositoryError::Storage("unrecognized provider result".into())
                )?,
            ),
        }
    }

    /// Query the real provider; never repeat a request as a way of probing.
    pub fn reconcile_external_with_provider(
        &self,ctx:&ExecutionContext,provider:&dyn ExternalEffectProvider,
    )->RepositoryResult<ExternalEffect> {
        let key=effect_key(ctx)?;
        self.require_active_project(key.project)?;
        let old=self.store.get_external_effect(key)?
            .ok_or_else(||RepositoryError::NotFound("external effect".into()))?;
        if !matches!(old.status,ExternalEffectStatus::Dispatched | ExternalEffectStatus::Uncertain) {
            return Err(RepositoryError::Conflict("effect is not awaiting reconciliation".into()));
        }
        let evidence=provider.probe(&old);
        if let ProviderEvidence::Accepted {ref request_id}=evidence {
            return self.store.record_external_ack(key,old.attempt,request_id);
        }
        if matches!(evidence,ProviderEvidence::Uncertain)
            && old.status==ExternalEffectStatus::Uncertain {return Ok(old);}
        self.store.reconcile_external_effect(
            key,old.attempt,evidence.resolution().ok_or_else(
                ||RepositoryError::Storage("unrecognized provider result".into())
            )?,
        )
    }

    pub fn stop_external_effect(&self,ctx:&ExecutionContext)->RepositoryResult<ExternalEffect> {
        let key=effect_key(ctx)?;
        self.require_active_project(key.project)?;
        self.store.request_external_cancel(key)
    }

    /// Only after trusted provider evidence proves non-execution. The policy
    /// must approve again, and the DB caps retries to three total attempts.
    pub fn prepare_external_after_verified_nonexecution(
        &self,ctx:&ExecutionContext,provider_name:&str,fingerprint:&str,
        limits:ExternalEffectLimits,policy:&dyn ExternalEffectPolicy,
    )->RepositoryResult<ExternalEffect> {
        let key=effect_key(ctx)?;
        self.require_active_project(key.project)?;
        policy.authorize(ctx,provider_name,fingerprint,limits)?;
        let old=self.store.get_external_effect(key)?
            .ok_or_else(||RepositoryError::NotFound("external effect".into()))?;
        if old.provider!=provider_name || old.fingerprint!=fingerprint
            || old.token_limit!=limits.tokens || old.cost_limit_microusd!=limits.cost_microusd {
            return Err(RepositoryError::Conflict("request identity or budget changed".into()));
        }
        self.store.prepare_external_retry(key,old.revision)
    }
}


#[cfg(test)]
mod recovery_driver_tests {
    use super::*;
    use scriptotar_agent::{
        AuditKind, CheckpointStore, Estimate, Limits, Plan, RunSnapshot, RunState,
        StepInput, StepPlan, StepRecord, StepState, Usage,
    };
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tempfile::TempDir;

    struct ExplicitlyApprovedMock;
    impl ExternalEffectPolicy for ExplicitlyApprovedMock {
        fn authorize(
            &self, _ctx: &ExecutionContext, _provider: &str, _fingerprint: &str,
            _limits: ExternalEffectLimits,
        ) -> RepositoryResult<()> {
            Ok(())
        }
    }

    #[derive(Default)]
    struct LocalProviderDouble {
        calls: AtomicUsize,
        probes: AtomicUsize,
    }
    impl ExternalEffectProvider for LocalProviderDouble {
        fn dispatch_once(
            &self, _ctx: &ExecutionContext, _fingerprint: &str,
            _attempt: i64, _limits: ExternalEffectLimits,
        ) -> ProviderEvidence {
            self.calls.fetch_add(1, Ordering::SeqCst);
            ProviderEvidence::Accepted { request_id: "mock_provider_71".to_owned() }
        }
        fn probe(&self, _effect: &ExternalEffect) -> ProviderEvidence {
            self.probes.fetch_add(1, Ordering::SeqCst);
            ProviderEvidence::Completed {
                request_id: Some("mock_provider_71".to_owned()),
                receipt_id: "mock_receipt_71".to_owned(),
                tokens: 25,
                cost_microusd: 450,
            }
        }
    }

    fn approved_host() -> (TempDir, AppServices, ExecutionContext) {
        let directory = TempDir::new().unwrap();
        let services = AppServices::new(directory.path()).unwrap();
        let project = services.active_project_id().unwrap();
        let study = services.store.create_study(project, "External host fence", "").unwrap().id;
        let run_id = Uuid::new_v4().to_string();
        let estimate = Estimate {
            tokens: 500, cost_microusd: 2500, max_output_bytes: 1000,
            cpu_ms: 1, peak_memory_mib: 1,
        };
        let plan = Plan {
            id: run_id.clone(),
            project_id: project.to_string(),
            study_id: study.to_string(),
            steps: vec![StepPlan {
                id: "draft".into(),
                input: StepInput::DraftScript { brief_id: Uuid::new_v4().to_string() },
                estimate: estimate.clone(),
                max_attempts: 3,
                requires_approval: true,
            }],
            limits: Limits {
                max_tool_calls: 1, max_tokens: 500, max_cost_microusd: 2500,
                max_elapsed_ms: 60000, max_output_bytes: 1000,
                max_cpu_ms: 100, max_peak_memory_mib: 4,
            },
        };
        let mut snapshot = RunSnapshot {
            schema_version: 1, plan, revision: 0, state: RunState::Planned,
            current_step: 0,
            steps: vec![StepRecord {
                id: "draft".into(), state: StepState::Pending, attempts: 0,
                approved: false, idempotency_key: format!("{run_id}:0:draft"),
                pending_effect_id: None, output: None, failure_code: None,
            }],
            usage: Usage::default(),
            started_ms: 100, cancel_requested: false, pause_requested: false, events: vec![],
        };
        snapshot.event(100, AuditKind::Created);
        let checkpoints = services.store.scoped_agent_checkpoints(project, study).unwrap();
        checkpoints.create(snapshot.clone()).unwrap();
        snapshot.revision = 1;
        snapshot.state = RunState::Running;
        snapshot.steps[0].state = StepState::Running;
        snapshot.steps[0].attempts = 1;
        snapshot.steps[0].approved = true;
        snapshot.usage.tool_calls = 1;
        snapshot.event(101, AuditKind::Dispatched);
        checkpoints.compare_and_swap(0, snapshot.clone()).unwrap();
        let ctx = ExecutionContext {
            run_id,
            project_id: project.to_string(),
            study_id: study.to_string(),
            step_id: "draft".to_owned(),
            idempotency_key: snapshot.steps[0].idempotency_key.clone(),
            deadline_ms: 100_000,
            estimate,
        };
        (directory, services, ctx)
    }

    #[test]
    fn real_app_services_compiles_and_reconciles_without_replaying_provider() {
        let (_directory, services, context) = approved_host();
        let double = LocalProviderDouble::default();
        let policy = ExplicitlyApprovedMock;
        let limits = ExternalEffectLimits { tokens: 500, cost_microusd: 2500 };
        let fingerprint = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        let first = services.dispatch_authorized_external_effect(
            &context, "openai", fingerprint, limits, &policy, &double,
        ).unwrap();
        assert_eq!(first.status, ExternalEffectStatus::Dispatched);
        assert_eq!(first.provider_request_id.as_deref(), Some("mock_provider_71"));
        assert!(services.dispatch_authorized_external_effect(
            &context, "openai", fingerprint, limits, &policy, &double,
        ).is_err());
        assert_eq!(double.calls.load(Ordering::SeqCst), 1);
        let project = Uuid::parse_str(&context.project_id).unwrap();
        let study = Uuid::parse_str(&context.study_id).unwrap();
        assert_eq!(
            services.store.recover_external_effects(project, study, &context.run_id).unwrap(), 1
        );
        assert!(services.dispatch_authorized_external_effect(
            &context, "openai", fingerprint, limits, &policy, &double,
        ).is_err());
        assert_eq!(double.calls.load(Ordering::SeqCst), 1);
        let settled = services.reconcile_external_with_provider(&context, &double).unwrap();
        assert_eq!(settled.status, ExternalEffectStatus::Completed);
        assert_eq!(settled.receipt_id.as_deref(), Some("mock_receipt_71"));
        assert_eq!(settled.actual_cost_microusd, 450);
        assert_eq!(double.probes.load(Ordering::SeqCst), 1);
        assert_eq!(double.calls.load(Ordering::SeqCst), 1);
        let v8 = services.store.get_agent_effect(
            project, study, &context.run_id, &context.idempotency_key,
        ).unwrap().unwrap();
        assert_eq!(v8.status, scriptotar_db::EffectStatus::Applied);
    }
}
