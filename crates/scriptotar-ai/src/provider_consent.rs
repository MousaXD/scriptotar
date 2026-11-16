//! Explicit, one-shot AI egress consent. No credentials or approval tokens are persisted here.
//!
//! A production caller must supply a DB-backed ScopeAuthority, a trusted and current pricing
//! catalog, and a durable, externally idempotent EffectFence. The legacy unmetered HTTP
//! adapters are deliberately NOT eligible to dispatch via this policy.
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{AiRequest, EndpointPolicy, ProviderConfig, ProviderKind};

const MAX_PROMPT_BYTES: usize = 500_000;
const MAX_RESPONSE_BYTES: usize = 1_048_576;
const MAX_SELECTED_SOURCES: usize = 8;
const MAX_OUTPUT_TOKENS: u64 = 8_192;
const MAX_ELAPSED_MS: u64 = 120_000;
const PROMPT_OVERHEAD_TOKENS: u64 = 512;
const CONSENT_TTL_MS: u64 = 300_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConsentError {
    InvalidRequest,
    WrongProjectOrStudy,
    ChangedContext,
    InvalidBudget,
    UnknownPricing,
    NoApproval,
    Denied,
    AlreadyUsed,
    Expired,
    CredentialsRequired,
    EffectFenceUnavailable,
    UncertainEffect,
    ProviderFailed,
    UsageOverBudget,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StudyScope {
    pub project_id: String,
    pub study_id: String,
    pub study_revision: i64,
    /// SHA-256 over the server-read brief, evidence, evidence reviews, and source content
    /// references. Never accept a client-supplied digest as an authoritative snapshot.
    pub material_digest: String,
    /// Digest of the exact canonical prompt rebuilt by the trusted Studio service.
    pub expected_prompt_digest: String,
    pub selected_source_ids: Vec<String>,
}

/// Implementations must use the currently active project and read canonical Study data.
/// A guard backed by the SAME synchronization boundary as every project switch,
/// Study/source/evidence write, and consent revocation. The approved snapshot
/// must remain immutable until this guard is dropped. Returning an unguarded
/// snapshot is a security bug in an authority implementation.
pub trait ScopeDispatchGuard {
    fn scope(&self) -> &StudyScope;

    /// Fresh trusted host time, read while the dispatch guard is held.
    /// The earlier dispatch(now_ms) argument may predate a blocked reservation.
    fn now_ms(&self) -> Result<u64, ConsentError>;
}

pub trait ScopeAuthority: Send + Sync {
    fn current_scope(&self, project_id: &str, study_id: &str) -> Result<StudyScope, ConsentError>;

    /// Linearization boundary for egress: the guard must block all changes that
    /// would invalidate this scope until the provider send has completed. A
    /// plain re-read cannot satisfy this contract. Fail closed until Agent 00
    /// installs a process-wide, shared, authoritative host implementation.
    fn lock_dispatch_scope(
        &self,
        _scope: &StudyScope,
    ) -> Result<Box<dyn ScopeDispatchGuard + '_>, ConsentError> {
        Err(ConsentError::EffectFenceUnavailable)
    }
}

/// This is an explicit run intent; the server must prepare the prompt and confirm its scope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderIntent {
    pub run_id: String,
    pub scope: StudyScope,
    pub provider: ProviderConfig,
    pub prompt: String,
    pub max_output_tokens: u64,
    pub limits: RunBudget,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunBudget {
    pub max_tokens: u64,
    pub max_tool_calls: u32,
    pub max_elapsed_ms: u64,
    pub max_cost_microusd: u64,
}

/// Prices are micro-USD per 1,000,000 tokens. Only the trusted host catalog may set them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PriceCeiling {
    pub input_microusd_per_million: u64,
    pub output_microusd_per_million: u64,
    pub catalog_revision: String,
}

pub trait TrustedPricing: Send + Sync {
    fn ceiling(&self, config: &ProviderConfig) -> Option<PriceCeiling>;
}

/// The production default intentionally contains no guessed or stale provider prices.
pub struct NoTrustedPricing;
impl TrustedPricing for NoTrustedPricing {
    fn ceiling(&self, _config: &ProviderConfig) -> Option<PriceCeiling> {
        None
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsentPreview {
    pub run_id: String,
    pub fingerprint: String,
    pub provider: ProviderKind,
    pub model: String,
    pub destination: String,
    pub prompt: String,
    pub selected_source_ids: Vec<String>,
    pub estimated_input_tokens: u64,
    pub reserved_input_tokens: u64,
    pub max_output_tokens: u64,
    pub max_total_tokens: u64,
    /// None means billing is unknown and paid approval is blocked.
    pub worst_case_cost_microusd: Option<u64>,
    pub limits: RunBudget,
    pub expires_at_ms: u64,
    pub egress_disclosure: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Decision {
    Pending,
    Approved,
    /// Approved request is awaiting durable reservation; cancellation is
    /// permitted until the thread reacquires this run's lock for pre-send CAS.
    Reserving,
    Denied,
    Uncertain,
    Complete,
}

struct SavedConsent {
    intent: ProviderIntent,
    preview: ConsentPreview,
    decision: Decision,
}

/// A process restart drops pending consent. It never silently reconstructs approval.
#[derive(Default)]
pub struct ConsentBoundary {
    records: Mutex<HashMap<String, Arc<Mutex<SavedConsent>>>>,
}

/// The fence must atomically claim a never-before-dispatched provider effect in the
/// durable ledger, then durably settle its receipt. Missing/ambiguous claim is NEVER
/// permission to send a second paid request. Agent 02 must supply this implementation.
pub trait PaidEffectFence: Send + Sync {
    fn reserve_once(&self, effect_id: &str) -> Result<(), ConsentError>;

    /// Commit the one-winner pre-send DISPATCHED CAS in the durable effect
    /// ledger, after acquiring the authoritative scope/consent gate and before
    /// provider I/O. A reserved effect alone never authorizes a network send.
    /// Agent 02's v10 driver must implement this; fail closed by default.
    fn mark_dispatched_once(&self, _effect_id: &str) -> Result<(), ConsentError> {
        Err(ConsentError::EffectFenceUnavailable)
    }

    /// Persist the authenticated provider request ID with metered usage.
    /// Never persist prompt text, credentials or provider response bodies.
    /// A failed acknowledgement/settlement is uncertain and must not replay.
    fn settle(
        &self,
        effect_id: &str,
        provider_request_id: &str,
        usage: &ActualUsage,
    ) -> Result<(), ConsentError>;
    fn mark_uncertain(&self, effect_id: &str);
}

pub struct DisabledPaidEffectFence;
impl PaidEffectFence for DisabledPaidEffectFence {
    fn reserve_once(&self, _effect_id: &str) -> Result<(), ConsentError> {
        Err(ConsentError::EffectFenceUnavailable)
    }

    fn settle(
        &self,
        _effect_id: &str,
        _provider_request_id: &str,
        _usage: &ActualUsage,
    ) -> Result<(), ConsentError> {
        Err(ConsentError::EffectFenceUnavailable)
    }

    fn mark_uncertain(&self, _effect_id: &str) {}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActualUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub tool_calls: u32,
    pub elapsed_ms: u64,
    pub cost_microusd: u64,
}

pub struct MeteredReply {
    pub text: String,
    /// Opaque ID received from the authenticated provider response. Never
    /// derive this from the prompt, API key or our local effect identity.
    pub provider_request_id: String,
    pub usage: ActualUsage,
}

/// The runner must enforce max_output_tokens and deadline *on the actual HTTP request*,
/// return verifiable usage and prohibit automatic provider retries or redirects.
pub trait BoundedProvider: Send + Sync {
    fn supports_hard_bounds(&self, config: &ProviderConfig) -> bool;

    /// Validate a real authenticated provider response with a nonempty
    /// provider-issued request ID and provider-metered token usage. A mock
    /// estimate, a locally synthesized ID, or a legacy text-only response
    /// MUST NOT settle a paid effect. Deny by default.
    fn verified_usage_receipt(&self, _config: &ProviderConfig, _reply: &MeteredReply) -> bool {
        false
    }

    fn generate_once(
        &self,
        config: &ProviderConfig,
        api_key: &str,
        request: &AiRequest,
        max_output_tokens: u64,
        deadline_ms: u64,
    ) -> Result<MeteredReply, ConsentError>;
}

/// Existing AiService/HttpAiProvider support endpoint preflight but DO NOT yet supply
/// hard output token limits or metered receipts across all providers. No paid dispatch.
pub struct ExistingAiServiceAdapter;
impl BoundedProvider for ExistingAiServiceAdapter {
    fn supports_hard_bounds(&self, _config: &ProviderConfig) -> bool {
        false
    }

    fn generate_once(
        &self,
        _config: &ProviderConfig,
        _api_key: &str,
        _request: &AiRequest,
        _max_output_tokens: u64,
        _deadline_ms: u64,
    ) -> Result<MeteredReply, ConsentError> {
        Err(ConsentError::EffectFenceUnavailable)
    }
}

fn scoped_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|value| value.is_ascii_alphanumeric() || matches!(value, b'-' | b'_'))
}

fn valid_provider_request_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn canonical_scope(scope: &StudyScope) -> bool {
    scoped_id(&scope.project_id)
        && scoped_id(&scope.study_id)
        && scope.study_revision >= 0
        && scope.material_digest.len() == 64
        && scope
            .material_digest
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
        && scope.expected_prompt_digest.len() == 64
        && scope
            .expected_prompt_digest
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
        && !scope.selected_source_ids.is_empty()
        && scope.selected_source_ids.len() <= MAX_SELECTED_SOURCES
        && scope.selected_source_ids.iter().all(|id| scoped_id(id))
        && scope
            .selected_source_ids
            .windows(2)
            .all(|pair| pair[0] < pair[1])
}

fn hash_intent(intent: &ProviderIntent, pricing: &Option<PriceCeiling>) -> String {
    // Serialization cannot fail for these concrete, finite, non-floating-point fields.
    let data = serde_json::to_vec(&(intent, pricing)).expect("intent is serializable");
    sha256_hex(&data)
}

fn bounded_microusd(tokens: u64, per_million: u64) -> Option<u64> {
    let numerator = u128::from(tokens).checked_mul(u128::from(per_million))?;
    let result = numerator.checked_add(999_999)?.checked_div(1_000_000)?;
    u64::try_from(result).ok()
}

fn reserved_cost(input: u64, output: u64, ceiling: &PriceCeiling) -> Option<u64> {
    if ceiling.input_microusd_per_million == 0
        || ceiling.output_microusd_per_million == 0
        || ceiling.catalog_revision.is_empty()
    {
        return None;
    }
    bounded_microusd(input, ceiling.input_microusd_per_million)?.checked_add(bounded_microusd(
        output,
        ceiling.output_microusd_per_million,
    )?)
}

fn validate_context(
    scope: &StudyScope,
    authority: &dyn ScopeAuthority,
) -> Result<(), ConsentError> {
    let current = authority.current_scope(&scope.project_id, &scope.study_id)?;
    if !canonical_scope(&current)
        || current.project_id != scope.project_id
        || current.study_id != scope.study_id
    {
        return Err(ConsentError::WrongProjectOrStudy);
    }
    if current != *scope {
        return Err(ConsentError::ChangedContext);
    }
    Ok(())
}

impl ConsentBoundary {
    pub fn prepare(
        &self,
        intent: ProviderIntent,
        authority: &dyn ScopeAuthority,
        pricing: &dyn TrustedPricing,
        now_ms: u64,
    ) -> Result<ConsentPreview, ConsentError> {
        if !scoped_id(&intent.run_id)
            || !canonical_scope(&intent.scope)
            || intent.prompt.trim().is_empty()
            || intent.prompt.len() > MAX_PROMPT_BYTES
            || intent.max_output_tokens == 0
            || intent.max_output_tokens > MAX_OUTPUT_TOKENS
            || intent.provider.provider == ProviderKind::Local
            || intent.limits.max_tool_calls != 1
            || intent.limits.max_elapsed_ms == 0
            || intent.limits.max_elapsed_ms > MAX_ELAPSED_MS
            || intent.limits.max_cost_microusd == 0
        {
            return Err(ConsentError::InvalidRequest);
        }
        validate_context(&intent.scope, authority)?;
        if sha256_hex(intent.prompt.as_bytes()) != intent.scope.expected_prompt_digest {
            return Err(ConsentError::ChangedContext);
        }
        let endpoint = EndpointPolicy
            .endpoint_for(&intent.provider)
            .map_err(|_| ConsentError::InvalidRequest)?;
        let reserved_input_tokens = u64::try_from(intent.prompt.len())
            .map_err(|_| ConsentError::InvalidBudget)?
            .checked_add(PROMPT_OVERHEAD_TOKENS)
            .ok_or(ConsentError::InvalidBudget)?;
        let total = reserved_input_tokens
            .checked_add(intent.max_output_tokens)
            .ok_or(ConsentError::InvalidBudget)?;
        if total > intent.limits.max_tokens {
            return Err(ConsentError::InvalidBudget);
        }
        let ceiling = pricing.ceiling(&intent.provider);
        let worst_case_cost_microusd = ceiling.as_ref().and_then(|price| {
            reserved_cost(reserved_input_tokens, intent.max_output_tokens, price)
        });
        if worst_case_cost_microusd.is_some_and(|max| max > intent.limits.max_cost_microusd) {
            return Err(ConsentError::InvalidBudget);
        }
        let fingerprint = hash_intent(&intent, &ceiling);
        let preview = ConsentPreview {
            run_id: intent.run_id.clone(),
            fingerprint,
            provider: intent.provider.provider,
            model: intent.provider.model.clone(),
            destination: endpoint.as_url().to_string(),
            prompt: intent.prompt.clone(),
            selected_source_ids: intent.scope.selected_source_ids.clone(),
            estimated_input_tokens: u64::try_from(intent.prompt.chars().count())
                .unwrap_or(u64::MAX)
                .div_ceil(4),
            reserved_input_tokens,
            max_output_tokens: intent.max_output_tokens,
            max_total_tokens: total,
            worst_case_cost_microusd,
            limits: intent.limits,
            expires_at_ms: now_ms
                .saturating_add(intent.limits.max_elapsed_ms.min(CONSENT_TTL_MS)),
            egress_disclosure: format!(
                "The exact previewed prompt and selected Study evidence will be sent to {}.                  The provider may process or retain the data under its own terms.                  Approval is for one request only; credentials are supplied only at dispatch.",
                endpoint.as_url().host_str().unwrap_or("the selected provider")
            ),
        };
        let mut records = self
            .records
            .lock()
            .map_err(|_| ConsentError::EffectFenceUnavailable)?;
        match records.entry(intent.run_id.clone()) {
            std::collections::hash_map::Entry::Vacant(slot) => {
                slot.insert(Arc::new(Mutex::new(SavedConsent {
                    intent,
                    preview: preview.clone(),
                    decision: Decision::Pending,
                })));
                Ok(preview)
            }
            std::collections::hash_map::Entry::Occupied(_) => Err(ConsentError::AlreadyUsed),
        }
    }

    pub fn decide(
        &self,
        run_id: &str,
        fingerprint: &str,
        approve: bool,
        authority: &dyn ScopeAuthority,
        now_ms: u64,
    ) -> Result<(), ConsentError> {
        let record = self
            .records
            .lock()
            .map_err(|_| ConsentError::EffectFenceUnavailable)?
            .get(run_id)
            .cloned()
            .ok_or(ConsentError::NoApproval)?;
        let mut record = record
            .lock()
            .map_err(|_| ConsentError::EffectFenceUnavailable)?;
        if record.decision != Decision::Pending {
            return Err(ConsentError::AlreadyUsed);
        }
        if record.preview.fingerprint != fingerprint {
            return Err(ConsentError::ChangedContext);
        }
        if now_ms > record.preview.expires_at_ms {
            record.decision = Decision::Denied;
            return Err(ConsentError::Expired);
        }
        if !approve {
            record.decision = Decision::Denied;
            return Ok(());
        }
        validate_context(&record.intent.scope, authority)?;
        if record.preview.worst_case_cost_microusd.is_none() {
            return Err(ConsentError::UnknownPricing);
        }
        record.decision = Decision::Approved;
        Ok(())
    }

    pub fn cancel(&self, run_id: &str) -> Result<(), ConsentError> {
        let record = self
            .records
            .lock()
            .map_err(|_| ConsentError::EffectFenceUnavailable)?
            .get(run_id)
            .cloned()
            .ok_or(ConsentError::NoApproval)?;
        let mut record = record
            .lock()
            .map_err(|_| ConsentError::EffectFenceUnavailable)?;
        match record.decision {
            Decision::Pending | Decision::Approved | Decision::Reserving => {
                record.decision = Decision::Denied;
                Ok(())
            }
            Decision::Uncertain => Err(ConsentError::UncertainEffect),
            _ => Err(ConsentError::AlreadyUsed),
        }
    }

    /// There is intentionally no retry entry point. Any transport failure, cancellation
    /// race or receipt-write failure becomes an uncertain paid effect until reconciliation.
    // The safety boundary keeps each dependency explicit for auditing and test injection.
    #[allow(clippy::too_many_arguments)]
    pub fn dispatch(
        &self,
        intent: &ProviderIntent,
        authority: &dyn ScopeAuthority,
        pricing: &dyn TrustedPricing,
        fence: &dyn PaidEffectFence,
        runner: &dyn BoundedProvider,
        api_key: &str,
        now_ms: u64,
    ) -> Result<MeteredReply, ConsentError> {
        if !runner.supports_hard_bounds(&intent.provider) {
            return Err(ConsentError::EffectFenceUnavailable);
        }
        if api_key.trim().is_empty() {
            return Err(ConsentError::CredentialsRequired);
        }
        let consent = self
            .records
            .lock()
            .map_err(|_| ConsentError::EffectFenceUnavailable)?
            .get(&intent.run_id)
            .cloned()
            .ok_or(ConsentError::NoApproval)?;
        let mut record = consent
            .lock()
            .map_err(|_| ConsentError::EffectFenceUnavailable)?;
        match record.decision {
            Decision::Pending => return Err(ConsentError::NoApproval),
            Decision::Denied => return Err(ConsentError::Denied),
            Decision::Uncertain => return Err(ConsentError::UncertainEffect),
            Decision::Complete | Decision::Reserving => return Err(ConsentError::AlreadyUsed),
            Decision::Approved => {}
        }
        if now_ms > record.preview.expires_at_ms {
            record.decision = Decision::Denied;
            return Err(ConsentError::Expired);
        }
        // Reject any mutation of the approved request *before* looking up its
        // provider/model in the trusted catalog. An unknown price for a
        // substituted model must never mask the changed-context denial.
        if record.intent != *intent {
            return Err(ConsentError::ChangedContext);
        }
        validate_context(&record.intent.scope, authority)?;
        let trusted_price = pricing
            .ceiling(&record.intent.provider)
            .ok_or(ConsentError::UnknownPricing)?;
        if record.preview.fingerprint != hash_intent(&record.intent, &Some(trusted_price.clone())) {
            return Err(ConsentError::ChangedContext);
        }
        let expected_ceiling = record
            .preview
            .worst_case_cost_microusd
            .ok_or(ConsentError::UnknownPricing)?;
        if expected_ceiling > intent.limits.max_cost_microusd {
            return Err(ConsentError::InvalidBudget);
        }
        let effect_id = format!("provider:{}:{}", intent.run_id, record.preview.fingerprint);
        let deadline_ms = now_ms.saturating_add(intent.limits.max_elapsed_ms);

        // Release ONLY the per-run approval lock while a durable reservation
        // waits on SQLite or another process. A cancellation can then
        // linearize and deny this run *before* provider dispatch. Reserving
        // prevents another caller from attempting the same durable effect.
        record.decision = Decision::Reserving;
        drop(record);
        let reservation = fence.reserve_once(&effect_id);
        let mut record = match consent.lock() {
            Ok(record) => record,
            Err(_) => {
                // A claim may already have committed before its response.
                fence.mark_uncertain(&effect_id);
                return Err(ConsentError::UncertainEffect);
            }
        };
        // A failed reservation is ambiguous: it may have committed already.
        // Never retry it, even if cancellation completed while it was pending.
        if let Err(error) = reservation {
            record.decision = Decision::Uncertain;
            return Err(error);
        }
        if record.decision != Decision::Reserving {
            // The cancellation acquired the approval lock first. A reserved
            // effect is not authorization to send and must be reconciled.
            fence.mark_uncertain(&effect_id);
            return Err(ConsentError::Denied);
        }
        // Waiting on the host's authorization gate must NOT hold the per-run
        // consent mutex. Cancellation can linearize before the final durable
        // dispatch CAS even if another host mutation holds that gate.
        drop(record);

        // Agent 10's regression changes the Study inside reserve_once. This
        // re-read catches that specific transition; the exclusive guard below
        // is still the required pre-egress concurrency boundary.
        if let Err(error) = validate_context(&intent.scope, authority) {
            fence.mark_uncertain(&effect_id);
            if let Ok(mut pending) = consent.lock() {
                if pending.decision == Decision::Reserving {
                    pending.decision = Decision::Uncertain;
                }
            }
            return Err(error);
        }

        // All project/Study/source mutations and revocations must acquire this
        // SAME gate. Do not hold a SQLite write transaction during HTTP I/O.
        let scope_guard = match authority.lock_dispatch_scope(&intent.scope) {
            Ok(guard) => guard,
            Err(_) => {
                fence.mark_uncertain(&effect_id);
                if let Ok(mut pending) = consent.lock() {
                    if pending.decision == Decision::Reserving {
                        pending.decision = Decision::Uncertain;
                    }
                }
                return Err(ConsentError::UncertainEffect);
            }
        };

        // Reacquire consent only after the authoritative guard. A cancel that
        // won while the gate was blocked must prevent both the pre-send CAS
        // and network I/O. Retain the lock from this point through the send.
        let mut record = match consent.lock() {
            Ok(record) => record,
            Err(_) => {
                fence.mark_uncertain(&effect_id);
                return Err(ConsentError::UncertainEffect);
            }
        };
        if record.decision != Decision::Reserving {
            fence.mark_uncertain(&effect_id);
            return Err(ConsentError::Denied);
        }
        record.decision = Decision::Uncertain;
        if scope_guard.scope() != &record.intent.scope
            || pricing.ceiling(&record.intent.provider) != Some(trusted_price.clone())
        {
            fence.mark_uncertain(&effect_id);
            return Err(ConsentError::ChangedContext);
        }
        // Validate the *actual* time under the guard. A slow effect reservation
        // must never extend the one-shot approval's lifetime.
        let fresh_now = match scope_guard.now_ms() {
            Ok(value) => value,
            Err(_) => {
                fence.mark_uncertain(&effect_id);
                return Err(ConsentError::UncertainEffect);
            }
        };
        if fresh_now > record.preview.expires_at_ms || fresh_now >= deadline_ms {
            fence.mark_uncertain(&effect_id);
            return Err(ConsentError::Expired);
        }

        // Agent 02's CAS must commit DISPATCHED before network I/O; never
        // treat the RESERVED state as permission to call a provider.
        if fence.mark_dispatched_once(&effect_id).is_err() {
            fence.mark_uncertain(&effect_id);
            return Err(ConsentError::UncertainEffect);
        }
        // The CAS can also block. Re-read trusted time at the final send
        // boundary, not only when reservation or gate acquisition completed.
        match scope_guard.now_ms() {
            Ok(value) if value <= record.preview.expires_at_ms && value < deadline_ms => {}
            Ok(_) => {
                fence.mark_uncertain(&effect_id);
                return Err(ConsentError::Expired);
            }
            Err(_) => {
                fence.mark_uncertain(&effect_id);
                return Err(ConsentError::UncertainEffect);
            }
        }

        // Keep only this run's consent mutex through the send. A concurrent
        // cancellation linearizes after the already-started attempt, but other
        // runs can still prepare, approve, cancel or dispatch independently.
        let generated = runner.generate_once(
            &intent.provider,
            api_key,
            &AiRequest {
                prompt: intent.prompt.clone(),
            },
            intent.max_output_tokens,
            deadline_ms,
        );
        drop(scope_guard);
        drop(record);
        let mut generated = match generated {
            Ok(response) => response,
            Err(_) => {
                fence.mark_uncertain(&effect_id);
                return Err(ConsentError::UncertainEffect);
            }
        };
        // A fabricated usage estimate or an HTTP response missing its actual
        // authenticated provider request ID cannot resolve the durable effect.
        // Bound text before returning it; real adapters must also bound
        // response buffering at the transport layer.
        if !valid_provider_request_id(&generated.provider_request_id)
            || !runner.verified_usage_receipt(&intent.provider, &generated)
        {
            fence.mark_uncertain(&effect_id);
            return Err(ConsentError::UncertainEffect);
        }
        if generated.text.len() > MAX_RESPONSE_BYTES {
            fence.mark_uncertain(&effect_id);
            return Err(ConsentError::UsageOverBudget);
        }
        let mut usage = generated.usage;
        // USD accounting is owned by this trusted policy, never by a provider
        // adapter's estimated/request-supplied cost field.
        usage.cost_microusd =
            match reserved_cost(usage.input_tokens, usage.output_tokens, &trusted_price) {
                Some(cost) => cost,
                None => {
                    fence.mark_uncertain(&effect_id);
                    return Err(ConsentError::UncertainEffect);
                }
            };
        generated.usage = usage;
        if usage.input_tokens
            > u64::try_from(intent.prompt.len())
                .unwrap_or(u64::MAX)
                .saturating_add(PROMPT_OVERHEAD_TOKENS)
            || usage.output_tokens > intent.max_output_tokens
            || usage.input_tokens.saturating_add(usage.output_tokens) > intent.limits.max_tokens
            || usage.tool_calls > intent.limits.max_tool_calls
            || usage.elapsed_ms > intent.limits.max_elapsed_ms
            || usage.cost_microusd > intent.limits.max_cost_microusd
            || usage.cost_microusd > expected_ceiling
        {
            fence.mark_uncertain(&effect_id);
            return Err(ConsentError::UsageOverBudget);
        }
        if fence
            .settle(&effect_id, &generated.provider_request_id, &usage)
            .is_err()
        {
            fence.mark_uncertain(&effect_id);
            return Err(ConsentError::UncertainEffect);
        }
        let mut record = consent.lock().map_err(|_| ConsentError::UncertainEffect)?;
        record.decision = Decision::Complete;
        Ok(generated)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

    use super::*;

    // Static authority is immutable by construction; no writer can race the
    // returned guard. Mutable authorities below use a real exclusive mutex.
    struct FrozenGuard<'a>(&'a StudyScope);
    impl ScopeDispatchGuard for FrozenGuard<'_> {
        fn scope(&self) -> &StudyScope {
            self.0
        }

        fn now_ms(&self) -> Result<u64, ConsentError> {
            // Synthetic time for the immutable fixture; production must read
            // the actual trusted host clock while holding the shared gate.
            Ok(103)
        }
    }

    struct Authority(StudyScope);
    impl ScopeAuthority for Authority {
        fn current_scope(
            &self,
            project_id: &str,
            study_id: &str,
        ) -> Result<StudyScope, ConsentError> {
            if project_id != self.0.project_id || study_id != self.0.study_id {
                return Err(ConsentError::WrongProjectOrStudy);
            }
            Ok(self.0.clone())
        }

        fn lock_dispatch_scope(
            &self,
            _scope: &StudyScope,
        ) -> Result<Box<dyn ScopeDispatchGuard + '_>, ConsentError> {
            Ok(Box::new(FrozenGuard(&self.0)))
        }
    }

    struct Prices;
    impl TrustedPricing for Prices {
        fn ceiling(&self, config: &ProviderConfig) -> Option<PriceCeiling> {
            (config.provider == ProviderKind::OpenAi && config.model == "test-model").then(|| {
                PriceCeiling {
                    input_microusd_per_million: 100_000,
                    output_microusd_per_million: 200_000,
                    catalog_revision: "fixture".into(),
                }
            })
        }
    }

    #[derive(Default)]
    struct Fence {
        claims: Mutex<Vec<String>>,
        receipts: Mutex<Vec<String>>,
        settled: AtomicUsize,
        uncertain: AtomicUsize,
    }
    impl PaidEffectFence for Fence {
        fn reserve_once(&self, id: &str) -> Result<(), ConsentError> {
            let mut claims = self.claims.lock().unwrap();
            if claims.iter().any(|item| item == id) {
                return Err(ConsentError::UncertainEffect);
            }
            claims.push(id.into());
            Ok(())
        }

        fn mark_dispatched_once(&self, _id: &str) -> Result<(), ConsentError> {
            Ok(())
        }

        fn settle(
            &self,
            _id: &str,
            provider_request_id: &str,
            _usage: &ActualUsage,
        ) -> Result<(), ConsentError> {
            self.receipts.lock().unwrap().push(provider_request_id.into());
            self.settled.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        fn mark_uncertain(&self, _id: &str) {
            self.uncertain.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[derive(Default)]
    struct Runner {
        calls: AtomicUsize,
        fail: bool,
        over_budget: bool,
    }
    impl BoundedProvider for Runner {
        fn supports_hard_bounds(&self, _: &ProviderConfig) -> bool {
            true
        }

        fn verified_usage_receipt(&self, _: &ProviderConfig, reply: &MeteredReply) -> bool {
            reply.provider_request_id == "fixture-request-id"
        }

        fn generate_once(
            &self,
            _: &ProviderConfig,
            _: &str,
            request: &AiRequest,
            _: u64,
            _: u64,
        ) -> Result<MeteredReply, ConsentError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            if self.fail {
                return Err(ConsentError::ProviderFailed);
            }
            Ok(MeteredReply {
                text: request.prompt.clone(),
                provider_request_id: "fixture-request-id".into(),
                usage: ActualUsage {
                    input_tokens: 9,
                    output_tokens: if self.over_budget { 9_999 } else { 30 },
                    tool_calls: 1,
                    elapsed_ms: 100,
                    cost_microusd: 50,
                },
            })
        }
    }

    fn fixture() -> (ProviderIntent, Authority) {
        let scope = StudyScope {
            project_id: "project-1".into(),
            study_id: "study-1".into(),
            study_revision: 3,
            material_digest: "a".repeat(64),
            expected_prompt_digest: sha256_hex("Original brief and evidence".as_bytes()),
            selected_source_ids: vec!["source-1".into()],
        };
        (
            ProviderIntent {
                run_id: "run-1".into(),
                scope: scope.clone(),
                provider: ProviderConfig {
                    provider: ProviderKind::OpenAi,
                    model: "test-model".into(),
                    base_url: None,
                },
                prompt: "Original brief and evidence".into(),
                max_output_tokens: 100,
                limits: RunBudget {
                    max_tokens: 10_000,
                    max_tool_calls: 1,
                    max_elapsed_ms: 10_000,
                    max_cost_microusd: 1000,
                },
            },
            Authority(scope),
        )
    }

    fn approved() -> (ConsentBoundary, ProviderIntent, Authority) {
        let (intent, authority) = fixture();
        let boundary = ConsentBoundary::default();
        let preview = boundary
            .prepare(intent.clone(), &authority, &Prices, 100)
            .unwrap();
        assert!(preview.egress_disclosure.contains("api.openai.com"));
        boundary
            .decide(&intent.run_id, &preview.fingerprint, true, &authority, 101)
            .unwrap();
        (boundary, intent, authority)
    }

    #[test]
    fn never_dispatch_without_approval_or_after_denial() {
        let (intent, authority) = fixture();
        let boundary = ConsentBoundary::default();
        let runner = Runner::default();
        let fence = Fence::default();
        let preview = boundary
            .prepare(intent.clone(), &authority, &Prices, 10)
            .unwrap();
        assert!(matches!(
            boundary.dispatch(&intent, &authority, &Prices, &fence, &runner, "key", 11),
            Err(ConsentError::NoApproval)
        ));
        boundary
            .decide(&intent.run_id, &preview.fingerprint, false, &authority, 12)
            .unwrap();
        assert!(matches!(
            boundary.dispatch(&intent, &authority, &Prices, &fence, &runner, "key", 13),
            Err(ConsentError::Denied)
        ));
        assert_eq!(runner.calls.load(Ordering::SeqCst), 0);
        assert!(fence.claims.lock().unwrap().is_empty());
    }

    #[test]
    fn exact_prompt_model_sources_and_evidence_are_bound() {
        let (boundary, intent, authority) = approved();
        let runner = Runner::default();
        let fence = Fence::default();
        for changed in [
            ProviderIntent {
                prompt: "changed".into(),
                ..intent.clone()
            },
            ProviderIntent {
                provider: ProviderConfig {
                    model: "other".into(),
                    ..intent.provider.clone()
                },
                ..intent.clone()
            },
            ProviderIntent {
                scope: StudyScope {
                    material_digest: "b".repeat(64),
                    ..intent.scope.clone()
                },
                ..intent.clone()
            },
            ProviderIntent {
                scope: StudyScope {
                    selected_source_ids: vec!["source-2".into()],
                    ..intent.scope.clone()
                },
                ..intent.clone()
            },
            ProviderIntent {
                limits: RunBudget {
                    max_cost_microusd: 2_000,
                    ..intent.limits
                },
                ..intent.clone()
            },
        ] {
            assert!(matches!(
                boundary.dispatch(&changed, &authority, &Prices, &fence, &runner, "key", 102),
                Err(ConsentError::ChangedContext)
            ));
        }
        assert_eq!(runner.calls.load(Ordering::SeqCst), 0);
        assert!(fence.claims.lock().unwrap().is_empty());
    }

    #[test]
    fn foreign_project_and_stale_study_never_call_provider() {
        let (intent, authority) = fixture();
        let boundary = ConsentBoundary::default();
        let foreign = ProviderIntent {
            scope: StudyScope {
                project_id: "foreign".into(),
                ..intent.scope.clone()
            },
            ..intent.clone()
        };
        assert!(matches!(
            boundary.prepare(foreign, &authority, &Prices, 10),
            Err(ConsentError::WrongProjectOrStudy)
        ));
        let (boundary, intent, _) = approved();
        let changed_authority = Authority(StudyScope {
            study_revision: 4,
            ..authority.0
        });
        let runner = Runner::default();
        assert!(matches!(
            boundary.dispatch(
                &intent,
                &changed_authority,
                &Prices,
                &Fence::default(),
                &runner,
                "key",
                103
            ),
            Err(ConsentError::ChangedContext)
        ));
        assert_eq!(runner.calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn unknown_price_and_insufficient_limits_fail_closed() {
        let (intent, authority) = fixture();
        let boundary = ConsentBoundary::default();
        let preview = boundary
            .prepare(intent.clone(), &authority, &NoTrustedPricing, 10)
            .unwrap();
        assert_eq!(preview.worst_case_cost_microusd, None);
        assert!(matches!(
            boundary.decide(&intent.run_id, &preview.fingerprint, true, &authority, 11),
            Err(ConsentError::UnknownPricing)
        ));
        let mut tiny = intent;
        tiny.run_id = "run-2".into();
        tiny.limits.max_tokens = 10;
        assert!(matches!(
            boundary.prepare(tiny, &authority, &Prices, 11),
            Err(ConsentError::InvalidBudget)
        ));
    }

    #[test]
    fn hard_output_ceiling_and_one_time_claim() {
        let (boundary, intent, authority) = approved();
        let runner = Runner::default();
        let fence = Fence::default();
        boundary
            .dispatch(&intent, &authority, &Prices, &fence, &runner, "key", 103)
            .unwrap();
        assert_eq!(fence.settled.load(Ordering::SeqCst), 1);
        assert_eq!(
            fence.receipts.lock().unwrap().as_slice(),
            &["fixture-request-id"]
        );
        assert!(matches!(
            boundary.dispatch(&intent, &authority, &Prices, &fence, &runner, "key", 104),
            Err(ConsentError::AlreadyUsed)
        ));
        assert_eq!(runner.calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn disabled_fence_or_existing_unbounded_adapter_makes_no_request() {
        let (boundary, intent, authority) = approved();
        let runner = Runner::default();
        assert!(matches!(
            boundary.dispatch(
                &intent,
                &authority,
                &Prices,
                &DisabledPaidEffectFence,
                &runner,
                "key",
                104
            ),
            Err(ConsentError::EffectFenceUnavailable)
        ));
        assert_eq!(runner.calls.load(Ordering::SeqCst), 0);
        assert!(matches!(
            boundary.dispatch(
                &intent,
                &authority,
                &Prices,
                &Fence::default(),
                &ExistingAiServiceAdapter,
                "key",
                104
            ),
            Err(ConsentError::EffectFenceUnavailable)
        ));
    }

    #[test]
    fn provider_failure_or_overspend_is_uncertain_and_cannot_retry() {
        for failing in [false, true] {
            let (boundary, intent, authority) = approved();
            let fence = Fence::default();
            let runner = Runner {
                fail: failing,
                over_budget: !failing,
                ..Runner::default()
            };
            assert!(matches!(
                boundary.dispatch(&intent, &authority, &Prices, &fence, &runner, "key", 103),
                Err(ConsentError::UncertainEffect | ConsentError::UsageOverBudget)
            ));
            assert!(matches!(
                boundary.dispatch(&intent, &authority, &Prices, &fence, &runner, "key", 104),
                Err(ConsentError::UncertainEffect)
            ));
            assert_eq!(runner.calls.load(Ordering::SeqCst), 1);
            assert_eq!(fence.uncertain.load(Ordering::SeqCst), 1);
        }
    }

    #[test]
    fn cancellation_expiry_and_missing_key_prevent_spend() {
        let (boundary, intent, authority) = approved();
        let runner = Runner::default();
        let fence = Fence::default();
        assert!(matches!(
            boundary.dispatch(&intent, &authority, &Prices, &fence, &runner, "", 103),
            Err(ConsentError::CredentialsRequired)
        ));
        boundary.cancel(&intent.run_id).unwrap();
        assert!(matches!(
            boundary.dispatch(&intent, &authority, &Prices, &fence, &runner, "key", 103),
            Err(ConsentError::Denied)
        ));
        assert_eq!(runner.calls.load(Ordering::SeqCst), 0);

        let (boundary, intent, authority) = approved();
        assert!(matches!(
            boundary.dispatch(&intent, &authority, &Prices, &fence, &runner, "key", 10_102),
            Err(ConsentError::Expired)
        ));
        assert_eq!(runner.calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn ambiguous_reservation_never_attempts_a_second_paid_claim() {
        struct AmbiguousFence;
        impl PaidEffectFence for AmbiguousFence {
            fn reserve_once(&self, _: &str) -> Result<(), ConsentError> {
                Err(ConsentError::UncertainEffect)
            }
            fn settle(&self, _: &str, _: &str, _: &ActualUsage) -> Result<(), ConsentError> {
                Err(ConsentError::UncertainEffect)
            }
            fn mark_uncertain(&self, _: &str) {}
        }

        let (boundary, intent, authority) = approved();
        let runner = Runner::default();
        for _ in 0..2 {
            assert!(matches!(
                boundary.dispatch(
                    &intent,
                    &authority,
                    &Prices,
                    &AmbiguousFence,
                    &runner,
                    "key",
                    104
                ),
                Err(ConsentError::UncertainEffect)
            ));
        }
        assert_eq!(runner.calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn quoted_dollar_cap_is_enforced_before_approval() {
        let (mut intent, authority) = fixture();
        intent.limits.max_cost_microusd = 1;
        let boundary = ConsentBoundary::default();
        assert!(matches!(
            boundary.prepare(intent, &authority, &Prices, 100),
            Err(ConsentError::InvalidBudget)
        ));
    }

    #[test]
    fn concurrent_dispatches_have_only_one_winner() {
        let (boundary, intent, authority) = approved();
        let runner = Runner::default();
        let fence = Fence::default();
        std::thread::scope(|scope| {
            for _ in 0..8 {
                scope.spawn(|| {
                    let _ = boundary
                        .dispatch(&intent, &authority, &Prices, &fence, &runner, "key", 103);
                });
            }
        });
        assert_eq!(runner.calls.load(Ordering::SeqCst), 1);
        assert_eq!(fence.settled.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn trusted_pricing_overrides_adapter_supplied_cost() {
        let (boundary, intent, authority) = approved();
        let response = boundary
            .dispatch(
                &intent,
                &authority,
                &Prices,
                &Fence::default(),
                &Runner::default(),
                "key",
                103,
            )
            .unwrap();
        // Input: ceil(9 * 100_000 / 1_000_000) = 1 micro-USD.
        // Output: ceil(30 * 200_000 / 1_000_000) = 6 micro-USD.
        // The fake adapter reports 50; the server-authoritative ceiling is 7.
        assert_eq!(response.usage.cost_microusd, 7);
    }

    #[test]
    fn missing_or_unverified_provider_usage_receipts_never_settle() {
        struct ReceiptProvider {
            request_id: &'static str,
            verified: bool,
            calls: AtomicUsize,
        }
        impl BoundedProvider for ReceiptProvider {
            fn supports_hard_bounds(&self, _: &ProviderConfig) -> bool {
                true
            }
            fn verified_usage_receipt(&self, _: &ProviderConfig, _: &MeteredReply) -> bool {
                self.verified
            }
            fn generate_once(
                &self,
                _: &ProviderConfig,
                _: &str,
                _: &AiRequest,
                _: u64,
                _: u64,
            ) -> Result<MeteredReply, ConsentError> {
                self.calls.fetch_add(1, Ordering::SeqCst);
                Ok(MeteredReply {
                    text: "response".into(),
                    provider_request_id: self.request_id.into(),
                    usage: ActualUsage {
                        input_tokens: 9,
                        output_tokens: 30,
                        tool_calls: 1,
                        elapsed_ms: 100,
                        cost_microusd: 0,
                    },
                })
            }
        }

        for (request_id, verified) in [("", true), ("fixture-request-id", false)] {
            let (boundary, intent, authority) = approved();
            let fence = Fence::default();
            let provider = ReceiptProvider {
                request_id,
                verified,
                calls: AtomicUsize::new(0),
            };
            assert!(matches!(
                boundary.dispatch(&intent, &authority, &Prices, &fence, &provider, "key", 103),
                Err(ConsentError::UncertainEffect)
            ));
            assert_eq!(provider.calls.load(Ordering::SeqCst), 1);
            assert_eq!(fence.settled.load(Ordering::SeqCst), 0);
            assert_eq!(fence.uncertain.load(Ordering::SeqCst), 1);
        }
    }

    struct LockedAuthority(Mutex<StudyScope>);

    struct LockedScopeGuard<'a>(std::sync::MutexGuard<'a, StudyScope>);
    impl ScopeDispatchGuard for LockedScopeGuard<'_> {
        fn scope(&self) -> &StudyScope {
            &self.0
        }

        fn now_ms(&self) -> Result<u64, ConsentError> {
            Ok(103)
        }
    }

    impl ScopeAuthority for LockedAuthority {
        fn current_scope(
            &self,
            project_id: &str,
            study_id: &str,
        ) -> Result<StudyScope, ConsentError> {
            let current = self
                .0
                .lock()
                .map_err(|_| ConsentError::EffectFenceUnavailable)?;
            if current.project_id != project_id || current.study_id != study_id {
                return Err(ConsentError::WrongProjectOrStudy);
            }
            Ok(current.clone())
        }

        fn lock_dispatch_scope(
            &self,
            _scope: &StudyScope,
        ) -> Result<Box<dyn ScopeDispatchGuard + '_>, ConsentError> {
            let guard = self
                .0
                .lock()
                .map_err(|_| ConsentError::EffectFenceUnavailable)?;
            Ok(Box::new(LockedScopeGuard(guard)))
        }
    }

    #[test]
    fn revocation_while_reservation_is_paused_never_starts_provider_send() {
        use std::sync::mpsc;

        struct PausedFence {
            reached: mpsc::SyncSender<()>,
            resume: Mutex<mpsc::Receiver<()>>,
            uncertain: AtomicUsize,
            dispatched: AtomicUsize,
        }
        impl PaidEffectFence for PausedFence {
            fn reserve_once(&self, _: &str) -> Result<(), ConsentError> {
                self.reached.send(()).unwrap();
                self.resume.lock().unwrap().recv().unwrap();
                Ok(())
            }
            fn mark_dispatched_once(&self, _: &str) -> Result<(), ConsentError> {
                self.dispatched.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }
            fn settle(&self, _: &str, _: &str, _: &ActualUsage) -> Result<(), ConsentError> {
                Ok(())
            }
            fn mark_uncertain(&self, _: &str) {
                self.uncertain.fetch_add(1, Ordering::SeqCst);
            }
        }

        let (intent, _) = fixture();
        let authority = LockedAuthority(Mutex::new(intent.scope.clone()));
        let boundary = ConsentBoundary::default();
        let preview = boundary
            .prepare(intent.clone(), &authority, &Prices, 100)
            .unwrap();
        boundary
            .decide(&intent.run_id, &preview.fingerprint, true, &authority, 101)
            .unwrap();
        let (reached_tx, reached_rx) = mpsc::sync_channel(0);
        let (resume_tx, resume_rx) = mpsc::sync_channel(0);
        let fence = PausedFence {
            reached: reached_tx,
            resume: Mutex::new(resume_rx),
            uncertain: AtomicUsize::new(0),
            dispatched: AtomicUsize::new(0),
        };
        let provider = Runner::default();

        std::thread::scope(|scope| {
            let attempt = scope.spawn(|| {
                boundary.dispatch(&intent, &authority, &Prices, &fence, &provider, "key", 102)
            });
            reached_rx.recv().unwrap();
            authority.0.lock().unwrap().material_digest = "b".repeat(64);
            resume_tx.send(()).unwrap();
            assert!(matches!(
                attempt.join().unwrap(),
                Err(ConsentError::ChangedContext)
            ));
        });
        assert_eq!(provider.calls.load(Ordering::SeqCst), 0);
        assert_eq!(fence.dispatched.load(Ordering::SeqCst), 0);
        assert_eq!(fence.uncertain.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn dispatch_guard_is_held_through_actual_provider_invocation() {
        use std::sync::{mpsc, TryLockError};

        struct PausedProvider {
            reached: mpsc::SyncSender<()>,
            resume: Mutex<mpsc::Receiver<()>>,
            calls: AtomicUsize,
        }
        impl BoundedProvider for PausedProvider {
            fn supports_hard_bounds(&self, _: &ProviderConfig) -> bool {
                true
            }
            fn verified_usage_receipt(&self, _: &ProviderConfig, reply: &MeteredReply) -> bool {
                reply.provider_request_id == "fixture-request-id"
            }
            fn generate_once(
                &self,
                _: &ProviderConfig,
                _: &str,
                _: &AiRequest,
                _: u64,
                _: u64,
            ) -> Result<MeteredReply, ConsentError> {
                self.calls.fetch_add(1, Ordering::SeqCst);
                self.reached.send(()).unwrap();
                self.resume.lock().unwrap().recv().unwrap();
                Ok(MeteredReply {
                    text: "complete".into(),
                    provider_request_id: "fixture-request-id".into(),
                    usage: ActualUsage {
                        input_tokens: 9,
                        output_tokens: 30,
                        tool_calls: 1,
                        elapsed_ms: 100,
                        cost_microusd: 0,
                    },
                })
            }
        }

        let (intent, _) = fixture();
        let authority = LockedAuthority(Mutex::new(intent.scope.clone()));
        let boundary = ConsentBoundary::default();
        let preview = boundary
            .prepare(intent.clone(), &authority, &Prices, 100)
            .unwrap();
        boundary
            .decide(&intent.run_id, &preview.fingerprint, true, &authority, 101)
            .unwrap();
        let (reached_tx, reached_rx) = mpsc::sync_channel(0);
        let (resume_tx, resume_rx) = mpsc::sync_channel(0);
        let runner = PausedProvider {
            reached: reached_tx,
            resume: Mutex::new(resume_rx),
            calls: AtomicUsize::new(0),
        };
        let fence = Fence::default();
        std::thread::scope(|scope| {
            let attempt = scope.spawn(|| {
                boundary.dispatch(&intent, &authority, &Prices, &fence, &runner, "key", 102)
            });
            reached_rx.recv().unwrap();
            // A project switch, source removal or Study edit must acquire this
            // exact mutex. They cannot linearize while egress is in flight.
            assert!(matches!(
                authority.0.try_lock(),
                Err(TryLockError::WouldBlock)
            ));
            resume_tx.send(()).unwrap();
            assert!(attempt.join().unwrap().is_ok());
        });
        assert_eq!(runner.calls.load(Ordering::SeqCst), 1);
        assert_eq!(fence.settled.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn ungated_authority_or_missing_dispatch_cas_fails_closed() {
        struct UngatedAuthority(StudyScope);
        impl ScopeAuthority for UngatedAuthority {
            fn current_scope(
                &self,
                project_id: &str,
                study_id: &str,
            ) -> Result<StudyScope, ConsentError> {
                if project_id != self.0.project_id || study_id != self.0.study_id {
                    return Err(ConsentError::WrongProjectOrStudy);
                }
                Ok(self.0.clone())
            }
        }

        struct ReserveOnlyFence;
        impl PaidEffectFence for ReserveOnlyFence {
            fn reserve_once(&self, _: &str) -> Result<(), ConsentError> {
                Ok(())
            }
            fn settle(&self, _: &str, _: &str, _: &ActualUsage) -> Result<(), ConsentError> {
                Ok(())
            }
            fn mark_uncertain(&self, _: &str) {}
        }

        let (intent, _) = fixture();
        let authority = UngatedAuthority(intent.scope.clone());
        let boundary = ConsentBoundary::default();
        let preview = boundary
            .prepare(intent.clone(), &authority, &Prices, 100)
            .unwrap();
        boundary
            .decide(&intent.run_id, &preview.fingerprint, true, &authority, 101)
            .unwrap();
        let runner = Runner::default();
        let fence = Fence::default();
        assert!(matches!(
            boundary.dispatch(&intent, &authority, &Prices, &fence, &runner, "key", 102),
            Err(ConsentError::UncertainEffect)
        ));
        assert_eq!(runner.calls.load(Ordering::SeqCst), 0);
        assert_eq!(fence.uncertain.load(Ordering::SeqCst), 1);

        let (boundary, intent, authority) = approved();
        assert!(matches!(
            boundary.dispatch(
                &intent,
                &authority,
                &Prices,
                &ReserveOnlyFence,
                &runner,
                "key",
                103
            ),
            Err(ConsentError::UncertainEffect)
        ));
        assert_eq!(runner.calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn cancellation_preempts_stalled_reservation_and_prevents_duplicate_send() {
        use std::sync::mpsc;

        struct PendingFence {
            entered: mpsc::SyncSender<()>,
            resume: Mutex<mpsc::Receiver<()>>,
            uncertain: AtomicUsize,
            dispatched: AtomicUsize,
        }
        impl PaidEffectFence for PendingFence {
            fn reserve_once(&self, _: &str) -> Result<(), ConsentError> {
                self.entered.send(()).unwrap();
                self.resume.lock().unwrap().recv().unwrap();
                Ok(())
            }
            fn mark_dispatched_once(&self, _: &str) -> Result<(), ConsentError> {
                self.dispatched.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }
            fn settle(&self, _: &str, _: &str, _: &ActualUsage) -> Result<(), ConsentError> {
                Ok(())
            }
            fn mark_uncertain(&self, _: &str) {
                self.uncertain.fetch_add(1, Ordering::SeqCst);
            }
        }

        let (boundary, intent, authority) = approved();
        let (entered_tx, entered_rx) = mpsc::sync_channel(0);
        let (resume_tx, resume_rx) = mpsc::sync_channel(0);
        let fence = PendingFence {
            entered: entered_tx,
            resume: Mutex::new(resume_rx),
            uncertain: AtomicUsize::new(0),
            dispatched: AtomicUsize::new(0),
        };
        let runner = Runner::default();
        std::thread::scope(|scope| {
            let worker = scope.spawn(|| {
                boundary.dispatch(&intent, &authority, &Prices, &fence, &runner, "key", 102)
            });
            entered_rx.recv().unwrap();

            // The first worker has reserved its in-memory run identity but
            // has not received a durable reservation permit. No second caller
            // may claim the same run or begin a provider call.
            assert!(matches!(
                boundary.dispatch(&intent, &authority, &Prices, &fence, &runner, "key", 102),
                Err(ConsentError::AlreadyUsed)
            ));

            // Cancellation is allowed to complete *while* the external
            // reservation is blocked. It must not wait for provider I/O.
            boundary.cancel(&intent.run_id).unwrap();
            resume_tx.send(()).unwrap();
            assert!(matches!(worker.join().unwrap(), Err(ConsentError::Denied)));
        });
        assert_eq!(runner.calls.load(Ordering::SeqCst), 0);
        assert_eq!(fence.dispatched.load(Ordering::SeqCst), 0);
        assert_eq!(fence.uncertain.load(Ordering::SeqCst), 1);
        assert!(matches!(
            boundary.dispatch(&intent, &authority, &Prices, &fence, &runner, "key", 103),
            Err(ConsentError::Denied)
        ));
    }

    #[test]
    fn cancellation_while_authorization_gate_is_blocked_prevents_dispatch_cas() {
        use std::sync::mpsc;

        // Models a host mutation owning the common scope gate while a
        // reservation has completed but the dispatch guard is still blocked.
        struct WaitingGateAuthority {
            scope: StudyScope,
            entered: mpsc::SyncSender<()>,
            resume: Mutex<mpsc::Receiver<()>>,
        }
        impl ScopeAuthority for WaitingGateAuthority {
            fn current_scope(
                &self,
                project_id: &str,
                study_id: &str,
            ) -> Result<StudyScope, ConsentError> {
                if project_id != self.scope.project_id || study_id != self.scope.study_id {
                    return Err(ConsentError::WrongProjectOrStudy);
                }
                Ok(self.scope.clone())
            }

            fn lock_dispatch_scope(
                &self,
                _scope: &StudyScope,
            ) -> Result<Box<dyn ScopeDispatchGuard + '_>, ConsentError> {
                self.entered.send(()).unwrap();
                self.resume.lock().unwrap().recv().unwrap();
                Ok(Box::new(FrozenGuard(&self.scope)))
            }
        }

        let (intent, _) = fixture();
        let (entered_tx, entered_rx) = mpsc::sync_channel(0);
        let (resume_tx, resume_rx) = mpsc::sync_channel(0);
        let authority = WaitingGateAuthority {
            scope: intent.scope.clone(),
            entered: entered_tx,
            resume: Mutex::new(resume_rx),
        };
        let boundary = ConsentBoundary::default();
        let preview = boundary
            .prepare(intent.clone(), &authority, &Prices, 100)
            .unwrap();
        boundary
            .decide(&intent.run_id, &preview.fingerprint, true, &authority, 101)
            .unwrap();
        let fence = Fence::default();
        let provider = Runner::default();
        std::thread::scope(|scope| {
            let attempt = scope.spawn(|| {
                boundary.dispatch(&intent, &authority, &Prices, &fence, &provider, "key", 102)
            });
            // The dispatch has reserved its effect and is waiting to acquire
            // the authoritative host scope gate. Cancel MUST return now,
            // without waiting on the held gate or the HTTP provider.
            entered_rx.recv().unwrap();
            boundary.cancel(&intent.run_id).unwrap();
            resume_tx.send(()).unwrap();
            assert!(matches!(attempt.join().unwrap(), Err(ConsentError::Denied)));
        });
        assert_eq!(provider.calls.load(Ordering::SeqCst), 0);
        assert_eq!(fence.settled.load(Ordering::SeqCst), 0);
        assert_eq!(fence.uncertain.load(Ordering::SeqCst), 1);
        assert_eq!(fence.claims.lock().unwrap().len(), 1);
    }

    #[test]
    fn consent_expiring_inside_reserved_effect_never_starts_a_late_send() {
        use std::sync::mpsc;

        struct ClockAuthority {
            scope: StudyScope,
            now: AtomicU64,
        }
        struct ClockGuard<'a>(&'a ClockAuthority);
        impl ScopeDispatchGuard for ClockGuard<'_> {
            fn scope(&self) -> &StudyScope {
                &self.0.scope
            }

            fn now_ms(&self) -> Result<u64, ConsentError> {
                Ok(self.0.now.load(Ordering::SeqCst))
            }
        }
        impl ScopeAuthority for ClockAuthority {
            fn current_scope(
                &self,
                project_id: &str,
                study_id: &str,
            ) -> Result<StudyScope, ConsentError> {
                if self.scope.project_id != project_id || self.scope.study_id != study_id {
                    return Err(ConsentError::WrongProjectOrStudy);
                }
                Ok(self.scope.clone())
            }

            fn lock_dispatch_scope(
                &self,
                _scope: &StudyScope,
            ) -> Result<Box<dyn ScopeDispatchGuard + '_>, ConsentError> {
                Ok(Box::new(ClockGuard(self)))
            }
        }

        struct WaitingReservation {
            reached: mpsc::SyncSender<()>,
            resume: Mutex<mpsc::Receiver<()>>,
            uncertain: AtomicUsize,
            dispatched: AtomicUsize,
        }
        impl PaidEffectFence for WaitingReservation {
            fn reserve_once(&self, _: &str) -> Result<(), ConsentError> {
                self.reached.send(()).unwrap();
                self.resume.lock().unwrap().recv().unwrap();
                Ok(())
            }
            fn mark_dispatched_once(&self, _: &str) -> Result<(), ConsentError> {
                self.dispatched.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }
            fn settle(&self, _: &str, _: &str, _: &ActualUsage) -> Result<(), ConsentError> {
                Ok(())
            }
            fn mark_uncertain(&self, _: &str) {
                self.uncertain.fetch_add(1, Ordering::SeqCst);
            }
        }

        let (intent, _) = fixture();
        let authority = ClockAuthority {
            scope: intent.scope.clone(),
            now: AtomicU64::new(102),
        };
        let boundary = ConsentBoundary::default();
        let preview = boundary
            .prepare(intent.clone(), &authority, &Prices, 100)
            .unwrap();
        boundary
            .decide(&intent.run_id, &preview.fingerprint, true, &authority, 101)
            .unwrap();

        let (reached_tx, reached_rx) = mpsc::sync_channel(0);
        let (resume_tx, resume_rx) = mpsc::sync_channel(0);
        let fence = WaitingReservation {
            reached: reached_tx,
            resume: Mutex::new(resume_rx),
            uncertain: AtomicUsize::new(0),
            dispatched: AtomicUsize::new(0),
        };
        let provider = Runner::default();
        std::thread::scope(|scope| {
            let attempt = scope.spawn(|| {
                boundary.dispatch(&intent, &authority, &Prices, &fence, &provider, "key", 102)
            });
            reached_rx.recv().unwrap();
            authority.now.store(20_000, Ordering::SeqCst);
            resume_tx.send(()).unwrap();
            assert!(matches!(
                attempt.join().unwrap(),
                Err(ConsentError::Expired)
            ));
        });
        assert_eq!(provider.calls.load(Ordering::SeqCst), 0);
        assert_eq!(fence.dispatched.load(Ordering::SeqCst), 0);
        assert_eq!(fence.uncertain.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn no_approval_reconstruction_after_process_restart() {
        let (boundary, intent, authority) = approved();
        drop(boundary);
        assert!(matches!(
            ConsentBoundary::default().dispatch(
                &intent,
                &authority,
                &Prices,
                &Fence::default(),
                &Runner::default(),
                "key",
                104
            ),
            Err(ConsentError::NoApproval)
        ));
    }
}
