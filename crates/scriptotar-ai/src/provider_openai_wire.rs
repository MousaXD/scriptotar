//! OpenAI Responses wire transport candidate. Intentionally NOT registered in Tauri.
//!
//! A caller must first obtain a reviewed TrustedPricing ceiling and synchronize
//! *all* relevant origin writers against the host dispatch gate. The transport
//! guarantees exact request bytes, no redirect/retry, bounded HTTP I/O, and a
//! single-use receipt derived from a TLS-authenticated provider response. It
//! cannot authenticate a provider's billing ledger or authorize spending alone.
//! Until independent certification, do not construct this in a public command.
use std::{
    collections::HashSet,
    io::Read,
    sync::Mutex,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use reqwest::{
    blocking::Client,
    header::{ACCEPT, CONTENT_TYPE},
    redirect::Policy,
};
use crate::{
    provider_consent::{sha256_hex, ActualUsage, ConsentError, MeteredReply},
    AiRequest, ProviderConfig, ProviderKind,
};
use serde_json::{json, Value};


const OPENAI_RESPONSES_URL: &str = "https://api.openai.com/v1/responses";
const MAX_WIRE_BYTES: usize = 524_288;
const MAX_RESPONSE_BYTES: u64 = 1_048_576;
const MAX_OUTPUT_TOKENS: u64 = 8_192;
const MAX_ELAPSED_MS: u64 = 120_000;
const MAX_RECEIPTS: usize = 128;

/// Not a public approval grant. A receipt is trusted only after successful
/// HTTPS response parsing and may be consumed exactly once by the consent gate.
pub struct OpenAiResponsesWire {
    client: Client,
    authenticated: Mutex<HashSet<String>>,
}

impl OpenAiResponsesWire {
    pub fn new() -> Result<Self, ConsentError> {
        let client = Client::builder()
            .no_proxy()
            .redirect(Policy::none())
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_millis(MAX_ELAPSED_MS))
            .build()
            .map_err(|_| ConsentError::EffectFenceUnavailable)?;
        Ok(Self {
            client,
            authenticated: Mutex::new(HashSet::new()),
        })
    }

    fn model_allowed(config: &ProviderConfig) -> bool {
        config.provider == ProviderKind::OpenAi
            && config.base_url.is_none()
            && (1..=128).contains(&config.model.len())
            && config
                .model
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b':'))
    }

    fn body(
        config: &ProviderConfig,
        request: &AiRequest,
        max_output_tokens: u64,
    ) -> Result<Vec<u8>, ConsentError> {
        if !Self::model_allowed(config)
            || request.prompt.trim().is_empty()
            || request.prompt.len() > 500_000
            || !(1..=MAX_OUTPUT_TOKENS).contains(&max_output_tokens)
        {
            return Err(ConsentError::InvalidRequest);
        }
        let bytes = serde_json::to_vec(&json!({
            "model": config.model,
            "input": request.prompt,
            "store": false,
            "max_output_tokens": max_output_tokens,
            "tools": []
        }))
        .map_err(|_| ConsentError::InvalidRequest)?;
        if bytes.len() > MAX_WIRE_BYTES {
            return Err(ConsentError::InvalidRequest);
        }
        Ok(bytes)
    }

    /// Strictly decode one provider-issued response. Never invent a request ID,
    /// derive token usage from text, or accept an interrupted/incomplete result.
    fn decode_response(value: &[u8], elapsed_ms: u64) -> Result<MeteredReply, ConsentError> {
        if value.len() > MAX_RESPONSE_BYTES as usize {
            return Err(ConsentError::UncertainEffect);
        }
        let body: Value =
            serde_json::from_slice(value).map_err(|_| ConsentError::UncertainEffect)?;
        if body.get("status").and_then(Value::as_str) != Some("completed") {
            return Err(ConsentError::UncertainEffect);
        }
        let id = body
            .get("id")
            .and_then(Value::as_str)
            .ok_or(ConsentError::UncertainEffect)?;
        if !(1..=128).contains(&id.len())
            || !id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'/' | b':'))
        {
            return Err(ConsentError::UncertainEffect);
        }
        let usage = body.get("usage").ok_or(ConsentError::UncertainEffect)?;
        let input = usage
            .get("input_tokens")
            .and_then(Value::as_u64)
            .ok_or(ConsentError::UncertainEffect)?;
        let output = usage
            .get("output_tokens")
            .and_then(Value::as_u64)
            .ok_or(ConsentError::UncertainEffect)?;
        let total = usage
            .get("total_tokens")
            .and_then(Value::as_u64)
            .ok_or(ConsentError::UncertainEffect)?;
        if input.checked_add(output) != Some(total) {
            return Err(ConsentError::UncertainEffect);
        }
        let events = body
            .get("output")
            .and_then(Value::as_array)
            .ok_or(ConsentError::UncertainEffect)?;
        if events.is_empty() {
            return Err(ConsentError::UncertainEffect);
        }
        let mut text = String::new();
        for event in events {
            // Tool calls, refusal markers, and unknown event types are not
            // valid plain-text script results and must never be auto-retried.
            if event.get("type").and_then(Value::as_str) != Some("message")
                || event.get("role").and_then(Value::as_str) != Some("assistant")
            {
                return Err(ConsentError::UncertainEffect);
            }
            let parts = event
                .get("content")
                .and_then(Value::as_array)
                .ok_or(ConsentError::UncertainEffect)?;
            if parts.is_empty() {
                return Err(ConsentError::UncertainEffect);
            }
            for part in parts {
                if part.get("type").and_then(Value::as_str) != Some("output_text") {
                    return Err(ConsentError::UncertainEffect);
                }
                let segment = part
                    .get("text")
                    .and_then(Value::as_str)
                    .ok_or(ConsentError::UncertainEffect)?;
                if segment.len() > MAX_RESPONSE_BYTES as usize - text.len() {
                    return Err(ConsentError::UncertainEffect);
                }
                text.push_str(segment);
            }
        }
        if text.trim().is_empty() || elapsed_ms > MAX_ELAPSED_MS {
            return Err(ConsentError::UncertainEffect);
        }
        Ok(MeteredReply {
            text,
            provider_request_id: id.to_owned(),
            usage: ActualUsage {
                input_tokens: input,
                output_tokens: output,
                tool_calls: 1,
                elapsed_ms,
                // The host consent boundary computes the actual rounded-up
                // USD amount from its own reviewed model-price ceiling.
                cost_microusd: 0,
            },
        })
    }

    fn receipt_proof(config: &ProviderConfig, reply: &MeteredReply) -> Option<String> {
        let bytes = serde_json::to_vec(&(
            config,
            &reply.provider_request_id,
            &reply.text,
            reply.usage.input_tokens,
            reply.usage.output_tokens,
            reply.usage.tool_calls,
            reply.usage.elapsed_ms,
            reply.usage.cost_microusd,
        ))
        .ok()?;
        Some(sha256_hex(&bytes))
    }

    fn now_ms() -> Result<u64, ConsentError> {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| ConsentError::Expired)?
            .as_millis()
            .try_into()
            .map_err(|_| ConsentError::Expired)
    }
}

impl OpenAiResponsesWire {
    pub fn supports_hard_bounds(&self, config: &ProviderConfig) -> bool {
        Self::model_allowed(config)
    }

    pub fn serialize_exact_request(
        &self,
        config: &ProviderConfig,
        request: &AiRequest,
        max_output_tokens: u64,
    ) -> Result<Vec<u8>, ConsentError> {
        Self::body(config, request, max_output_tokens)
    }

    pub fn send_prepared_once(
        &self,
        config: &ProviderConfig,
        api_key: &str,
        exact_bytes: &[u8],
        max_output_tokens: u64,
        deadline_ms: u64,
    ) -> Result<MeteredReply, ConsentError> {
        if !self.supports_hard_bounds(config)
            || api_key.trim().is_empty()
            || api_key.len() > 4096
            || exact_bytes.is_empty()
            || exact_bytes.len() > MAX_WIRE_BYTES
        {
            return Err(ConsentError::InvalidRequest);
        }
        // Never reserialize an altered instruction body or use a custom
        // endpoint. This is a validation copy; the original slice is sent.
        let parsed: Value =
            serde_json::from_slice(exact_bytes).map_err(|_| ConsentError::ChangedContext)?;
        let prompt = parsed
            .get("input")
            .and_then(Value::as_str)
            .ok_or(ConsentError::ChangedContext)?;
        if Self::body(
            config,
            &AiRequest {
                prompt: prompt.to_owned(),
            },
            max_output_tokens,
        )? != exact_bytes
        {
            return Err(ConsentError::ChangedContext);
        }
        let remaining = deadline_ms
            .checked_sub(Self::now_ms()?)
            .ok_or(ConsentError::Expired)?;
        if remaining == 0 {
            return Err(ConsentError::Expired);
        }
        let deadline = Duration::from_millis(remaining.min(MAX_ELAPSED_MS));
        let started = Instant::now();
        let response = self
            .client
            .post(OPENAI_RESPONSES_URL)
            .bearer_auth(api_key)
            .header(CONTENT_TYPE, "application/json")
            .header(ACCEPT, "application/json")
            .timeout(deadline)
            .body(exact_bytes.to_vec())
            .send()
            .map_err(|_| ConsentError::UncertainEffect)?;
        if response.status().as_u16() != 200
            || response
                .content_length()
                .is_some_and(|len| len > MAX_RESPONSE_BYTES)
        {
            return Err(ConsentError::UncertainEffect);
        }
        let mut bytes = Vec::new();
        response
            .take(MAX_RESPONSE_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| ConsentError::UncertainEffect)?;
        if bytes.len() > MAX_RESPONSE_BYTES as usize {
            return Err(ConsentError::UncertainEffect);
        }
        let elapsed = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
        if elapsed > remaining {
            return Err(ConsentError::Expired);
        }
        let reply = Self::decode_response(&bytes, elapsed)?;
        let proof = Self::receipt_proof(config, &reply).ok_or(ConsentError::UncertainEffect)?;
        let mut authenticated = self
            .authenticated
            .lock()
            .map_err(|_| ConsentError::UncertainEffect)?;
        if authenticated.len() >= MAX_RECEIPTS || !authenticated.insert(proof) {
            return Err(ConsentError::UncertainEffect);
        }
        Ok(reply)
    }

    pub fn authenticates_receipt(&self, config: &ProviderConfig, reply: &MeteredReply) -> bool {
        let Some(proof) = Self::receipt_proof(config, reply) else {
            return false;
        };
        self.authenticated
            .lock()
            .is_ok_and(|mut receipts| receipts.remove(&proof))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model() -> ProviderConfig {
        ProviderConfig {
            provider: ProviderKind::OpenAi,
            model: "test-model".into(),
            base_url: None,
        }
    }

    fn completed() -> Vec<u8> {
        serde_json::to_vec(&json!({
            "id": "resp_1234",
            "status": "completed",
            "output": [{
                "type": "message",
                "role": "assistant",
                "content": [{"type": "output_text", "text": "Original script"}]
            }],
            "usage": {
                "input_tokens": 11,
                "output_tokens": 6,
                "total_tokens": 17
            }
        }))
        .unwrap()
    }

    #[test]
    fn immutable_bounded_wire_body_has_explicit_model_and_output_limit() {
        let provider = OpenAiResponsesWire::new().unwrap();
        let config = model();
        let input = AiRequest {
            prompt: "Source-bound original brief".into(),
        };
        let first = provider
            .serialize_exact_request(&config, &input, 128)
            .unwrap();
        let second = provider
            .serialize_exact_request(&config, &input, 128)
            .unwrap();
        assert_eq!(first, second);
        let value: Value = serde_json::from_slice(&first).unwrap();
        assert_eq!(value["max_output_tokens"], 128);
        assert_eq!(value["store"], false);
        assert_eq!(value["tools"], json!([]));
        assert_eq!(value["model"], "test-model");
        assert_eq!(value["input"], input.prompt);
        assert!(provider
            .serialize_exact_request(&config, &input, 0)
            .is_err());
        assert!(provider
            .serialize_exact_request(&config, &input, 8193)
            .is_err());
        assert!(provider
            .serialize_exact_request(
                &ProviderConfig {
                    provider: ProviderKind::OpenAi,
                    model: "test-model".into(),
                    base_url: Some("https://other.example/v1".into()),
                },
                &input,
                128
            )
            .is_err());
        assert!(provider
            .serialize_exact_request(
                &ProviderConfig {
                    provider: ProviderKind::OpenAiCompatible,
                    model: "test-model".into(),
                    base_url: Some("https://somewhere.example".into()),
                },
                &input,
                128
            )
            .is_err());
    }

    #[test]
    fn response_requires_provider_id_completed_status_and_exact_metering() {
        let reply = OpenAiResponsesWire::decode_response(&completed(), 35).unwrap();
        assert_eq!(reply.provider_request_id, "resp_1234");
        assert_eq!(reply.text, "Original script");
        assert_eq!(reply.usage.input_tokens, 11);
        assert_eq!(reply.usage.output_tokens, 6);
        assert_eq!(reply.usage.cost_microusd, 0);
        let mut value: Value = serde_json::from_slice(&completed()).unwrap();
        for bad in [
            ("status", json!("incomplete")),
            ("id", Value::Null),
            ("usage", Value::Null),
            ("output", json!([])),
        ] {
            let before = value[bad.0].clone();
            value[bad.0] = bad.1;
            assert!(
                OpenAiResponsesWire::decode_response(&serde_json::to_vec(&value).unwrap(), 35)
                    .is_err()
            );
            value[bad.0] = before;
        }
        value["usage"]["total_tokens"] = json!(16);
        assert!(
            OpenAiResponsesWire::decode_response(&serde_json::to_vec(&value).unwrap(), 35).is_err()
        );
        value["usage"]["total_tokens"] = json!(17);
        value["output"][0]["type"] = json!("function_call");
        assert!(
            OpenAiResponsesWire::decode_response(&serde_json::to_vec(&value).unwrap(), 35).is_err()
        );
        assert!(OpenAiResponsesWire::decode_response(&completed(), MAX_ELAPSED_MS + 1).is_err());
        assert!(OpenAiResponsesWire::decode_response(
            &vec![b'x'; MAX_RESPONSE_BYTES as usize + 1],
            1
        )
        .is_err());
    }

    #[test]
    fn receipt_proof_is_single_use_and_binds_exact_response_fields() {
        let provider = OpenAiResponsesWire::new().unwrap();
        let reply = OpenAiResponsesWire::decode_response(&completed(), 4).unwrap();
        let original = OpenAiResponsesWire::receipt_proof(&model(), &reply).unwrap();
        provider.authenticated.lock().unwrap().insert(original);
        let altered = MeteredReply {
            text: "Changed answer".into(),
            provider_request_id: reply.provider_request_id.clone(),
            usage: reply.usage,
        };
        assert!(!provider.authenticates_receipt(&model(), &altered));
        assert!(provider.authenticates_receipt(&model(), &reply));
        assert!(!provider.authenticates_receipt(&model(), &reply));
    }
}
