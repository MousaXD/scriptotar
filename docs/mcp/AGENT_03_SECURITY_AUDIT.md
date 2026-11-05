# Agent 03 — MCP security audit and threat model (2026-11-05)

## Scope and baseline
- Base: `feature/chatgpt-mcp-ci-20261026` at `517877b26df9028cdd32af69f6f5a88a68b948e9`.
- Components: 14 registered MCP tools, stdio JSON-RPC, localhost HTTP, Tauri permission/connection manager, Cloudflare Quick Tunnel, OpenAI Secure MCP Tunnel, manual HTTPS, wizard and CI.
- Trust boundary: ChatGPT and external MCP clients are untrusted. The desktop-approved policy is the authority. Authenticated transport is **not** a substitute for authorization.
- Access scope: An approved executable is not sandboxed and has the user's OS identity. Local malicious same-account processes may defeat application-level filesystem and process restrictions.

## Findings, impacts and remediations
| Severity | Finding | Remediation in Agent 03 branch |
| --- | --- | --- |
| High | HTTP verified the bearer or URL credential before accepting a potentially slow body, but did not reverify after token rotation. A request authenticated with a revoked token could later be dispatched if application permissions still allowed that tool. | Reauthenticate after body reception, before JSON-RPC dispatch. Negative HTTP E2E streams part of a request, rotates both secrets, completes the body and expects HTTP 401. |
| High | Protected local policy/SQLite files were denied by filename only. A hard link inside an approved directory to one of those files was not recognized; temporary policy files also held secrets under different names. | Reject policy staging paths, and on Unix reject matching device/inode identities for protected policy and SQLite files. Unit and stdio negative tests. |
| Medium | HTTP transport applied the 4-worker limit only after fully reading request bodies and did not time out slow clients; the socket accept loop was unbounded. | 32 accepted-connection limit, 15-second body timeout, 10-minute total connection cap. Retain 4 active tool workers. |
| Medium | Allowlisted executable commands inherited arbitrary parent environment variables, including possible credentials. | Explicitly allow only common OS execution variables; deny inherited application/CI secrets. Black-box Python subprocess test. |
| Medium | Permission or executable grant revocation was checked only at the start of a command, so a long-running child continued for up to 15 seconds after revocation. | Re-read app policy and revalidate executable and cwd grants while command runs; terminate the direct child on revocation. |
| Low | HTTP accepted misleading media types beginning with `application/json`, such as `application/jsonp`. | Require JSON MIME type prior to optional parameters. |
| Medium | Unexpected managed tunnel process exit was only reported by status polling, without gateway cleanup or URL-secret revocation. | On status detection, clear managed state, stop local gateway and rotate the URL secret; add revocation regression test. |

## Protected functionality
All 14 tools and three guided transport options remain registered. No new MCP tool can change permissions, tokens or tunnel configuration. Default-deny remains enabled. We did not add unrestricted filesystem access, implicit shell execution, public listeners or unrestricted URL fetching.

## Residual risks and mandatory external validation
- The pathname-based filesystem policy does **not** provide an OS sandbox against a concurrent hostile local process that can race directory or symlink changes. Strong no-follow, directory-descriptor-relative open primitives and a separate OS account remain future hardening work. Non-Unix hard-link identity protection is not yet implemented.
- An approved shell/interpreter can bypass file roots using its OS privileges, spawn child/grandchild processes, and read filesystem files itself. Terminating the direct child does not guarantee descendant cleanup. Forbid untrusted executables; consider OS process groups or sandboxing in a separate design.
- Encrypted connection through a public Quick Tunnel is still possession-of-secret-URL authorization plus app permissions. A leak of a still-active URL is a credential leak. Cloudflare Quick Tunnel availability is not production SLA.
- Keyring availability, installed Windows/macOS behavior and real ChatGPT OpenAI Secure MCP Tunnel credentials are **not verified** by fixture tests.
- HTTP body and connection caps are DoS bounds, not a global cross-process abuse-prevention service. Long synchronous network research calls can consume a worker.
- The app-managed tunnel process is now checked during status polling; an exited or uninspectable child triggers local gateway shutdown and URL rotation. Polling is not an instantaneous death notification: there can be a short interval before the next status refresh. Manually managed reverse proxies cannot be monitored by Scriptotar.

## Certification gates
Run `.github/workflows/mcp-e2e.yml` on Agent 03 PR: Rust MCP unit, real binary stdio and HTTP E2E, Svelte tests/build, Tauri compiled MCP CLI/IPC tests, workspace tests. Inspect run logs at exact PR head SHA; do not claim all tests passed unless run concludes successfully. Real ChatGPT tunnel authentication remains **unverified** without authorized credentials and a genuine end-to-end session.

## Handoff
Agent 00 reviews severity, CI, operational constraints and residual risk. This branch must never merge itself into the feature branch or main.
