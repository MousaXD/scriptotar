# Scriptotar MCP — setup wizard and security

Scriptotar has 14 native Rust MCP tools and a **guided connection wizard**, patterned
on Chat On Steroids' provider choices. Permissions remain **app-owned**, never
controlled by ChatGPT. For a production installer, Scriptotar bundles **pinned,
SHA-256-verified** OpenAI `tunnel-client` and Cloudflare `cloudflared` binaries,
including provenance and license files. These programs are not downloaded or
updated in the background on customer computers.

## Setup in Scriptotar (no terminal required)

1. Launch Scriptotar and choose **MCP Setup** in the left sidebar, or open
   **Settings → MCP setup wizard** (displayed first in Settings).
2. **Permissions**: enable MCP, select only the actions you want ChatGPT to
   perform, and click **Save permissions & continue**. Reading project data,
   searching creators, downloading videos, and writing or executing are
   independent grants. File roots and executable paths require explicit local
   approval. All permissions are disabled on new installations.
3. **Connection**: select one of the three choices and click **Connect**:

   - **Cloudflare Quick Tunnel** (easy URL): starts the packaged `cloudflared`
     and displays a temporary `https://<random>.trycloudflare.com/mcp/p/<secret>`
     URL. No Cloudflare account is required. Copy the complete URL and add it
     as a **Server URL** in ChatGPT's custom MCP server setup. Cloudflare Quick
     Tunnels are for development/testing, not production hosting. The hostname
     changes on reconnection; the path token is also rotated.
   - **OpenAI Secure MCP Tunnel** (private): create a tunnel ID at
     https://platform.openai.com/settings/organization/tunnels and a restricted
     runtime API key with Tunnels Read+Use permission. Paste them into the
     wizard; Scriptotar starts its packaged `tunnel-client` and polls the
     client's localhost `/readyz` status. Optionally remember the key securely
     in your operating-system keyring (never in SQLite or the Git repo). In
     ChatGPT choose **Connection: Tunnel** and select your tunnel ID. OpenAI
     does not need a public port and receives the connection via its tunnel.
   - **My own HTTPS hostname** (named Cloudflare or reverse proxy): set up
     your tunnel to forward to `http://127.0.0.1:39761` and enter only your
     public HTTPS origin, e.g. `https://mcp.example.com`. Scriptotar displays
     the resulting HTTPS URL with a new unguessable secret path. You manage
     the external tunnel process yourself.

4. **Connect in ChatGPT**: open https://chatgpt.com/plugins, choose **Add custom
   MCP server**, select the matching Connection type and paste the provided ID
   or URL. Workspace permissions may restrict custom MCP connectors. Test
   `list_projects` and a deliberately denied tool before granting write/execute.
5. **Disconnect & revoke URL** to stop the app-managed connection and local
   gateway, kill its child processes and rotate the path secret. The URL is
   a bearer credential: anyone who learns it can request *locally permitted*
   MCP actions, so treat it like a password. Do not share it or commit it.

There is also a lower-level `--mcp-stdio` mode for tools that spawn stdio MCP
processes directly. This requires no public URL and uses the same app-owned
permissions. Run `/usr/bin/scriptotar-desktop --mcp-stdio` in a separate stdio
client; `tunnel-client` is not needed if the client supports native local MCP.

## What is and is not verified

- A successful **Ready** status for Cloudflare means the local MCP handshake
  responded and `cloudflared` published a temporary URL. It does not prove a
  real ChatGPT call succeeded. OpenAI Ready is tied to the client's `/readyz`
  result. Manual hostname mode does not verify external DNS or Cloudflare
  ingress configuration.
- The HTTP listener binds to **127.0.0.1** only, checks a 64-character
  path secret on `/mcp/p/<secret>`, and continues to require a separate HTTP
  bearer token on `/mcp`. Origin validation and request limits remain active.
  A permissions change is enforced on the next tool call and can cut off
  access immediately; disabling MCP also stops the active app connection.
- The wizard's API key input is cleared after the connection request. The
  restricted runtime key is passed to `tunnel-client` via an environment
  variable and never as a command-line argument. Only an explicit **Remember**
  choice stores it in the operating system's secure keyring. If the keyring
  cannot be opened, uncheck Remember and connect for the current session.
- Automatic reconnection across application restarts is not guaranteed: the
  wizard keeps the tunnel running while Scriptotar is open. Quick Tunnel URLs
  intentionally change when a new connection starts.

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
