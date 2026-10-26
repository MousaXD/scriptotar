//! Read-only local MCP server over stdio. No HTTP listener or network egress.
use scriptotar_core::{ContentRepository, ProjectRepository, ResearchRepository};
use scriptotar_db::SqliteStore;
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};
use uuid::Uuid;

const MAX_REQUEST: usize = 1024 * 1024;

fn response(request: &Value, store: &SqliteStore) -> Option<Value> {
    let id = request.get("id")?;
    let version = request.get("jsonrpc").and_then(Value::as_str);
    if version != Some("2.0") {
        return Some(error(id, -32600, "invalid JSON-RPC version"));
    }
    let result = match request.get("method").and_then(Value::as_str) {
        Some("initialize") => {
            json!({"protocolVersion":"2025-03-26","capabilities":{"tools":{}},"serverInfo":{"name":"scriptotar","version":env!("CARGO_PKG_VERSION")}})
        }
        Some("ping") => json!({}),
        Some("tools/list") => json!({"tools": [
            {"name":"list_projects","description":"List local Scriptotar projects (read-only)","inputSchema":{"type":"object","properties":{},"additionalProperties":false}},
            {"name":"list_research","description":"List research items for a Scriptotar project (up to 50)","inputSchema":{"type":"object","properties":{"project_id":{"type":"string","format":"uuid"}},"required":["project_id"],"additionalProperties":false}},
            {"name":"list_transcripts","description":"List local transcript metadata for a Scriptotar project, without returning transcript text (up to 50)","inputSchema":{"type":"object","properties":{"project_id":{"type":"string","format":"uuid"}},"required":["project_id"],"additionalProperties":false}}
        ]}),
        Some("tools/call") => return Some(call_tool(id, request.get("params"), store)),
        _ => return Some(error(id, -32601, "method not found")),
    };
    Some(json!({"jsonrpc":"2.0","id":id,"result":result}))
}

fn error(id: &Value, code: i32, message: &str) -> Value {
    json!({"jsonrpc":"2.0","id":id,"error":{"code":code,"message":message}})
}

fn call_tool(id: &Value, params: Option<&Value>, store: &SqliteStore) -> Value {
    let Some(params) = params.and_then(Value::as_object) else {
        return error(id, -32602, "params must be an object");
    };
    let Some(name) = params.get("name").and_then(Value::as_str) else {
        return error(id, -32602, "tool name required");
    };
    let args = params.get("arguments").cloned().unwrap_or_else(|| json!({}));
    let Some(args) = args.as_object() else {
        return error(id, -32602, "arguments must be an object");
    };
    let project = || -> Result<Uuid, String> {
        if args.len() != 1 {
            return Err("only project_id is permitted".into());
        }
        let input = args.get("project_id").and_then(Value::as_str).ok_or("project_id required")?;
        Uuid::parse_str(input).map_err(|_| "invalid project_id".into())
    };
    let content: Result<Value,String> = match name {
        "list_projects" => if args.is_empty() { store.list_projects().map(|v|json!(v)).map_err(|e|e.to_string()) } else { Err("no arguments permitted".into()) },
        "list_research" => project().and_then(|p| store.list_research_items(Some(p)).map_err(|e|e.to_string()).map(|items| json!(items.into_iter().take(50).map(|i|json!({"id":i.id,"source_url":i.source_url,"platform":i.platform,"title":i.title,"view_count":i.view_count,"published_at":i.published_at})).collect::<Vec<_>>()))),
        "list_transcripts" => project().and_then(|p| store.list_transcripts(Some(p)).map_err(|e|e.to_string()).map(|items|json!(items.into_iter().take(50).map(|i|json!({"id":i.transcript.id,"source_url":i.source.locator,"language":i.transcript.language,"updated_at":i.transcript.updated_at})).collect::<Vec<_>>()))),
        _ => return error(id, -32602, "unknown tool"),
    };
    let result = match content {
        Ok(value) => json!({"content":[{"type":"text","text":value.to_string()}]}),
        Err(reason) => json!({"content":[{"type":"text","text":reason}],"isError":true}),
    };
    json!({"jsonrpc":"2.0","id":id,"result":result})
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::var_os("SCRIPTOTAR_DB_PATH")
        .ok_or("SCRIPTOTAR_DB_PATH is required; explicitly select your local database")?;
    let store = SqliteStore::open(path)?;
    let stdin = io::stdin();
    let mut input = stdin.lock();
    let mut output = io::stdout().lock();
    loop {
        let mut line = Vec::new();
        let n = input.read_until(b'\n', &mut line)?;
        if n == 0 {
            break;
        }
        if n > MAX_REQUEST {
            eprintln!("oversized MCP request rejected");
            continue;
        }
        let request: Value = match serde_json::from_slice(&line) {
            Ok(value) => value,
            Err(_) => {
                writeln!(output, "{}", error(&Value::Null, -32700, "invalid JSON"))?;
                output.flush()?;
                continue;
            }
        };
        if let Some(reply) = response(&request, &store) {
            writeln!(output, "{reply}")?;
            output.flush()?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn errors_are_jsonrpc_compatible() {
        let e = error(&json!(4), -32601, "missing");
        assert_eq!(e["error"]["code"], -32601);
        assert_eq!(e["id"], 4);
    }
}
