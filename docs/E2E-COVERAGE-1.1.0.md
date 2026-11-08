# Scriptotar 1.1.0 — E2E certification inventory

This matrix separates **deterministic code contracts**, **real compiled application
workflows**, and **live external providers**. Passing one layer does not prove
that a different layer passed. Never count a mock as a real download or an
external provider outage as a product success.

## CI workflow layers

| CI check | E2E evidence | Must pass |
| --- | --- | --- |
| `Scriptotar MCP E2E` | Entire Rust workspace unit/integration tests, 34 registered Tauri command bindings, 14 MCP tools, sidecar protocol, recovery, media validation, local command/file permissions, DB access, lockfile identities, frontend UI tests and build | Yes |
| `Scriptotar MCP E2E` | Real compiled Tauri **desktop executable** over stdio; project/research/transcript creation and searches, URL/local jobs through a deterministic sidecar fixture, permissions, completed persisted records across restart | Yes |
| `Scriptotar MCP E2E` | Real localhost HTTP MCP authorization with bearer/path secrets, Origin checks, token changes, disable/revoke and negative cases | Yes |
| `Linux Tauri Package` | Actual installed `.deb`, bundled engine, FFmpeg, verified tunnel binaries, read-only MCP startup, HTTP URL auth, GUI launch, IPC and restart persistence | Yes for Linux release |
| `Windows Tauri Installer` | NSIS clean install, bundled media runtime, MCP default deny, GUI/backend check and uninstall | Yes for Windows release |
| `Real YouTube Instagram TikTok E2E` | **Real network downloads** via Scriptotar `download_url`, no cookies or fake metadata, validates audio using FFprobe and runs actual Faster Whisper `small` CPU on a 15-second excerpt from each download; checks SRT/VTT/JSON/text exports | Yes; platform failures are visible failures |
| `Weekly Real Whisper Canary` | Real bundled Whisper model downloaded once, known spoken phrase transcribed and repeated offline with cached model | Yes when run (separate weekly/manual workflow) |

## API surface checks

- `tests/test_api_surface_contract.py` checks all **34** Tauri commands are
  registered, bridged to `ScriptotarApi`, mocked and invoked by the frontend
  command contract test. New untested commands fail automatically.
- `tests/test_mcp_api_surface.py` discovers **14** native MCP catalog tools,
  checks for the matching Rust handler, invocation by compiled-process E2E,
  and a documented local permission. The unknown `change_permissions` tool
  must be denied.
- Sidecar `ping`, `transcribe`, `cancel`, and `shutdown` are each exercised
  through the real sidecar subprocess; oversized lines and broken engine
  subprocess output have regression coverage.
- Source controls use strict app-owned file roots, explicit executable
  allowlists and per-tool data permissions. No CI job grants access to user
  home directories, real accounts or browser cookies.

## Real source URLs

- **YouTube (user-provided):** `https://youtu.be/2PuFyjAs7JA?si=yqHYehEncAJH67oB`
- **Instagram (historical public Scriptotar research item):** `https://www.instagram.com/reel/DaLCUltyZ7p/?hl=en`
- **TikTok (official TikTok developers' public embed sample):** `https://www.tiktok.com/@scout2015/video/6718335390845095173`
  Source: https://developers.tiktok.com/docs/en/embed-videos

These are publicly addressable **test inputs**, not a promise that the hosting
platform will always allow anonymous downloads. Real external platforms may
rate-limit CI IP ranges, remove media, region-block content or require login.
We do **not** use cookie imports or defeat access controls in CI. Each provider
receives an independent explicit pass/failure row; the whole workflow fails if
any provider fails, rather than showing a fake green check.

## Remaining external/manual checks

- A **real ChatGPT plugin session** through a user-authorized Cloudflare/OpenAI
  tunnel requires that workspace's credentials and permission grants. Offline
  CI tests cannot claim that live cloud connection was exercised.
- Live external **paid AI providers** (OpenAI, Anthropic, Gemini) require user
  API keys and billing. Their prompt building, provider routing and validation
  are included in deterministic Rust/UI tests, but external paid API responses
  cannot be certified from an unauthenticated CI runner.
- Windows installed GUI user journeys and platform-specific dialogs require
  interactive Windows acceptance testing beyond NSIS smoke tests.
- No claim is made that every conceivable state combination is tested; this
  suite enforces registered API surface coverage and exercises major features,
  including negative and revocation behavior.
