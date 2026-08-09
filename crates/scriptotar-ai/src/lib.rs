use serde::{Deserialize, Serialize};
use thiserror::Error;
use url::{Host, Url};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProviderKind {
    OpenAi,
    Anthropic,
    Gemini,
    OpenAiCompatible,
    Local,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderConfig {
    pub provider: ProviderKind,
    pub model: String,
    pub base_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AiRequest {
    pub prompt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AiResponse {
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedEndpoint(Url);

impl ValidatedEndpoint {
    pub fn as_url(&self) -> &Url {
        &self.0
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum AiError {
    #[error("AI model cannot be empty")]
    EmptyModel,
    #[error("provider endpoint is required")]
    MissingEndpoint,
    #[error("provider endpoint is invalid: {0}")]
    InvalidEndpoint(String),
    #[error("plaintext HTTP is only allowed for loopback endpoints")]
    InsecureEndpoint,
    #[error("credentials must not be embedded in provider URLs")]
    EmbeddedCredentials,
    #[error("provider request failed: {0}")]
    Provider(String),
}

pub trait AiProvider: Send + Sync {
    fn generate(
        &self,
        endpoint: &ValidatedEndpoint,
        model: &str,
        api_key: &str,
        request: &AiRequest,
    ) -> Result<AiResponse, AiError>;
}

#[derive(Debug, Clone, Default)]
pub struct EndpointPolicy;

impl EndpointPolicy {
    pub fn endpoint_for(&self, config: &ProviderConfig) -> Result<ValidatedEndpoint, AiError> {
        if config.model.trim().is_empty() && config.provider != ProviderKind::Local {
            return Err(AiError::EmptyModel);
        }
        let raw = match config.provider {
            ProviderKind::OpenAi => "https://api.openai.com/v1/responses".to_owned(),
            ProviderKind::Anthropic => "https://api.anthropic.com/v1/messages".to_owned(),
            ProviderKind::Gemini => "https://generativelanguage.googleapis.com/v1beta".to_owned(),
            ProviderKind::OpenAiCompatible | ProviderKind::Local => {
                config.base_url.clone().ok_or(AiError::MissingEndpoint)?
            }
        };
        self.validate(&raw)
    }

    pub fn validate(&self, raw: &str) -> Result<ValidatedEndpoint, AiError> {
        let url = Url::parse(raw).map_err(|error| AiError::InvalidEndpoint(error.to_string()))?;
        if !url.username().is_empty() || url.password().is_some() {
            return Err(AiError::EmbeddedCredentials);
        }
        match url.scheme() {
            "https" => Ok(ValidatedEndpoint(url)),
            "http" if is_loopback(&url) => Ok(ValidatedEndpoint(url)),
            "http" => Err(AiError::InsecureEndpoint),
            scheme => Err(AiError::InvalidEndpoint(format!(
                "unsupported URL scheme {scheme}"
            ))),
        }
    }
}

fn is_loopback(url: &Url) -> bool {
    match url.host() {
        Some(Host::Domain(domain)) => domain.eq_ignore_ascii_case("localhost"),
        Some(Host::Ipv4(address)) => address.is_loopback(),
        Some(Host::Ipv6(address)) => address.is_loopback(),
        None => false,
    }
}

#[derive(Debug)]
pub struct AiService<P> {
    provider: P,
    policy: EndpointPolicy,
}

impl<P> AiService<P>
where
    P: AiProvider,
{
    pub fn new(provider: P) -> Self {
        Self {
            provider,
            policy: EndpointPolicy,
        }
    }

    pub fn generate(
        &self,
        config: &ProviderConfig,
        api_key: &str,
        request: &AiRequest,
    ) -> Result<AiResponse, AiError> {
        let endpoint = self.policy.endpoint_for(config)?;
        self.provider
            .generate(&endpoint, config.model.trim(), api_key, request)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;

    #[derive(Clone, Default)]
    struct FakeProvider {
        called: Arc<Mutex<bool>>,
    }

    impl AiProvider for FakeProvider {
        fn generate(
            &self,
            _endpoint: &ValidatedEndpoint,
            _model: &str,
            _api_key: &str,
            request: &AiRequest,
        ) -> Result<AiResponse, AiError> {
            *self.called.lock().unwrap() = true;
            Ok(AiResponse {
                text: request.prompt.clone(),
            })
        }
    }

    fn compatible(base_url: &str) -> ProviderConfig {
        ProviderConfig {
            provider: ProviderKind::OpenAiCompatible,
            model: "model".to_owned(),
            base_url: Some(base_url.to_owned()),
        }
    }

    #[test]
    fn rejects_plaintext_non_loopback_endpoint_before_provider_call() {
        let provider = FakeProvider::default();
        let called = provider.called.clone();
        let service = AiService::new(provider);
        let error = service
            .generate(
                &compatible("http://example.com/v1"),
                "secret",
                &AiRequest {
                    prompt: "hello".to_owned(),
                },
            )
            .unwrap_err();
        assert_eq!(error, AiError::InsecureEndpoint);
        assert!(!*called.lock().unwrap());
    }

    #[test]
    fn allows_plaintext_loopback_for_local_compatible_servers() {
        let policy = EndpointPolicy;
        assert!(policy.validate("http://127.0.0.1:11434/v1").is_ok());
        assert!(policy.validate("http://localhost:8080/v1").is_ok());
        assert!(policy.validate("http://[::1]:8080/v1").is_ok());
    }

    #[test]
    fn rejects_embedded_credentials() {
        let policy = EndpointPolicy;
        assert_eq!(
            policy
                .validate("https://user:pass@example.com/v1")
                .unwrap_err(),
            AiError::EmbeddedCredentials
        );
    }

    #[test]
    fn builtin_provider_endpoints_are_https() {
        let policy = EndpointPolicy;
        for provider in [
            ProviderKind::OpenAi,
            ProviderKind::Anthropic,
            ProviderKind::Gemini,
        ] {
            let endpoint = policy
                .endpoint_for(&ProviderConfig {
                    provider,
                    model: "model".to_owned(),
                    base_url: None,
                })
                .unwrap();
            assert_eq!(endpoint.as_url().scheme(), "https");
        }
    }
}
