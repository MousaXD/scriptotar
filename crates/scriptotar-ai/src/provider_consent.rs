//! Explicit, one-shot AI egress consent. No credentials or approval tokens are persisted here.
//!
//! A production caller must supply a DB-backed ScopeAuthority, a trusted and current pricing
//! catalog, and a durable, externally idempotent EffectFence. The legacy unmetered HTTP
//! adapters are deliberately NOT eligible to dispatch via this policy.
use std::{collections::HashMap, sync::Mutex};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{AiRequest, EndpointPolicy, ProviderConfig, ProviderKind};

const MAX_PROMPT_BYTES: usize = 500_000;
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
pub trait ScopeAuthority: Send + Sync {
    fn current_scope(&self, project_id: &str, study_id: &str) -> Result<StudyScope, ConsentError>;
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
    records: Mutex<HashMap<String, SavedConsent>>,
}

/// The fence must atomically claim a never-before-dispatched provider effect in the
/// durable ledger, then durably settle its receipt. Missing/ambiguous claim is NEVER
/// permission to send a second paid request. Agent 02 must supply this implementation.
pub trait PaidEffectFence: Send + Sync {
    fn reserve_once(&self, effect_id: &str) -> Result<(), ConsentError>;
    fn settle(&self, effect_id: &str, usage: &ActualUsage) -> Result<(), ConsentError>;
    fn mark_uncertain(&self, effect_id: &str);
}

pub struct DisabledPaidEffectFence;
impl PaidEffectFence for DisabledPaidEffectFence {
    fn reserve_once(&self, _effect_id: &str) -> Result<(), ConsentError> {
        Err(ConsentError::EffectFenceUnavailable)
    }

    fn settle(&self, _effect_id: &str, _usage: &ActualUsage) -> Result<(), ConsentError> {
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
    pub usage: ActualUsage,
}

/// The runner must enforce max_output_tokens and deadline *on the actual HTTP request*,
/// return verifiable usage and prohibit automatic provider retries or redirects.
pub trait BoundedProvider: Send + Sync {
    fn supports_hard_bounds(&self, config: &ProviderConfig) -> bool;
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
                slot.insert(SavedConsent {
                    intent,
                    preview: preview.clone(),
                    decision: Decision::Pending,
                });
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
        let mut records = self
            .records
            .lock()
            .map_err(|_| ConsentError::EffectFenceUnavailable)?;
        let record = records.get_mut(run_id).ok_or(ConsentError::NoApproval)?;
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
        let mut records = self
            .records
            .lock()
            .map_err(|_| ConsentError::EffectFenceUnavailable)?;
        let record = records.get_mut(run_id).ok_or(ConsentError::NoApproval)?;
        match record.decision {
            Decision::Pending | Decision::Approved => {
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
        let mut records = self
            .records
            .lock()
            .map_err(|_| ConsentError::EffectFenceUnavailable)?;
        let record = records
            .get_mut(&intent.run_id)
            .ok_or(ConsentError::NoApproval)?;
        match record.decision {
            Decision::Pending => return Err(ConsentError::NoApproval),
            Decision::Denied => return Err(ConsentError::Denied),
            Decision::Uncertain => return Err(ConsentError::UncertainEffect),
            Decision::Complete => return Err(ConsentError::AlreadyUsed),
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
        // Even a failed reservation may have committed before a lost response.
        // Mark uncertain rather than allowing another call to attempt the claim.
        if let Err(error) = fence.reserve_once(&effect_id) {
            record.decision = Decision::Uncertain;
            return Err(error);
        }
        record.decision = Decision::Uncertain;
        let deadline_ms = now_ms.saturating_add(intent.limits.max_elapsed_ms);
        drop(records);

        let generated = runner.generate_once(
            &intent.provider,
            api_key,
            &AiRequest {
                prompt: intent.prompt.clone(),
            },
            intent.max_output_tokens,
            deadline_ms,
        );
        let mut generated = match generated {
            Ok(response) => response,
            Err(_) => {
                fence.mark_uncertain(&effect_id);
                return Err(ConsentError::UncertainEffect);
            }
        };
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
        if fence.settle(&effect_id, &usage).is_err() {
            fence.mark_uncertain(&effect_id);
            return Err(ConsentError::UncertainEffect);
        }
        let mut records = self
            .records
            .lock()
            .map_err(|_| ConsentError::UncertainEffect)?;
        if let Some(record) = records.get_mut(&intent.run_id) {
            record.decision = Decision::Complete;
        }
        Ok(generated)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;

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

        fn settle(&self, _id: &str, _usage: &ActualUsage) -> Result<(), ConsentError> {
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
            fn settle(&self, _: &str, _: &ActualUsage) -> Result<(), ConsentError> {
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
