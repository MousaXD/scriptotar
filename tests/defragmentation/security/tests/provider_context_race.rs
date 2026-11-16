//! Independent Agent 10 red-team regression for PR #118.
 //! Test-only providers and fences: no network calls, keys, or money.
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Mutex,
};

use scriptotar_ai::{AiRequest, ProviderConfig, ProviderKind};
use scriptotar_ai::provider_consent::{
    sha256_hex, ActualUsage, BoundedProvider, ConsentBoundary, ConsentError, MeteredReply,
    PaidEffectFence, PriceCeiling, ProviderIntent, RunBudget, ScopeAuthority, StudyScope,
    TrustedPricing,
};

struct MutableAuthority(Mutex<StudyScope>);
impl ScopeAuthority for MutableAuthority {
    fn current_scope(&self, project: &str, study: &str) -> Result<StudyScope, ConsentError> {
        let current = self.0.lock().unwrap().clone();
        if current.project_id != project || current.study_id != study {
            return Err(ConsentError::WrongProjectOrStudy);
        }
        Ok(current)
    }
}

struct Catalog;
impl TrustedPricing for Catalog {
    fn ceiling(&self, config: &ProviderConfig) -> Option<PriceCeiling> {
        (config.provider == ProviderKind::OpenAi && config.model == "test-model")
            .then(|| PriceCeiling {
                input_microusd_per_million: 100_000,
                output_microusd_per_million: 200_000,
                catalog_revision: "fixture-2026-11-16".into(),
            })
    }
}

#[derive(Default)]
struct CountingProvider(AtomicUsize);
impl BoundedProvider for CountingProvider {
    fn supports_hard_bounds(&self, _: &ProviderConfig) -> bool { true }
    fn generate_once(
        &self, _: &ProviderConfig, _: &str, _: &AiRequest,
        _: u64, _: u64,
    ) -> Result<MeteredReply, ConsentError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Ok(MeteredReply {
            text: "test response".into(),
            usage: ActualUsage {
                input_tokens: 10, output_tokens: 10,
                tool_calls: 1, elapsed_ms: 10, cost_microusd: 0,
            },
        })
    }
}

/// Adversarial but legitimate sequence: the local study changes after the
/// external-effect reservation and BEFORE the mock provider is invoked.
struct ChangeContextAtFence<'a>(&'a MutableAuthority);
impl PaidEffectFence for ChangeContextAtFence<'_> {
    fn reserve_once(&self, _: &str) -> Result<(), ConsentError> {
        self.0.0.lock().unwrap().material_digest = "b".repeat(64);
        Ok(())
    }
    fn settle(&self, _: &str, _: &ActualUsage) -> Result<(), ConsentError> { Ok(()) }
    fn mark_uncertain(&self, _: &str) {}
}

struct RejectingFence;
impl PaidEffectFence for RejectingFence {
    fn reserve_once(&self, _: &str) -> Result<(), ConsentError> {
        Err(ConsentError::UncertainEffect)
    }
    fn settle(&self, _: &str, _: &ActualUsage) -> Result<(), ConsentError> {
        Err(ConsentError::UncertainEffect)
    }
    fn mark_uncertain(&self, _: &str) {}
}

fn approval() -> (ConsentBoundary, ProviderIntent, MutableAuthority) {
    let prompt = "Original brief and evidence";
    let scope = StudyScope {
        project_id: "project-1".into(),
        study_id: "study-1".into(),
        study_revision: 3,
        material_digest: "a".repeat(64),
        expected_prompt_digest: sha256_hex(prompt.as_bytes()),
        selected_source_ids: vec!["source-1".into()],
    };
    let authority = MutableAuthority(Mutex::new(scope.clone()));
    let intent = ProviderIntent {
        run_id: "run-1".into(), scope,
        provider: ProviderConfig {
            provider: ProviderKind::OpenAi,
            model: "test-model".into(),
            base_url: None,
        },
        prompt: prompt.into(),
        max_output_tokens: 100,
        limits: RunBudget {
            max_tokens: 3_000, max_tool_calls: 1,
            max_elapsed_ms: 10_000, max_cost_microusd: 100_000,
        },
    };
    let boundary = ConsentBoundary::default();
    let preview = boundary.prepare(intent.clone(), &authority, &Catalog, 100).unwrap();
    boundary.decide(&intent.run_id, &preview.fingerprint, true, &authority, 101).unwrap();
    (boundary, intent, authority)
}

#[test]
fn control_existing_fence_rejection_never_sends() {
    let (boundary, intent, authority) = approval();
    let provider = CountingProvider::default();
    let result = boundary.dispatch(
        &intent, &authority, &Catalog, &RejectingFence, &provider, "fixture-key", 102
    );
    assert!(matches!(result, Err(ConsentError::UncertainEffect)));
    assert_eq!(provider.0.load(Ordering::SeqCst), 0);
}

#[test]
fn changed_study_after_effect_reservation_must_not_egress() {
    let (boundary, intent, authority) = approval();
    let provider = CountingProvider::default();
    let fence = ChangeContextAtFence(&authority);
    let result = boundary.dispatch(
        &intent, &authority, &Catalog, &fence, &provider, "fixture-key", 102
    );
    assert!(
        matches!(result, Err(ConsentError::ChangedContext | ConsentError::UncertainEffect | ConsentError::Denied)),
        "a changed Study after durable claim must fail before any provider egress"
    );
    assert_eq!(
        provider.0.load(Ordering::SeqCst), 0,
        "changed Study was sent to provider after approval became stale"
    );
}


/// This race has a deterministic linearization point: the authoritative Study
/// revocation commits before reserve_once returns its dispatch permit. The
/// consent-record mutex must not be mistaken for a lock on canonical sources.
struct PausedReservation<'a> {
    reservation_reached: &'a std::sync::Barrier,
    allow_reservation_return: &'a std::sync::Barrier,
}
impl PaidEffectFence for PausedReservation<'_> {
    fn reserve_once(&self, _: &str) -> Result<(), ConsentError> {
        self.reservation_reached.wait();
        self.allow_reservation_return.wait();
        Ok(())
    }
    fn settle(&self, _: &str, _: &ActualUsage) -> Result<(), ConsentError> { Ok(()) }
    fn mark_uncertain(&self, _: &str) {}
}

#[test]
fn concurrent_source_revocation_before_effect_permit_never_egresses() {
    let (boundary, intent, authority) = approval();
    let provider = CountingProvider::default();
    let reservation_reached = std::sync::Barrier::new(2);
    let allow_reservation_return = std::sync::Barrier::new(2);
    let fence = PausedReservation {
        reservation_reached: &reservation_reached,
        allow_reservation_return: &allow_reservation_return,
    };
    std::thread::scope(|threads| {
        threads.spawn(|| {
            // The dispatch thread already performed its authorization check.
            reservation_reached.wait();
            authority.0.lock().unwrap().material_digest = "b".repeat(64);
            // Canonical revocation has now completed before the external-effect
            // reservation is allowed to return to the sender.
            allow_reservation_return.wait();
        });
        let result = boundary.dispatch(
            &intent, &authority, &Catalog, &fence, &provider, "fixture-key", 102
        );
        assert!(
            matches!(result, Err(ConsentError::ChangedContext |
                ConsentError::WrongProjectOrStudy |
                ConsentError::UncertainEffect | ConsentError::Denied)),
            "source revocation committed before dispatch permit must prevent egress"
        );
    });
    assert_eq!(
        provider.0.load(Ordering::SeqCst), 0,
        "provider invoked after canonical Study revocation finished"
    );
    assert!(matches!(
        boundary.dispatch(&intent, &authority, &Catalog, &RejectingFence,
            &provider, "fixture-key", 103),
        Err(ConsentError::ChangedContext | ConsentError::UncertainEffect |
            ConsentError::Denied | ConsentError::AlreadyUsed)
    ));
    assert_eq!(provider.0.load(Ordering::SeqCst), 0);
}
