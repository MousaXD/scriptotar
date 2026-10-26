//! CoS-style MCP connection manager. The UI only chooses how to connect; tool
//! authorizations always live in the app's independent fail-closed policy.
use std::{
    fs,
    io::{BufRead, BufReader, Read, Write},
    net::TcpStream,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

use serde::Serialize;
use tauri::Manager;
use url::Url;

use super::{start_gateway_for_data, stop_gateway, McpGatewayState, MCP_HTTP_PORT};
use scriptotar_mcp::{load_permissions, new_token, save_permissions, McpPermissions};

const CONNECTION_TIMEOUT: Duration = Duration::from_secs(60);
const LOCAL_ROOT: &str = "http://127.0.0.1:39761";
const TUNNEL_PROFILE: &str = "scriptotar";
const KEYRING_SERVICE: &str = "io.github.mousaxd.scriptotar.mcp";

/// Authentication secrets are backend-owned, even though the settings form
/// sends back a full permissions object. A stale tab must never be able to
/// restore a revoked public URL or replace the local bearer token.
pub(crate) fn preserve_authentication_secrets(next: &mut McpPermissions, current: &McpPermissions) {
    next.http_token.clone_from(&current.http_token);
    next.url_token.clone_from(&current.url_token);
}

#[derive(Default)]
pub(crate) struct McpConnectionState {
    active: Mutex<Option<ManagedConnection>>,
}

struct ManagedConnection {
    mode: &'static str,
    started: Instant,
    child: Option<Child>,
    report: Arc<Mutex<ConnectionReport>>,
    local_url: String,
    tunnel_id: Option<String>,
    _temp: Option<tempfile::TempDir>,
}

#[derive(Default)]
struct ConnectionReport {
    public_url: Option<String>,
    health_file: Option<PathBuf>,
    detail: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct McpConnectionStatus {
    pub mode: String,
    pub state: String,
    pub detail: String,
    pub public_url: Option<String>,
    pub local_url: Option<String>,
    pub tunnel_id: Option<String>,
    pub cloudflare_available: bool,
    pub openai_available: bool,
}

impl Drop for McpConnectionState {
    fn drop(&mut self) {
        if let Ok(active) = self.active.get_mut() {
            if let Some(mut managed) = active.take() {
                if let Some(child) = managed.child.as_mut() {
                    let _ = child.kill();
                    let _ = child.wait();
                }
            }
        }
    }
}

pub(crate) fn valid_tunnel_id(tunnel_id: &str) -> bool {
    tunnel_id.strip_prefix("tunnel_").is_some_and(|id| {
        id.len() == 32
            && id
                .bytes()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
    })
}

pub(crate) fn validated_origin(raw: &str) -> Result<String, String> {
    let url = Url::parse(raw.trim())
        .map_err(|_| "Enter your Cloudflare HTTPS hostname, e.g. https://mcp.example.com")?;
    if url.scheme() != "https"
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || url.path() != "/"
        || url.port().is_some()
    {
        return Err(
            "Use only a public HTTPS origin without a path, credentials, port or query".into(),
        );
    }
    let host = url.host_str().unwrap();
    if !host.contains('.')
        || host == "localhost"
        || host.ends_with(".localhost")
        || host.parse::<std::net::IpAddr>().is_ok()
    {
        return Err("The custom URL must use a public HTTPS hostname".into());
    }
    Ok(format!("https://{host}"))
}

fn quick_tunnel_host(line: &str) -> Option<String> {
    // Cloudflared reports a temporary hostname on stderr or stdout. Parse each
    // candidate URL instead of matching substrings: an attacker-controlled
    // hostname such as foo.trycloudflare.com@evil.com must never be accepted.
    for word in line.split_whitespace() {
        let Some(pos) = word.find("https://") else {
            continue;
        };
        let candidate = word[pos..].trim_end_matches(['\"', '\'', ',', ')', ']', ';']);
        let Ok(url) = Url::parse(candidate) else {
            continue;
        };
        let Some(host) = url.host_str() else { continue };
        let Some(label) = host.strip_suffix(".trycloudflare.com") else {
            continue;
        };
        if url.scheme() == "https"
            && !label.is_empty()
            && label
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
            && url.username().is_empty()
            && url.password().is_none()
            && url.port().is_none()
            && url.path() == "/"
            && url.query().is_none()
            && url.fragment().is_none()
        {
            return Some(format!("https://{host}"));
        }
    }
    None
}

fn built_in_executable(app: &tauri::AppHandle, filename: &str) -> Option<PathBuf> {
    let extension = if cfg!(windows) { ".exe" } else { "" };
    let binary = format!("{filename}{extension}");
    if let Ok(root) = app.path().resource_dir() {
        let bundled = root.join("mcp-tools").join(&binary);
        if bundled.is_file() {
            return Some(bundled);
        }
    }
    // App releases prefer the audited bundle. PATH fallback supports dev builds.
    if let Some(paths) = std::env::var_os("PATH") {
        for path in std::env::split_paths(&paths) {
            let candidate = path.join(&binary);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

fn safe_child_environment(command: &mut Command) {
    // A GUI launched from a developer terminal can inherit API credentials.
    // Neither cloudflared nor the tunnel client needs unrelated parent secrets.
    let names = [
        "PATH",
        "HOME",
        "USERPROFILE",
        "SYSTEMROOT",
        "WINDIR",
        "TMPDIR",
        "TEMP",
        "TMP",
        "LANG",
        "SSL_CERT_FILE",
        "SSL_CERT_DIR",
        "HTTPS_PROXY",
        "HTTP_PROXY",
        "NO_PROXY",
        "APPDATA",
        "LOCALAPPDATA",
        "XDG_RUNTIME_DIR",
        "DBUS_SESSION_BUS_ADDRESS",
    ];
    let safe = names
        .into_iter()
        .filter_map(|name| std::env::var_os(name).map(|value| (name, value)))
        .collect::<Vec<_>>();
    command.env_clear();
    for (name, value) in safe {
        command.env(name, value);
    }
}

/// stdout and stderr are different Rust types, so consume each through the
/// same generic reader rather than attempting to put them in a homogeneous
/// array. Both pipes must be drained to prevent cloudflared from blocking.
fn read_cloudflared_output<R: Read + Send + 'static>(
    stream: R,
    report: Arc<Mutex<ConnectionReport>>,
    secret_path: String,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        for line in BufReader::new(stream).lines() {
            let Ok(line) = line else { break };
            if let Some(host) = quick_tunnel_host(&line) {
                if let Ok(mut state) = report.lock() {
                    state.public_url = Some(format!("{host}{secret_path}"));
                    state.detail =
                        "Cloudflare URL ready. Add it to ChatGPT, then verify a tool call.".into();
                }
            }
        }
    })
}

fn launch_cloudflared(
    binary: &Path,
    report: Arc<Mutex<ConnectionReport>>,
    local_url: &str,
) -> Result<Child, String> {
    let mut command = Command::new(binary);
    command.args([
        "tunnel",
        "--no-autoupdate",
        "--url",
        LOCAL_ROOT,
        "--http-host-header",
        "127.0.0.1:39761",
    ]);
    safe_child_environment(&mut command);
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| {
            "Cloudflare could not start. Verify the packaged cloudflared executable.".to_owned()
        })?;
    let path = local_url
        .strip_prefix(LOCAL_ROOT)
        .ok_or("invalid local URL")?
        .to_owned();
    if let Some(stdout) = child.stdout.take() {
        let _ = read_cloudflared_output(stdout, Arc::clone(&report), path.clone());
    }
    if let Some(stderr) = child.stderr.take() {
        let _ = read_cloudflared_output(stderr, report, path);
    }
    Ok(child)
}

fn keyring_entry() -> Result<keyring::Entry, String> {
    keyring::Entry::new(KEYRING_SERVICE, TUNNEL_PROFILE).map_err(|_| {
        "The system keyring is unavailable. You can connect without saving the API key.".to_owned()
    })
}

fn tunnel_key(passed: &str, save: bool) -> Result<String, String> {
    if !passed.is_empty() {
        if passed.len() > 2048
            || !passed.starts_with("sk-")
            || passed.chars().any(char::is_whitespace)
        {
            return Err("Enter a valid OpenAI restricted API key".into());
        }
        if save {
            keyring_entry()?.set_password(passed).map_err(|_| {
                "Cannot save the key to your OS keyring. Uncheck Remember key and try again."
                    .to_owned()
            })?;
        }
        return Ok(passed.into());
    }
    keyring_entry()?
        .get_password()
        .map_err(|_| "Enter a runtime API key or unlock your OS keyring".to_owned())
}

fn healthy_local_mcp(local_url: &str) -> bool {
    let Ok(url) = Url::parse(local_url) else {
        return false;
    };
    if url.host_str() != Some("127.0.0.1") || url.port() != Some(MCP_HTTP_PORT) {
        return false;
    }
    let Ok(mut stream) = TcpStream::connect_timeout(
        &format!("127.0.0.1:{}", MCP_HTTP_PORT).parse().unwrap(),
        Duration::from_millis(400),
    ) else {
        return false;
    };
    let _ = stream.set_read_timeout(Some(Duration::from_millis(600)));
    let request = br#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#;
    let header = format!(
        "POST {} HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nAccept: application/json\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        url.path(), MCP_HTTP_PORT, request.len()
    );
    if stream
        .write_all(header.as_bytes())
        .and_then(|_| stream.write_all(request))
        .is_err()
    {
        return false;
    }
    let mut body = Vec::new();
    stream.take(64 * 1024).read_to_end(&mut body).is_ok()
        && body.starts_with(b"HTTP/1.1 200")
        && body.windows(b"\"tools\"".len()).any(|w| w == b"\"tools\"")
}

fn http_health_ready(file: &Path) -> bool {
    let Ok(raw) = fs::read_to_string(file) else {
        return false;
    };
    let Ok(url) = Url::parse(raw.trim()) else {
        return false;
    };
    if url.scheme() != "http"
        || !["127.0.0.1", "localhost"].contains(&url.host_str().unwrap_or_default())
    {
        return false;
    }
    let Some(port) = url.port() else { return false };
    let Ok(mut stream) = TcpStream::connect_timeout(
        &format!("127.0.0.1:{port}").parse().unwrap(),
        Duration::from_millis(400),
    ) else {
        return false;
    };
    let _ = stream.set_read_timeout(Some(Duration::from_millis(700)));
    if stream
        .write_all(b"GET /readyz HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
        .is_err()
    {
        return false;
    }
    let mut result = [0u8; 80];
    match stream.read(&mut result) {
        Ok(n) => {
            String::from_utf8_lossy(&result[..n]).starts_with("HTTP/1.1 200")
                || String::from_utf8_lossy(&result[..n]).starts_with("HTTP/1.0 200")
        }
        Err(_) => false,
    }
}

impl McpConnectionState {
    pub(crate) fn stop(&self, data_dir: &Path, gateway: &McpGatewayState) -> Result<(), String> {
        let mut active = self
            .active
            .lock()
            .map_err(|_| "MCP connection state unavailable")?;
        if let Some(mut connected) = active.take() {
            if let Some(child) = connected.child.as_mut() {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
        stop_gateway(gateway)?;
        // Stopping a shared HTTPS tunnel revokes its old URL secret immediately.
        let mut policy = load_permissions(data_dir)?;
        if !policy.url_token.is_empty() {
            policy.url_token = new_token();
            save_permissions(data_dir, policy)?;
        }
        Ok(())
    }

    pub(crate) fn status(&self, app: &tauri::AppHandle) -> McpConnectionStatus {
        let cloudflare_available = built_in_executable(app, "cloudflared").is_some();
        let openai_available = built_in_executable(app, "tunnel-client").is_some();
        let result = || McpConnectionStatus {
            mode: "none".into(),
            state: "disconnected".into(),
            detail: "Choose a connection method to begin.".into(),
            public_url: None,
            local_url: None,
            tunnel_id: None,
            cloudflare_available,
            openai_available,
        };
        let Ok(mut state) = self.active.lock() else {
            return result();
        };
        let Some(connection) = state.as_mut() else {
            return result();
        };
        if let Some(child) = connection.child.as_mut() {
            match child.try_wait() {
                Ok(Some(_)) => {
                    let mode = connection.mode;
                    *state = None;
                    return McpConnectionStatus {
                        mode: mode.into(),
                        state: "error".into(),
                        detail: "The tunnel process exited. Reconnect from Scriptotar.".into(),
                        ..result()
                    };
                }
                Err(_) => {
                    return McpConnectionStatus {
                        state: "error".into(),
                        detail: "Unable to inspect the tunnel process.".into(),
                        ..result()
                    }
                }
                _ => {}
            }
        }
        let report = connection.report.lock().ok();
        let public_url = report.as_ref().and_then(|r| r.public_url.clone());
        let detail = report
            .as_ref()
            .map(|r| r.detail.clone())
            .unwrap_or_default();
        let ready = match connection.mode {
            "cloudflare" => public_url.is_some() && healthy_local_mcp(&connection.local_url),
            "openai" => report
                .as_ref()
                .and_then(|r| r.health_file.as_deref())
                .is_some_and(http_health_ready),
            "manual" => healthy_local_mcp(&connection.local_url),
            _ => false,
        };
        let state_name = if ready {
            "ready"
        } else if connection.started.elapsed() > CONNECTION_TIMEOUT {
            "error"
        } else {
            "connecting"
        };
        McpConnectionStatus {
            mode: connection.mode.into(),
            state: state_name.into(),
            detail: if ready {
                detail
            } else if state_name == "error" {
                "Connection did not become ready. Review your tunnel credentials and local network."
                    .into()
            } else {
                "Establishing a connection...".into()
            },
            public_url,
            local_url: Some(connection.local_url.clone()),
            tunnel_id: connection.tunnel_id.clone(),
            cloudflare_available,
            openai_available,
        }
    }

    fn begin(&self, data_dir: &Path, gateway: &McpGatewayState) -> Result<String, String> {
        self.stop(data_dir, gateway)?;
        let mut policy = load_permissions(data_dir)?;
        if !policy.enabled {
            return Err("First enable MCP in Scriptotar Settings and save your permissions".into());
        }
        policy.url_token = new_token();
        let policy = save_permissions(data_dir, policy)?;
        start_gateway_for_data(data_dir, gateway)?;
        Ok(format!("{LOCAL_ROOT}/mcp/p/{}", policy.url_token))
    }

    pub(crate) fn connect_cloudflare(
        &self,
        app: &tauri::AppHandle,
        data_dir: &Path,
        gateway: &McpGatewayState,
    ) -> Result<McpConnectionStatus, String> {
        let binary = built_in_executable(app, "cloudflared")
            .ok_or("Cloudflare is not bundled. Install the verified cloudflared binary to use this option.")?;
        let local_url = self.begin(data_dir, gateway)?;
        let report = Arc::new(Mutex::new(ConnectionReport {
            detail: "Starting Cloudflare Quick Tunnel...".into(),
            ..Default::default()
        }));
        let child = match launch_cloudflared(&binary, Arc::clone(&report), &local_url) {
            Ok(child) => child,
            Err(e) => {
                let _ = self.stop(data_dir, gateway);
                return Err(e);
            }
        };
        *self
            .active
            .lock()
            .map_err(|_| "MCP connection state unavailable")? = Some(ManagedConnection {
            mode: "cloudflare",
            started: Instant::now(),
            child: Some(child),
            report,
            local_url,
            tunnel_id: None,
            _temp: None,
        });
        Ok(self.status(app))
    }

    pub(crate) fn connect_manual(
        &self,
        app: &tauri::AppHandle,
        data_dir: &Path,
        gateway: &McpGatewayState,
        origin: &str,
    ) -> Result<McpConnectionStatus, String> {
        let hostname = validated_origin(origin)?;
        let local_url = self.begin(data_dir, gateway)?;
        let suffix = local_url.strip_prefix(LOCAL_ROOT).unwrap_or_default();
        let report = Arc::new(Mutex::new(ConnectionReport {
            public_url: Some(format!("{hostname}{suffix}")),
            detail: "Point your Cloudflare named tunnel at 127.0.0.1:39761, then add the HTTPS URL in ChatGPT.".into(),
            ..Default::default()
        }));
        *self
            .active
            .lock()
            .map_err(|_| "MCP connection state unavailable")? = Some(ManagedConnection {
            mode: "manual",
            started: Instant::now(),
            child: None,
            report,
            local_url,
            tunnel_id: None,
            _temp: None,
        });
        Ok(self.status(app))
    }

    pub(crate) fn connect_openai(
        &self,
        app: &tauri::AppHandle,
        data_dir: &Path,
        gateway: &McpGatewayState,
        id: &str,
        api_key: &str,
        remember: bool,
    ) -> Result<McpConnectionStatus, String> {
        if !valid_tunnel_id(id) {
            return Err(
                "Enter a tunnel ID like tunnel_ followed by 32 lowercase hex characters".into(),
            );
        }
        let binary = built_in_executable(app, "tunnel-client").ok_or(
            "OpenAI tunnel-client is not bundled. Install the verified client to use this option.",
        )?;
        let secret = tunnel_key(api_key, remember)?;
        let local_url = self.begin(data_dir, gateway)?;
        let temp = tempfile::tempdir_in(data_dir)
            .map_err(|_| "Could not create a private tunnel working directory")?;
        let health_file = temp.path().join("health.url");
        let mut command = Command::new(binary);
        command
            .args([
                "run",
                "--control-plane.tunnel-id",
                id,
                "--health.listen-addr",
                "127.0.0.1:0",
                "--health.url-file",
            ])
            .arg(&health_file)
            .arg("--log.level")
            .arg("warn");
        safe_child_environment(&mut command);
        command
            .env("CONTROL_PLANE_API_KEY", secret)
            .env("MCP_SERVER_URL", format!("url={local_url},channel=main"));
        let child = match command
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(child) => child,
            Err(_) => {
                let _ = self.stop(data_dir, gateway);
                return Err("OpenAI tunnel-client could not start".into());
            }
        };
        let report = Arc::new(Mutex::new(ConnectionReport {
            health_file: Some(health_file),
            detail:
                "OpenAI tunnel is ready. Select this tunnel ID in ChatGPT's custom MCP connector."
                    .into(),
            ..Default::default()
        }));
        *self
            .active
            .lock()
            .map_err(|_| "MCP connection state unavailable")? = Some(ManagedConnection {
            mode: "openai",
            started: Instant::now(),
            child: Some(child),
            report,
            local_url,
            tunnel_id: Some(id.into()),
            _temp: Some(temp),
        });
        Ok(self.status(app))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stale_settings_cannot_resurrect_rotated_url_secrets() {
        let current = McpPermissions {
            enabled: true,
            http_token: "a".repeat(64),
            url_token: "b".repeat(64),
            ..Default::default()
        };
        let mut edited = McpPermissions {
            read_data: true,
            http_token: "c".repeat(64),
            url_token: "d".repeat(64),
            ..Default::default()
        };
        preserve_authentication_secrets(&mut edited, &current);
        assert_eq!(edited.http_token, current.http_token);
        assert_eq!(edited.url_token, current.url_token);
        assert!(
            edited.read_data,
            "Ordinary permission edits must still be honored"
        );
    }
    #[test]
    fn reader_handles_stdout_and_stderr_without_losing_secret_path() {
        let report = Arc::new(Mutex::new(ConnectionReport::default()));
        let stdout = std::io::Cursor::new(b"INF some unrelated startup message\n".to_vec());
        let stderr = std::io::Cursor::new(
            b"INF Quick Tunnel https://valid-name.trycloudflare.com\n".to_vec(),
        );
        let secret_path = "/mcp/p/".to_owned() + &"f".repeat(64);
        let stdout_reader =
            read_cloudflared_output(stdout, Arc::clone(&report), secret_path.clone());
        let stderr_reader =
            read_cloudflared_output(stderr, Arc::clone(&report), secret_path.clone());
        stdout_reader.join().expect("stdout reader must finish");
        stderr_reader.join().expect("stderr reader must finish");
        assert_eq!(
            report.lock().unwrap().public_url.as_deref(),
            Some(format!("https://valid-name.trycloudflare.com{secret_path}").as_str())
        );
    }
    #[test]
    fn only_valid_cloudflare_hostname_is_accepted() {
        assert_eq!(
            quick_tunnel_host("INF Requesting https://nice-random.trycloudflare.com"),
            Some("https://nice-random.trycloudflare.com".into())
        );
        for value in [
            "https://evil.trycloudflare.com.attacker.test",
            "http://foo.trycloudflare.com",
            "https://foo.trycloudflare.com/evil",
            "https://foo.trycloudflare.com@evil.com",
        ] {
            assert!(quick_tunnel_host(value).is_none(), "{value}");
        }
    }
    #[test]
    fn manual_hostname_rejects_local_or_url_injection() {
        assert_eq!(
            validated_origin("https://mcp.example.com/").unwrap(),
            "https://mcp.example.com"
        );
        for url in [
            "http://mcp.example.com",
            "https://localhost",
            "https://127.0.0.1",
            "https://user:pass@evil.com",
            "https://mcp.example.com/mcp/abc",
            "https://mcp.example.com/?secret=foo",
        ] {
            assert!(validated_origin(url).is_err(), "{url}");
        }
    }
    #[test]
    fn quick_url_parser_rejects_other_hosts() {
        assert!(quick_tunnel_host("ERR https://evil.com.trycloudflare.com.evil.test").is_none());
        assert!(quick_tunnel_host("ERR https://evil.example.com").is_none());
    }
    #[test]
    fn openai_tunnel_id_is_strict() {
        assert!(valid_tunnel_id(&format!("tunnel_{}", "a".repeat(32))));
        assert!(!valid_tunnel_id("tunnel_BAD"));
        assert!(!valid_tunnel_id(&format!("tunnel_{}", "g".repeat(32))));
    }
}
