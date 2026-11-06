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

use super::{
    start_gateway_for_data, stop_gateway, McpGatewayState, OperationalState, MCP_HTTP_PORT,
};
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
    // A connection transition spans URL rotation, child spawn and activation.
    // Serializing only the active-state assignment can orphan a tunnel.
    transition: Mutex<()>,
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
        // Drop runs during teardown even if a previous transition panicked.
        // Recover a poisoned lock only for terminating/reaping an owned child.
        let active = self
            .active
            .get_mut()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(mut managed) = active.take() {
            if let Some(child) = managed.child.as_mut() {
                let _ = child.kill();
                let _ = child.wait();
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
    // Only development builds may use arbitrary binaries from PATH.
    // Production must fail closed when the reviewed bundle is absent: otherwise
    // an unrelated PATH entry could receive the restricted OpenAI API key.
    if !cfg!(debug_assertions) {
        return None;
    }
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

/// A failed or disconnected managed tunnel must not leave a live URL
/// credential or HTTP gateway behind.
fn revoke_public_gateway(data_dir: &Path, gateway: &McpGatewayState) -> Result<(), String> {
    let stop_result = stop_gateway(gateway);
    let mut policy = load_permissions(data_dir)?;
    if !policy.url_token.is_empty() {
        policy.url_token = new_token();
        save_permissions(data_dir, policy)?;
    }
    stop_result
}

/// Preserve both the original startup failure and any failure to invalidate
/// the public URL. Do not silently claim cleanup succeeded when persistence
/// failed, even when the local gateway was stopped.
fn failed_gateway_start(error: String, data_dir: &Path, gateway: &McpGatewayState) -> String {
    match revoke_public_gateway(data_dir, gateway) {
        Ok(()) => error,
        Err(revocation_error) => {
            format!("{error}; URL revocation or gateway cleanup failed: {revocation_error}")
        }
    }
}

impl McpConnectionState {
    pub(crate) fn save_app_permissions(
        &self,
        data_dir: &Path,
        gateway: &McpGatewayState,
        mut permissions: McpPermissions,
    ) -> Result<McpPermissions, String> {
        // Read and persist the live authentication secrets atomically with
        // managed Connect/Disconnect. A stale settings form must not restore a
        // URL secret after a concurrent disconnect revokes it.
        let _transition = self
            .transition
            .lock()
            .map_err(|_| "MCP connection transition unavailable")?;
        let current = load_permissions(data_dir)?;
        preserve_authentication_secrets(&mut permissions, &current);
        let saved = save_permissions(data_dir, permissions)?;
        if !saved.enabled {
            self.stop_inner(data_dir, gateway)?;
            return load_permissions(data_dir);
        }
        Ok(saved)
    }

    pub(crate) fn start_local_gateway(
        &self,
        data_dir: &Path,
        gateway: &McpGatewayState,
    ) -> Result<super::McpGatewayStatus, String> {
        let _transition = self
            .transition
            .lock()
            .map_err(|_| "MCP connection transition unavailable")?;
        start_gateway_for_data(data_dir, gateway)
    }

    pub(crate) fn stop(&self, data_dir: &Path, gateway: &McpGatewayState) -> Result<(), String> {
        // Disconnect is an emergency revocation operation. A panic that
        // poisoned the transition mutex must not prevent killing an owned
        // tunnel, stopping the local gateway, and rotating public credentials.
        let _transition = self
            .transition
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        self.stop_inner(data_dir, gateway)
    }

    // The caller holds the transition lock, so it cannot recurse into stop().
    fn stop_inner(&self, data_dir: &Path, gateway: &McpGatewayState) -> Result<(), String> {
        // Recover poisoned ownership only to shut down and revoke; Connect and
        // settings mutations still refuse a poisoned transition.
        let mut active = self
            .active
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(mut connected) = active.take() {
            if let Some(child) = connected.child.as_mut() {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
        // Rotate even if stopping the gateway reported an error. A live old
        // URL must never survive a partially failed disconnect attempt.
        revoke_public_gateway(data_dir, gateway)
    }

    fn activate_connection(
        &self,
        data_dir: &Path,
        gateway: &McpGatewayState,
        mut connection: ManagedConnection,
    ) -> Result<(), String> {
        match self.active.lock() {
            Ok(mut active) => {
                *active = Some(connection);
                Ok(())
            }
            Err(_) => {
                // Do not silently detach an owned process on a poisoned mutex.
                if let Some(child) = connection.child.as_mut() {
                    let _ = child.kill();
                    let _ = child.wait();
                }
                let _ = revoke_public_gateway(data_dir, gateway);
                Err("MCP connection state unavailable; tunnel was stopped".into())
            }
        }
    }

    pub(crate) fn status(&self, app: &tauri::AppHandle) -> McpConnectionStatus {
        let operational = app.try_state::<OperationalState>();
        let gateway = app.try_state::<McpGatewayState>();
        self.status_with_resources(
            operational.as_ref().map(|state| state.data_dir.as_path()),
            gateway.as_deref(),
            built_in_executable(app, "cloudflared").is_some(),
            built_in_executable(app, "tunnel-client").is_some(),
        )
    }

    /// An unavailable connection mutex must never be reported as a harmless
    /// disconnection while a previously published gateway might still run.
    fn emergency_status(
        &self,
        data_dir: Option<&Path>,
        gateway: Option<&McpGatewayState>,
        cloudflare_available: bool,
        openai_available: bool,
    ) -> McpConnectionStatus {
        let detail = match (data_dir, gateway) {
            (Some(data_dir), Some(gateway)) => match self.stop_inner(data_dir, gateway) {
                Ok(()) => "MCP connection state failed. Managed tunnel stopped, local gateway closed and URL revoked; restart Scriptotar.".to_owned(),
                Err(_) => "MCP connection state failed and automatic URL revocation could not be verified. Disable MCP locally and restart Scriptotar.".to_owned(),
            },
            _ => "MCP connection state failed and gateway resources were unavailable for revocation. Restart Scriptotar and verify MCP is disabled.".to_owned(),
        };
        McpConnectionStatus {
            mode: "none".into(),
            state: "error".into(),
            detail,
            public_url: None,
            local_url: None,
            tunnel_id: None,
            cloudflare_available,
            openai_available,
        }
    }

    // This is the public polling lock boundary. Unit tests exercise this same
    // path with fixture resources, without needing a graphical Tauri app.
    fn status_with_resources(
        &self,
        data_dir: Option<&Path>,
        gateway: Option<&McpGatewayState>,
        cloudflare_available: bool,
        openai_available: bool,
    ) -> McpConnectionStatus {
        let _transition = match self.transition.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                // Hold the recovered transition guard while emergency cleanup
                // runs, preserving the same lock order even in a panic path.
                let _guard = poisoned.into_inner();
                return self.emergency_status(
                    data_dir,
                    gateway,
                    cloudflare_available,
                    openai_available,
                );
            }
        };
        self.status_inner_with_resources(data_dir, gateway, cloudflare_available, openai_available)
    }

    // Called by Connect while already holding transition: never reacquire it.
    fn status_inner(&self, app: &tauri::AppHandle) -> McpConnectionStatus {
        let operational = app.try_state::<OperationalState>();
        let gateway = app.try_state::<McpGatewayState>();
        self.status_inner_with_resources(
            operational.as_ref().map(|state| state.data_dir.as_path()),
            gateway.as_deref(),
            built_in_executable(app, "cloudflared").is_some(),
            built_in_executable(app, "tunnel-client").is_some(),
        )
    }

    // The caller holds the transition lock, including on auto-revocation.
    fn status_inner_with_resources(
        &self,
        data_dir: Option<&Path>,
        gateway: Option<&McpGatewayState>,
        cloudflare_available: bool,
        openai_available: bool,
    ) -> McpConnectionStatus {
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
        let mut state = match self.active.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                // A poisoned active mutex may still own a live tunnel.
                // Release its guard before stop_inner reacquires it to reap.
                drop(poisoned.into_inner());
                return self.emergency_status(
                    data_dir,
                    gateway,
                    cloudflare_available,
                    openai_available,
                );
            }
        };
        let Some(connection) = state.as_mut() else {
            return result();
        };
        if let Some(child) = connection.child.as_mut() {
            let failed = match child.try_wait() {
                Ok(Some(_)) => Some("The tunnel process exited."),
                Err(_) => Some("Unable to inspect the tunnel process."),
                Ok(None) => None,
            };
            if let Some(reason) = failed {
                let mode = connection.mode;
                if let Some(mut expired) = state.take() {
                    if let Some(child) = expired.child.as_mut() {
                        // try_wait() can also fail while the process is alive.
                        // Never drop an owned child without attempting cleanup.
                        let _ = child.kill();
                        let _ = child.wait();
                    }
                }
                // Hold the connection-state lock while revoking to prevent a
                // competing Connect from racing a delayed stale cleanup.
                let revoked = match (data_dir, gateway) {
                    (Some(data_dir), Some(gateway)) => revoke_public_gateway(data_dir, gateway),
                    _ => Err("MCP gateway state unavailable".into()),
                };
                return McpConnectionStatus {
                    mode: mode.into(),
                    state: "error".into(),
                    detail: if revoked.is_ok() {
                        format!("{reason} Local gateway stopped and URL revoked; reconnect from Scriptotar.")
                    } else {
                        format!("{reason} Automatic revocation failed; disable MCP permissions immediately.")
                    },
                    ..result()
                };
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
        self.stop_inner(data_dir, gateway)?;
        let mut policy = load_permissions(data_dir)?;
        if !policy.enabled {
            return Err("First enable MCP in Scriptotar Settings and save your permissions".into());
        }
        policy.url_token = new_token();
        let policy = save_permissions(data_dir, policy)?;
        if let Err(error) = start_gateway_for_data(data_dir, gateway) {
            // Failure must not leave a valid public credential or child.
            return Err(failed_gateway_start(error, data_dir, gateway));
        }
        Ok(format!("{LOCAL_ROOT}/mcp/p/{}", policy.url_token))
    }

    pub(crate) fn connect_cloudflare(
        &self,
        app: &tauri::AppHandle,
        data_dir: &Path,
        gateway: &McpGatewayState,
    ) -> Result<McpConnectionStatus, String> {
        let _transition = self
            .transition
            .lock()
            .map_err(|_| "MCP connection transition unavailable")?;
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
                let _ = self.stop_inner(data_dir, gateway);
                return Err(e);
            }
        };
        self.activate_connection(
            data_dir,
            gateway,
            ManagedConnection {
                mode: "cloudflare",
                started: Instant::now(),
                child: Some(child),
                report,
                local_url,
                tunnel_id: None,
                _temp: None,
            },
        )?;
        Ok(self.status_inner(app))
    }

    pub(crate) fn connect_manual(
        &self,
        app: &tauri::AppHandle,
        data_dir: &Path,
        gateway: &McpGatewayState,
        origin: &str,
    ) -> Result<McpConnectionStatus, String> {
        let hostname = validated_origin(origin)?;
        let _transition = self
            .transition
            .lock()
            .map_err(|_| "MCP connection transition unavailable")?;
        let local_url = self.begin(data_dir, gateway)?;
        let suffix = local_url.strip_prefix(LOCAL_ROOT).unwrap_or_default();
        let report = Arc::new(Mutex::new(ConnectionReport {
            public_url: Some(format!("{hostname}{suffix}")),
            detail: "Point your Cloudflare named tunnel at 127.0.0.1:39761, then add the HTTPS URL in ChatGPT.".into(),
            ..Default::default()
        }));
        self.activate_connection(
            data_dir,
            gateway,
            ManagedConnection {
                mode: "manual",
                started: Instant::now(),
                child: None,
                report,
                local_url,
                tunnel_id: None,
                _temp: None,
            },
        )?;
        Ok(self.status_inner(app))
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
        let _transition = self
            .transition
            .lock()
            .map_err(|_| "MCP connection transition unavailable")?;
        let binary = built_in_executable(app, "tunnel-client").ok_or(
            "OpenAI tunnel-client is not bundled. Install the verified client to use this option.",
        )?;
        let secret = tunnel_key(api_key, remember)?;
        let local_url = self.begin(data_dir, gateway)?;
        let temp = match tempfile::tempdir_in(data_dir) {
            Ok(temp) => temp,
            Err(_) => {
                let _ = self.stop_inner(data_dir, gateway);
                return Err("Could not create a private tunnel working directory".into());
            }
        };
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
                let _ = self.stop_inner(data_dir, gateway);
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
        self.activate_connection(
            data_dir,
            gateway,
            ManagedConnection {
                mode: "openai",
                started: Instant::now(),
                child: Some(child),
                report,
                local_url,
                tunnel_id: Some(id.into()),
                _temp: Some(temp),
            },
        )?;
        Ok(self.status_inner(app))
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
    fn public_gateway_revocation_rotates_path_token_even_if_no_child_is_running() {
        let dir = tempfile::tempdir().unwrap();
        let initial = save_permissions(
            dir.path(),
            McpPermissions {
                enabled: true,
                url_token: "a".repeat(64),
                ..Default::default()
            },
        )
        .unwrap();
        let gateway = McpGatewayState::default();
        revoke_public_gateway(dir.path(), &gateway).unwrap();
        let current = load_permissions(dir.path()).unwrap();
        assert_ne!(initial.url_token, current.url_token);
        assert!(!current.url_token.is_empty());
    }

    #[test]
    fn disconnect_waits_for_transition_then_revokes_url() {
        let dir = tempfile::tempdir().unwrap();
        let initial = save_permissions(
            dir.path(),
            McpPermissions {
                enabled: true,
                url_token: "a".repeat(64),
                ..Default::default()
            },
        )
        .unwrap();
        let state = Arc::new(McpConnectionState::default());
        let gateway = Arc::new(McpGatewayState::default());
        let transition = state.transition.lock().unwrap();
        let (started_tx, started_rx) = std::sync::mpsc::channel();
        let (done_tx, done_rx) = std::sync::mpsc::channel();
        let data_dir = dir.path().to_path_buf();
        let worker = {
            let state = Arc::clone(&state);
            let gateway = Arc::clone(&gateway);
            thread::spawn(move || {
                started_tx.send(()).unwrap();
                done_tx.send(state.stop(&data_dir, &gateway)).unwrap();
            })
        };
        started_rx.recv().unwrap();
        assert!(
            done_rx.try_recv().is_err(),
            "disconnect must not bypass an active transition"
        );
        drop(transition);
        done_rx
            .recv_timeout(Duration::from_secs(10))
            .unwrap()
            .unwrap();
        worker.join().unwrap();
        let current = load_permissions(dir.path()).unwrap();
        assert_ne!(initial.url_token, current.url_token);
    }

    #[test]
    fn stale_settings_save_never_restores_rotated_public_token() {
        let dir = tempfile::tempdir().unwrap();
        let initial = save_permissions(
            dir.path(),
            McpPermissions {
                enabled: true,
                url_token: "a".repeat(64),
                ..Default::default()
            },
        )
        .unwrap();
        let state = McpConnectionState::default();
        let gateway = McpGatewayState::default();
        let mut stale = initial.clone();
        stale.read_data = true;
        state.stop(dir.path(), &gateway).unwrap();
        let rotated = load_permissions(dir.path()).unwrap();
        assert_ne!(initial.url_token, rotated.url_token);
        let saved = state
            .save_app_permissions(dir.path(), &gateway, stale)
            .unwrap();
        assert_eq!(saved.url_token, rotated.url_token);
        assert!(saved.read_data);
        assert_eq!(load_permissions(dir.path()).unwrap(), saved);
    }

    #[test]
    fn disabling_policy_rotates_public_token_and_returns_current_policy() {
        let dir = tempfile::tempdir().unwrap();
        let initial = save_permissions(
            dir.path(),
            McpPermissions {
                enabled: true,
                url_token: "a".repeat(64),
                ..Default::default()
            },
        )
        .unwrap();
        let state = McpConnectionState::default();
        let gateway = McpGatewayState::default();
        let saved = state
            .save_app_permissions(
                dir.path(),
                &gateway,
                McpPermissions {
                    enabled: false,
                    ..initial.clone()
                },
            )
            .unwrap();
        assert!(!saved.enabled);
        assert_ne!(saved.url_token, initial.url_token);
        assert_eq!(saved, load_permissions(dir.path()).unwrap());
    }

    #[cfg(unix)]
    #[test]
    fn unexpected_tunnel_exit_cannot_race_a_stale_permission_save() {
        use std::sync::mpsc;
        let dir = tempfile::tempdir().unwrap();
        let initial = save_permissions(
            dir.path(),
            McpPermissions {
                enabled: true,
                url_token: "a".repeat(64),
                ..Default::default()
            },
        )
        .unwrap();
        let mut terminated = Command::new("sh")
            .args(["-c", "exit 0"])
            .spawn()
            .expect("spawn deterministic exited tunnel");
        terminated.wait().unwrap();
        let state = Arc::new(McpConnectionState::default());
        let gateway = Arc::new(McpGatewayState::default());
        *state.active.lock().unwrap() = Some(ManagedConnection {
            mode: "cloudflare",
            started: Instant::now(),
            child: Some(terminated),
            report: Arc::new(Mutex::new(ConnectionReport::default())),
            local_url: String::from("http://127.0.0.1:39761/mcp/p/test"),
            tunnel_id: None,
            _temp: None,
        });

        // With the active lock held, status must acquire transition and then
        // block. A stale UI Save must not complete while status is revoking.
        let active_guard = state.active.lock().unwrap();
        let (started_tx, started_rx) = mpsc::channel();
        let (status_tx, status_rx) = mpsc::channel();
        let data = dir.path().to_path_buf();
        let status_thread = {
            let state = Arc::clone(&state);
            let gateway = Arc::clone(&gateway);
            thread::spawn(move || {
                started_tx.send(()).unwrap();
                let result = state.status_with_resources(Some(&data), Some(&gateway), false, false);
                status_tx.send(result).unwrap();
            })
        };
        started_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            match state.transition.try_lock() {
                Err(std::sync::TryLockError::WouldBlock) => break,
                Err(err) => panic!("unexpected transition lock error: {err}"),
                Ok(guard) => drop(guard),
            }
            assert!(
                Instant::now() < deadline,
                "status did not acquire transition"
            );
            thread::yield_now();
        }

        let (save_started_tx, save_started_rx) = mpsc::channel();
        let (save_tx, save_rx) = mpsc::channel();
        let data = dir.path().to_path_buf();
        let save_thread = {
            let state = Arc::clone(&state);
            let gateway = Arc::clone(&gateway);
            thread::spawn(move || {
                save_started_tx.send(()).unwrap();
                let result = state.save_app_permissions(
                    &data,
                    &gateway,
                    McpPermissions {
                        read_data: true,
                        ..initial
                    },
                );
                save_tx.send(result).unwrap();
            })
        };
        save_started_rx
            .recv_timeout(Duration::from_secs(5))
            .unwrap();
        assert!(
            save_rx.try_recv().is_err(),
            "stale Save cannot bypass the status revocation transition"
        );
        drop(active_guard);
        let status = status_rx.recv_timeout(Duration::from_secs(10)).unwrap();
        assert_eq!(status.state, "error");
        assert!(status.detail.contains("URL revoked"));
        let saved = save_rx
            .recv_timeout(Duration::from_secs(10))
            .unwrap()
            .unwrap();
        status_thread.join().unwrap();
        save_thread.join().unwrap();
        let final_policy = load_permissions(dir.path()).unwrap();
        assert_ne!(final_policy.url_token, "a".repeat(64));
        assert_eq!(saved.url_token, final_policy.url_token);
        assert!(saved.read_data);
    }

    #[cfg(unix)]
    #[test]
    fn poisoned_managed_connection_drop_terminates_real_child() {
        let state = Arc::new(McpConnectionState::default());
        let child = Command::new("sleep")
            .arg("60")
            .spawn()
            .expect("spawn controlled managed tunnel");
        let pid = child.id();
        *state.active.lock().unwrap() = Some(ManagedConnection {
            mode: "cloudflare",
            started: Instant::now(),
            child: Some(child),
            report: Arc::new(Mutex::new(ConnectionReport::default())),
            local_url: String::new(),
            tunnel_id: None,
            _temp: None,
        });
        let cloned = Arc::clone(&state);
        assert!(thread::spawn(move || {
            let _lock = cloned.active.lock().unwrap();
            panic!("simulate poisoned managed tunnel ownership");
        })
        .join()
        .is_err());
        drop(state);
        assert!(
            !Command::new("kill")
                .arg("-0")
                .arg(pid.to_string())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .unwrap()
                .success(),
            "managed tunnel child was not reaped during poisoned teardown"
        );
    }

    #[cfg(unix)]
    fn owned_fixture_connection() -> (ManagedConnection, u32) {
        let child = Command::new("sleep")
            .arg("60")
            .spawn()
            .expect("spawn controlled owned tunnel");
        let pid = child.id();
        (
            ManagedConnection {
                mode: "cloudflare",
                started: Instant::now(),
                child: Some(child),
                report: Arc::new(Mutex::new(ConnectionReport::default())),
                local_url: String::new(),
                tunnel_id: None,
                _temp: None,
            },
            pid,
        )
    }

    #[cfg(unix)]
    fn assert_fixture_reaped(pid: u32) {
        assert!(
            !Command::new("kill")
                .arg("-0")
                .arg(pid.to_string())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .unwrap()
                .success(),
            "owned tunnel {pid} remained running or unreaped"
        );
    }

    #[cfg(unix)]
    #[test]
    fn disconnect_recovers_poisoned_active_ownership_and_revokes_url() {
        let dir = tempfile::tempdir().unwrap();
        let previous = save_permissions(
            dir.path(),
            McpPermissions {
                enabled: true,
                url_token: "a".repeat(64),
                ..Default::default()
            },
        )
        .unwrap();
        let gateway = McpGatewayState::default();
        let state = Arc::new(McpConnectionState::default());
        let (connected, pid) = owned_fixture_connection();
        *state.active.lock().unwrap() = Some(connected);
        let cloned = Arc::clone(&state);
        assert!(thread::spawn(move || {
            let _guard = cloned.active.lock().unwrap();
            panic!("simulate panic while tunnel ownership was locked");
        })
        .join()
        .is_err());
        assert!(state.active.is_poisoned());
        state.stop(dir.path(), &gateway).unwrap();
        assert_fixture_reaped(pid);
        let recovered = state
            .active
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        assert!(recovered.is_none());
        assert_ne!(load_permissions(dir.path()).unwrap().url_token, previous.url_token);
    }

    #[cfg(unix)]
    #[test]
    fn poisoned_transition_polling_stops_owned_tunnel_and_revokes_url() {
        let dir = tempfile::tempdir().unwrap();
        let previous = save_permissions(
            dir.path(),
            McpPermissions {
                enabled: true,
                url_token: "b".repeat(64),
                ..Default::default()
            },
        )
        .unwrap();
        let gateway = McpGatewayState::default();
        let state = Arc::new(McpConnectionState::default());
        let (connected, pid) = owned_fixture_connection();
        *state.active.lock().unwrap() = Some(connected);
        let cloned = Arc::clone(&state);
        assert!(thread::spawn(move || {
            let _guard = cloned.transition.lock().unwrap();
            panic!("simulate panic inside connection transition");
        })
        .join()
        .is_err());
        assert!(state.transition.is_poisoned());
        let status = state.status_with_resources(Some(dir.path()), Some(&gateway), false, false);
        assert_eq!(status.state, "error");
        assert!(status.detail.contains("URL revoked"));
        assert_fixture_reaped(pid);
        assert_ne!(load_permissions(dir.path()).unwrap().url_token, previous.url_token);
        assert!(state.active.lock().unwrap().is_none());
    }

    #[cfg(unix)]
    #[test]
    fn poisoned_active_polling_stops_owned_tunnel_and_revokes_url() {
        let dir = tempfile::tempdir().unwrap();
        let previous = save_permissions(
            dir.path(),
            McpPermissions {
                enabled: true,
                url_token: "c".repeat(64),
                ..Default::default()
            },
        )
        .unwrap();
        let gateway = McpGatewayState::default();
        let state = Arc::new(McpConnectionState::default());
        let (connected, pid) = owned_fixture_connection();
        *state.active.lock().unwrap() = Some(connected);
        let cloned = Arc::clone(&state);
        assert!(thread::spawn(move || {
            let _guard = cloned.active.lock().unwrap();
            panic!("simulate panic inside active tunnel ownership");
        })
        .join()
        .is_err());
        let status = state.status_with_resources(Some(dir.path()), Some(&gateway), false, false);
        assert_eq!(status.state, "error");
        assert!(status.detail.contains("URL revoked"));
        assert_fixture_reaped(pid);
        assert_ne!(load_permissions(dir.path()).unwrap().url_token, previous.url_token);
    }

    #[test]
    fn failed_revocation_is_reported_in_startup_error() {
        let dir = tempfile::tempdir().unwrap();
        let gateway = McpGatewayState::default();
        // A directory at the policy-file path is invalid, and the revocation
        // path must surface this real persistence failure instead of masking it.
        fs::create_dir(scriptotar_mcp::permissions_path(dir.path())).unwrap();
        let error = failed_gateway_start("gateway failed".into(), dir.path(), &gateway);
        assert!(error.contains("gateway failed"));
        assert!(error.contains("URL revocation or gateway cleanup failed"));
    }

    #[test]
    fn openai_tunnel_id_is_strict() {
        assert!(valid_tunnel_id(&format!("tunnel_{}", "a".repeat(32))));
        assert!(!valid_tunnel_id("tunnel_BAD"));
        assert!(!valid_tunnel_id(&format!("tunnel_{}", "g".repeat(32))));
    }
}
