# Scriptotar Next 1.2.0 — Release Readiness

**As of 2026-11-07:** RELEASE CANDIDATE / **NO-GO** for `main` and public preview publication until exact-versioned-SHA gates and independent QA are completed. The separate archived Classic 1.3.0 release is unchanged; Next stays on its rolling prerelease channel.

## Integrated scope and version

Agent 00 integration [PR #80](https://github.com/MousaXD/scriptotar/pull/80) combines seven specialist PRs (#73, #74, #75, #76, #77, #78, #79). The Linux installed-MCP client/HTTP 413 test correction in [PR #81](https://github.com/MousaXD/scriptotar/pull/81) passed actual package/HTTP/restart acceptance and was merged into the isolated integration branch. The 1.2.0 candidate aligns Tauri, Rust workspace and nine local Cargo.lock packages, Svelte package and npm lockfile, Python sidecar and docs. MCP JSONL protocol stays at version 1.

## Mandatory exact-commit evidence

- Rust formatting, Clippy, workspace and migration/recovery tests; full Python sidecar and engine imports; Svelte checks/tests/build; MCP stdio/HTTP/14-tool authorization; full Tauri integration and strict Cargo/npm/pip audits.
- **Windows**: clean NSIS install, packaged compliance/runtime and actual native Win32 descendant-process termination regression executed, not skipped.
- **Linux**: package contents and install, MCP default deny and live localhost HTTP auth/origin/token rotation/revocation, executable shell launch and all three installed-app IPC/persistence restart tests.
- CI security and publisher contract: exact source SHA and required jobs, not historical green runs or unverified cancelled/skipped jobs.

## Live providers and other pending acceptance

The strict [real YouTube/Instagram/TikTok canary](https://github.com/MousaXD/scriptotar/actions/runs/42395493246) still **FAILS** for user-pinned YouTube `2PuFyjAs7JA` due to platform sign-in/bot protection, before download/inference. Instagram download + real Faster Whisper passed (3 segments); TikTok download + inference returned 0 segments (not an accuracy proof). Preserve exact URL, no cookie bypass/source substitution, no mock passing result. Release owner and independent QA must explicitly resolve or record an accepted external-access risk consistent with the published test contract.

Hardware CUDA/CPU fallback, Arabic and multilingual transcription accuracy, real authorized ChatGPT tunnel session, interactive Windows WebView user journey, actual historical user database upgrades and performance benchmarks are not certified by unrelated automation. Upstream FFmpeg archive URL is mutable even while digest and size are enforced; provenance remains under review. Three low-severity npm advisory paths remain disclosed.

## Promotion protocol — not yet authorized

1. Get green exact-HEAD checks on this version candidate and explicit Agent 07 independent **GO**; no suppressions or feature removal.
2. Merge review-approved candidate into Agent 00 integration branch; rerun exact-combined-head gates including installed Windows and Linux.
3. Promote [#80](https://github.com/MousaXD/scriptotar/pull/80) to `feature/chatgpt-mcp-ci-20261026` and [#71](https://github.com/MousaXD/scriptotar/pull/71) to `main` only after final review of scope and versions; never force push or bypass checks.
4. Keep existing sole `tauri-next-latest` preview publisher, with exact-`main`-SHA five-gate validation and `SCRIPTOTAR_NEXT_DISTRIBUTION_APPROVED=true`. Do not invent a stable Next release tag.

## Historical CI evidence (NOT certification of final candidate SHA)

| Workflow | Evidence | Result |
| --- | --- | --- |
| Installed Linux after HTTP test fix | [#42398429828](https://github.com/MousaXD/scriptotar/actions/runs/42398429828) | PASS, PR #81 SHA |
| MCP E2E after fix | [#42398429791](https://github.com/MousaXD/scriptotar/actions/runs/42398429791) | PASS, PR #81 SHA |
| Windows NSIS and native child test | [#42395493363](https://github.com/MousaXD/scriptotar/actions/runs/42395493363) | PASS, previous integration SHA |
| Rust/npm/Python security audit | [#42395493283](https://github.com/MousaXD/scriptotar/actions/runs/42395493283) | PASS, previous integration SHA |
| Real providers | [#42395493246](https://github.com/MousaXD/scriptotar/actions/runs/42395493246) | FAIL, YouTube source access |

**No public versioned distribution has been generated or approved by this document.**
