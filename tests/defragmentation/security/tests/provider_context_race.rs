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
    fn verified_usage_receipt(&self, _: &ProviderConfig, reply: &MeteredReply) -> bool {
        reply.provider_request_id == "fixture-request-id"
    }
    fn generate_once(
        &self, _: &ProviderConfig, _: &str, _: &AiRequest,
        _: u64, _: u64,
    ) -> Result<MeteredReply, ConsentError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Ok(MeteredReply {
            text: "test response".into(),
            provider_request_id: "fixture-request-id".into(),
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
    fn settle(&self, _: &str, _: &str, _: &ActualUsage) -> Result<(), ConsentError> { Ok(()) }
    fn mark_uncertain(&self, _: &str) {}
}

struct RejectingFence;
impl PaidEffectFence for RejectingFence {
    fn reserve_once(&self, _: &str) -> Result<(), ConsentError> {
        Err(ConsentError::UncertainEffect)
    }
    fn settle(&self, _: &str, _: &str, _: &ActualUsage) -> Result<(), ConsentError> {
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
    fn settle(&self, _: &str, _: &str, _: &ActualUsage) -> Result<(), ConsentError> { Ok(()) }
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

/// A fully functional test authority: unlike the fail-closed default, it
/// enforces the identical read/write gate on scope reads and mutations.
struct GuardAuthority(std::sync::RwLock<StudyScope>);
struct HeldGuard<'a>(std::sync::RwLockReadGuard<'a, StudyScope>);
impl scriptotar_ai::provider_consent::ScopeDispatchGuard for HeldGuard<'_> {
    fn scope(&self) -> &StudyScope { &self.0 }
    fn now_ms(&self) -> Result<u64, ConsentError> { Ok(103) }
}
impl ScopeAuthority for GuardAuthority {
    fn current_scope(&self, project: &str, study: &str) -> Result<StudyScope, ConsentError> {
        let scope = self.0.read().unwrap().clone();
        if scope.project_id != project || scope.study_id != study {
            return Err(ConsentError::WrongProjectOrStudy);
        }
        Ok(scope)
    }
    fn lock_dispatch_scope(
        &self, approved: &StudyScope,
    ) -> Result<Box<dyn scriptotar_ai::provider_consent::ScopeDispatchGuard + '_>, ConsentError> {
        let guard = self.0.read().unwrap();
        if &*guard != approved { return Err(ConsentError::ChangedContext); }
        Ok(Box::new(HeldGuard(guard)))
    }
}

#[derive(Default)]
struct DispatchedFence {
    reservations: AtomicUsize,
    dispatches: AtomicUsize,
    receipts: AtomicUsize,
}
impl PaidEffectFence for DispatchedFence {
    fn reserve_once(&self, _: &str) -> Result<(), ConsentError> {
        self.reservations.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
    fn mark_dispatched_once(&self, _: &str) -> Result<(), ConsentError> {
        self.dispatches.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
    fn settle(&self, _: &str, _: &str, _: &ActualUsage) -> Result<(), ConsentError> {
        self.receipts.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
    fn mark_uncertain(&self, _: &str) {}
}

struct PausedProvider<'a> {
    entered: &'a std::sync::Barrier,
    release: &'a std::sync::Barrier,
    sent: &'a AtomicUsize,
}
impl BoundedProvider for PausedProvider<'_> {
    fn supports_hard_bounds(&self, _: &ProviderConfig) -> bool { true }
    fn verified_usage_receipt(&self, _: &ProviderConfig, reply: &MeteredReply) -> bool {
        reply.provider_request_id == "fixture-request-id"
    }
    fn generate_once(
        &self, _: &ProviderConfig, _: &str, _: &AiRequest, _: u64, _: u64,
    ) -> Result<MeteredReply, ConsentError> {
        self.sent.fetch_add(1, Ordering::SeqCst);
        self.entered.wait();
        self.release.wait();
        Ok(MeteredReply {
            text: "bounded test receipt".into(),
            provider_request_id: "fixture-request-id".into(),
            usage: ActualUsage {
                input_tokens: 10, output_tokens: 10,
                tool_calls: 1, elapsed_ms: 10, cost_microusd: 0,
            },
        })
    }
}

#[test]
fn valid_guarded_dispatch_blocks_concurrent_revocation_until_send_finishes() {
    let (boundary, intent, _) = approval();
    let authority = GuardAuthority(std::sync::RwLock::new(intent.scope.clone()));
    let entered = std::sync::Barrier::new(2);
    let release = std::sync::Barrier::new(2);
    let calls = AtomicUsize::new(0);
    let provider = PausedProvider { entered: &entered, release: &release, sent: &calls };
    let fence = DispatchedFence::default();
    std::thread::scope(|threads| {
        let revoke = threads.spawn(|| {
            entered.wait();
            // Network send has begun; the scoped read lock must prevent a
            // source revocation from linearizing until that attempt finishes.
            assert!(matches!(
                authority.0.try_write(),
                Err(std::sync::TryLockError::WouldBlock)
            ));
            release.wait();
            authority.0.write().unwrap().material_digest = "b".repeat(64);
        });
        let result = boundary.dispatch(
            &intent, &authority, &Catalog, &fence, &provider, "fixture-key", 102
        );
        assert!(result.is_ok(), "a valid, fenced request must still be executable");
        revoke.join().unwrap();
    });
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(fence.reservations.load(Ordering::SeqCst), 1);
    assert_eq!(fence.dispatches.load(Ordering::SeqCst), 1);
    assert_eq!(fence.receipts.load(Ordering::SeqCst), 1);
    assert_eq!(authority.0.read().unwrap().material_digest, "b".repeat(64));
}

#[test]
fn missing_shared_authority_guard_fails_closed_even_with_valid_fence() {
    let (boundary, intent, authority) = approval();
    let provider = CountingProvider::default();
    let fence = DispatchedFence::default();
    assert!(matches!(
        boundary.dispatch(&intent, &authority, &Catalog, &fence,
            &provider, "fixture-key", 102),
        Err(ConsentError::UncertainEffect | ConsentError::EffectFenceUnavailable)
    ));
    assert_eq!(provider.0.load(Ordering::SeqCst), 0);
    assert_eq!(fence.dispatches.load(Ordering::SeqCst), 0);
}

#[derive(Default)]
struct UnverifiedReceiptProvider(AtomicUsize);
impl BoundedProvider for UnverifiedReceiptProvider {
    fn supports_hard_bounds(&self, _: &ProviderConfig) -> bool { true }
    // Deliberately do NOT override verified_usage_receipt: an arbitrary
    // model/server-supplied string cannot authenticate a paid receipt.
    fn generate_once(
        &self, _: &ProviderConfig, _: &str, _: &AiRequest, _: u64, _: u64,
    ) -> Result<MeteredReply, ConsentError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Ok(MeteredReply {
            text: "unverified paid response".into(),
            provider_request_id: "self-claimed-unverified-id".into(),
            usage: ActualUsage {
                input_tokens: 10, output_tokens: 10,
                tool_calls: 1, elapsed_ms: 10, cost_microusd: 0,
            },
        })
    }
}

#[test]
fn unauthenticated_paid_receipt_is_uncertain_and_never_replayed() {
    let (boundary, intent, _) = approval();
    let authority = GuardAuthority(std::sync::RwLock::new(intent.scope.clone()));
    let provider = UnverifiedReceiptProvider::default();
    let fence = DispatchedFence::default();
    let first = boundary.dispatch(
        &intent, &authority, &Catalog, &fence, &provider, "fixture-key", 102
    );
    assert!(matches!(first, Err(ConsentError::UncertainEffect)));
    assert_eq!(provider.0.load(Ordering::SeqCst), 1);
    assert_eq!(fence.dispatches.load(Ordering::SeqCst), 1);
    assert_eq!(fence.receipts.load(Ordering::SeqCst), 0);
    let retry = boundary.dispatch(
        &intent, &authority, &Catalog, &fence, &provider, "fixture-key", 104
    );
    assert!(matches!(retry, Err(ConsentError::UncertainEffect | ConsentError::AlreadyUsed)));
    assert_eq!(provider.0.load(Ordering::SeqCst), 1, "unverified acknowledgement caused duplicate paid send");
    assert_eq!(fence.receipts.load(Ordering::SeqCst), 0);
}


/// The approved ProviderIntent must reach the bounded provider without a
/// host-side model, prompt or output/deadline substitution. This inspects the
/// BoundedProvider call, NOT the as-yet-unintegrated serialized HTTP body.
#[derive(Default)]
struct CaptureApprovedRequest(Mutex<Vec<(ProviderConfig, String, u64, u64)>>);
impl BoundedProvider for CaptureApprovedRequest {
    fn supports_hard_bounds(&self, _: &ProviderConfig) -> bool { true }
    fn verified_usage_receipt(&self, _: &ProviderConfig, reply: &MeteredReply) -> bool {
        reply.provider_request_id == "fixture-request-id"
    }
    fn generate_once(
        &self,
        config: &ProviderConfig,
        api_key: &str,
        request: &AiRequest,
        max_output_tokens: u64,
        deadline_ms: u64,
    ) -> Result<MeteredReply, ConsentError> {
        assert_eq!(api_key, "fixture-key");
        self.0.lock().unwrap().push((
            config.clone(), request.prompt.clone(), max_output_tokens, deadline_ms,
        ));
        Ok(MeteredReply {
            text: "trusted bounded fixture reply".into(),
            provider_request_id: "fixture-request-id".into(),
            usage: ActualUsage {
                input_tokens: 10, output_tokens: 10,
                tool_calls: 1, elapsed_ms: 10, cost_microusd: 0,
            },
        })
    }
}

#[test]
fn consent_boundary_preserves_approved_provider_prompt_and_hard_bounds() {
    let (boundary, intent, _) = approval();
    let authority = GuardAuthority(std::sync::RwLock::new(intent.scope.clone()));
    let runner = CaptureApprovedRequest::default();
    let fence = DispatchedFence::default();
    let reply = boundary.dispatch(
        &intent, &authority, &Catalog, &fence, &runner, "fixture-key", 102,
    );
    assert!(reply.is_ok(), "an unchanged approved intent must reach the bounded provider");
    let sent = runner.0.lock().unwrap();
    assert_eq!(sent.len(), 1, "approved run must result in exactly one provider call");
    assert_eq!(sent[0].0, intent.provider, "provider/model changed after consent");
    assert_eq!(sent[0].1, intent.prompt, "approved prompt changed after consent");
    assert_eq!(sent[0].2, intent.max_output_tokens, "output bound changed after consent");
    assert_eq!(
        sent[0].3, 102 + intent.limits.max_elapsed_ms,
        "deadline must be derived from the approved runtime budget"
    );
    assert_eq!(fence.dispatches.load(Ordering::SeqCst), 1);
    assert_eq!(fence.receipts.load(Ordering::SeqCst), 1);
}

#[test]
fn unauthorized_model_prompt_and_budget_substitution_never_claim_or_send() {
    let (boundary, intent, _) = approval();
    let authority = GuardAuthority(std::sync::RwLock::new(intent.scope.clone()));
    let provider = CountingProvider::default();
    let fence = DispatchedFence::default();

    let mut changed_model = intent.clone();
    changed_model.provider.model = "substituted-model".into();
    let mut changed_prompt = intent.clone();
    changed_prompt.prompt.push_str(" attacker instruction");
    let mut changed_output_cap = intent.clone();
    changed_output_cap.max_output_tokens += 1;
    let mut changed_budget = intent.clone();
    changed_budget.limits.max_cost_microusd += 1;

    for changed in [&changed_model, &changed_prompt, &changed_output_cap, &changed_budget] {
        let result = boundary.dispatch(
            changed, &authority, &Catalog, &fence, &provider, "fixture-key", 102,
        );
        assert!(
            matches!(result, Err(ConsentError::ChangedContext)),
            "a mutated approved intent must be rejected before external-effect reservation"
        );
    }
    assert_eq!(fence.reservations.load(Ordering::SeqCst), 0);
    assert_eq!(fence.dispatches.load(Ordering::SeqCst), 0);
    assert_eq!(provider.0.load(Ordering::SeqCst), 0);
}

#[test]
fn cancellation_linearized_while_reservation_is_paused_prevents_cas_and_send() {
    let (boundary, intent, _) = approval();
    let authority = GuardAuthority(std::sync::RwLock::new(intent.scope.clone()));
    let provider = CountingProvider::default();
    let reservation_reached = std::sync::Barrier::new(2);
    let allow_reservation_return = std::sync::Barrier::new(2);
    let fence = PausedReservation {
        reservation_reached: &reservation_reached,
        allow_reservation_return: &allow_reservation_return,
    };
    std::thread::scope(|threads| {
        threads.spawn(|| {
            reservation_reached.wait();
            boundary.cancel(&intent.run_id).unwrap();
            allow_reservation_return.wait();
        });
        let result = boundary.dispatch(
            &intent, &authority, &Catalog, &fence, &provider, "fixture-key", 102,
        );
        assert!(matches!(result, Err(ConsentError::Denied)));
    });
    assert_eq!(provider.0.load(Ordering::SeqCst), 0);
}

#[test]
fn concurrent_single_use_dispatch_has_only_one_pre_send_cas_winner() {
    let (boundary, intent, _) = approval();
    let authority = GuardAuthority(std::sync::RwLock::new(intent.scope.clone()));
    let provider = CountingProvider::default();
    let fence = DispatchedFence::default();
    let start = std::sync::Barrier::new(3);
    let successes = AtomicUsize::new(0);
    std::thread::scope(|threads| {
        for _ in 0..2 {
            threads.spawn(|| {
                start.wait();
                let result = boundary.dispatch(
                    &intent, &authority, &Catalog, &fence, &provider, "fixture-key", 102,
                );
                if result.is_ok() {
                    successes.fetch_add(1, Ordering::SeqCst);
                } else {
                    assert!(matches!(result, Err(ConsentError::AlreadyUsed)));
                }
            });
        }
        start.wait();
    });
    assert_eq!(successes.load(Ordering::SeqCst), 1);
    assert_eq!(provider.0.load(Ordering::SeqCst), 1);
    assert_eq!(fence.reservations.load(Ordering::SeqCst), 1);
    assert_eq!(fence.dispatches.load(Ordering::SeqCst), 1);
    assert_eq!(fence.receipts.load(Ordering::SeqCst), 1);
}
