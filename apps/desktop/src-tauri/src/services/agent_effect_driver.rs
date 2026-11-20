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
        self.execute_fenced_external_attempt(ctx,fingerprint,limits,
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
        self.execute_fenced_external_attempt(ctx,fingerprint,limits,
            policy,provider,ready)
    }

    fn execute_fenced_external_attempt(
        &self,ctx:&ExecutionContext,fingerprint:&str,
        limits:ExternalEffectLimits,policy:&dyn ExternalEffectPolicy,
        provider:&dyn ExternalEffectProvider,ready:ExternalEffect,
    )->RepositoryResult<ExternalEffect> {
        let key=effect_key(ctx)?;
        self.require_active_project(key.project)?;
        policy.authorize(ctx,&ready.provider,fingerprint,limits)?;
        let dispatched=self.store.mark_external_dispatched(key,ready.revision,ready.attempt)?;
        // This transaction is committed before ANY external call. If the app
        // crashes now, a later caller must reconcile rather than replay.
        // A pre-I/O revocation means the service can prove nonexecution locally.
        if self.require_active_project(key.project).is_err()
            || policy.authorize(ctx,&ready.provider,fingerprint,limits).is_err() {
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
                estimate,
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
    fn consent_fence_commits_one_winner_and_never_replays_after_receipt() {
        use scriptotar_ai::provider_consent::{ActualUsage, ConsentError, PaidEffectFence};
        use super::super::provider_consent::SqlitePaidEffectFence;

        let (_directory, services, context) = approved_host();
        let approval = "a".repeat(64);
        // The wire hash comes from final serialized bytes, not prompt text.
        let wire_sha256 = "b".repeat(64);
        let limits = ExternalEffectLimits {
            tokens: 500,
            cost_microusd: 2500,
        };
        let fence = SqlitePaidEffectFence::from_approved_context(
            services.operational_store(),
            &context,
            &approval,
            &wire_sha256,
            "openai:test-model",
            limits,
        )
        .unwrap();
        let effect_id = format!("provider:{}:{}", context.run_id, approval);
        assert_eq!(fence.reserve_once(&effect_id), Ok(()));
        assert_eq!(
            fence.reserve_once(&effect_id),
            Err(ConsentError::UncertainEffect)
        );
        assert_eq!(fence.mark_dispatched_once(&effect_id), Ok(()));
        assert_eq!(
            fence.mark_dispatched_once(&effect_id),
            Err(ConsentError::UncertainEffect)
        );
        let usage = ActualUsage {
            input_tokens: 35,
            output_tokens: 15,
            tool_calls: 1,
            elapsed_ms: 200,
            cost_microusd: 450,
        };
        assert_eq!(
            fence.settle(&effect_id, "verified-provider-1", &usage),
            Ok(())
        );
        let effect = services
            .store
            .get_external_effect(effect_key(&context).unwrap())
            .unwrap()
            .unwrap();
        assert_eq!(effect.status, ExternalEffectStatus::Completed);
        assert_eq!(effect.fingerprint, wire_sha256);
        assert_eq!(effect.actual_tokens, 50);
        assert_eq!(effect.actual_cost_microusd, 450);
        assert_eq!(effect.provider_request_id.as_deref(), Some("verified-provider-1"));
        assert_eq!(
            fence.mark_dispatched_once(&effect_id),
            Err(ConsentError::UncertainEffect)
        );
        fence.mark_uncertain(&effect_id);
        assert_eq!(
            services
                .store
                .get_external_effect(effect_key(&context).unwrap())
                .unwrap()
                .unwrap()
                .status,
            ExternalEffectStatus::Completed
        );
    }

    #[test]
    fn consent_fence_refuses_forged_identity_and_freezes_uncertain_send() {
        use scriptotar_ai::provider_consent::{ConsentError, PaidEffectFence};
        use super::super::provider_consent::SqlitePaidEffectFence;

        let (_directory, services, context) = approved_host();
        let approval = "c".repeat(64);
        let fence = SqlitePaidEffectFence::from_approved_context(
            services.operational_store(),
            &context,
            &approval,
            &"d".repeat(64),
            "openai:test-model",
            ExternalEffectLimits {
                tokens: 500,
                cost_microusd: 2500,
            },
        )
        .unwrap();
        assert_eq!(
            fence.reserve_once("provider:forged"),
            Err(ConsentError::ChangedContext)
        );
        let effect_id = format!("provider:{}:{}", context.run_id, approval);
        fence.reserve_once(&effect_id).unwrap();
        fence.mark_dispatched_once(&effect_id).unwrap();
        fence.mark_uncertain(&effect_id);
        let status = services
            .store
            .get_external_effect(effect_key(&context).unwrap())
            .unwrap()
            .unwrap()
            .status;
        assert_eq!(status, ExternalEffectStatus::Uncertain);
        assert_eq!(
            fence.reserve_once(&effect_id),
            Err(ConsentError::UncertainEffect)
        );
        assert_eq!(
            fence.mark_dispatched_once(&effect_id),
            Err(ConsentError::UncertainEffect)
        );
    }

    #[test]
    fn authenticated_late_receipt_recovers_uncertain_without_another_send() {
        use super::super::provider_consent::SqlitePaidEffectFence;
        use scriptotar_ai::provider_consent::{ActualUsage, ConsentError, PaidEffectFence};

        let (_directory, services, context) = approved_host();
        let store = services.operational_store();
        let approval = "e".repeat(64);
        let wire_sha256 = "f".repeat(64);
        let limits = ExternalEffectLimits {
            tokens: 500,
            cost_microusd: 2500,
        };
        let effect_id = format!("provider:{}:{}", context.run_id, approval);
        let fence = SqlitePaidEffectFence::from_approved_context(
            store.clone(), &context, &approval, &wire_sha256, "openai:test-model", limits,
        ).unwrap();
        fence.reserve_once(&effect_id).unwrap();
        fence.mark_dispatched_once(&effect_id).unwrap();

        // Simulate a lost response after the provider may already have charged.
        fence.mark_uncertain(&effect_id);
        let uncertain = store.get_external_effect(effect_key(&context).unwrap())
            .unwrap().unwrap();
        assert_eq!(uncertain.status, ExternalEffectStatus::Uncertain);
        assert_eq!(
            fence.mark_dispatched_once(&effect_id),
            Err(ConsentError::UncertainEffect)
        );

        let usage = ActualUsage {
            input_tokens: 35,
            output_tokens: 15,
            tool_calls: 1,
            elapsed_ms: 200,
            cost_microusd: 450,
        };

        // Even with the correct effect key, a fence with different outbound
        // bytes must never be able to settle another approval's reservation.
        let stale_wire_fence = SqlitePaidEffectFence::from_approved_context(
            store.clone(), &context, &approval, &"a".repeat(64),
            "openai:test-model", limits,
        ).unwrap();
        assert_eq!(
            stale_wire_fence.settle(&effect_id, "verified-late-1", &usage),
            Err(ConsentError::ChangedContext)
        );
        assert_eq!(
            store.get_external_effect(effect_key(&context).unwrap()).unwrap().unwrap().revision,
            uncertain.revision
        );

        // A delayed, trusted usage receipt completes the original attempt;
        // neither this operation nor a duplicate callback sends any request.
        assert_eq!(fence.settle(&effect_id, "verified-late-1", &usage), Ok(()));
        let completed = store.get_external_effect(effect_key(&context).unwrap())
            .unwrap().unwrap();
        assert_eq!(completed.status, ExternalEffectStatus::Completed);
        assert_eq!(completed.attempt, 1);
        assert_eq!(completed.actual_tokens, 50);
        assert_eq!(completed.actual_cost_microusd, 450);
        assert_eq!(completed.receipt_id.as_deref(), Some("verified-late-1"));

        let reopened = scriptotar_db::SqliteStore::open(store.path()).unwrap();
        let after_restart = SqlitePaidEffectFence::from_approved_context(
            reopened, &context, &approval, &wire_sha256, "openai:test-model", limits,
        ).unwrap();
        assert_eq!(
            after_restart.settle(&effect_id, "verified-late-1", &usage),
            Ok(())
        );
        assert_eq!(
            after_restart.settle(&effect_id, "contradictory-receipt", &usage),
            Err(ConsentError::UncertainEffect)
        );
        assert_eq!(
            after_restart.mark_dispatched_once(&effect_id),
            Err(ConsentError::UncertainEffect)
        );
        let unchanged = store.get_external_effect(effect_key(&context).unwrap())
            .unwrap().unwrap();
        assert_eq!(unchanged.revision, completed.revision);
        assert_eq!(unchanged.receipt_id, completed.receipt_id);
    }

    #[test]
    fn cancelled_reservation_cannot_send_or_take_a_late_receipt() {
        use super::super::provider_consent::SqlitePaidEffectFence;
        use scriptotar_ai::provider_consent::{ActualUsage, ConsentError, PaidEffectFence};

        let (_directory, services, context) = approved_host();
        let store = services.operational_store();
        let approval = "1".repeat(64);
        let wire_sha256 = "2".repeat(64);
        let limits = ExternalEffectLimits {
            tokens: 500,
            cost_microusd: 2500,
        };
        let effect_id = format!("provider:{}:{}", context.run_id, approval);
        let fence = SqlitePaidEffectFence::from_approved_context(
            store.clone(),
            &context,
            &approval,
            &wire_sha256,
            "openai:test-model",
            limits,
        )
        .unwrap();
        fence.reserve_once(&effect_id).unwrap();
        let cancelled = store.request_external_cancel(effect_key(&context).unwrap()).unwrap();
        assert_eq!(cancelled.status, ExternalEffectStatus::Cancelled);
        assert!(cancelled.cancel_requested);

        assert_eq!(
            fence.mark_dispatched_once(&effect_id),
            Err(ConsentError::UncertainEffect)
        );
        let usage = ActualUsage {
            input_tokens: 35,
            output_tokens: 15,
            tool_calls: 1,
            elapsed_ms: 200,
            cost_microusd: 450,
        };
        assert_eq!(
            fence.settle(&effect_id, "never-sent", &usage),
            Err(ConsentError::UncertainEffect)
        );
        let project = Uuid::parse_str(&context.project_id).unwrap();
        let study = Uuid::parse_str(&context.study_id).unwrap();
        let v8 = store
            .get_agent_effect(project, study, &context.run_id, &context.idempotency_key)
            .unwrap()
            .unwrap();
        assert_eq!(v8.status, scriptotar_db::EffectStatus::NotApplied);
        let reopened = scriptotar_db::SqliteStore::open(store.path()).unwrap();
        assert_eq!(
            reopened
                .get_external_effect(effect_key(&context).unwrap())
                .unwrap()
                .unwrap()
                .status,
            ExternalEffectStatus::Cancelled
        );
    }

    #[test]
    fn post_dispatch_cancellation_does_not_hide_a_verified_late_charge() {
        use super::super::provider_consent::SqlitePaidEffectFence;
        use scriptotar_ai::provider_consent::{ActualUsage, ConsentError, PaidEffectFence};

        let (_directory, services, context) = approved_host();
        let store = services.operational_store();
        let approval = "3".repeat(64);
        let wire_sha256 = "4".repeat(64);
        let limits = ExternalEffectLimits {
            tokens: 500,
            cost_microusd: 2500,
        };
        let effect_id = format!("provider:{}:{}", context.run_id, approval);
        let fence = SqlitePaidEffectFence::from_approved_context(
            store.clone(),
            &context,
            &approval,
            &wire_sha256,
            "openai:test-model",
            limits,
        )
        .unwrap();

        fence.reserve_once(&effect_id).unwrap();
        fence.mark_dispatched_once(&effect_id).unwrap();
        let cancelled = store.request_external_cancel(effect_key(&context).unwrap()).unwrap();
        assert_eq!(cancelled.status, ExternalEffectStatus::Dispatched);
        assert!(cancelled.cancel_requested);
        fence.mark_uncertain(&effect_id);
        let pending = store
            .get_external_effect(effect_key(&context).unwrap())
            .unwrap()
            .unwrap();
        assert_eq!(pending.status, ExternalEffectStatus::Uncertain);
        assert!(pending.cancel_requested);
        assert_eq!(
            fence.mark_dispatched_once(&effect_id),
            Err(ConsentError::UncertainEffect)
        );

        // A local cancellation after the provider could have executed is
        // NOT remote nonexecution evidence. An authenticated late charge wins.
        let usage = ActualUsage {
            input_tokens: 35,
            output_tokens: 15,
            tool_calls: 1,
            elapsed_ms: 200,
            cost_microusd: 450,
        };
        fence.settle(&effect_id, "charged-after-cancel", &usage).unwrap();
        let completed = store
            .get_external_effect(effect_key(&context).unwrap())
            .unwrap()
            .unwrap();
        assert_eq!(completed.status, ExternalEffectStatus::Completed);
        assert!(completed.cancel_requested);
        assert_eq!(completed.receipt_id.as_deref(), Some("charged-after-cancel"));
        assert_eq!(completed.actual_cost_microusd, 450);
        assert!(store.request_external_cancel(effect_key(&context).unwrap()).is_err());

        let project = Uuid::parse_str(&context.project_id).unwrap();
        let study = Uuid::parse_str(&context.study_id).unwrap();
        let v8 = store
            .get_agent_effect(project, study, &context.run_id, &context.idempotency_key)
            .unwrap()
            .unwrap();
        assert_eq!(v8.status, scriptotar_db::EffectStatus::Applied);
    }

    #[test]
    fn late_receipt_sqlite_fault_rolls_back_identity_charge_and_legacy_effect() {
        use super::super::provider_consent::SqlitePaidEffectFence;
        use scriptotar_ai::provider_consent::{ActualUsage, ConsentError, PaidEffectFence};
        use rusqlite::Connection;

        let (_directory, services, context) = approved_host();
        let store = services.operational_store();
        let approval = "5".repeat(64);
        let wire_sha256 = "6".repeat(64);
        let limits = ExternalEffectLimits {
            tokens: 500,
            cost_microusd: 2500,
        };
        let effect_id = format!("provider:{}:{}", context.run_id, approval);
        let fence = SqlitePaidEffectFence::from_approved_context(
            store.clone(),
            &context,
            &approval,
            &wire_sha256,
            "openai",
            limits,
        )
        .unwrap();
        fence.reserve_once(&effect_id).unwrap();
        fence.mark_dispatched_once(&effect_id).unwrap();
        fence.mark_uncertain(&effect_id);
        let before = store
            .get_external_effect(effect_key(&context).unwrap())
            .unwrap()
            .unwrap();
        assert_eq!(before.status, ExternalEffectStatus::Uncertain);
        let project = Uuid::parse_str(&context.project_id).unwrap();
        let study = Uuid::parse_str(&context.study_id).unwrap();

        // Crash/fault after v12's external receipt update but before v8's
        // same-transaction Applied transition. The provider identity registry
        // and all accounting must roll back together.
        let conn = Connection::open(store.path()).unwrap();
        conn.execute_batch(
            "CREATE TRIGGER inject_v8_receipt_failure
             BEFORE UPDATE OF status ON agent_effects
             WHEN NEW.status='applied'
             BEGIN SELECT RAISE(ABORT,'injected v8 receipt failure'); END;",
        )
        .unwrap();
        let usage = ActualUsage {
            input_tokens: 15,
            output_tokens: 25,
            tool_calls: 1,
            elapsed_ms: 100,
            cost_microusd: 200,
        };
        assert_eq!(
            fence.settle(&effect_id, "late-faulted-receipt", &usage),
            Err(ConsentError::UncertainEffect)
        );
        let after = store
            .get_external_effect(effect_key(&context).unwrap())
            .unwrap()
            .unwrap();
        assert_eq!(after.status, ExternalEffectStatus::Uncertain);
        assert_eq!(after.revision, before.revision);
        assert_eq!(after.provider_request_id, None);
        assert_eq!(after.receipt_id, None);
        assert_eq!(after.actual_tokens, 0);
        assert_eq!(after.actual_cost_microusd, 0);
        let v8 = store
            .get_agent_effect(project, study, &context.run_id, &context.idempotency_key)
            .unwrap()
            .unwrap();
        assert_eq!(v8.status, scriptotar_db::EffectStatus::Unknown);
        let consumed: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM agent_external_provider_identities
                 WHERE provider='openai' AND identity='late-faulted-receipt'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(consumed, 0);

        conn.execute_batch("DROP TRIGGER inject_v8_receipt_failure;")
            .unwrap();
        drop(conn);
        let reopened = scriptotar_db::SqliteStore::open(store.path()).unwrap();
        let recovery_fence = SqlitePaidEffectFence::from_approved_context(
            reopened,
            &context,
            &approval,
            &wire_sha256,
            "openai",
            limits,
        )
        .unwrap();
        assert_eq!(
            recovery_fence.settle(&effect_id, "late-faulted-receipt", &usage),
            Ok(())
        );
        let completed = store
            .get_external_effect(effect_key(&context).unwrap())
            .unwrap()
            .unwrap();
        assert_eq!(completed.status, ExternalEffectStatus::Completed);
        assert_eq!(completed.actual_tokens, 40);
        assert_eq!(completed.actual_cost_microusd, 200);
        let v8 = store
            .get_agent_effect(project, study, &context.run_id, &context.idempotency_key)
            .unwrap()
            .unwrap();
        assert_eq!(v8.status, scriptotar_db::EffectStatus::Applied);
    }

    #[test]
    fn archived_study_can_record_prior_charge_but_not_reopen_a_dispatch() {
        use super::super::provider_consent::SqlitePaidEffectFence;
        use scriptotar_ai::provider_consent::{ActualUsage, ConsentError, PaidEffectFence};

        let (_directory, services, context) = approved_host();
        let store = services.operational_store();
        let project = Uuid::parse_str(&context.project_id).unwrap();
        let study = Uuid::parse_str(&context.study_id).unwrap();
        let approval = "7".repeat(64);
        let wire_sha256 = "8".repeat(64);
        let limits = ExternalEffectLimits {
            tokens: 500,
            cost_microusd: 2500,
        };
        let effect_id = format!("provider:{}:{}", context.run_id, approval);
        let fence = SqlitePaidEffectFence::from_approved_context(
            store.clone(),
            &context,
            &approval,
            &wire_sha256,
            "openai",
            limits,
        )
        .unwrap();
        fence.reserve_once(&effect_id).unwrap();
        fence.mark_dispatched_once(&effect_id).unwrap();

        // Archive is a legitimate user action AFTER irreversible provider I/O.
        // It may forbid future writes, but must not erase billing evidence.
        store.archive_study(project, study).unwrap();
        fence.mark_uncertain(&effect_id);
        let uncertain = store
            .get_external_effect(effect_key(&context).unwrap())
            .unwrap()
            .unwrap();
        assert_eq!(uncertain.status, ExternalEffectStatus::Uncertain);
        assert_eq!(
            fence.mark_dispatched_once(&effect_id),
            Err(ConsentError::UncertainEffect)
        );
        let usage = ActualUsage {
            input_tokens: 40,
            output_tokens: 10,
            tool_calls: 1,
            elapsed_ms: 10,
            cost_microusd: 300,
        };
        assert_eq!(
            fence.settle(&effect_id, "archived-charge-1", &usage),
            Ok(())
        );
        let record = store
            .get_external_effect(effect_key(&context).unwrap())
            .unwrap()
            .unwrap();
        assert_eq!(record.status, ExternalEffectStatus::Completed);
        assert_eq!(record.receipt_id.as_deref(), Some("archived-charge-1"));
        assert_eq!(record.actual_cost_microusd, 300);
    }

    #[test]
    fn archived_reserved_effect_cannot_gain_first_send_permit() {
        use super::super::provider_consent::SqlitePaidEffectFence;
        use scriptotar_ai::provider_consent::{ConsentError, PaidEffectFence};

        let (_directory, services, context) = approved_host();
        let store = services.operational_store();
        let project = Uuid::parse_str(&context.project_id).unwrap();
        let study = Uuid::parse_str(&context.study_id).unwrap();
        let approval = "9".repeat(64);
        let wire_sha256 = "a".repeat(64);
        let limits = ExternalEffectLimits {
            tokens: 500,
            cost_microusd: 2500,
        };
        let effect_id = format!("provider:{}:{}", context.run_id, approval);
        let fence = SqlitePaidEffectFence::from_approved_context(
            store.clone(),
            &context,
            &approval,
            &wire_sha256,
            "openai",
            limits,
        )
        .unwrap();
        fence.reserve_once(&effect_id).unwrap();
        store.archive_study(project, study).unwrap();
        assert_eq!(
            fence.mark_dispatched_once(&effect_id),
            Err(ConsentError::UncertainEffect)
        );
        let cancelled = store
            .request_external_cancel(effect_key(&context).unwrap())
            .unwrap();
        assert_eq!(cancelled.status, ExternalEffectStatus::Cancelled);
        let v8 = store
            .get_agent_effect(project, study, &context.run_id, &context.idempotency_key);
        // The historical v8 read API intentionally refuses archived scopes.
        assert!(v8.is_err());
        let stored = store
            .get_external_effect(effect_key(&context).unwrap())
            .unwrap()
            .unwrap();
        assert_eq!(stored.status, ExternalEffectStatus::Cancelled);
        assert_eq!(stored.actual_cost_microusd, 0);
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
