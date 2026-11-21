//! Host-owned project/Study scope for explicit provider-consent previews.
//!
//! No Tauri command is registered here: Agent 00 owns the public IPC contract.
//! No paid execution can occur through this adapter until Agent 02 delivers
//! transactional remote-effect fencing AND bounded provider usage receipts.
use super::AppServices;
use scriptotar_agent::{CheckpointStore, ExecutionContext, RunState, StepState};
use scriptotar_ai::{
    provider_consent::{
        sha256_hex, ActualUsage, ConsentBoundary, ConsentError, ConsentPreview,
        DisabledPaidEffectFence, ExistingAiServiceAdapter, NoTrustedPricing, PaidEffectFence,
        ProviderDispatchAttempt, ProviderIntent, RunBudget, ScopeAuthority, ScopeDispatchGuard,
        StudyScope,
    },
    ProviderConfig,
};
use scriptotar_core::{RepositoryError, RepositoryResult};
use scriptotar_db::{
    ExternalEffectKey, ExternalEffectLimits, ExternalEffectStatus, ExternalReservation,
    ExternalResolution, ProviderDispatchBarrier, ProviderDispatchEffect, SqliteStore,
};
use std::{
    collections::HashMap,
    sync::{Mutex, MutexGuard},
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

/// Scope authorization uses both AppServices' project-selection mutex and a
/// durable SQLite-wide writer barrier. The latter is committed before the
/// canonical Study snapshot is rebuilt; v13 triggers reject external connection
/// mutations of protected source/Study inputs until the send guard is dropped.
/// This is not protection against an attacker executing arbitrary DDL on the DB.
struct HostScopeDispatchGuard<'a> {
    _project_lock: MutexGuard<'a, Uuid>,
    db_barrier: ProviderDispatchBarrier,
    scope: StudyScope,
}

impl ScopeDispatchGuard for HostScopeDispatchGuard<'_> {
    fn scope(&self) -> &StudyScope {
        &self.scope
    }

    fn now_ms(&self) -> Result<u64, ConsentError> {
        // A removed or replaced persisted barrier is never authority to send.
        self.db_barrier
            .verify()
            .map_err(|_| ConsentError::EffectFenceUnavailable)?;
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
        // Legacy unbound guard is retained only for offline scope/race tests.
        // ConsentBoundary uses lock_effect_dispatch_scope for every send.
        self.lock_scope_barrier(scope, None)
    }

    fn lock_effect_dispatch_scope(
        &self,
        scope: &StudyScope,
        attempt: Option<&ProviderDispatchAttempt>,
    ) -> Result<Box<dyn ScopeDispatchGuard + '_>, ConsentError> {
        let attempt = attempt.ok_or(ConsentError::EffectFenceUnavailable)?;
        self.lock_scope_barrier(scope, Some(attempt))
    }
}

impl AppServices {
    fn lock_scope_barrier(
        &self,
        scope: &StudyScope,
        attempt: Option<&ProviderDispatchAttempt>,
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
        // Commit an exclusive DB-wide freeze after acquiring the host mutex.
        // A paid call additionally binds the exact reserved v10/v12 effect
        // in that same IMMEDIATE transaction, before fresh scope reread.
        let db_barrier = if let Some(effect) = attempt {
            self.store.acquire_provider_dispatch_barrier_for_effect(
                project,
                study,
                &scope.material_digest,
                ProviderDispatchEffect {
                    run_id: &effect.run_id,
                    step_id: &effect.step_id,
                    idempotency_key: &effect.idempotency_key,
                    attempt: effect.attempt,
                    provider: &effect.provider,
                    wire_sha256: &effect.wire_sha256,
                },
            )
        } else {
            self.store.acquire_provider_dispatch_barrier(project, study, &scope.material_digest)
        }
        .map_err(|_| ConsentError::EffectFenceUnavailable)?;
        let fresh = self.consent_scope_locked(project, study)?;
        if fresh != *scope {
            return Err(ConsentError::ChangedContext);
        }
        Ok(Box::new(HostScopeDispatchGuard {
            _project_lock: project_lock,
            db_barrier,
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
            // The durable v12 identity namespace must not be fragmentable
            // by case aliases (for example, openai vs OpenAI). Only the host's
            // canonical lowercase provider ID may reach the SQLite fence.
            || provider.bytes().any(|c| c.is_ascii_uppercase())
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

    fn dispatch_attempt(
        &self,
        effect_id: &str,
    ) -> Result<Option<ProviderDispatchAttempt>, ConsentError> {
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
        Ok(Some(ProviderDispatchAttempt {
            run_id: self.run_id.clone(),
            step_id: self.step_id.clone(),
            idempotency_key: self.idempotency_key.clone(),
            attempt: existing.attempt,
            provider: self.provider.clone(),
            wire_sha256: self.wire_sha256.clone(),
        }))
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

/// A transport must serialize the exact bytes it will send *before* a paid
/// reservation. Its send implementation MUST use only those bytes, with no
/// hidden reserialization, redirects or automatic retry. No production network
/// implementation is registered yet; controlled, no-network test transports can
/// exercise the real host/v10 admission flow without enabling billable egress.
#[allow(dead_code)]
pub(super) trait CertifiedWireTransport: Send + Sync {
    fn serialize_exact_request(
        &self,
        config: &ProviderConfig,
        request: &scriptotar_ai::AiRequest,
        max_output_tokens: u64,
    ) -> Result<Vec<u8>, ConsentError>;

    fn supports_hard_bounds(&self, config: &ProviderConfig) -> bool;

    fn send_prepared_once(
        &self,
        config: &ProviderConfig,
        api_key: &str,
        exact_bytes: &[u8],
        max_output_tokens: u64,
        deadline_ms: u64,
    ) -> Result<scriptotar_ai::provider_consent::MeteredReply, ConsentError>;

    fn authenticates_receipt(
        &self,
        config: &ProviderConfig,
        reply: &scriptotar_ai::provider_consent::MeteredReply,
    ) -> bool;
}

/// The actual OpenAI Responses wire implementation owns HTTPS transport,
/// immutable request-byte validation, response limits and one-use authenticated
/// usage evidence. Implementing this private bridge does NOT authorize egress:
/// there is still no trusted production pricing catalog, v12 receipt-identity
/// migration, or public Tauri dispatch command.
impl CertifiedWireTransport for scriptotar_ai::provider_openai_wire::OpenAiResponsesWire {
    fn serialize_exact_request(
        &self,
        config: &ProviderConfig,
        request: &scriptotar_ai::AiRequest,
        max_output_tokens: u64,
    ) -> Result<Vec<u8>, ConsentError> {
        scriptotar_ai::provider_openai_wire::OpenAiResponsesWire::serialize_exact_request(
            self,
            config,
            request,
            max_output_tokens,
        )
    }

    fn supports_hard_bounds(&self, config: &ProviderConfig) -> bool {
        scriptotar_ai::provider_openai_wire::OpenAiResponsesWire::supports_hard_bounds(self, config)
    }

    fn send_prepared_once(
        &self,
        config: &ProviderConfig,
        api_key: &str,
        exact_bytes: &[u8],
        max_output_tokens: u64,
        deadline_ms: u64,
    ) -> Result<scriptotar_ai::provider_consent::MeteredReply, ConsentError> {
        scriptotar_ai::provider_openai_wire::OpenAiResponsesWire::send_prepared_once(
            self,
            config,
            api_key,
            exact_bytes,
            max_output_tokens,
            deadline_ms,
        )
    }

    fn authenticates_receipt(
        &self,
        config: &ProviderConfig,
        reply: &scriptotar_ai::provider_consent::MeteredReply,
    ) -> bool {
        scriptotar_ai::provider_openai_wire::OpenAiResponsesWire::authenticates_receipt(
            self, config, reply,
        )
    }
}

/// The policy invokes BoundedProvider after the durable pre-send CAS. This
/// adapter verifies that the policy supplied the *same* model, prompt and token
/// ceiling that were sealed and hashed before reservation, and passes the
/// immutable wire bytes (not a newly serialized request) to the transport.
struct SealedWireProvider<'a, T: CertifiedWireTransport> {
    transport: &'a T,
    intent: &'a ProviderIntent,
    wire_bytes: Vec<u8>,
    deadline_ms: u64,
    max_output_bytes: u64,
}

impl<T: CertifiedWireTransport> scriptotar_ai::provider_consent::BoundedProvider
    for SealedWireProvider<'_, T>
{
    fn supports_hard_bounds(&self, config: &ProviderConfig) -> bool {
        config == &self.intent.provider && self.transport.supports_hard_bounds(config)
    }

    fn prepared_request_sha256(
        &self,
        config: &ProviderConfig,
        request: &scriptotar_ai::AiRequest,
        max_output_tokens: u64,
    ) -> Option<String> {
        if config != &self.intent.provider
            || request.prompt != self.intent.prompt
            || max_output_tokens != self.intent.max_output_tokens
        {
            return None;
        }
        Some(sha256_hex(&self.wire_bytes))
    }

    fn verified_usage_receipt(
        &self,
        config: &ProviderConfig,
        reply: &scriptotar_ai::provider_consent::MeteredReply,
    ) -> bool {
        config == &self.intent.provider && self.transport.authenticates_receipt(config, reply)
    }

    fn generate_once(
        &self,
        config: &ProviderConfig,
        api_key: &str,
        request: &scriptotar_ai::AiRequest,
        max_output_tokens: u64,
        deadline_ms: u64,
    ) -> Result<scriptotar_ai::provider_consent::MeteredReply, ConsentError> {
        if config != &self.intent.provider
            || request.prompt != self.intent.prompt
            || max_output_tokens != self.intent.max_output_tokens
            || current_ms() >= self.deadline_ms
        {
            return Err(ConsentError::ChangedContext);
        }
        let reply = self.transport.send_prepared_once(
            config,
            api_key,
            &self.wire_bytes,
            max_output_tokens,
            deadline_ms.min(self.deadline_ms),
        )?;
        if u64::try_from(reply.text.len()).unwrap_or(u64::MAX) > self.max_output_bytes {
            return Err(ConsentError::UsageOverBudget);
        }
        Ok(reply)
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

    /// Prepare a preview that includes the SHA-256 of the *exact* bytes the
    /// approved transport will later send. The user must be shown the returned
    /// sealed fingerprint; the original unbound preview is never approval.
    #[allow(dead_code, clippy::too_many_arguments)]
    pub(super) fn prepare_provider_angle_consent_sealed<T: CertifiedWireTransport>(
        &self,
        boundary: &ConsentBoundary,
        project: Uuid,
        study: Uuid,
        run_id: &str,
        provider: ProviderConfig,
        max_output_tokens: u64,
        limits: RunBudget,
        pricing: &dyn scriptotar_ai::provider_consent::TrustedPricing,
        transport: &T,
    ) -> RepositoryResult<(ProviderIntent, ConsentPreview)> {
        if !transport.supports_hard_bounds(&provider) {
            return Err(consent_error(ConsentError::EffectFenceUnavailable));
        }
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
        let request = scriptotar_ai::AiRequest {
            prompt: intent.prompt.clone(),
        };
        let bytes = transport
            .serialize_exact_request(&intent.provider, &request, intent.max_output_tokens)
            .map_err(consent_error)?;
        if bytes.is_empty() || bytes.len() > 524_288 {
            return Err(consent_error(ConsentError::InvalidRequest));
        }
        // No paid network access and no reservation during preparation.
        let unsealed = boundary
            .prepare(intent.clone(), self, pricing, current_ms())
            .map_err(consent_error)?;
        let sealed = boundary
            .bind_exact_wire_request(run_id, &unsealed.fingerprint, &sha256_hex(&bytes))
            .map_err(consent_error)?;
        Ok((intent, sealed))
    }

    /// An executable but NOT publicly registered authorization path. Only a
    /// transport that accepts the exact sealed request bytes can reach egress.
    /// v10 independently requires this context to match an approved, RUNNING
    /// checkpoint; the host does not create or fake that checkpoint here.
    ///
    /// There is deliberately no production CertifiedWireTransport for HTTP and
    /// no Tauri command that exposes this method. A controlled no-cost provider
    /// is required to certify admission, receipt and crash behavior first.
    #[allow(dead_code, clippy::too_many_arguments)]
    pub(super) fn dispatch_provider_angle_consent_sealed<T: CertifiedWireTransport>(
        &self,
        boundary: &ConsentBoundary,
        intent: &ProviderIntent,
        preview: &ConsentPreview,
        context: &ExecutionContext,
        pricing: &dyn scriptotar_ai::provider_consent::TrustedPricing,
        transport: &T,
        request_time_api_key: &str,
    ) -> RepositoryResult<scriptotar_ai::provider_consent::MeteredReply> {
        // The checkpoint context is host-owned. Never accept an unrelated
        // project, Study, run or stale plan budget as a v10 effect identity.
        let now = current_ms();
        if context.project_id != intent.scope.project_id
            || context.study_id != intent.scope.study_id
            || context.run_id != intent.run_id
            || context.deadline_ms <= now
            || context.step_id.is_empty()
            || context.idempotency_key.is_empty()
            || context.estimate.tokens < intent.limits.max_tokens
            || context.estimate.cost_microusd < intent.limits.max_cost_microusd
            || context.estimate.max_output_bytes == 0
            || context.estimate.max_output_bytes > 1_048_576
            || preview.run_id != intent.run_id
            || preview.provider != intent.provider.provider
            || preview.model != intent.provider.model
            || preview.selected_source_ids != intent.scope.selected_source_ids
            || preview.max_output_tokens != intent.max_output_tokens
            || preview.limits != intent.limits
        {
            return Err(consent_error(ConsentError::ChangedContext));
        }
        if !transport.supports_hard_bounds(&intent.provider) {
            return Err(consent_error(ConsentError::EffectFenceUnavailable));
        }
        let wire_request = scriptotar_ai::AiRequest {
            prompt: intent.prompt.clone(),
        };
        let exact_bytes = transport
            .serialize_exact_request(&intent.provider, &wire_request, intent.max_output_tokens)
            .map_err(consent_error)?;
        if exact_bytes.is_empty() || exact_bytes.len() > 524_288 {
            return Err(consent_error(ConsentError::InvalidRequest));
        }
        let wire_sha256 = sha256_hex(&exact_bytes);
        let fingerprint = &preview.fingerprint;
        let provider_id = format!("{:?}", intent.provider.provider).to_ascii_lowercase();
        let fence = SqlitePaidEffectFence::from_approved_context(
            self.store.clone(),
            context,
            fingerprint,
            &wire_sha256,
            &provider_id,
            ExternalEffectLimits {
                tokens: intent.limits.max_tokens,
                cost_microusd: intent.limits.max_cost_microusd,
            },
        )
        .map_err(consent_error)?;
        let sealed = SealedWireProvider {
            transport,
            intent,
            wire_bytes: exact_bytes,
            deadline_ms: context.deadline_ms,
            max_output_bytes: context.estimate.max_output_bytes,
        };
        boundary
            .dispatch(
                intent,
                self,
                pricing,
                &fence,
                &sealed,
                request_time_api_key,
                now,
            )
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

/// Per-application consent state. Approval is never reconstructed from SQLite or
/// from a client-supplied fingerprint after a restart. This state has no secrets.
#[derive(Default)]
pub(super) struct HostProviderSession {
    boundary: ConsentBoundary,
    prepared: Mutex<HashMap<String, (ProviderIntent, ConsentPreview)>>,
}

/// Production egress remains disabled until the independent security review
/// certifies current provider pricing, snapshot isolation across processes,
/// restart recovery, and canonical three-script persistence. An environment
/// variable, frontend flag, or caller cannot enable a billable request.
fn certified_paid_egress_enabled() -> bool {
    false
}

impl AppServices {
    /// Public IPC preview: derive the prompt from the authoritative current
    /// Study; seal the literal OpenAI request bytes. No HTTP or cost incurred.
    /// Pricing is deliberately unavailable; approval fails closed.
    #[allow(clippy::too_many_arguments)]
    pub fn preview_openai_provider_consent(
        &self,
        project: Uuid,
        study: Uuid,
        run_id: &str,
        model: String,
        max_output_tokens: u64,
        limits: RunBudget,
    ) -> RepositoryResult<ConsentPreview> {
        Uuid::parse_str(run_id)
            .map_err(|_| RepositoryError::Validation("invalid provider run id".into()))?;
        let mut prepared = self
            .provider_session
            .prepared
            .lock()
            .map_err(|_| consent_error(ConsentError::EffectFenceUnavailable))?;
        // Check capacity BEFORE creating a ConsentBoundary record. A rejected
        // client cannot leak unbounded denied records through repeated previews.
        if prepared.len() >= 64 {
            return Err(consent_error(ConsentError::EffectFenceUnavailable));
        }
        let transport =
            super::provider_openai_wire::HostOpenAiResponsesWire::new().map_err(consent_error)?;
        let provider = ProviderConfig {
            provider: scriptotar_ai::ProviderKind::OpenAi,
            model,
            base_url: None,
        };
        let (intent, preview) = self.prepare_provider_angle_consent_sealed(
            &self.provider_session.boundary,
            project,
            study,
            run_id,
            provider,
            max_output_tokens,
            limits,
            &NoTrustedPricing,
            &transport,
        )?;
        prepared.insert(run_id.to_owned(), (intent, preview.clone()));
        Ok(preview)
    }

    pub fn decide_openai_provider_consent(
        &self,
        project: Uuid,
        study: Uuid,
        run_id: &str,
        fingerprint: &str,
        approve: bool,
    ) -> RepositoryResult<()> {
        let (intent, preview) =
            self.lookup_provider_consent(project, study, run_id, fingerprint)?;
        // No public callback can approve a changed model, scope or sealed hash.
        if intent.scope.project_id != project.to_string()
            || intent.scope.study_id != study.to_string()
            || preview.fingerprint != fingerprint
        {
            return Err(consent_error(ConsentError::ChangedContext));
        }
        self.decide_provider_angle_consent(
            &self.provider_session.boundary,
            run_id,
            fingerprint,
            approve,
        )
    }

    fn lookup_provider_consent(
        &self,
        project: Uuid,
        study: Uuid,
        run_id: &str,
        fingerprint: &str,
    ) -> RepositoryResult<(ProviderIntent, ConsentPreview)> {
        let prepared = self
            .provider_session
            .prepared
            .lock()
            .map_err(|_| consent_error(ConsentError::EffectFenceUnavailable))?;
        let (intent, preview) = prepared
            .get(run_id)
            .cloned()
            .ok_or_else(|| consent_error(ConsentError::NoApproval))?;
        if intent.scope.project_id != project.to_string()
            || intent.scope.study_id != study.to_string()
            || preview.fingerprint != fingerprint
        {
            return Err(consent_error(ConsentError::ChangedContext));
        }
        Ok((intent, preview))
    }

    /// The public execution entrypoint reaches the real serialized OpenAI wire
    /// through the same consent, scoped ledger, and pre-send CAS adapter used by
    /// controlled integration tests. It is intentionally fail-closed in this
    /// release candidate: no trusted model-price catalog or complete paid
    /// output persistence has been independently certified.
    pub fn execute_openai_provider_consent(
        &self,
        project: Uuid,
        study: Uuid,
        run_id: &str,
        fingerprint: &str,
        request_time_api_key: &str,
    ) -> RepositoryResult<String> {
        let (intent, preview) =
            self.lookup_provider_consent(project, study, run_id, fingerprint)?;
        if !certified_paid_egress_enabled() {
            return Err(RepositoryError::Validation(
                "Paid provider execution is disabled pending independent certification".into(),
            ));
        }

        // Never trust a browser-provided execution context. The real durable
        // checkpoint must already have an explicitly approved RUNNING step.
        let checkpoints = self
            .store
            .scoped_agent_checkpoints(project, study)
            .map_err(|_| consent_error(ConsentError::EffectFenceUnavailable))?;
        let snapshot = checkpoints
            .load(run_id)
            .map_err(|_| consent_error(ConsentError::EffectFenceUnavailable))?
            .ok_or_else(|| consent_error(ConsentError::NoApproval))?;
        let index = snapshot.current_step;
        let record = snapshot
            .steps
            .get(index)
            .ok_or_else(|| consent_error(ConsentError::NoApproval))?;
        let plan_step = snapshot
            .plan
            .steps
            .get(index)
            .ok_or_else(|| consent_error(ConsentError::NoApproval))?;
        if snapshot.plan.id != run_id
            || snapshot.plan.project_id != project.to_string()
            || snapshot.plan.study_id != study.to_string()
            || snapshot.state != RunState::Running
            || record.state != StepState::Running
            || !record.approved
            || record.attempts != 1
            || snapshot.cancel_requested
        {
            return Err(consent_error(ConsentError::NoApproval));
        }
        let context = ExecutionContext {
            run_id: run_id.to_owned(),
            project_id: project.to_string(),
            study_id: study.to_string(),
            step_id: record.id.clone(),
            idempotency_key: record.idempotency_key.clone(),
            deadline_ms: preview.expires_at_ms,
            estimate: plan_step.estimate,
        };
        let transport =
            super::provider_openai_wire::HostOpenAiResponsesWire::new().map_err(consent_error)?;
        let reply = self.dispatch_provider_angle_consent_sealed(
            &self.provider_session.boundary,
            &intent,
            &preview,
            &context,
            &NoTrustedPricing,
            &transport,
            request_time_api_key,
        )?;
        Ok(reply.text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dto::UiStudioBrief;
    use tempfile::TempDir;

    #[test]
    fn public_provider_preview_is_sealed_but_cannot_approve_or_send_without_trusted_pricing() {
        use scriptotar_core::{StudySourceInput, StudySourceKind};

        let directory = TempDir::new().unwrap();
        let services = AppServices::new(directory.path()).unwrap();
        let project = services.active_project_id().unwrap();
        let study = services
            .create_studio_study(
                project,
                UiStudioBrief {
                    title: "No billable preview".into(),
                    goal: "Compare three creative directions".into(),
                    audience: "Creators".into(),
                    platform: "Reels".into(),
                    language: "English".into(),
                    style: "Clear".into(),
                    duration: "30".into(),
                },
            )
            .unwrap();
        let study_id = Uuid::parse_str(&study.id).unwrap();
        services
            .store
            .add_study_source(
                project,
                study_id,
                &StudySourceInput {
                    kind: StudySourceKind::Manual,
                    reference_id: None,
                    locator: Some("Authoritative local fixture".into()),
                    manual_text: Some("Source-grounded factual notes".into()),
                    source_revision: None,
                    uncertainty: None,
                },
            )
            .unwrap();
        let run_id = Uuid::new_v4().to_string();
        let preview = services
            .preview_openai_provider_consent(
                project,
                study_id,
                &run_id,
                "gpt-4o-mini".into(),
                128,
                RunBudget {
                    max_tokens: 50_000,
                    max_tool_calls: 1,
                    max_elapsed_ms: 30_000,
                    max_cost_microusd: 20_000,
                },
            )
            .unwrap();
        assert_eq!(preview.run_id, run_id);
        assert_eq!(preview.fingerprint.len(), 64);
        // Manual-source raw text is intentionally not treated as quoted
        // transcript evidence by scoped_writing_input_locked. Verify that the
        // sealed preview uses exactly the authoritative Studio prompt and
        // binds the selected source identity without laundering raw notes.
        let canonical = services
            .prepare_studio_angle_prompt(project, study_id)
            .unwrap();
        assert_eq!(preview.prompt, canonical.prompt);
        assert_eq!(preview.selected_source_ids.len(), 1);
        assert_eq!(preview.worst_case_cost_microusd, None);
        assert!(services
            .decide_openai_provider_consent(project, study_id, &run_id, &preview.fingerprint, true,)
            .is_err());
        assert!(services
            .execute_openai_provider_consent(
                project,
                study_id,
                &run_id,
                &preview.fingerprint,
                "fake-test-key",
            )
            .is_err());
        assert!(services
            .decide_openai_provider_consent(
                Uuid::new_v4(),
                study_id,
                &run_id,
                &preview.fingerprint,
                false,
            )
            .is_err());
        drop(services);
        let restarted = AppServices::new(directory.path()).unwrap();
        assert!(restarted
            .execute_openai_provider_consent(
                project,
                study_id,
                &run_id,
                &preview.fingerprint,
                "fake-test-key",
            )
            .is_err());
        // No live egress, no persisted credentials or resurrected approval.
    }

    #[test]
    fn production_openai_wire_can_only_prepare_bounded_sealed_bytes_without_egress() {
        use scriptotar_ai::{provider_openai_wire::OpenAiResponsesWire, AiRequest, ProviderKind};

        let transport = OpenAiResponsesWire::new().unwrap();
        let guarded: &dyn CertifiedWireTransport = &transport;
        let config = ProviderConfig {
            provider: ProviderKind::OpenAi,
            model: "test-model".into(),
            base_url: None,
        };
        let request = AiRequest {
            prompt: "Canonical Study prompt".into(),
        };
        assert!(guarded.supports_hard_bounds(&config));
        let bytes = guarded
            .serialize_exact_request(&config, &request, 128)
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["input"], "Canonical Study prompt");
        assert_eq!(json["max_output_tokens"], 128);
        assert_eq!(json["store"], false);
        assert_eq!(json["tools"], serde_json::json!([]));
        assert_eq!(sha256_hex(&bytes).len(), 64);
        assert!(guarded
            .serialize_exact_request(&config, &request, 0)
            .is_err());
        let foreign_endpoint = ProviderConfig {
            base_url: Some("https://untrusted.example".into()),
            ..config
        };
        assert!(!guarded.supports_hard_bounds(&foreign_endpoint));
        assert!(guarded
            .serialize_exact_request(&foreign_endpoint, &request, 128)
            .is_err());
        // No token, request, permission or payment is dispatched in this test.
    }

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
        // Production paid dispatch must not downgrade to a test-only unbound
        // SQLite writer freeze when there is no exact v10 reservation.
        assert!(matches!(
            services.lock_effect_dispatch_scope(&scope, None),
            Err(ConsentError::EffectFenceUnavailable)
        ));
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
    fn dispatch_guard_blocks_real_studio_brief_edit_until_provider_scope_releases() {
        use std::{sync::mpsc, thread, time::Duration};

        let directory = TempDir::new().unwrap();
        let services = AppServices::new(directory.path()).unwrap();
        let project = services.active_project_id().unwrap();
        let brief = UiStudioBrief {
            title: "Original consent brief".into(),
            goal: "Prepare grounded scripts".into(),
            audience: "Creators".into(),
            platform: "Reels".into(),
            language: "English".into(),
            style: "Clear".into(),
            duration: "30".into(),
        };
        let study = services
            .create_studio_study(project, brief.clone())
            .unwrap();
        let study_id = Uuid::parse_str(&study.id).unwrap();
        let approved = services
            .current_scope(&project.to_string(), &study_id.to_string())
            .unwrap();
        let dispatch_guard = services.lock_dispatch_scope(&approved).unwrap();

        let writer = services.clone();
        let (started_tx, started_rx) = mpsc::channel();
        let (completed_tx, completed_rx) = mpsc::channel();
        let worker = thread::spawn(move || {
            let mut changed_brief = brief;
            changed_brief.goal = "Different source-grounded objective".into();
            started_tx.send(()).unwrap();
            let result = writer.save_studio_study(
                project,
                study_id,
                study.revision,
                crate::dto::UiStudioStudyPatch {
                    brief: Some(changed_brief),
                    sources: None,
                    draft: None,
                    draft_id: None,
                    draft_revision: None,
                },
            );
            completed_tx
                .send(result.map(|value| value.revision))
                .unwrap();
        });

        started_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        // The real Studio adapter must use the SAME gate as paid dispatch.
        assert!(
            completed_rx
                .recv_timeout(Duration::from_millis(75))
                .is_err(),
            "Study change completed while a provider dispatch guard was held"
        );
        assert_eq!(dispatch_guard.scope(), &approved);
        drop(dispatch_guard);
        let new_revision = completed_rx
            .recv_timeout(Duration::from_secs(5))
            .unwrap()
            .unwrap();
        worker.join().unwrap();
        assert!(new_revision > 0);
        assert!(matches!(
            services.lock_dispatch_scope(&approved),
            Err(ConsentError::ChangedContext)
        ));
    }

    #[test]
    fn dispatch_guard_blocks_real_transcript_revocation_and_invalidates_scope() {
        use crate::dto::{UiStudioSourceRef, UiStudioStudyPatch};
        use std::{sync::mpsc, thread, time::Duration};

        let directory = TempDir::new().unwrap();
        let services = AppServices::new(directory.path()).unwrap();
        let project = services.active_project_id().unwrap();
        let study = services
            .create_studio_study(
                project,
                UiStudioBrief {
                    title: "Revocable consent source".into(),
                    goal: "Compare source-backed scripts".into(),
                    audience: "Creators".into(),
                    platform: "Reels".into(),
                    language: "English".into(),
                    style: "Clear".into(),
                    duration: "30".into(),
                },
            )
            .unwrap();
        let study_id = Uuid::parse_str(&study.id).unwrap();

        // Populate an actual canonical, project-owned transcript origin.
        // No external AI network access or synthetic approval is involved.
        let source_id = Uuid::new_v4();
        let media_id = Uuid::new_v4();
        let transcript_id = Uuid::new_v4();
        let db = rusqlite::Connection::open(services.store.path()).unwrap();
        db.execute(
            "INSERT INTO sources(id,project_id,creator_id,source_type,locator,created_at)
             VALUES(?1,?2,NULL,'url','https://example.org/revocable','2026-11-21T00:00:00Z')",
            rusqlite::params![source_id.to_string(), project.to_string()],
        )
        .unwrap();
        db.execute(
            "INSERT INTO media(id,source_id,local_path,created_at)
             VALUES(?1,?2,'/tmp/scriptotar-revocable.mp4','2026-11-21T00:00:00Z')",
            rusqlite::params![media_id.to_string(), source_id.to_string()],
        )
        .unwrap();
        db.execute(
            "INSERT INTO transcripts(id,media_id,text,created_at,updated_at)
             VALUES(?1,?2,'Canonical revocable source content',
                    '2026-11-21T00:00:00Z','2026-11-21T00:00:00Z')",
            rusqlite::params![transcript_id.to_string(), media_id.to_string()],
        )
        .unwrap();
        drop(db);

        let selected = services
            .save_studio_study(
                project,
                study_id,
                study.revision,
                UiStudioStudyPatch {
                    brief: None,
                    sources: Some(vec![UiStudioSourceRef {
                        kind: "transcript".into(),
                        id: transcript_id.to_string(),
                    }]),
                    draft: None,
                    draft_id: None,
                    draft_revision: None,
                },
            )
            .unwrap();

        // An independent SQLite connection can mutate the transcript without
        // taking AppServices::active_project. A subsequent scope recheck MUST
        // reject the stale approval even though this does NOT establish a
        // cross-process lock during an in-flight provider send.
        let stale_before_external_edit = services
            .current_scope(&project.to_string(), &study_id.to_string())
            .unwrap();
        let external = rusqlite::Connection::open(services.store.path()).unwrap();
        assert_eq!(
            external
                .execute(
                    "UPDATE transcripts SET text = ?1, updated_at = ?2 WHERE id = ?3",
                    rusqlite::params![
                        "Externally modified canonical transcript content",
                        "2026-11-21T00:01:00Z",
                        transcript_id.to_string()
                    ],
                )
                .unwrap(),
            1
        );
        drop(external);
        assert!(matches!(
            services.lock_dispatch_scope(&stale_before_external_edit),
            Err(ConsentError::ChangedContext)
        ));

        let approved = services
            .current_scope(&project.to_string(), &study_id.to_string())
            .unwrap();
        assert_eq!(approved.selected_source_ids.len(), 1);
        let dispatch_guard = services.lock_dispatch_scope(&approved).unwrap();

        // Agent 10's independent challenge: a separate SQLite connection
        // must be denied WHILE the dispatch guard is alive (rather than only
        // discovering the mutation on a later scope recheck).
        let external = rusqlite::Connection::open(services.store.path()).unwrap();
        let rejected = external
            .execute(
                "UPDATE transcripts SET text=?1, updated_at=?2 WHERE id=?3",
                rusqlite::params![
                    "Unauthorized mid-send transcript mutation",
                    "2026-11-21T00:02:00Z",
                    transcript_id.to_string()
                ],
            )
            .unwrap_err();
        assert!(
            rejected
                .to_string()
                .contains("provider dispatch scope is frozen"),
            "external writer was rejected for an unrelated reason: {rejected}"
        );
        assert_eq!(dispatch_guard.scope(), &approved);

        let writer = services.clone();
        let (started_tx, started_rx) = mpsc::channel();
        let (completed_tx, completed_rx) = mpsc::channel();
        let worker = thread::spawn(move || {
            started_tx.send(()).unwrap();
            let result = writer.revoke_studio_source_access(
                project,
                study_id,
                selected.revision,
                "transcript",
                transcript_id,
            );
            completed_tx
                .send(result.map(|value| value.revision))
                .unwrap();
        });
        started_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        assert!(
            completed_rx
                .recv_timeout(Duration::from_millis(75))
                .is_err(),
            "source revocation linearized before the provider guard released"
        );
        drop(dispatch_guard);
        completed_rx
            .recv_timeout(Duration::from_secs(5))
            .unwrap()
            .unwrap();
        worker.join().unwrap();
        assert!(
            services.lock_dispatch_scope(&approved).is_err(),
            "revoked source must invalidate the consent snapshot"
        );
    }

    /// Executes actual AppServices -> ConsentBoundary -> v10 reservation/CAS
    /// -> controlled presealed transport -> bounded receipt. This is a real
    /// host path test, not a policy-only or structural smoke test.
    #[test]
    fn sealed_host_dispatch_reaches_v10_once_and_never_replays() {
        use scriptotar_agent::{
            AuditKind, CheckpointStore, Estimate, ExecutionContext, Limits, Plan, RunSnapshot,
            RunState, StepInput, StepPlan, StepRecord, StepState, Usage,
        };
        use scriptotar_ai::provider_consent::{MeteredReply, PriceCeiling, TrustedPricing};
        use scriptotar_core::{StudySourceInput, StudySourceKind};
        use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

        struct ControlledPrices;
        impl TrustedPricing for ControlledPrices {
            fn ceiling(&self, _config: &ProviderConfig) -> Option<PriceCeiling> {
                Some(PriceCeiling {
                    input_microusd_per_million: 1_000,
                    output_microusd_per_million: 1_000,
                    catalog_revision: "controlled-v1".into(),
                })
            }
        }
        struct ControlledTransport {
            calls: AtomicUsize,
            corrupt_wire: AtomicBool,
        }
        impl CertifiedWireTransport for ControlledTransport {
            fn supports_hard_bounds(&self, _config: &ProviderConfig) -> bool {
                true
            }
            fn serialize_exact_request(
                &self,
                config: &ProviderConfig,
                request: &scriptotar_ai::AiRequest,
                max_output_tokens: u64,
            ) -> Result<Vec<u8>, ConsentError> {
                let mut bytes = serde_json::to_vec(&(config, &request.prompt, max_output_tokens))
                    .map_err(|_| ConsentError::InvalidRequest)?;
                if self.corrupt_wire.load(Ordering::SeqCst) {
                    bytes.extend_from_slice(b"changed-after-approval");
                }
                Ok(bytes)
            }
            fn send_prepared_once(
                &self,
                _config: &ProviderConfig,
                _api_key: &str,
                bytes: &[u8],
                _max_output_tokens: u64,
                _deadline_ms: u64,
            ) -> Result<MeteredReply, ConsentError> {
                if bytes.is_empty() {
                    return Err(ConsentError::InvalidRequest);
                }
                self.calls.fetch_add(1, Ordering::SeqCst);
                Ok(MeteredReply {
                    text: "controlled, no-network output".into(),
                    provider_request_id: "controlled_receipt_1".into(),
                    usage: ActualUsage {
                        input_tokens: 10,
                        output_tokens: 6,
                        tool_calls: 1,
                        elapsed_ms: 5,
                        cost_microusd: 0,
                    },
                })
            }
            fn authenticates_receipt(
                &self,
                _config: &ProviderConfig,
                reply: &MeteredReply,
            ) -> bool {
                reply.provider_request_id == "controlled_receipt_1"
            }
        }

        let directory = TempDir::new().unwrap();
        let services = AppServices::new(directory.path()).unwrap();
        let project = services.active_project_id().unwrap();
        let study = services
            .create_studio_study(
                project,
                UiStudioBrief {
                    title: "Controlled host admission".into(),
                    goal: "Prepare three original scripts".into(),
                    audience: "Creators".into(),
                    platform: "Reels".into(),
                    language: "English".into(),
                    style: "Clear".into(),
                    duration: "30".into(),
                },
            )
            .unwrap();
        let study_id = Uuid::parse_str(&study.id).unwrap();
        services
            .store
            .add_study_source(
                project,
                study_id,
                &StudySourceInput {
                    kind: StudySourceKind::Manual,
                    reference_id: None,
                    locator: Some("Controlled fixture".into()),
                    manual_text: Some("One original creative observation".into()),
                    source_revision: None,
                    uncertainty: None,
                },
            )
            .unwrap();
        let scope = services
            .current_scope(&project.to_string(), &study_id.to_string())
            .unwrap();
        assert_eq!(scope.selected_source_ids.len(), 1);
        let prompt = services
            .prepare_studio_angle_prompt(project, study_id)
            .unwrap()
            .prompt;
        let run_id = Uuid::new_v4().to_string();
        let estimate = Estimate {
            tokens: 50_000,
            cost_microusd: 20_000,
            max_output_bytes: 1024,
            cpu_ms: 1,
            peak_memory_mib: 1,
        };
        let plan = Plan {
            id: run_id.clone(),
            project_id: project.to_string(),
            study_id: study_id.to_string(),
            steps: vec![StepPlan {
                id: "draft".into(),
                input: StepInput::DraftScript {
                    brief_id: study_id.to_string(),
                },
                estimate,
                max_attempts: 1,
                requires_approval: true,
            }],
            limits: Limits {
                max_tool_calls: 1,
                max_tokens: 50_000,
                max_cost_microusd: 20_000,
                max_elapsed_ms: 30_000,
                max_output_bytes: 1024,
                max_cpu_ms: 100,
                max_peak_memory_mib: 4,
            },
        };
        let idem = format!("{run_id}:0:draft");
        let mut checkpoint = RunSnapshot {
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
                idempotency_key: idem.clone(),
                pending_effect_id: None,
                output: None,
                failure_code: None,
            }],
            usage: Usage::default(),
            started_ms: current_ms(),
            cancel_requested: false,
            pause_requested: false,
            events: Vec::new(),
        };
        checkpoint.event(current_ms(), AuditKind::Created);
        let checkpoints = services
            .store
            .scoped_agent_checkpoints(project, study_id)
            .unwrap();
        checkpoints.create(checkpoint.clone()).unwrap();
        checkpoint.revision = 1;
        checkpoint.state = RunState::Running;
        checkpoint.steps[0].state = StepState::Running;
        checkpoint.steps[0].attempts = 1;
        checkpoint.steps[0].approved = true;
        checkpoint.usage.tool_calls = 1;
        checkpoint.event(current_ms(), AuditKind::Dispatched);
        checkpoints.compare_and_swap(0, checkpoint).unwrap();

        let context = ExecutionContext {
            run_id: run_id.clone(),
            project_id: project.to_string(),
            study_id: study_id.to_string(),
            step_id: "draft".into(),
            idempotency_key: idem.clone(),
            deadline_ms: current_ms() + 30_000,
            estimate,
        };
        let limit = ExternalEffectLimits {
            tokens: 50_000,
            cost_microusd: 20_000,
        };
        let digest = "a".repeat(64);
        let wire = "b".repeat(64);
        // The production host must never create a second provider identity
        // namespace using only a case variant. Agent 02 also enforces this
        // independently at the actual database storage boundary.
        assert!(SqlitePaidEffectFence::from_approved_context(
            services.store.clone(),
            &context,
            &digest,
            &wire,
            "OpenAI",
            limit,
        )
        .is_err());
        assert!(SqlitePaidEffectFence::from_approved_context(
            services.store.clone(),
            &context,
            &digest,
            &wire,
            "openai",
            limit,
        )
        .is_ok());

        let intent = ProviderIntent {
            run_id: run_id.clone(),
            scope,
            provider: ProviderConfig {
                provider: scriptotar_ai::ProviderKind::OpenAi,
                model: "gpt-4o-mini".into(),
                base_url: None,
            },
            prompt,
            max_output_tokens: 64,
            limits: RunBudget {
                max_tokens: 50_000,
                max_tool_calls: 1,
                max_elapsed_ms: 30_000,
                max_cost_microusd: 20_000,
            },
        };
        let boundary = ConsentBoundary::default();
        let prices = ControlledPrices;
        let transport = ControlledTransport {
            calls: AtomicUsize::new(0),
            corrupt_wire: AtomicBool::new(false),
        };
        let (sealed_intent, preview) = services
            .prepare_provider_angle_consent_sealed(
                &boundary,
                project,
                study_id,
                &run_id,
                intent.provider.clone(),
                intent.max_output_tokens,
                intent.limits,
                &prices,
                &transport,
            )
            .unwrap();
        assert_eq!(sealed_intent, intent);
        assert!(preview.worst_case_cost_microusd.is_some());
        assert!(services
            .dispatch_provider_angle_consent_sealed(
                &boundary,
                &intent,
                &preview,
                &context,
                &prices,
                &transport,
                "test-only"
            )
            .is_err());
        assert_eq!(transport.calls.load(Ordering::SeqCst), 0);
        boundary
            .decide(&run_id, &preview.fingerprint, true, &services, current_ms())
            .unwrap();
        // Approval hashes the sealed request; a different serialized buffer
        // must be denied *before* reservation, CAS or provider execution.
        transport.corrupt_wire.store(true, Ordering::SeqCst);
        assert!(services
            .dispatch_provider_angle_consent_sealed(
                &boundary,
                &intent,
                &preview,
                &context,
                &prices,
                &transport,
                "test-only"
            )
            .is_err());
        assert_eq!(transport.calls.load(Ordering::SeqCst), 0);
        transport.corrupt_wire.store(false, Ordering::SeqCst);
        let reply = services
            .dispatch_provider_angle_consent_sealed(
                &boundary,
                &intent,
                &preview,
                &context,
                &prices,
                &transport,
                "test-only",
            )
            .unwrap();
        assert_eq!(reply.provider_request_id, "controlled_receipt_1");
        assert_eq!(transport.calls.load(Ordering::SeqCst), 1);

        let effect = services
            .store
            .get_external_effect(ExternalEffectKey {
                project,
                study: study_id,
                run_id: &run_id,
                step_id: "draft",
                idempotency_key: &idem,
            })
            .unwrap()
            .unwrap();
        let exact_bytes = transport
            .serialize_exact_request(
                &intent.provider,
                &scriptotar_ai::AiRequest {
                    prompt: intent.prompt.clone(),
                },
                intent.max_output_tokens,
            )
            .unwrap();
        assert_eq!(effect.fingerprint, sha256_hex(&exact_bytes));
        assert_eq!(effect.status, ExternalEffectStatus::Completed);
        assert_eq!(effect.attempt, 1);
        assert_eq!(
            effect.provider_request_id.as_deref(),
            Some("controlled_receipt_1")
        );
        assert!(services
            .dispatch_provider_angle_consent_sealed(
                &boundary,
                &intent,
                &preview,
                &context,
                &prices,
                &transport,
                "test-only"
            )
            .is_err());
        // A fresh in-memory approval must not replay a completed durable claim.
        let restarted_consent = ConsentBoundary::default();
        let (reopened_intent, same_preview) = services
            .prepare_provider_angle_consent_sealed(
                &restarted_consent,
                project,
                study_id,
                &run_id,
                intent.provider.clone(),
                intent.max_output_tokens,
                intent.limits,
                &prices,
                &transport,
            )
            .unwrap();
        assert_eq!(reopened_intent, intent);
        restarted_consent
            .decide(
                &run_id,
                &same_preview.fingerprint,
                true,
                &services,
                current_ms(),
            )
            .unwrap();
        assert!(services
            .dispatch_provider_angle_consent_sealed(
                &restarted_consent,
                &intent,
                &same_preview,
                &context,
                &prices,
                &transport,
                "test-only"
            )
            .is_err());
        assert_eq!(transport.calls.load(Ordering::SeqCst), 1);
        let persisted = services
            .store
            .get_external_effect(ExternalEffectKey {
                project,
                study: study_id,
                run_id: &run_id,
                step_id: "draft",
                idempotency_key: &idem,
            })
            .unwrap()
            .unwrap();
        assert_eq!(persisted.status, ExternalEffectStatus::Completed);
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
