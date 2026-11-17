//! Agent 10 independent, zero-network red-team tests for the private
//! OpenAI Responses transport. Exercise the compiled public transport from
//! scriptotar-ai; never use live keys, addresses, or paid requests.
use scriptotar_ai::{
    provider_consent::{ActualUsage, ConsentError, MeteredReply},
    provider_openai_wire::OpenAiResponsesWire,
    AiRequest, ProviderConfig, ProviderKind,
};

fn config() -> ProviderConfig {
    ProviderConfig {
        provider: ProviderKind::OpenAi,
        model: "test-model".into(),
        base_url: None,
    }
}

fn exact_bytes(transport: &OpenAiResponsesWire) -> Vec<u8> {
    transport
        .serialize_exact_request(
            &config(),
            &AiRequest {
                prompt: "Canonical Study source context".into(),
            },
            42,
        )
        .unwrap()
}

/// Expired deadline (zero Unix ms) prevents any network attempt even if
/// the implementation accidentally misorders a future validation step.
#[test]
fn sealed_wire_allows_only_canonical_bytes_before_deadline() {
    let transport = OpenAiResponsesWire::new().unwrap();
    let exact = exact_bytes(&transport);
    let expected = String::from_utf8(exact.clone()).unwrap();
    for (old, replacement) in [
        ("\"model\":\"test-model\"", "\"model\":\"other-model\""),
        ("\"store\":false", "\"store\":true"),
        ("\"tools\":[]", "\"tools\":[\"web_search\"]"),
        ("\"max_output_tokens\":42", "\"max_output_tokens\":43"),
        ("Canonical Study source context", "Injected or stale source context"),
    ] {
        assert!(expected.contains(old), "expected canonical JSON field absent");
        let altered = expected.replacen(old, replacement, 1);
        assert!(
            matches!(
                transport.send_prepared_once(
                    &config(),
                    "fixture-key-never-sent",
                    altered.as_bytes(),
                    42,
                    0,
                ),
                Err(ConsentError::ChangedContext)
            ),
            "mutated sealed request was not rejected before network admission"
        );
    }
    let mut whitespace_changed = exact.clone();
    whitespace_changed.push(b' ');
    assert!(matches!(
        transport.send_prepared_once(
            &config(), "fixture-key-never-sent", &whitespace_changed, 42, 0,
        ),
        Err(ConsentError::ChangedContext)
    ));
    // Canonical bytes pass payload comparison, but EXPIRED still bars egress.
    assert!(matches!(
        transport.send_prepared_once(
            &config(), "fixture-key-never-sent", &exact, 42, 0,
        ),
        Err(ConsentError::Expired)
    ));
}

#[test]
fn untrusted_endpoint_and_unbounded_token_requests_fail_before_egress() {
    let transport = OpenAiResponsesWire::new().unwrap();
    let request = AiRequest {
        prompt: "Canonical Study source context".into(),
    };
    let hostile = ProviderConfig {
        base_url: Some("https://attacker.example/v1/responses".into()),
        ..config()
    };
    assert!(!transport.supports_hard_bounds(&hostile));
    assert!(transport
        .serialize_exact_request(&hostile, &request, 42)
        .is_err());
    let compatible = ProviderConfig {
        provider: ProviderKind::OpenAiCompatible,
        base_url: None,
        ..config()
    };
    assert!(!transport.supports_hard_bounds(&compatible));
    assert!(transport
        .serialize_exact_request(&compatible, &request, 42)
        .is_err());
    for cap in [0, 8193, u64::MAX] {
        assert!(transport
            .serialize_exact_request(&config(), &request, cap)
            .is_err());
    }
    assert!(transport
        .serialize_exact_request(
            &config(),
            &AiRequest { prompt: " ".into() },
            42
        )
        .is_err());
}

#[test]
fn invented_or_unseen_provider_usage_receipts_are_never_authenticated() {
    let transport = OpenAiResponsesWire::new().unwrap();
    let reply = MeteredReply {
        text: "Untrusted response".into(),
        provider_request_id: "resp_fake_1234".into(),
        usage: ActualUsage {
            input_tokens: 13,
            output_tokens: 7,
            tool_calls: 1,
            elapsed_ms: 3,
            cost_microusd: 0,
        },
    };
    assert!(!transport.authenticates_receipt(&config(), &reply));
    assert!(!transport.authenticates_receipt(&config(), &reply));
    let altered_provider = ProviderConfig {
        model: "other-model".into(),
        ..config()
    };
    assert!(!transport.authenticates_receipt(&altered_provider, &reply));
}
