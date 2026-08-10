use std::{
    io::{Read, Write},
    net::TcpListener,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use scriptotar_ai::{AiError, AiProvider, AiRequest, EndpointPolicy, HttpAiProvider, ProviderKind};

fn spawn_server(
    status: u16,
    headers: &[(&str, &str)],
    body: &'static str,
    delay: Duration,
) -> (String, Arc<Mutex<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let captured = Arc::new(Mutex::new(String::new()));
    let captured_thread = captured.clone();
    let headers = headers
        .iter()
        .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
        .collect::<Vec<_>>();
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
        *captured_thread.lock().unwrap() = String::from_utf8_lossy(&data).into_owned();
        thread::sleep(delay);
        let reason = if status < 400 { "OK" } else { "Error" };
        let mut response = format!("HTTP/1.1 {status} {reason}\r\n");
        for (name, value) in headers {
            response.push_str(&format!("{name}: {value}\r\n"));
        }
        response.push_str(&format!(
            "Content-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        ));
        let _ = stream.write_all(response.as_bytes());
    });
    (format!("http://{address}"), captured)
}

fn endpoint(base: &str) -> scriptotar_ai::ValidatedEndpoint {
    EndpointPolicy.validate(base).unwrap()
}

fn generate(kind: ProviderKind, base: &str, model: &str, key: &str) -> Result<String, AiError> {
    let provider = HttpAiProvider::with_timeout(kind, Duration::from_millis(500)).unwrap();
    provider
        .generate(
            &endpoint(base),
            model,
            key,
            &AiRequest {
                prompt: "fixture prompt".to_owned(),
            },
        )
        .map(|response| response.text)
}

#[test]
fn openai_anthropic_gemini_and_compatible_success() {
    let (base, request) = spawn_server(
        200,
        &[],
        r#"{"output":[{"content":[{"type":"output_text","text":"openai ok"}]}]}"#,
        Duration::ZERO,
    );
    assert_eq!(
        generate(ProviderKind::OpenAi, &base, "gpt-test", "openai-secret").unwrap(),
        "openai ok"
    );
    assert!(request
        .lock()
        .unwrap()
        .to_ascii_lowercase()
        .contains("authorization: bearer openai-secret"));

    let (base, request) = spawn_server(
        200,
        &[],
        r#"{"content":[{"type":"text","text":"anthropic ok"}]}"#,
        Duration::ZERO,
    );
    assert_eq!(
        generate(
            ProviderKind::Anthropic,
            &base,
            "claude-test",
            "anthropic-secret"
        )
        .unwrap(),
        "anthropic ok"
    );
    assert!(request
        .lock()
        .unwrap()
        .to_ascii_lowercase()
        .contains("x-api-key: anthropic-secret"));

    let (base, request) = spawn_server(
        200,
        &[],
        r#"{"candidates":[{"content":{"parts":[{"text":"gemini ok"}]}}]}"#,
        Duration::ZERO,
    );
    assert_eq!(
        generate(ProviderKind::Gemini, &base, "gemini-test", "gemini-secret").unwrap(),
        "gemini ok"
    );
    let request = request.lock().unwrap().to_ascii_lowercase();
    assert!(request.contains("post /models/gemini-test:generatecontent "));
    assert!(request.contains("x-goog-api-key: gemini-secret"));

    let (base, _) = spawn_server(
        200,
        &[],
        r#"{"choices":[{"message":{"content":"compatible ok"}}]}"#,
        Duration::ZERO,
    );
    assert_eq!(
        generate(
            ProviderKind::OpenAiCompatible,
            &format!("{base}/v1"),
            "compatible-test",
            "compatible-secret"
        )
        .unwrap(),
        "compatible ok"
    );
}

#[test]
fn provider_4xx_5xx_and_malformed_json_are_structured_errors() {
    for status in [400_u16, 503_u16] {
        let (base, _) = spawn_server(
            status,
            &[],
            r#"{"error":{"message":"provider fixture error"}}"#,
            Duration::ZERO,
        );
        let error = generate(ProviderKind::OpenAiCompatible, &base, "model", "secret").unwrap_err();
        assert!(matches!(error, AiError::Provider(_)));
        assert!(error.to_string().contains(&status.to_string()));
    }

    let (base, _) = spawn_server(200, &[], "not-json", Duration::ZERO);
    assert!(matches!(
        generate(ProviderKind::OpenAiCompatible, &base, "model", "secret"),
        Err(AiError::InvalidResponse(_))
    ));
}

#[test]
fn timeout_is_reported_without_panicking() {
    let (base, _) = spawn_server(
        200,
        &[],
        r#"{"choices":[{"message":{"content":"late"}}]}"#,
        Duration::from_millis(800),
    );
    assert_eq!(
        generate(ProviderKind::OpenAiCompatible, &base, "model", "secret").unwrap_err(),
        AiError::Timeout
    );
}

fn assert_redirect_blocked(location: &str) {
    let (base, captured) = spawn_server(302, &[("Location", location)], "", Duration::ZERO);
    let error = generate(ProviderKind::OpenAi, &base, "gpt-test", "redirect-secret").unwrap_err();
    assert_eq!(error, AiError::RedirectBlocked(302));
    let request = captured.lock().unwrap().clone();
    assert!(request.contains("redirect-secret"));
}

#[test]
fn openai_style_redirects_are_blocked_for_every_target_class() {
    assert_redirect_blocked("http://example.com/unapproved");
    assert_redirect_blocked("http://localhost:65535/unapproved");
    assert_redirect_blocked("https://another.example/unapproved");
}

#[test]
fn ordinary_non_redirect_request_still_works() {
    let (base, _) = spawn_server(
        200,
        &[],
        r#"{"output":[{"content":[{"type":"output_text","text":"no redirect"}]}]}"#,
        Duration::ZERO,
    );
    assert_eq!(
        generate(ProviderKind::OpenAi, &base, "gpt-test", "secret").unwrap(),
        "no redirect"
    );
}
