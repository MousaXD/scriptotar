//! Scriptotar MCP stdio process: all privileges are enforced by desktop-owned policy.
use std::{
    fs,
    io::{self, BufRead, Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use crate::{
    authorized_directory, authorized_program, authorized_read, authorized_write, load_permissions,
    McpPermissions,
};
use scriptotar_core::{
    now_rfc3339, ContentRepository, Creator, JobInput, JobRepository, Project, ProjectRepository,
    ResearchRepository,
};
use scriptotar_db::SqliteStore;
use scriptotar_jobs::JobService;
use scriptotar_media::MediaPolicy;
use scriptotar_orchestrator::{JobOrchestrator, RuntimeConfig};
use scriptotar_research::{NetworkPolicy, ResearchService, YtDlpCommand, YtDlpProvider};
use serde_json::{json, Map, Value};
use uuid::Uuid;

const MAX_REQUEST: usize = 1024 * 1024;
const MAX_FILE: u64 = 128 * 1024;
const MAX_WRITE: usize = 256 * 1024;
const MAX_COMMAND_OUTPUT: u64 = 64 * 1024;
const COMMAND_TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Clone)]
pub(crate) struct Server {
    store: SqliteStore,
    pub(crate) data_dir: PathBuf,
    orchestrator: JobOrchestrator<SqliteStore>,
}

impl Server {
    pub(crate) fn new(db_path: &Path) -> Result<Self, String> {
        if !db_path.is_absolute() {
            return Err("SCRIPTOTAR_DB_PATH must be absolute".into());
        }
        let store = SqliteStore::open(db_path).map_err(|e| e.to_string())?;
        let data_dir = fs::canonicalize(
            db_path
                .parent()
                .ok_or("database parent directory required")?,
        )
        .map_err(|_| "could not canonicalize Scriptotar application data directory")?;
        store
            .run_integration_migrations()
            .map_err(|e| e.to_string())?;
        let python = std::env::var_os("SCRIPTOTAR_SIDECAR_PYTHON")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(if cfg!(windows) { "python" } else { "python3" }));
        let sidecar = std::env::var_os("SCRIPTOTAR_SIDECAR_SCRIPT")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../../sidecars/transcription/sidecar.py")
            });
        let config = RuntimeConfig::new(python, sidecar, data_dir.join("transcription-output"));
        let orchestrator = JobOrchestrator::start(store.clone(), config);
        Ok(Self {
            store,
            data_dir,
            orchestrator,
        })
    }

    pub(crate) fn response(&self, request: &Value) -> Option<Value> {
        let id = request.get("id")?;
        if request.get("jsonrpc").and_then(Value::as_str) != Some("2.0") {
            return Some(rpc_error(id, -32600, "invalid JSON-RPC version"));
        }
        let result = match request.get("method").and_then(Value::as_str) {
            Some("initialize") => {
                let requested = request
                    .pointer("/params/protocolVersion")
                    .and_then(Value::as_str);
                let protocol = match requested {
                    Some("2025-11-25") => "2025-11-25",
                    Some("2025-06-18") => "2025-06-18",
                    _ => "2025-03-26",
                };
                json!({"protocolVersion":protocol,"capabilities":{"tools":{"listChanged":false}},"serverInfo":{"name":"scriptotar","version":env!("CARGO_PKG_VERSION")}})
            }
            Some("ping") => json!({}),
            Some("tools/list") => tool_catalog(),
            Some("tools/call") => return Some(self.call_tool(id, request.get("params"))),
            _ => return Some(rpc_error(id, -32601, "method not found")),
        };
        Some(json!({"jsonrpc":"2.0","id":id,"result":result}))
    }

    fn call_tool(&self, id: &Value, params: Option<&Value>) -> Value {
        let Some(params) = params.and_then(Value::as_object) else {
            return rpc_error(id, -32602, "params must be an object");
        };
        let Some(name) = params.get("name").and_then(Value::as_str) else {
            return rpc_error(id, -32602, "tool name required");
        };
        let args = params
            .get("arguments")
            .cloned()
            .unwrap_or_else(|| json!({}));
        let Some(args) = args.as_object() else {
            return rpc_error(id, -32602, "arguments must be an object");
        };
        let policy = match load_permissions(&self.data_dir) {
            Ok(p) => p,
            Err(message) => return tool_error(id, &message),
        };
        let result = self.run_tool(name, args, &policy);
        match result {
            Ok(value) => {
                json!({"jsonrpc":"2.0","id":id,"result":{"content":[{"type":"text","text":value.to_string()}]}})
            }
            Err(message) => tool_error(id, &message),
        }
    }

    fn run_tool(
        &self,
        name: &str,
        args: &Map<String, Value>,
        policy: &McpPermissions,
    ) -> Result<Value, String> {
        match name {
            "list_projects" => {
                keys(args, &[], &[])?;
                policy.require(policy.read_data, "read-data")?;
                let projects = self.store.list_projects().map_err(|e| e.to_string())?;
                Ok(json!(projects.into_iter().take(50).collect::<Vec<_>>()))
            }
            "list_research" => {
                keys(args, &["project_id"], &[])?;
                policy.require(policy.read_data, "read-data")?;
                let project = uuid_arg(args, "project_id")?;
                self.store.get_project(project).map_err(|e| e.to_string())?;
                let items = self
                    .store
                    .list_research_items(Some(project))
                    .map_err(|e| e.to_string())?;
                Ok(json!(items.into_iter().take(50).map(|i|json!({"id":i.id,"source_url":i.source_url,"platform":i.platform,"title":i.title,"view_count":i.view_count,"published_at":i.published_at})).collect::<Vec<_>>()))
            }
            "list_transcripts" => {
                keys(args, &["project_id"], &[])?;
                policy.require(policy.read_data, "read-data")?;
                let project = uuid_arg(args, "project_id")?;
                self.store.get_project(project).map_err(|e| e.to_string())?;
                let items = self
                    .store
                    .list_transcripts(Some(project))
                    .map_err(|e| e.to_string())?;
                Ok(json!(items.into_iter().take(50).map(|i|json!({"id":i.transcript.id,"source_url":i.source.locator,"language":i.transcript.language,"updated_at":i.transcript.updated_at})).collect::<Vec<_>>()))
            }
            "get_transcript" => {
                keys(args, &["project_id", "transcript_id"], &[])?;
                policy.require(policy.read_data, "read-data")?;
                let project = uuid_arg(args, "project_id")?;
                let transcript = uuid_arg(args, "transcript_id")?;
                self.store.get_project(project).map_err(|e| e.to_string())?;
                let matches = self
                    .store
                    .list_transcripts(Some(project))
                    .map_err(|e| e.to_string())?;
                let bundle = matches
                    .into_iter()
                    .find(|item| item.transcript.id == transcript)
                    .ok_or("transcript not found in requested project")?;
                let text = bundle.transcript.text;
                if text.len() > 128 * 1024 {
                    return Err("transcript exceeds MCP response limit".into());
                }
                Ok(json!({"id":transcript,"text":text,"language":bundle.transcript.language}))
            }
            "search_transcripts" => {
                keys(args, &["project_id", "query"], &["limit"])?;
                policy.require(policy.read_data, "read-data")?;
                let project = uuid_arg(args, "project_id")?;
                let query = string_arg(args, "query", 200)?;
                let limit = optional_number(args, "limit", 10, 1, 50)?;
                self.store.get_project(project).map_err(|e| e.to_string())?;
                let matches = self
                    .store
                    .search_transcript_ids(project, query, limit)
                    .map_err(|e| e.to_string())?;
                Ok(json!(matches))
            }
            "list_jobs" => {
                keys(args, &["project_id"], &[])?;
                policy.require(policy.read_data, "read-data")?;
                let project = uuid_arg(args, "project_id")?;
                self.store.get_project(project).map_err(|e| e.to_string())?;
                let jobs = self
                    .store
                    .list_jobs(Some(project))
                    .map_err(|e| e.to_string())?;
                Ok(json!(jobs.into_iter().take(50).collect::<Vec<_>>()))
            }
            "create_project" => {
                keys(args, &["name"], &[])?;
                policy.require(policy.write_data, "write-data")?;
                let name = string_arg(args, "name", 120)?.trim();
                if name.is_empty() {
                    return Err("project name required".into());
                }
                let project = Project::new(name);
                self.store
                    .create_project(&project)
                    .map_err(|e| e.to_string())?;
                Ok(json!(project))
            }
            "list_directory" => {
                keys(args, &["path"], &[])?;
                policy.require(policy.read_files, "read-files")?;
                let path = authorized_directory(policy, string_arg(args, "path", 4096)?)?;
                let mut entries = Vec::new();
                for item in fs::read_dir(&path).map_err(|_| "cannot list directory")? {
                    let entry = item.map_err(|_| "directory listing failed")?;
                    let meta = fs::symlink_metadata(entry.path())
                        .map_err(|_| "cannot inspect directory entry")?;
                    entries.push(json!({"name":entry.file_name().to_string_lossy(), "kind": if meta.file_type().is_symlink() { "symlink" } else if meta.is_dir() { "directory" } else if meta.is_file() { "file" } else { "other" }}));
                    if entries.len() >= 200 {
                        break;
                    }
                }
                Ok(json!({"path":path,"entries":entries}))
            }
            "read_file" => {
                keys(args, &["path"], &[])?;
                let path =
                    authorized_read(policy, &self.data_dir, string_arg(args, "path", 4096)?)?;
                // Enforce the byte cap while reading, not just with a pre-read
                // metadata check: a file can grow after metadata is checked.
                let file = fs::File::open(&path).map_err(|_| "cannot open file")?;
                let mut data = Vec::new();
                file.take(MAX_FILE + 1)
                    .read_to_end(&mut data)
                    .map_err(|_| "cannot read file")?;
                if data.len() as u64 > MAX_FILE {
                    return Err("file exceeds 128 KiB text limit".into());
                }
                let text = String::from_utf8(data).map_err(|_| "file must contain UTF-8 text")?;
                Ok(json!({"path":path,"text":text}))
            }
            "write_file" => {
                keys(args, &["path", "content"], &["overwrite"])?;
                let overwrite = args
                    .get("overwrite")
                    .map(Value::as_bool)
                    .unwrap_or(Some(false))
                    .ok_or("overwrite must be a boolean")?;
                let path = authorized_write(
                    policy,
                    &self.data_dir,
                    string_arg(args, "path", 4096)?,
                    overwrite,
                )?;
                let content = string_arg(args, "content", MAX_WRITE)?;
                let mut tmp = tempfile::NamedTempFile::new_in(
                    path.parent().ok_or("invalid output directory")?,
                )
                .map_err(|_| "cannot create temporary output")?;
                tmp.write_all(content.as_bytes())
                    .and_then(|_| tmp.as_file_mut().sync_all())
                    .map_err(|_| "cannot write file")?;
                if !overwrite {
                    // Persist without replacing an existing destination.
                    tmp.persist_noclobber(&path)
                        .map_err(|_| "file exists or could not be saved")?;
                } else {
                    tmp.persist(&path).map_err(|_| "could not replace file")?;
                }
                Ok(json!({"path":path,"bytes":content.len()}))
            }
            "execute_command" => {
                keys(args, &["program"], &["args", "cwd"])?;
                let program = authorized_program(policy, string_arg(args, "program", 4096)?)?;
                let arguments = args.get("args").cloned().unwrap_or_else(|| json!([]));
                let values = arguments.as_array().ok_or("args must be an array")?;
                if values.len() > 32 {
                    return Err("at most 32 command arguments allowed".into());
                }
                let mut cli_args = Vec::new();
                for value in values {
                    let arg = value.as_str().ok_or("command arguments must be strings")?;
                    if arg.len() > 4096 || arg.contains('\0') {
                        return Err("invalid command argument".into());
                    }
                    cli_args.push(arg.to_owned());
                }
                let cwd = match args.get("cwd") {
                    Some(value) => {
                        authorized_directory(policy, value.as_str().ok_or("cwd must be a string")?)?
                    }
                    None => policy
                        .allowed_paths
                        .first()
                        .map(PathBuf::from)
                        .unwrap_or_else(|| {
                            program.parent().unwrap_or(Path::new("/")).to_path_buf()
                        }),
                };
                run_command(&program, &cli_args, &cwd)
            }
            "search_creator" => {
                keys(args, &["project_id", "profile_url"], &["limit"])?;
                policy.require(policy.search_creators, "search-creators")?;
                let project = uuid_arg(args, "project_id")?;
                self.store.get_project(project).map_err(|e| e.to_string())?;
                let url = NetworkPolicy
                    .validate(string_arg(args, "profile_url", 2048)?)
                    .map_err(|e| e.to_string())?;
                let limit = optional_number(args, "limit", 20, 1, 200)? as u16;
                let source = url.as_url();
                let canonical_url = source.to_string();
                let host = source.host_str().unwrap_or_default();
                let platform = if host.ends_with("instagram.com") {
                    "instagram"
                } else if host.ends_with("tiktok.com") {
                    "tiktok"
                } else {
                    "youtube"
                };
                let creator = self
                    .store
                    .upsert_creator(&Creator {
                        id: Uuid::new_v4(),
                        project_id: project,
                        platform: platform.into(),
                        profile_url: canonical_url.clone(),
                        display_name: None,
                        created_at: now_rfc3339(),
                    })
                    .map_err(|e| e.to_string())?;
                // No browser cookies are read by the MCP research tool.
                let provider = YtDlpProvider::new(YtDlpCommand::from_environment());
                let items = ResearchService::new(provider)
                    .scan(project, Some(creator.id), &canonical_url, limit)
                    .map_err(|e| e.to_string())?;
                let saved = self
                    .store
                    .upsert_research_items(&items)
                    .map_err(|e| e.to_string())?;
                Ok(json!({"project_id":project,"creator_id":creator.id,"saved":saved.len()}))
            }
            "enqueue_local_media" => {
                keys(args, &["project_id", "path"], &[])?;
                policy.require(policy.download_videos, "download-videos")?;
                let project = uuid_arg(args, "project_id")?;
                self.store.get_project(project).map_err(|e| e.to_string())?;
                let path =
                    authorized_read(policy, &self.data_dir, string_arg(args, "path", 4096)?)?;
                MediaPolicy
                    .validate_local_input(&path)
                    .map_err(|e| e.to_string())?;
                let job = JobService::new(self.store.clone())
                    .enqueue(
                        project,
                        JobInput::LocalFile(path.to_string_lossy().into_owned()),
                    )
                    .map_err(|e| e.to_string())?;
                self.orchestrator
                    .enqueue(job.id)
                    .map_err(|e| e.to_string())?;
                Ok(json!({"job_id":job.id,"state":"queued"}))
            }
            "download_video" => {
                keys(args, &["project_id", "url"], &[])?;
                policy.require(policy.download_videos, "download-videos")?;
                let project = uuid_arg(args, "project_id")?;
                self.store.get_project(project).map_err(|e| e.to_string())?;
                let validated = NetworkPolicy
                    .validate(string_arg(args, "url", 2048)?)
                    .map_err(|e| e.to_string())?;
                let job = JobService::new(self.store.clone())
                    .enqueue(project, JobInput::Url(validated.as_url().to_string()))
                    .map_err(|e| e.to_string())?;
                self.orchestrator
                    .enqueue(job.id)
                    .map_err(|e| e.to_string())?;
                Ok(
                    json!({"job_id":job.id,"state":"queued","note":"queued to Scriptotar's existing download and transcription orchestrator; use list_jobs to follow progress"}),
                )
            }
            _ => Err("unknown tool".into()),
        }
    }
}

fn keys(args: &Map<String, Value>, required: &[&str], optional: &[&str]) -> Result<(), String> {
    for key in required {
        if !args.contains_key(*key) {
            return Err(format!("missing required argument: {key}"));
        }
    }
    for key in args.keys() {
        if !required.contains(&key.as_str()) && !optional.contains(&key.as_str()) {
            return Err(format!("unsupported argument: {key}"));
        }
    }
    Ok(())
}
fn string_arg<'a>(args: &'a Map<String, Value>, key: &str, max: usize) -> Result<&'a str, String> {
    let value = args
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("{key} must be a string"))?;
    if value.len() > max || value.contains('\0') {
        return Err(format!(
            "{key} exceeds allowed size or contains invalid NUL"
        ));
    }
    Ok(value)
}
fn uuid_arg(args: &Map<String, Value>, key: &str) -> Result<Uuid, String> {
    Uuid::parse_str(string_arg(args, key, 64)?).map_err(|_| format!("invalid {key}"))
}
fn optional_number(
    args: &Map<String, Value>,
    key: &str,
    default: usize,
    min: usize,
    max: usize,
) -> Result<usize, String> {
    let value = match args.get(key) {
        Some(v) => usize::try_from(
            v.as_u64()
                .ok_or_else(|| format!("{key} must be an integer"))?,
        )
        .map_err(|_| "number too large")?,
        None => default,
    };
    if !(min..=max).contains(&value) {
        return Err(format!("{key} must be between {min} and {max}"));
    }
    Ok(value)
}
fn rpc_error(id: &Value, code: i32, message: &str) -> Value {
    json!({"jsonrpc":"2.0","id":id,"error":{"code":code,"message":message}})
}
fn tool_error(id: &Value, message: &str) -> Value {
    json!({"jsonrpc":"2.0","id":id,"result":{"content":[{"type":"text","text":message}],"isError":true}})
}
fn tool_catalog() -> Value {
    let p = |fields: Value, required: Vec<&str>| json!({"type":"object","properties":fields,"required":required,"additionalProperties":false});
    let project = json!({"project_id":{"type":"string","format":"uuid"}});
    let ro = json!({"readOnlyHint":true,"openWorldHint":false});
    let write = json!({"readOnlyHint":false,"destructiveHint":false,"openWorldHint":false});
    let mut tools = vec![
        json!({"name":"list_projects","description":"List up to 50 Scriptotar projects, subject to desktop read-data permission","inputSchema":p(json!({}),vec![]),"annotations":ro}),
        json!({"name":"list_research","description":"List saved research metadata in a project","inputSchema":p(project.clone(),vec!["project_id"]),"annotations":ro}),
        json!({"name":"list_transcripts","description":"List transcript metadata in a project","inputSchema":p(project.clone(),vec!["project_id"]),"annotations":ro}),
        json!({"name":"get_transcript","description":"Read the full text of one transcript within its project","inputSchema":p(json!({"project_id":{"type":"string"},"transcript_id":{"type":"string"}}),vec!["project_id","transcript_id"]),"annotations":ro}),
        json!({"name":"search_transcripts","description":"Search transcript IDs in a project","inputSchema":p(json!({"project_id":{"type":"string"},"query":{"type":"string"},"limit":{"type":"integer","minimum":1,"maximum":50}}),vec!["project_id","query"]),"annotations":ro}),
        json!({"name":"list_jobs","description":"Check queued downloads and transcriptions","inputSchema":p(project.clone(),vec!["project_id"]),"annotations":ro}),
        json!({"name":"create_project","description":"Create a local Scriptotar project, if app permits database writes","inputSchema":p(json!({"name":{"type":"string","maxLength":120}}),vec!["name"]),"annotations":write}),
        json!({"name":"list_directory","description":"List up to 200 entries within an app-approved directory","inputSchema":p(json!({"path":{"type":"string"}}),vec!["path"]),"annotations":ro}),
        json!({"name":"read_file","description":"Read a UTF-8 text file in app-approved directories (maximum 128 KiB)","inputSchema":p(json!({"path":{"type":"string"}}),vec!["path"]),"annotations":ro}),
        json!({"name":"write_file","description":"Write UTF-8 text to an app-approved directory (maximum 256 KiB); overwrite requires separate permission","inputSchema":p(json!({"path":{"type":"string"},"content":{"type":"string"},"overwrite":{"type":"boolean"}}),vec!["path","content"]),"annotations":json!({"readOnlyHint":false,"destructiveHint":true,"openWorldHint":false})}),
        json!({"name":"execute_command","description":"Run only executable binaries approved by Scriptotar Settings; max 15 seconds; no shell is added implicitly","inputSchema":p(json!({"program":{"type":"string"},"args":{"type":"array","items":{"type":"string"}},"cwd":{"type":"string"}}),vec!["program"]),"annotations":json!({"readOnlyHint":false,"destructiveHint":true,"openWorldHint":true})}),
        json!({"name":"search_creator","description":"Scan and save up to 200 public creator posts with Scriptotar's research engine","inputSchema":p(json!({"project_id":{"type":"string"},"profile_url":{"type":"string"},"limit":{"type":"integer","minimum":1,"maximum":200}}),vec!["project_id","profile_url"]),"annotations":json!({"readOnlyHint":false,"destructiveHint":false,"openWorldHint":true})}),
        json!({"name":"enqueue_local_media","description":"Queue an approved local media file into Scriptotar's existing transcription workflow","inputSchema":p(json!({"project_id":{"type":"string"},"path":{"type":"string"}}),vec!["project_id","path"]),"annotations":json!({"readOnlyHint":false,"destructiveHint":false,"openWorldHint":false})}),
        json!({"name":"download_video","description":"Queue a video URL for download and transcription through Scriptotar's existing job engine","inputSchema":p(json!({"project_id":{"type":"string"},"url":{"type":"string"}}),vec!["project_id","url"]),"annotations":json!({"readOnlyHint":false,"destructiveHint":false,"openWorldHint":true})}),
    ];
    // Keep tools stable across permission changes so ChatGPT can discover them. Calls are app-gated.
    tools.shrink_to_fit();
    json!({"tools":tools})
}

// Drain all output without ever accumulating more than MAX_COMMAND_OUTPUT
// bytes in memory. This avoids the previous unbounded temporary-disk files.
fn bounded_process_output<R: Read + Send + 'static>(
    mut stream: R,
) -> std::sync::mpsc::Receiver<io::Result<(Vec<u8>, bool)>> {
    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    thread::spawn(move || {
        let mut output = Vec::with_capacity(MAX_COMMAND_OUTPUT as usize);
        let mut buffer = [0u8; 8192];
        let mut truncated = false;
        let result = loop {
            match stream.read(&mut buffer) {
                Ok(0) => break Ok((output, truncated)),
                Ok(read) => {
                    let remaining = (MAX_COMMAND_OUTPUT as usize).saturating_sub(output.len());
                    output.extend_from_slice(&buffer[..read.min(remaining)]);
                    if read > remaining {
                        truncated = true;
                    }
                }
                Err(error) => break Err(error),
            }
        };
        let _ = tx.send(result);
    });
    rx
}

fn run_command(program: &Path, args: &[String], cwd: &Path) -> Result<Value, String> {
    let mut child = Command::new(program)
        .args(args)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| "approved command could not start")?;
    let out_rx = bounded_process_output(child.stdout.take().ok_or("could not capture stdout")?);
    let err_rx = bounded_process_output(child.stderr.take().ok_or("could not capture stderr")?);
    let deadline = Instant::now() + COMMAND_TIMEOUT;
    let status = loop {
        match child.try_wait().map_err(|_| "command wait failed")? {
            Some(status) => break status,
            None if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                return Err("command exceeded 15-second limit and was terminated".into());
            }
            None => thread::sleep(Duration::from_millis(50)),
        }
    };
    // A child may launch descendants that inherit stdout handles. Do not hang
    // the MCP process waiting for them to close those handles indefinitely.
    let (out, stdout_truncated) = out_rx
        .recv_timeout(Duration::from_secs(1))
        .map_err(|_| "command stdout remained open after process exit")?
        .map_err(|_| "could not read command stdout")?;
    let (err, stderr_truncated) = err_rx
        .recv_timeout(Duration::from_secs(1))
        .map_err(|_| "command stderr remained open after process exit")?
        .map_err(|_| "could not read command stderr")?;
    Ok(
        json!({"exit_code":status.code(),"stdout":String::from_utf8_lossy(&out),"stderr":String::from_utf8_lossy(&err),"output_truncated":stdout_truncated || stderr_truncated}),
    )
}

/// Keeps a bounded buffer even for attacker-supplied unbounded lines.
fn bounded_line(reader: &mut impl BufRead) -> io::Result<Option<(Vec<u8>, bool)>> {
    let mut result = Vec::new();
    let mut overflow = false;
    loop {
        let buffer = reader.fill_buf()?;
        if buffer.is_empty() {
            return if result.is_empty() && !overflow {
                Ok(None)
            } else {
                Ok(Some((result, overflow)))
            };
        }
        let end = buffer.iter().position(|b| *b == b'\n').map(|i| i + 1);
        let taken = end.unwrap_or(buffer.len());
        if result.len() + taken > MAX_REQUEST {
            overflow = true;
        }
        if !overflow {
            result.extend_from_slice(&buffer[..taken]);
        }
        reader.consume(taken);
        if end.is_some() {
            return Ok(Some((result, overflow)));
        }
    }
}

pub fn run_stdio_from_env() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::var_os("SCRIPTOTAR_DB_PATH")
        .ok_or("SCRIPTOTAR_DB_PATH is required; explicitly select your local database")?;
    let server = Server::new(Path::new(&path))?;
    let stdin = io::stdin();
    let mut input = stdin.lock();
    let mut output = io::stdout().lock();
    while let Some((line, too_long)) = bounded_line(&mut input)? {
        if too_long {
            writeln!(
                output,
                "{}",
                rpc_error(&Value::Null, -32600, "request exceeds 1 MiB limit")
            )?;
            output.flush()?;
            continue;
        }
        let request: Value = match serde_json::from_slice(&line) {
            Ok(value) => value,
            Err(_) => {
                writeln!(
                    output,
                    "{}",
                    rpc_error(&Value::Null, -32700, "invalid JSON")
                )?;
                output.flush()?;
                continue;
            }
        };
        if let Some(reply) = server.response(&request) {
            writeln!(output, "{reply}")?;
            output.flush()?;
        }
    }
    Ok(())
}

/// The remote tunnel terminates at an authenticated, localhost-only Streamable HTTP endpoint.
pub fn run_http_from_env() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::var_os("SCRIPTOTAR_DB_PATH")
        .ok_or("SCRIPTOTAR_DB_PATH is required for MCP HTTP")?;
    let server = Server::new(Path::new(&path))?;
    let policy = load_permissions(&server.data_dir)?;
    if !policy.enabled || policy.http_token.is_empty() {
        return Err(
            "Enable MCP in Scriptotar Settings and save a token before starting HTTP".into(),
        );
    }
    let port = std::env::var("SCRIPTOTAR_MCP_PORT")
        .unwrap_or_else(|_| "39761".into())
        .parse::<u16>()?;
    if port < 1024 {
        return Err("MCP HTTP port must be above 1023".into());
    }
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    runtime.block_on(crate::http::serve(server, port))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn jsonrpc_errors_have_correct_codes() {
        let e = rpc_error(&json!(4), -32601, "missing");
        assert_eq!(e["error"]["code"], -32601);
        assert_eq!(e["id"], 4);
    }
    #[test]
    fn tool_catalog_advertises_all_gated_capabilities() {
        let tools = tool_catalog()["tools"].as_array().unwrap().clone();
        assert_eq!(tools.len(), 14);
    }
}
