# Scriptotar MCP: local authority, remote ChatGPT transport

Scriptotar's **Rust MCP server** works over stdio and, when explicitly started, over a
**localhost-only Streamable HTTP** endpoint (`http://127.0.0.1:39761/mcp`).
The installed Tauri executable also supports `--mcp-stdio` and `--mcp-http`;
the standalone workspace binary `scriptotar-mcp` offers the same transports.
ChatGPT cannot connect to stdio directly; use the HTTP endpoint with an official
Secure MCP Tunnel, or an independently configured HTTPS reverse proxy with TLS
and authentication. **Do not publicly expose the HTTP listener or disable authentication.**

## Local setup

1. Open **Scriptotar > Settings > MCP access**. All permissions default to **off**.
2. Grant only the capabilities you intend: read/write Scriptotar project data,
   read/write/overwrite UTF-8 files, execute approved programs, research creators,
   or queue videos and local media for download/transcription.
3. For file permissions, enter existing absolute **approved directory** paths;
   for command execution, enter exact absolute **approved executable** paths.
4. Acknowledge powerful access **in the desktop app**, then apply permissions.
5. Click **Start local MCP gateway**. Its address is `http://127.0.0.1:39761/mcp`.
6. Reveal and securely copy the **bearer token** from Scriptotar Settings; it is
   generated locally and stored in the application's private permissions file.
7. Connect your chosen **Secure MCP Tunnel** to `http://127.0.0.1:39761/mcp`.
   Configure your connected MCP client with `Authorization: Bearer <the-token>`.
   You must make both the tunneled endpoint and bearer authentication available
   to the connector; the exact configuration depends on the connector UI.

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
executable paths as needed. The packaged desktop process supplies its bundled
runtime to the gateway child when launched from the Settings UI.

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

**Not certified by these tests:** a real ChatGPT connected-plugin session through
Secure MCP Tunnel, external social-platform content availability, platform
cookies, real media download/transcription, installed Windows/macOS UX, or all
possible unsafe command behaviors. Those require separate authenticated and
real-device E2E tests. Avoid claiming full production certification from a
passing protocol test alone.
