use std::time::Duration;

use reqwest::blocking::{Client, Response};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use thiserror::Error;
use url::{Host, Url};

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(90);
const MAX_MODEL_CHARS: usize = 256;
const MAX_PROMPT_CHARS: usize = 500_000;
const MAX_ERROR_CHARS: usize = 1_000;
const MAX_RESPONSE_BYTES: u64 = 8 * 1024 * 1024;

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
    #[error("AI model is invalid: {0}")]
    InvalidModel(String),
    #[error("AI prompt cannot be empty")]
    EmptyPrompt,
    #[error("AI prompt is too large")]
    PromptTooLarge,
    #[error("API key is required")]
    MissingApiKey,
    #[error("provider endpoint is required")]
    MissingEndpoint,
    #[error("provider endpoint is invalid: {0}")]
    InvalidEndpoint(String),
    #[error("plaintext HTTP is only allowed for loopback endpoints")]
    InsecureEndpoint,
    #[error("credentials must not be embedded in provider URLs")]
    EmbeddedCredentials,
    #[error("provider request timed out")]
    Timeout,
    #[error("provider response is too large")]
    ResponseTooLarge,
    #[error("provider returned an invalid response: {0}")]
    InvalidResponse(String),
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
        validate_model(config.provider, &config.model)?;
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
        if raw.chars().count() > 2_048 {
            return Err(AiError::InvalidEndpoint("URL is too long".to_owned()));
        }
        let url = Url::parse(raw).map_err(|error| AiError::InvalidEndpoint(error.to_string()))?;
        if !url.username().is_empty() || url.password().is_some() {
            return Err(AiError::EmbeddedCredentials);
        }
        if url.host().is_none() {
            return Err(AiError::InvalidEndpoint("URL host is required".to_owned()));
        }
        if url.fragment().is_some() {
            return Err(AiError::InvalidEndpoint(
                "URL fragments are not supported".to_owned(),
            ));
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

fn validate_model(provider: ProviderKind, raw: &str) -> Result<&str, AiError> {
    let model = raw.trim();
    if model.is_empty() && provider != ProviderKind::Local {
        return Err(AiError::EmptyModel);
    }
    if model.chars().count() > MAX_MODEL_CHARS {
        return Err(AiError::InvalidModel("model name is too long".to_owned()));
    }
    if model.chars().any(char::is_control) {
        return Err(AiError::InvalidModel(
            "model name contains control characters".to_owned(),
        ));
    }
    Ok(model)
}

fn validate_request(request: &AiRequest) -> Result<(), AiError> {
    if request.prompt.trim().is_empty() {
        return Err(AiError::EmptyPrompt);
    }
    if request.prompt.chars().count() > MAX_PROMPT_CHARS {
        return Err(AiError::PromptTooLarge);
    }
    Ok(())
}

fn is_loopback(url: &Url) -> bool {
    match url.host() {
        Some(Host::Domain(domain)) => domain.eq_ignore_ascii_case("localhost"),
        Some(Host::Ipv4(address)) => address.is_loopback(),
        Some(Host::Ipv6(address)) => address.is_loopback(),
        None => false,
    }
}

#[derive(Debug, Clone)]
pub struct HttpAiProvider {
    provider: ProviderKind,
    client: Client,
}

impl HttpAiProvider {
    pub fn new(provider: ProviderKind) -> Result<Self, AiError> {
        Self::with_timeout(provider, DEFAULT_TIMEOUT)
    }

    pub fn with_timeout(provider: ProviderKind, timeout: Duration) -> Result<Self, AiError> {
        let client = Client::builder()
            .timeout(timeout)
            .user_agent("Scriptotar-Next/0.1")
            .build()
            .map_err(|error| AiError::Provider(safe_transport_error(&error)))?;
        Ok(Self { provider, client })
    }

    fn openai(
        &self,
        endpoint: &ValidatedEndpoint,
        model: &str,
        api_key: &str,
        request: &AiRequest,
    ) -> Result<AiResponse, AiError> {
        let response = self
            .client
            .post(endpoint.as_url().clone())
            .bearer_auth(api_key)
            .json(&json!({
                "model": model,
                "input": request.prompt,
                "store": false
            }))
            .send()
            .map_err(map_transport_error)?;
        parse_json_response(response, api_key, parse_openai_response)
    }

    fn anthropic(
        &self,
        endpoint: &ValidatedEndpoint,
        model: &str,
        api_key: &str,
        request: &AiRequest,
    ) -> Result<AiResponse, AiError> {
        let response = self
            .client
            .post(endpoint.as_url().clone())
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&json!({
                "model": model,
                "max_tokens": 4096,
                "messages": [{"role": "user", "content": request.prompt}]
            }))
            .send()
            .map_err(map_transport_error)?;
        parse_json_response(response, api_key, parse_anthropic_response)
    }

    fn gemini(
        &self,
        endpoint: &ValidatedEndpoint,
        model: &str,
        api_key: &str,
        request: &AiRequest,
    ) -> Result<AiResponse, AiError> {
        let model = model.strip_prefix("models/").unwrap_or(model);
        if model.is_empty()
            || model.contains('/')
            || model.chars().any(|character| {
                !(character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.' | ':'))
            })
        {
            return Err(AiError::InvalidModel(
                "Gemini model name contains unsupported characters".to_owned(),
            ));
        }
        let raw = format!(
            "{}/models/{model}:generateContent",
            endpoint.as_url().as_str().trim_end_matches('/')
        );
        let url = Url::parse(&raw).map_err(|error| AiError::InvalidEndpoint(error.to_string()))?;
        let response = self
            .client
            .post(url)
            .header("x-goog-api-key", api_key)
            .json(&json!({
                "contents": [{"role": "user", "parts": [{"text": request.prompt}]}]
            }))
            .send()
            .map_err(map_transport_error)?;
        parse_json_response(response, api_key, parse_gemini_response)
    }

    fn compatible(
        &self,
        endpoint: &ValidatedEndpoint,
        model: &str,
        api_key: &str,
        request: &AiRequest,
    ) -> Result<AiResponse, AiError> {
        let url = chat_completions_url(endpoint.as_url())?;
        let mut builder = self.client.post(url).json(&json!({
            "model": model,
            "messages": [{"role": "user", "content": request.prompt}]
        }));
        if !api_key.trim().is_empty() {
            builder = builder.bearer_auth(api_key);
        }
        let response = builder.send().map_err(map_transport_error)?;
        parse_json_response(response, api_key, parse_chat_completions_response)
    }
}

impl AiProvider for HttpAiProvider {
    fn generate(
        &self,
        endpoint: &ValidatedEndpoint,
        model: &str,
        api_key: &str,
        request: &AiRequest,
    ) -> Result<AiResponse, AiError> {
        validate_model(self.provider, model)?;
        validate_request(request)?;
        if api_key.trim().is_empty() && self.provider != ProviderKind::Local {
            return Err(AiError::MissingApiKey);
        }
        match self.provider {
            ProviderKind::OpenAi => self.openai(endpoint, model, api_key, request),
            ProviderKind::Anthropic => self.anthropic(endpoint, model, api_key, request),
            ProviderKind::Gemini => self.gemini(endpoint, model, api_key, request),
            ProviderKind::OpenAiCompatible | ProviderKind::Local => {
                self.compatible(endpoint, model, api_key, request)
            }
        }
    }
}

fn chat_completions_url(base: &Url) -> Result<Url, AiError> {
    let mut url = base.clone();
    url.set_query(None);
    url.set_fragment(None);
    let path = url.path().trim_end_matches('/');
    let path = if path.ends_with("/chat/completions") {
        path.to_owned()
    } else if path.is_empty() {
        "/v1/chat/completions".to_owned()
    } else {
        format!("{path}/chat/completions")
    };
    url.set_path(&path);
    Ok(url)
}

fn map_transport_error(error: reqwest::Error) -> AiError {
    if error.is_timeout() {
        AiError::Timeout
    } else {
        AiError::Provider(safe_transport_error(&error))
    }
}

fn safe_transport_error(error: &reqwest::Error) -> String {
    if error.is_connect() {
        "could not connect to provider".to_owned()
    } else if error.is_request() {
        "provider request could not be sent".to_owned()
    } else {
        "provider transport failed".to_owned()
    }
}

fn parse_json_response(
    response: Response,
    api_key: &str,
    parser: fn(&Value) -> Result<String, AiError>,
) -> Result<AiResponse, AiError> {
    if response
        .content_length()
        .is_some_and(|size| size > MAX_RESPONSE_BYTES)
    {
        return Err(AiError::ResponseTooLarge);
    }
    let status = response.status();
    let body = response.text().map_err(map_transport_error)?;
    if body.len() as u64 > MAX_RESPONSE_BYTES {
        return Err(AiError::ResponseTooLarge);
    }
    if !status.is_success() {
        let message = provider_error_message(&body);
        return Err(AiError::Provider(format!(
            "HTTP {}: {}",
            status.as_u16(),
            sanitize_error(&message, api_key)
        )));
    }
    let value: Value = serde_json::from_str(&body)
        .map_err(|_| AiError::InvalidResponse("response was not valid JSON".to_owned()))?;
    let text = parser(&value)?;
    if text.trim().is_empty() {
        return Err(AiError::InvalidResponse(
            "provider returned no text".to_owned(),
        ));
    }
    Ok(AiResponse { text })
}

fn provider_error_message(body: &str) -> String {
    serde_json::from_str::<Value>(body)
        .ok()
        .and_then(|value| {
            value
                .pointer("/error/message")
                .and_then(Value::as_str)
                .or_else(|| value.get("message").and_then(Value::as_str))
                .map(str::to_owned)
        })
        .unwrap_or_else(|| body.trim().to_owned())
}

fn sanitize_error(raw: &str, api_key: &str) -> String {
    let redacted = if api_key.is_empty() {
        raw.to_owned()
    } else {
        raw.replace(api_key, "[redacted]")
    };
    let mut chars = redacted.chars();
    let shortened = chars.by_ref().take(MAX_ERROR_CHARS).collect::<String>();
    if chars.next().is_some() {
        format!("{shortened}…")
    } else if shortened.is_empty() {
        "provider returned an error without details".to_owned()
    } else {
        shortened
    }
}

fn parse_openai_response(value: &Value) -> Result<String, AiError> {
    if let Some(text) = value.get("output_text").and_then(Value::as_str) {
        return Ok(text.to_owned());
    }
    let mut parts = Vec::new();
    if let Some(output) = value.get("output").and_then(Value::as_array) {
        for item in output {
            if let Some(content) = item.get("content").and_then(Value::as_array) {
                for part in content {
                    if let Some(text) = part.get("text").and_then(Value::as_str) {
                        parts.push(text);
                    }
                }
            }
        }
    }
    if parts.is_empty() {
        Err(AiError::InvalidResponse(
            "OpenAI response did not contain output text".to_owned(),
        ))
    } else {
        Ok(parts.join("\n"))
    }
}

fn parse_chat_completions_response(value: &Value) -> Result<String, AiError> {
    let content = value.pointer("/choices/0/message/content").ok_or_else(|| {
        AiError::InvalidResponse("chat completion did not contain message content".to_owned())
    })?;
    if let Some(text) = content.as_str() {
        return Ok(text.to_owned());
    }
    let mut parts = Vec::new();
    if let Some(content) = content.as_array() {
        for part in content {
            if let Some(text) = part.get("text").and_then(Value::as_str) {
                parts.push(text);
            }
        }
    }
    if parts.is_empty() {
        Err(AiError::InvalidResponse(
            "chat completion did not contain text".to_owned(),
        ))
    } else {
        Ok(parts.join("\n"))
    }
}

fn parse_anthropic_response(value: &Value) -> Result<String, AiError> {
    let content = value
        .get("content")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            AiError::InvalidResponse("Anthropic response did not contain content".to_owned())
        })?;
    let parts = content
        .iter()
        .filter_map(|part| part.get("text").and_then(Value::as_str))
        .collect::<Vec<_>>();
    if parts.is_empty() {
        Err(AiError::InvalidResponse(
            "Anthropic response did not contain text".to_owned(),
        ))
    } else {
        Ok(parts.join("\n"))
    }
}

fn parse_gemini_response(value: &Value) -> Result<String, AiError> {
    if let Some(reason) = value
        .pointer("/promptFeedback/blockReason")
        .and_then(Value::as_str)
    {
        return Err(AiError::Provider(format!(
            "Gemini blocked the prompt: {reason}"
        )));
    }
    let parts = value
        .pointer("/candidates/0/content/parts")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            AiError::InvalidResponse("Gemini response did not contain candidates".to_owned())
        })?
        .iter()
        .filter_map(|part| part.get("text").and_then(Value::as_str))
        .collect::<Vec<_>>();
    if parts.is_empty() {
        Err(AiError::InvalidResponse(
            "Gemini response did not contain text".to_owned(),
        ))
    } else {
        Ok(parts.join("\n"))
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
        validate_request(request)?;
        let endpoint = self.policy.endpoint_for(config)?;
        self.provider
            .generate(&endpoint, config.model.trim(), api_key, request)
    }
}

#[cfg(test)]
mod tests {
    use std::{
        io::{Read, Write},
        net::TcpListener,
        sync::{Arc, Mutex},
        thread,
        time::Duration,
    };

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

    fn spawn_json_server(status: u16, body: &'static str) -> (String, Arc<Mutex<String>>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let captured = Arc::new(Mutex::new(String::new()));
        let captured_for_thread = captured.clone();
        thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut data = Vec::new();
            let mut buffer = [0_u8; 4096];
            let mut expected = None;
            loop {
                let read = stream.read(&mut buffer).unwrap();
                if read == 0 {
                    break;
                }
                data.extend_from_slice(&buffer[..read]);
                if expected.is_none() {
                    if let Some(index) = data.windows(4).position(|window| window == b"\r\n\r\n") {
                        let header_end = index + 4;
                        let headers = String::from_utf8_lossy(&data[..header_end]);
                        let content_length = headers
                            .lines()
                            .find_map(|line| {
                                let (name, value) = line.split_once(':')?;
                                name.eq_ignore_ascii_case("content-length")
                                    .then(|| value.trim().parse::<usize>().ok())
                                    .flatten()
                            })
                            .unwrap_or(0);
                        expected = Some(header_end + content_length);
                    }
                }
                if expected.is_some_and(|size| data.len() >= size) {
                    break;
                }
            }
            *captured_for_thread.lock().unwrap() = String::from_utf8_lossy(&data).into_owned();
            let reason = if status == 200 { "OK" } else { "Error" };
            let response = format!(
                "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            stream.write_all(response.as_bytes()).unwrap();
        });
        (format!("http://{address}"), captured)
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

    #[test]
    fn compatible_provider_executes_against_local_mock_server() {
        let (base, captured) = spawn_json_server(
            200,
            r#"{"choices":[{"message":{"content":"mock result"}}]}"#,
        );
        let service = AiService::new(
            HttpAiProvider::with_timeout(ProviderKind::OpenAiCompatible, Duration::from_secs(2))
                .unwrap(),
        );
        let response = service
            .generate(
                &compatible(&format!("{base}/v1")),
                "session-secret",
                &AiRequest {
                    prompt: "hello mock".to_owned(),
                },
            )
            .unwrap();
        assert_eq!(response.text, "mock result");
        let request = captured.lock().unwrap().clone();
        assert!(request.starts_with("POST /v1/chat/completions "));
        assert!(request
            .to_ascii_lowercase()
            .contains("authorization: bearer session-secret"));
        assert!(request.contains("hello mock"));
    }

    #[test]
    fn provider_error_redacts_session_key() {
        let (base, _) = spawn_json_server(401, r#"{"error":{"message":"bad key session-secret"}}"#);
        let service = AiService::new(
            HttpAiProvider::with_timeout(ProviderKind::OpenAiCompatible, Duration::from_secs(2))
                .unwrap(),
        );
        let error = service
            .generate(
                &compatible(&format!("{base}/v1")),
                "session-secret",
                &AiRequest {
                    prompt: "hello".to_owned(),
                },
            )
            .unwrap_err();
        let rendered = error.to_string();
        assert!(!rendered.contains("session-secret"));
        assert!(rendered.contains("[redacted]"));
    }

    #[test]
    fn parses_builtin_provider_response_shapes() {
        assert_eq!(
            parse_openai_response(
                &json!({"output":[{"content":[{"type":"output_text","text":"openai"}]}]})
            )
            .unwrap(),
            "openai"
        );
        assert_eq!(
            parse_anthropic_response(&json!({"content":[{"type":"text","text":"anthropic"}]}))
                .unwrap(),
            "anthropic"
        );
        assert_eq!(
            parse_gemini_response(
                &json!({"candidates":[{"content":{"parts":[{"text":"gemini"}]}}]})
            )
            .unwrap(),
            "gemini"
        );
    }
}
