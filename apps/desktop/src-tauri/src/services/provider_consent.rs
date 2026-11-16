//! Host-owned project/Study scope for explicit provider-consent previews.
//!
//! No Tauri command is registered here: Agent 00 owns the public IPC contract.
//! No paid execution can occur through this adapter until Agent 02 delivers
//! transactional remote-effect fencing AND bounded provider usage receipts.
use super::AppServices;
use scriptotar_agent::ExecutionContext;
use scriptotar_ai::{
    provider_consent::{
        sha256_hex, ActualUsage, ConsentBoundary, ConsentError, ConsentPreview,
        DisabledPaidEffectFence, ExistingAiServiceAdapter, NoTrustedPricing, PaidEffectFence,
        ProviderIntent, RunBudget, ScopeAuthority, ScopeDispatchGuard, StudyScope,
    },
    ProviderConfig,
};
use scriptotar_core::{RepositoryError, RepositoryResult};
use scriptotar_db::{
    ExternalEffectKey, ExternalEffectLimits, ExternalEffectStatus, ExternalReservation,
    ExternalResolution, SqliteStore,
};
use std::{
    sync::MutexGuard,
    time::{SystemTime, UNIX_EPOCH},
};
use uuid::Uuid;

#[allow(dead_code)] // Agent 00 has not yet registered the IPC lifecycle.
fn current_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

fn scope_error(_error: RepositoryError) -> ConsentError {
    // Do not leak project existence, source material, credentials or SQL errors.
    ConsentError::WrongProjectOrStudy
}

#[allow(dead_code)] // Used only when the owner wires the host approval lifecycle.
fn consent_error(error: ConsentError) -> RepositoryError {
    RepositoryError::Validation(format!("AI provider consent: {error:?}"))
}

/// Existing Studio writes and project switching already serialize on
/// AppServices::active_project. Holding that SAME lock across dispatch admission
/// ensures those mutations cannot slip between the authoritative scope read
/// and the provider send. This is a process-local guard, not a distributed lock:
/// paid network transport remains disabled until all origin writers and external
/// mutation paths have been audited against this boundary.
struct HostScopeDispatchGuard<'a> {
    _project_lock: MutexGuard<'a, Uuid>,
    scope: StudyScope,
}

impl ScopeDispatchGuard for HostScopeDispatchGuard<'_> {
    fn scope(&self) -> &StudyScope {
        &self.scope
    }

    fn now_ms(&self) -> Result<u64, ConsentError> {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| ConsentError::Expired)?
            .as_millis()
            .try_into()
            .map_err(|_| ConsentError::Expired)
    }
}

impl AppServices {
    /// Caller MUST own active_project for the entire snapshot. Use the same
    /// lock-free-internal Studio builder as Copy Prompt to avoid recursive
    /// locking and bind the literal excerpt/review content to the digest.
    fn consent_scope_locked(&self, project: Uuid, study: Uuid) -> Result<StudyScope, ConsentError> {
        let current = self.store.get_study(project, study).map_err(scope_error)?;
        if current.archived_at.is_some() {
            return Err(ConsentError::WrongProjectOrStudy);
        }
        let brief = self
            .store
            .get_creative_brief(project, study)
            .map_err(scope_error)?;
        let sources = self
            .store
            .list_study_sources(project, study)
            .map_err(scope_error)?;
        let evidence = self
            .store
            .list_evidence_references(project, study)
            .map_err(scope_error)?;

        let (writing_input, _, _, _, _) = self
            .scoped_writing_input_locked(project, study)
            .map_err(scope_error)?;
        let prompt = scriptotar_ai::writing::prepare_writing_request(&writing_input)
            .map_err(|_| ConsentError::ChangedContext)?
            .prompt;

        let material = serde_json::to_vec(&(&current, &brief, &sources, &evidence, &prompt))
            .map_err(|_| ConsentError::ChangedContext)?;
        let mut selected_source_ids = sources
            .iter()
            .filter(|source| source.available)
            .take(8)
            .map(|source| source.id.to_string())
            .collect::<Vec<_>>();
        selected_source_ids.sort();
        selected_source_ids.dedup();
        Ok(StudyScope {
            project_id: current.project_id.to_string(),
            study_id: current.id.to_string(),
            study_revision: current.revision,
            material_digest: sha256_hex(&material),
            expected_prompt_digest: sha256_hex(prompt.as_bytes()),
            selected_source_ids,
        })
    }
}

impl ScopeAuthority for AppServices {
    fn current_scope(&self, project_id: &str, study_id: &str) -> Result<StudyScope, ConsentError> {
        let project = Uuid::parse_str(project_id).map_err(|_| ConsentError::WrongProjectOrStudy)?;
        let study = Uuid::parse_str(study_id).map_err(|_| ConsentError::WrongProjectOrStudy)?;
        let active = self
            .active_project
            .lock()
            .map_err(|_| ConsentError::EffectFenceUnavailable)?;
        if *active != project {
            return Err(ConsentError::WrongProjectOrStudy);
        }
        self.consent_scope_locked(project, study)
    }

    fn lock_dispatch_scope(
        &self,
        scope: &StudyScope,
    ) -> Result<Box<dyn ScopeDispatchGuard + '_>, ConsentError> {
        let project =
            Uuid::parse_str(&scope.project_id).map_err(|_| ConsentError::WrongProjectOrStudy)?;
        let study =
            Uuid::parse_str(&scope.study_id).map_err(|_| ConsentError::WrongProjectOrStudy)?;
        let project_lock = self
            .active_project
            .lock()
            .map_err(|_| ConsentError::EffectFenceUnavailable)?;
        if *project_lock != project {
            return Err(ConsentError::WrongProjectOrStudy);
        }
        let fresh = self.consent_scope_locked(project, study)?;
        if fresh != *scope {
            return Err(ConsentError::ChangedContext);
        }
        Ok(Box::new(HostScopeDispatchGuard {
            _project_lock: project_lock,
            scope: fresh,
        }))
    }
}

/// A real v10 durable effect bridge, but NOT a network provider. Only a trusted
/// host path may construct this from an approved run and the actual serialized
/// provider wire-byte hash. The existing legacy HTTP adapter remains disabled.
#[allow(dead_code)]
pub(super) struct SqlitePaidEffectFence {
    store: SqliteStore,
    project: Uuid,
    study: Uuid,
    run_id: String,
    step_id: String,
    idempotency_key: String,
    consent_effect_id: String,
    wire_sha256: String,
    provider: String,
    limits: ExternalEffectLimits,
}

#[allow(dead_code)]
impl SqlitePaidEffectFence {
    pub(super) fn from_approved_context(
        store: SqliteStore,
        context: &ExecutionContext,
        approved_fingerprint: &str,
        wire_sha256: &str,
        provider: &str,
        limits: ExternalEffectLimits,
    ) -> Result<Self, ConsentError> {
        fn digest(value: &str) -> bool {
            value.len() == 64
                && value
                    .bytes()
                    .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
        }
        let project =
            Uuid::parse_str(&context.project_id).map_err(|_| ConsentError::WrongProjectOrStudy)?;
        let study =
            Uuid::parse_str(&context.study_id).map_err(|_| ConsentError::WrongProjectOrStudy)?;
        if !digest(approved_fingerprint)
            || !digest(wire_sha256)
            || context.run_id.is_empty()
            || context.step_id.is_empty()
            || context.idempotency_key.is_empty()
            || provider.is_empty()
            || provider.len() > 64
            || !provider
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_' | b':' | b'.'))
            || limits.tokens == 0
            || limits.cost_microusd == 0
        {
            return Err(ConsentError::InvalidRequest);
        }
        Ok(Self {
            store,
            project,
            study,
            run_id: context.run_id.clone(),
            step_id: context.step_id.clone(),
            idempotency_key: context.idempotency_key.clone(),
            consent_effect_id: format!("provider:{}:{}", context.run_id, approved_fingerprint),
            wire_sha256: wire_sha256.to_owned(),
            provider: provider.to_owned(),
            limits,
        })
    }

    fn key(&self) -> ExternalEffectKey<'_> {
        ExternalEffectKey {
            project: self.project,
            study: self.study,
            run_id: &self.run_id,
            step_id: &self.step_id,
            idempotency_key: &self.idempotency_key,
        }
    }

    fn check_effect_id(&self, effect_id: &str) -> Result<(), ConsentError> {
        if effect_id != self.consent_effect_id {
            return Err(ConsentError::ChangedContext);
        }
        Ok(())
    }
}

impl PaidEffectFence for SqlitePaidEffectFence {
    fn reserve_once(&self, effect_id: &str) -> Result<(), ConsentError> {
        self.check_effect_id(effect_id)?;
        match self
            .store
            .reserve_external_effect(self.key(), &self.wire_sha256, &self.provider, self.limits)
            .map_err(|_| ConsentError::UncertainEffect)?
        {
            ExternalReservation::Fresh(_) => Ok(()),
            // An existing record is a reconciliation obligation, never a
            // second authorization even if its status is still RESERVED.
            ExternalReservation::Existing(_) => Err(ConsentError::UncertainEffect),
        }
    }

    fn mark_dispatched_once(&self, effect_id: &str) -> Result<(), ConsentError> {
        self.check_effect_id(effect_id)?;
        let existing = self
            .store
            .get_external_effect(self.key())
            .map_err(|_| ConsentError::UncertainEffect)?
            .ok_or(ConsentError::UncertainEffect)?;
        if existing.status != ExternalEffectStatus::Reserved
            || existing.attempt != 1
            || existing.cancel_requested
            || existing.fingerprint != self.wire_sha256
            || existing.provider != self.provider
            || existing.token_limit != self.limits.tokens
            || existing.cost_limit_microusd != self.limits.cost_microusd
        {
            return Err(ConsentError::UncertainEffect);
        }
        // SQLite IMMEDIATE CAS commits DISPATCHED before the caller may send.
        self.store
            .mark_external_dispatched(self.key(), existing.revision, existing.attempt)
            .map_err(|_| ConsentError::UncertainEffect)?;
        Ok(())
    }

    fn settle(
        &self,
        effect_id: &str,
        provider_request_id: &str,
        usage: &ActualUsage,
    ) -> Result<(), ConsentError> {
        self.check_effect_id(effect_id)?;
        // BoundedProvider::verified_usage_receipt MUST authenticate this ID
        // before calling settle. The v10 ledger uses it as the receipt key
        // until the wire adapter exposes a distinct provider receipt ID.
        if provider_request_id.is_empty()
            || provider_request_id.len() > 128
            || !provider_request_id
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_' | b'/' | b':'))
            || usage
                .input_tokens
                .checked_add(usage.output_tokens)
                .is_none_or(|tokens| tokens > self.limits.tokens)
            || usage.cost_microusd > self.limits.cost_microusd
        {
            return Err(ConsentError::UsageOverBudget);
        }
        let existing = self
            .store
            .get_external_effect(self.key())
            .map_err(|_| ConsentError::UncertainEffect)?
            .ok_or(ConsentError::UncertainEffect)?;
        if existing.status != ExternalEffectStatus::Dispatched || existing.attempt != 1 {
            return Err(ConsentError::UncertainEffect);
        }
        self.store
            .reconcile_external_effect(
                self.key(),
                existing.attempt,
                ExternalResolution::Completed {
                    provider_request_id: Some(provider_request_id),
                    receipt_id: provider_request_id,
                    tokens: usage.input_tokens + usage.output_tokens,
                    cost_microusd: usage.cost_microusd,
                },
            )
            .map_err(|_| ConsentError::UncertainEffect)?;
        Ok(())
    }

    fn mark_uncertain(&self, effect_id: &str) {
        if self.check_effect_id(effect_id).is_err() {
            return;
        }
        if let Ok(Some(previous)) = self.store.get_external_effect(self.key()) {
            match previous.status {
                ExternalEffectStatus::Reserved => {
                    // No network permit existed: cancel the unused reservation.
                    let _ = self.store.request_external_cancel(self.key());
                }
                ExternalEffectStatus::Dispatched => {
                    // Egress may already have occurred. Never retry.
                    let _ = self.store.reconcile_external_effect(
                        self.key(),
                        previous.attempt,
                        ExternalResolution::Uncertain,
                    );
                }
                _ => {}
            }
        }
    }
}

#[allow(dead_code)] // Rust-only integration surface reserved for Agent 00.
impl AppServices {
    /// Prepare the exact server-derived three-angle request, never an arbitrary
    /// model prompt supplied by the client. Approval remains blocked if the
    /// trusted catalog has no verified ceiling for this specific model.
    // Explicit project, Study, run, provider, and independent limits are intentional.
    #[allow(clippy::too_many_arguments)]
    pub fn prepare_provider_angle_consent(
        &self,
        boundary: &ConsentBoundary,
        project: Uuid,
        study: Uuid,
        run_id: &str,
        provider: ProviderConfig,
        max_output_tokens: u64,
        limits: RunBudget,
    ) -> RepositoryResult<ConsentPreview> {
        let prompt = self.prepare_studio_angle_prompt(project, study)?.prompt;
        let scope = self
            .current_scope(&project.to_string(), &study.to_string())
            .map_err(consent_error)?;
        let intent = ProviderIntent {
            run_id: run_id.to_owned(),
            scope,
            provider,
            prompt,
            max_output_tokens,
            limits,
        };
        boundary
            .prepare(intent, self, &NoTrustedPricing, current_ms())
            .map_err(consent_error)
    }

    pub fn decide_provider_angle_consent(
        &self,
        boundary: &ConsentBoundary,
        run_id: &str,
        fingerprint: &str,
        approve: bool,
    ) -> RepositoryResult<()> {
        boundary
            .decide(run_id, fingerprint, approve, self, current_ms())
            .map_err(consent_error)
    }

    /// Must not dispatch: the production adapter has no hard token ceiling or
    /// trustworthy usage receipt, and the paid effect fence is not integrated.
    /// This remains a callable deny-by-default boundary for the owner to wire
    /// after approving the typed API and Agent 02's ledger contract.
    pub fn dispatch_provider_angle_consent_disabled(
        &self,
        boundary: &ConsentBoundary,
        intent: &ProviderIntent,
        request_time_api_key: &str,
    ) -> RepositoryResult<String> {
        boundary
            .dispatch(
                intent,
                self,
                &NoTrustedPricing,
                &DisabledPaidEffectFence,
                &ExistingAiServiceAdapter,
                request_time_api_key,
                current_ms(),
            )
            .map(|result| result.text)
            .map_err(consent_error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dto::UiStudioBrief;
    use tempfile::TempDir;

    #[test]
    fn dispatch_scope_guard_serializes_project_switch_and_rechecks_snapshot() {
        use std::{sync::mpsc, thread, time::Duration};

        let directory = TempDir::new().unwrap();
        let services = AppServices::new(directory.path()).unwrap();
        let project = services.active_project_id().unwrap();
        let study = services
            .create_studio_study(
                project,
                UiStudioBrief {
                    title: "Locked consent scope".into(),
                    goal: "Create original angles".into(),
                    audience: "Viewers".into(),
                    platform: "Reels".into(),
                    language: "English".into(),
                    style: "Clear".into(),
                    duration: "30".into(),
                },
            )
            .unwrap();
        let study_id = Uuid::parse_str(&study.id).unwrap();
        let next = services.create_project("Alternate".into()).unwrap();
        let next_id = Uuid::parse_str(&next.active_project_id).unwrap();
        services.select_project(project).unwrap();

        let scope = services
            .current_scope(&project.to_string(), &study_id.to_string())
            .unwrap();
        let stale = StudyScope {
            material_digest: "f".repeat(64),
            ..scope.clone()
        };
        assert!(matches!(
            services.lock_dispatch_scope(&stale),
            Err(ConsentError::ChangedContext)
        ));
        let guard = services.lock_dispatch_scope(&scope).unwrap();
        assert_eq!(guard.scope(), &scope);
        assert!(guard.now_ms().unwrap() > 0);

        let worker = services.clone();
        let (started_tx, started_rx) = mpsc::channel();
        let (done_tx, done_rx) = mpsc::channel();
        let handle = thread::spawn(move || {
            started_tx.send(()).unwrap();
            worker.select_project(next_id).unwrap();
            done_tx.send(()).unwrap();
        });
        started_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        assert!(done_rx.recv_timeout(Duration::from_millis(75)).is_err());
        drop(guard);
        done_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        handle.join().unwrap();
        assert!(matches!(
            services.current_scope(&project.to_string(), &study_id.to_string()),
            Err(ConsentError::WrongProjectOrStudy)
        ));
    }

    #[test]
    fn host_rejects_foreign_project_and_never_approves_unknown_pricing() {
        let directory = TempDir::new().unwrap();
        let services = AppServices::new(directory.path()).unwrap();
        let project = services.active_project_id().unwrap();
        assert!(matches!(
            services.current_scope(&Uuid::new_v4().to_string(), &Uuid::new_v4().to_string()),
            Err(ConsentError::WrongProjectOrStudy)
        ));

        let study = services
            .create_studio_study(
                project,
                UiStudioBrief {
                    title: "Consent boundary".into(),
                    goal: "Draft original script".into(),
                    audience: "Viewers".into(),
                    platform: "Reels".into(),
                    language: "English".into(),
                    style: "Clear".into(),
                    duration: "30".into(),
                },
            )
            .unwrap();
        let boundary = ConsentBoundary::default();
        let result = services.prepare_provider_angle_consent(
            &boundary,
            project,
            Uuid::parse_str(&study.id).unwrap(),
            "provider-test",
            ProviderConfig {
                provider: scriptotar_ai::ProviderKind::OpenAi,
                model: "unsupported-model".into(),
                base_url: None,
            },
            500,
            RunBudget {
                max_tokens: 10_000,
                max_tool_calls: 1,
                max_elapsed_ms: 10_000,
                max_cost_microusd: 2_000,
            },
        );
        // No selected source is a valid input for agentic paid Study generation.
        assert!(result.is_err());
    }
}
