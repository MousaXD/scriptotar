//! Private desktop adapter to the sealed OpenAI Responses transport.
//! No public Tauri command, trusted live price catalog or paid dispatch exists.
use scriptotar_ai::{
    provider_consent::{ConsentError, MeteredReply},
    provider_openai_wire::OpenAiResponsesWire,
    AiRequest, ProviderConfig,
};

use super::provider_consent::CertifiedWireTransport;

/// Only a host-owned, reviewed provider runtime may construct this. The real
/// transport implementation and its no-network contract tests live in
/// scriptotar-ai, which already depends on reqwest.
#[allow(dead_code)]
pub(super) struct HostOpenAiResponsesWire(OpenAiResponsesWire);

impl HostOpenAiResponsesWire {
    pub(super) fn new() -> Result<Self, ConsentError> {
        OpenAiResponsesWire::new().map(Self)
    }
}

impl CertifiedWireTransport for HostOpenAiResponsesWire {
    fn serialize_exact_request(
        &self,
        config: &ProviderConfig,
        request: &AiRequest,
        max_output_tokens: u64,
    ) -> Result<Vec<u8>, ConsentError> {
        self.0
            .serialize_exact_request(config, request, max_output_tokens)
    }

    fn supports_hard_bounds(&self, config: &ProviderConfig) -> bool {
        self.0.supports_hard_bounds(config)
    }

    fn send_prepared_once(
        &self,
        config: &ProviderConfig,
        api_key: &str,
        exact_bytes: &[u8],
        max_output_tokens: u64,
        deadline_ms: u64,
    ) -> Result<MeteredReply, ConsentError> {
        self.0
            .send_prepared_once(config, api_key, exact_bytes, max_output_tokens, deadline_ms)
    }

    fn authenticates_receipt(&self, config: &ProviderConfig, reply: &MeteredReply) -> bool {
        self.0.authenticates_receipt(config, reply)
    }
}
