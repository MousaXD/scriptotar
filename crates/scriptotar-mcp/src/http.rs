//! Localhost-only, bearer-authenticated Streamable HTTP MCP for secure-tunnel connections.
use std::{convert::Infallible, sync::Arc};

use bytes::Bytes;
use http_body_util::{BodyExt, Full, Limited};
use hyper::{
    body::Incoming,
    header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE, ORIGIN},
    server::conn::http1,
    service::service_fn,
    Method, Request, Response, StatusCode,
};
use hyper_util::rt::TokioIo;
use serde_json::Value;
use tokio::net::TcpListener;

use crate::{load_permissions, server::Server};

const MAX_HTTP_BODY: usize = 1024 * 1024;
type HttpResponse = Response<Full<Bytes>>;

pub(crate) async fn serve(server: Server, port: u16) -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port)).await?;
    let server = Arc::new(server);
    let limit = Arc::new(tokio::sync::Semaphore::new(4));
    eprintln!("[scriptotar-mcp] local HTTP transport on 127.0.0.1:{port}/mcp");
    loop {
        let (stream, _) = listener.accept().await?;
        let local = Arc::clone(&server);
        let limit = Arc::clone(&limit);
        tokio::spawn(async move {
            let io = TokioIo::new(stream);
            let service = service_fn(move |req| {
                let local = Arc::clone(&local);
                let limit = Arc::clone(&limit);
                async move { handle(req, local, port, limit).await }
            });
            if let Err(error) = http1::Builder::new().serve_connection(io, service).await {
                eprintln!("[scriptotar-mcp] HTTP connection error: {error}");
            }
        });
    }
}

async fn handle(
    request: Request<Incoming>,
    server: Arc<Server>,
    port: u16,
    limit: Arc<tokio::sync::Semaphore>,
) -> Result<HttpResponse, Infallible> {
    let path = request.uri().path();
    let is_bearer_path = path == "/mcp";
    let public_secret = path.strip_prefix("/mcp/p/");
    if !is_bearer_path && public_secret.is_none() {
        return Ok(reply(StatusCode::NOT_FOUND, "not found"));
    }
    if let Some(origin) = request.headers().get(ORIGIN) {
        let origin = origin.to_str().unwrap_or_default();
        if origin != format!("http://127.0.0.1:{port}")
            && origin != format!("http://localhost:{port}")
            && origin != "https://chatgpt.com"
            && origin != "https://chat.openai.com"
        {
            return Ok(reply(StatusCode::FORBIDDEN, "invalid Origin"));
        }
    }
    let policy = match load_permissions(&server.data_dir) {
        Ok(value) if value.enabled && !value.http_token.is_empty() => value,
        _ => return Ok(reply(StatusCode::FORBIDDEN, "MCP disabled")),
    };
    // A secret in the path is the authentication method for a remote ChatGPT
    // URL connection, which cannot set custom bearer headers. /mcp requires the
    // original bearer header and never accepts a path secret as a fallback.
    let authorized = if let Some(secret) = public_secret {
        policy.url_token.len() == 64 && constant_time_eq(secret.as_bytes(), policy.url_token.as_bytes())
    } else {
        let auth = request.headers().get(AUTHORIZATION).and_then(|v| v.to_str().ok()).unwrap_or("");
        auth.starts_with("Bearer ")
            && constant_time_eq(
                auth.strip_prefix("Bearer ").unwrap_or_default().as_bytes(),
                policy.http_token.as_bytes(),
            )
    };
    if !authorized {
        return Ok(reply(StatusCode::UNAUTHORIZED, "unauthorized"));
    }
    if request.method() == Method::GET {
        return Ok(reply(StatusCode::METHOD_NOT_ALLOWED, "SSE is not supported"));
    }
    if request.method() != Method::POST {
        return Ok(reply(StatusCode::METHOD_NOT_ALLOWED, "method not allowed"));
    }
    if !request.headers().get(CONTENT_TYPE).and_then(|v| v.to_str().ok()).unwrap_or("").starts_with("application/json")
    {
        return Ok(reply(StatusCode::UNSUPPORTED_MEDIA_TYPE, "expected application/json"));
    }
    let accept = request.headers().get(ACCEPT).and_then(|v| v.to_str().ok()).unwrap_or("");
    if !accept.contains("application/json") {
        return Ok(reply(StatusCode::NOT_ACCEPTABLE, "Accept must include application/json"));
    }
    if request
        .headers()
        .get(hyper::header::CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<usize>().ok())
        .is_some_and(|v| v > MAX_HTTP_BODY)
    {
        return Ok(reply(StatusCode::PAYLOAD_TOO_LARGE, "MCP request too large"));
    }
    let body = match Limited::new(request.into_body(), MAX_HTTP_BODY).collect().await {
        Ok(body) => body.to_bytes(),
        Err(_) => return Ok(reply(StatusCode::PAYLOAD_TOO_LARGE, "MCP request too large")),
    };
    let message: Value = match serde_json::from_slice(&body) {
        Ok(message) => message,
        Err(_) => {
            return Ok(json_reply(
                StatusCode::BAD_REQUEST,
                &serde_json::json!({"jsonrpc":"2.0","id":null,"error":{"code":-32700,"message":"invalid JSON"}}),
            ))
        }
    };
    let Ok(_permit) = limit.try_acquire() else {
        return Ok(reply(StatusCode::TOO_MANY_REQUESTS, "MCP busy"));
    };
    // Tool calls may include bounded subprocess or SQLite work; keep Tokio's accept loop responsive.
    let result = tokio::task::spawn_blocking(move || server.response(&message)).await;
    match result {
        Ok(Some(value)) => Ok(json_reply(StatusCode::OK, &value)),
        Ok(None) => Ok(reply(StatusCode::ACCEPTED, "")),
        Err(_) => Ok(reply(StatusCode::INTERNAL_SERVER_ERROR, "MCP worker failed")),
    }
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    let mut different = a.len() ^ b.len();
    for i in 0..a.len().max(b.len()) {
        different |= usize::from(a.get(i).copied().unwrap_or(0) ^ b.get(i).copied().unwrap_or(0));
    }
    different == 0
}

fn reply(status: StatusCode, content: &str) -> HttpResponse {
    Response::builder()
        .status(status)
        .header("Cache-Control", "no-store")
        .header("Content-Type", "text/plain; charset=utf-8")
        .body(Full::new(Bytes::copy_from_slice(content.as_bytes())))
        .expect("valid HTTP response")
}
fn json_reply(status: StatusCode, content: &Value) -> HttpResponse {
    Response::builder()
        .status(status)
        .header("Cache-Control", "no-store")
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(content.to_string())))
        .expect("valid JSON response")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bearer_comparison_rejects_wrong_length_and_value() {
        assert!(constant_time_eq(b"abcd", b"abcd"));
        assert!(!constant_time_eq(b"abcd", b"abce"));
        assert!(!constant_time_eq(b"abcd", b"abc"));
    }
}
