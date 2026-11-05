# Agent 07 — Scriptotar independent QA baseline and release certification

**As inspected:** 2026-11-05 (UTC)  
**Repository:** https://github.com/MousaXD/scriptotar  
**Reference branch:** feature/chatgpt-mcp-ci-20261026  
**Pinned reference HEAD:** 517877b26df9028cdd32af69f6f5a88a68b948e9  
**Agent branch:** agent-07/qa-release-certification  
**Overall status:** IN PROGRESS; Phase A baseline recorded; Phase B/Phase C outstanding  
**Baseline release recommendation:** NO-GO; current reference HEAD is incompletely tested

This is an **evidence log, not a release approval**. All claims about GitHub Actions below refer to the exact named commit and event. A successful run on an older SHA cannot certify this HEAD; cancelled, skipped, queued and in-progress jobs cannot count as successful. A future integrated candidate must receive a new exact-SHA assessment by Agent 07 before Agent 00 can decide on merging or publishing.

## 1. Existing functionality: preservation inventory

| Surface | Preserved public behavior/contract | Where to review |
| --- | --- | --- |
| Desktop UI | Dashboard, Research, Jobs, Transcript, AI Studio, Library, Settings; project navigation, appearance, Arabic/English translations and RTL | apps/desktop-ui/src/views, src/types.ts, src/i18n, src/rtl.css; frontend tests |
| Tauri IPC | 34 registered Rust commands and 35 frontend ScriptotarApi methods, including their arguments, responses, mock bindings and invocation tests | apps/desktop/src-tauri/src/lib.rs; tests/test_api_surface_contract.py; apps/desktop-ui/src/api |
| Local database | Rust-owned SQLite persistence, FTS search, projects, research/creators, watchlists, jobs, transcripts, AI runs, migration, interrupted-job recovery | crates/scriptotar-db, scriptotar-jobs; apps/desktop/src-tauri/src/migration.rs |
| Media | URL and local files; yt-dlp/FFmpeg acquisition; Faster Whisper CPU and CUDA policy, automatic CPU fallback, cancellation, persistence, exports | crates/scriptotar-media, scriptotar-orchestrator; sidecars/transcription; PROTOCOL.md |
| Creator and AI workflows | Watchlists, research/source metadata, source selection, prompt generation, Copy Prompt and BYOK provider choices | scriptotar-research, scriptotar-ai; ResearchView, AiStudioView |
| MCP | All 14 registered tools; default-deny, app-owned capability grants, approved file roots, command allowlists, stdio and HTTP transport, bearer/path secrets, revocation, connection wizard | crates/scriptotar-mcp, desktop mcp_connect.rs, docs/mcp/README.md, tests/mcp_*.py |
| Installers | Actual bundled transcription runtime, validated MCP tunnel binaries, Windows NSIS install/uninstall, Linux Debian install/launch/restart tests | windows-tauri.yml; linux-tauri.yml; sidecar build/validate tools |
| Distribution | Sole rolling Next preview publisher with exact-SHA gate and approval variable; archived Classic app remains intact | tauri-next-release.yml; scripts/release_ci_gate.py; archive/legacy-python |

Inventory is a **minimum regression contract**, not a declaration of complete feature parity or user-journey coverage. Additive features must extend the corresponding API and integration tests. Do not delete historical functionality to make an acceptance check pass.

## 2. Evidence on pinned baseline SHA

Reference HEAD: **517877b26df9028cdd32af69f6f5a88a68b948e9** (October 26, 2026).

| Workflow | Exact-SHA run | Evidence | Certification status |
| --- | --- | --- | --- |
| Rust foundation | [40397288085](https://github.com/MousaXD/scriptotar/actions/runs/40397288085) | Completed success; fmt, check, Clippy, tests | PASS for the listed workflow on this SHA |
| Security hygiene | [40397288189](https://github.com/MousaXD/scriptotar/actions/runs/40397288189) | Completed success | PASS for the listed workflow on this SHA |
| Release Ownership Contract | [40397288195](https://github.com/MousaXD/scriptotar/actions/runs/40397288195) | Completed success | PASS for the listed workflow on this SHA |
| Preview release validation | [40397288018](https://github.com/MousaXD/scriptotar/actions/runs/40397288018) | Contract job succeeded; publish-preview was skipped on PR event | CONTRACT PASS, no release proof |
| Main CI | [40397287926](https://github.com/MousaXD/scriptotar/actions/runs/40397287926) | Cancelled | **NOT PROVEN** |
| Tauri migration integration | [40397287946](https://github.com/MousaXD/scriptotar/actions/runs/40397287946) | Cancelled; Python sidecar and Rust-sidecar job succeeded, other mandatory jobs cancelled | **NOT PROVEN** |
| Desktop UI | [40397288066](https://github.com/MousaXD/scriptotar/actions/runs/40397288066) | Cancelled | **NOT PROVEN** |
| Transcription Sidecar | [40397287922](https://github.com/MousaXD/scriptotar/actions/runs/40397287922) | Cancelled; protocol/unit job succeeded, engine-import job cancelled | **NOT PROVEN** |
| MCP E2E | [40397288231](https://github.com/MousaXD/scriptotar/actions/runs/40397288231) | Cancelled before desktop integration, gateway tests and compiled-binary fixtures completed | **NOT PROVEN** |
| Windows Tauri Installer | [40397288071](https://github.com/MousaXD/scriptotar/actions/runs/40397288071) | Cancelled | **NOT PROVEN** |
| Linux Tauri Package | [40397288264](https://github.com/MousaXD/scriptotar/actions/runs/40397288264) | Cancelled | **NOT PROVEN** |
| Real YouTube/Instagram/TikTok E2E | [40397288173](https://github.com/MousaXD/scriptotar/actions/runs/40397288173) | Cancelled | **NOT PROVEN** |
| Weekly Real Whisper Canary | No matching check on reference HEAD identified | Independently scheduled/manual | **NOT PROVEN** on this SHA |

**Earlier-commit distinction:** 608226ab7b2e15eb76b9ef0590f73cc8da6628d8 had successful Windows [40394502926](https://github.com/MousaXD/scriptotar/actions/runs/40394502926) and Linux [40394502465](https://github.com/MousaXD/scriptotar/actions/runs/40394502465) package jobs; those are NOT evidence for 517877b. That earlier SHA's Rust foundation [40394502906](https://github.com/MousaXD/scriptotar/actions/runs/40394502906) failed during Ubuntu Jammy dependency installation with timeout exit code 124, before Rustfmt/check/test; the pinned reference SHA subsequently passed a newer-runner Rust foundation job.

**Confirmed external E2E failure on earlier SHA:** Real-provider run [40394502484](https://github.com/MousaXD/scriptotar/actions/runs/40394502484) failed because YouTube requested bot verification for the exact URL. Its Instagram and TikTok downloaded/transcribed successfully. This is a *real provider access blocker on that runner*, not proof that the Rust/sidecar pipeline failed. No mock, cookie import, credential invention or silent skip is permitted to convert it to green.

## 3. Release test matrix and ownership

| Gate | Required proof on the integrated candidate | Location |
| --- | --- | --- |
| Rust quality | cargo fmt --all -- --check; cargo check --workspace --all-targets --locked; cargo clippy --workspace --all-targets --all-features --locked -- -D warnings; cargo test --workspace --locked | rust-foundation.yml; integration.yml |
| Frontend | npm ci; npm run check; npm run test; npm run build; API invocation and Arabic/RTL UI coverage | desktop-ui.yml; integration.yml; CI |
| Sidecar | Python syntax/compile, unit and subprocess protocol tests, real dependency installation/imports, pip check | transcription-sidecar.yml; integration.yml |
| Orchestration and recovery | Rust ↔ sidecar fixture protocol, cancellation cleanup, SQLite migration, interrupted job restart and transcript persistence | integration.yml; crates/scriptotar-*/tests |
| MCP security | 14-tool catalog/handler/invocation contract; denied permission mutations; local path/command security; stdio, HTTP auth/Origin/revocation and compiled desktop CLI E2E | mcp-e2e.yml; tests/test_mcp_api_surface.py; tests/mcp_*.py |
| Supply chain and license | cargo audit, npm audit (high+), pip-audit, action-pin/repository hygiene, runtime licensing and packaging compliance | integration.yml; security-hygiene.yml; sidecar compliance scripts |
| Windows | Complete self-contained runtime/NSIS installer, clean install, GUI/backend startup, SQLite initialization, MCP deny, runtime validation and uninstall | windows-tauri.yml |
| Linux | Build .deb on Ubuntu 22.04 for release ABI, inspect contents and checksum, install, shell launch, HTTP MCP auth, repeat 3 installed-app restart/IPC E2Es, uninstall | linux-tauri.yml |
| Real model | Actual bundled Faster Whisper model download and offline repeat | whisper-runtime-canary.yml (separate weekly/manual) |
| Live social platforms | Real YouTube/Instagram/TikTok download and transcript canaries; failures remain explicit | live-platform-e2e.yml (separate scheduled/manual/path-filtered) |
| Release ownership and race | Single publisher, exact-SHA requirement for every mandatory workflow job, stale-main rejection before draft/publish, explicit distribution approval | release-ownership-contract.yml; tauri-next-release.yml; scripts/release_ci_gate.py |

## 4. Findings and release blockers

1. **BLOCKER — Incomplete current-SHA baseline.** Mandatory integration, MCP, CI, UI, installers and live canaries at 517877b are cancelled/not proven. Re-run against the **actual final integration SHA**, not a prior green commit.
2. **BLOCKER — Specialist implementation and integration pending.** Agents 01–06 must supply reviewable PRs; Agent 00 must identify an integration candidate and full SHA. No final PASS matrix exists yet.
3. **RELEASE GATE GAP — Dedicated MCP security workflow not required by automated preview publisher.** On the reference HEAD, scripts/release_ci_gate.py required only integration, security, Windows and Linux. The Agent 07 PR adds MCP E2E as a fifth exact-SHA mandatory gate and runs it on main pushes. The enhanced contract must itself pass CI before adoption. It does NOT replace manual/live tests.
4. **EXTERNAL BLOCKER — Live YouTube provider authentication challenge.** The earlier social canary failed explicitly on YouTube; final candidate needs an honest rerun/recorded access outcome. Do not bypass the platform, change tests to mocks, or report GO for an unproven required canary.
5. **GOVERNANCE RISK — main is not protected.** GitHub branch metadata showed protected=false and repository rulesets were empty at inspection time. Required checks/reviews are not GitHub-enforced merge barriers; Agent 00 must enforce them operationally or configure protection before release promotion.
6. **DOCUMENTATION DRIFT — docs/NEXT_DISTRIBUTION.md still mentions 0.1.0; README.md and docs/VERSIONING.md specify 1.1.0.** Clarify before release-facing publication. This audit does not change the application version or shipping artifacts.
7. **COVERAGE BOUNDARY — Tests do not establish live ChatGPT tunnel authorization, live paid BYOK provider behavior, full interactive Windows WebView journeys, macOS packaging, or exhaustive concurrency/race safety.** Record manual checks explicitly; do not infer them from local fixtures.

**Publishing safety:** The rolling preview publisher has an existing condition requiring the repository variable SCRIPTOTAR_NEXT_DISTRIBUTION_APPROVED to equal 'true'. The value of that variable was not inspected. Agent 07 makes no release, tag, or publish operation and must never merge its own PR.

## 5. Independent agent review requirements

For every agent 01–06 PR, record the exact head SHA, changed files, API/tool count, test implications, security risks, migration and rollback concerns, cancelled/failed/green workflow jobs, and blockers with line-level evidence. Report findings on the responsible PR; do not edit agent-owned branches.

**Initial Agent 01 observation (PR #73):** an async Tauri blocking-worker adapter and linear project summary projection are proposed. Existing API tests need to recognize async commands; Agent 01 adjusted that regex. At inspection time its full integration, MCP, packaging and security runs were cancelled, with Rust foundation still queued and Main CI in progress. **No approval or final rejection is issued until exact-HEAD tests are resolved.** Re-evaluate at review time; later commits invalidate this observation.

## 6. Final candidate handoff contract

Agent 00 must provide:
- Integrated branch and exact immutable 40-character commit SHA, with PRs/merge SHAs for agents 01–06 and Agent 07;
- All required workflow runs bound to that exact candidate commit, with individual mandatory job results and artifact links;
- Real-media/model canary status (PASS/FAIL/BLOCKED by provider), plus explicit credentials and manual-test limitations;
- Security blocker disposition and release publishing approval recorded separately.

Agent 07 will then publish a **PASS / FAIL / BLOCKED** table with every workflow/job/run URL, record unresolved blockers and manual verification, and report **GO** or **NO-GO** to Agent 00. Prior to that, the only honest recommendation is **PENDING final certification**, and the current pinned baseline is **NO-GO**.

**Phase A handoff:** Status IN PROGRESS (baseline captured); ready for Agent 00 to review baseline findings: YES; final release readiness: NO; release recommendation: PENDING final candidate / NO-GO on baseline.
