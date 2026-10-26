# Scriptotar MCP: local authority, remote ChatGPT transport

Scriptotar's **Rust MCP server** supports local stdio and an explicitly started,
**localhost-only** JSON/HTTP endpoint (`http://127.0.0.1:39761/mcp`).
The installed Tauri executable also supports `--mcp-stdio` and `--mcp-http`;
the standalone workspace binary `scriptotar-mcp` offers both transports.

**For ChatGPT, use OpenAI's Secure MCP Tunnel with the installed binary's
stdio command.** The tunnel starts Scriptotar's MCP process locally, so there
is no public listener and no need to send the HTTP-only static bearer token
through ChatGPT. ChatGPT cannot connect to your local stdio process directly.
The tunnel is an outbound-only transport and still requires your ChatGPT
workspace and OpenAI Platform organization to authorize it. Scriptotar itself
controls all tool permissions, independently of ChatGPT.

Official guide: https://developers.openai.com/api/docs/guides/secure-mcp-tunnels

## Local setup

1. Open **Scriptotar > Settings > MCP access**. All permissions default to **off**.
2. Grant only the capabilities you intend: read/write Scriptotar project data,
   read/write/overwrite UTF-8 files, execute approved programs, research creators,
   or queue videos and local media for download/transcription.
3. For file permissions, enter existing absolute **approved directory** paths;
   for command execution, enter exact absolute **approved executable** paths.
4. Acknowledge powerful access **in the desktop app**, then apply permissions.
5. For ChatGPT, leave the optional HTTP gateway stopped and follow the
   **recommended stdio Secure MCP Tunnel** procedure below.
6. Start the tunnel client and test a safe read-only tool. Only enable write,
   execute or download capabilities after validating the connection.

### Recommended: Secure MCP Tunnel from ChatGPT via stdio

1. Install a Scriptotar build that includes this feature. The old Scriptotar
   1.0.0 package does not include these MCP changes until an updated package
   is built and installed.
2. From [OpenAI Platform's tunnel settings](https://platform.openai.com/),
   create a tunnel associated with the correct ChatGPT workspace and Platform
   organization, and obtain its `tunnel_id`. Check that your account has the
   required tunnel permissions and can create custom MCP plugins.
3. Install the **official** `tunnel-client` according to the [Secure MCP Tunnel
   guide](https://developers.openai.com/api/docs/guides/secure-mcp-tunnels).
   Keep runtime API keys and secrets in local secure environment variables,
   never in GitHub Actions, this repo, or a chat message.
4. For a Linux Debian installation, the app binary is typically
   `/usr/bin/scriptotar-desktop`. Select it as the stdio MCP command:

   ```bash
   # Export CONTROL_PLANE_API_KEY securely in your local terminal first.
   # Replace the example tunnel ID with the one assigned to your workspace.
   tunnel-client init \
     --sample sample_mcp_stdio_local \
     --profile scriptotar-stdio \
     --tunnel-id 'tunnel_YOUR_ASSIGNED_ID' \
     --mcp-command '/usr/bin/scriptotar-desktop --mcp-stdio'
   tunnel-client doctor --profile scriptotar-stdio --explain
   tunnel-client run --profile scriptotar-stdio
   ```

   On Windows, use the installed `scriptotar-desktop.exe` absolute path and
   quote it according to `tunnel-client help quickstart`.
5. In ChatGPT **Plugins > Add custom MCP server**, choose **Connection:
   Tunnel**, select that same tunnel ID, and review the discovered tools.
   The ChatGPT workspace or administrator must allow custom MCP servers.
6. Install/connect the resulting plugin and invoke `list_projects` with
   Scriptotar's **read-data** permission enabled. Then test denied write
   requests with write-data disabled. Do not claim full ChatGPT E2E until
   that actual connected tool call succeeds.

The `tunnel-client` must be running while ChatGPT discovers or invokes MCP
calls. The app's Settings file is reloaded for each call. No Scriptotar
password or HTTP bearer token is required inside ChatGPT for this **stdio**
connection. The tunnel client authenticates to OpenAI using its own identity.

### Optional local HTTP MCP gateway (not recommended for ChatGPT)

Click **Start local MCP gateway** in Scriptotar Settings to use
`http://127.0.0.1:39761/mcp` with a local tool that supports a static HTTP
bearer header. Reveal the private token only for that client. **ChatGPT's
custom MCP connection UI does not support arbitrarily supplying this static
header; use the stdio tunnel above instead.**

The HTTP server binds only to `127.0.0.1`. It validates `Origin` if supplied,
requires a bearer token on every request, caps request bodies, and reloads the
local permissions file before each call. It supports JSON responses to POST
requests. GET/SSE is not implemented and returns HTTP 405.

### Developer stdio launch

```bash
export SCRIPTOTAR_DB_PATH="$HOME/.local/share/io.github.mousaxd.scriptotar/scriptotar.sqlite3"
cargo run -p scriptotar-mcp --locked
```

The installed desktop binary can be launched with `--mcp-stdio` without
opening the graphical UI; it discovers the standard Scriptotar data directory
unless `SCRIPTOTAR_DB_PATH` or `SCRIPTOTAR_DATA_DIR` overrides it.
The Rust process **does not grant permissions via MCP**. It reads the desktop-
owned permissions file; disabled or malformed policies fail closed. Changes
and revocations take effect on subsequent calls without restarting the server.

### Local HTTP developer launch

```bash
export SCRIPTOTAR_DB_PATH="$HOME/.local/share/io.github.mousaxd.scriptotar/scriptotar.sqlite3"
cargo run -p scriptotar-mcp --locked -- --mcp-http
```

This requires the desktop application to have **enabled MCP** and persisted a
bearer token. Never copy the bearer token into source code, CI logs, or the repo.
The standalone binary's download/transcription engine uses the same Rust job
orchestrator and its configured sidecar; developers should set
`SCRIPTOTAR_SIDECAR_PYTHON`, `SCRIPTOTAR_SIDECAR_SCRIPT`, and bundled yt-dlp
executable paths as needed. The packaged desktop executable now discovers its bundled runtime for
`--mcp-stdio` or `--mcp-http`, without needing to launch the GUI. The GUI
also passes the bundled runtime environment to the local HTTP gateway child.

## Tool capabilities

| Tool | Required local permission |
| --- | --- |
| `list_projects`, `list_research`, `list_transcripts`, `get_transcript`, `search_transcripts`, `list_jobs` | Read Scriptotar data |
| `create_project` | Write Scriptotar data |
| `list_directory`, `read_file` | Read files + approved directories |
| `write_file` | Write files + approved directories; separate overwrite grant for replacements |
| `execute_command` | Execute commands + explicitly approved binary path |
| `search_creator` | Search creators (network via existing yt-dlp research provider) |
| `download_video` | Download videos (URL constrained by existing NetworkPolicy) |
| `enqueue_local_media` | Download videos + read files + approved local-media path |

**No tool modifies permissions, starts the gateway, or changes bearer tokens.**
All policies are saved through Tauri desktop commands. Unknown tool arguments,
relative paths, path traversals, unapproved programs, unexpected URL domains,
invalid UUIDs and oversized text inputs are rejected.

## Safety boundaries

- The HTTP bearer token **authenticates** a connection; app permissions
  **authorize** every operation independently. ChatGPT-side approvals are
  optional extra checks, not the Scriptotar authorization boundary.
- Approved commands run with **your operating-system account's privileges**.
  A shell or interpreter on the allowlist is *not* sandboxed; it may read/write
  outside the configured file roots. Never grant one unless you accept that
  scope of access, and do not use Scriptotar MCP as an OS security sandbox.
- Files are limited to approved directories with normalized absolute paths,
  canonicalized path checks, and atomic writes. A separate OS account/sandbox
  is necessary for an adversarial local process that can race filesystem path
  changes. The MCP process itself runs as the same OS user as Scriptotar.
- `write_file` handles UTF-8 files (up to 256 KiB); `read_file` returns up to
  128 KiB. Process execution is bounded to 15 seconds and 64 KiB per output
  stream, though child processes may spawn other processes. Media work uses
  Scriptotar's existing job scheduler. `search_creator` intentionally does not
  access browser cookies from MCP requests.
- No general-purpose external URL fetch, arbitrary HTTP endpoints, remote
  shell, or OS privilege escalation has been added. These would require
  additional explicit authorization and independent security review.

## CI certification scope

`.github/workflows/mcp-e2e.yml` runs on GitHub Actions and checks:
- Rust MCP library unit tests and actual compiled binary.
- Stdio JSON-RPC initialize, tool discovery, supported calls, denied calls,
  scopes, symlink escape, overwrites, command allowlist, revocation and oversized
  requests.
- Real local HTTP listener: authentication, invalid Origin, wrong content type,
  live policy/token changes and revocation.
- Svelte/TypeScript settings checks, unit tests and frontend build.
- Compiled Tauri desktop executable with real `--mcp-stdio`, an isolated
  SQLite fixture, fake local yt-dlp research, deterministic queued URL/local
  media transcription, persisted transcript reads and restart.
- Linux `.deb` and Windows NSIS install workflows also execute a default-deny
  MCP startup smoke on the **installed** application, when their full packaging
  workflows run. These package tests are separate from the focused MCP gate.

**Not certified by these tests:** a real ChatGPT connected-plugin session through
Secure MCP Tunnel, external social-platform content availability, platform
cookies, real media download/transcription, installed Windows/macOS UX, or all
possible unsafe command behaviors. Those require separate authenticated and
real-device E2E tests. Avoid claiming full production certification from a
passing protocol test alone.
