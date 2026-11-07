# Agent 01 — Rust architecture and bootstrap performance audit

**Repository:** `MousaXD/scriptotar`  
**Baseline ref:** `feature/chatgpt-mcp-ci-20261026`  
**Baseline SHA (2026-11-05 inspection):** `517877b26df9028cdd32af69f6f5a88a68b948e9`  
**Working branch:** `agent-01/rust-architecture-overhaul`

## Scope and preservation constraints

Read `README.md`, `CONTRIBUTING.md`, `docs/architecture/rust-foundation.md`,
Rust crate manifests, the desktop service/IPC/DTO implementations, and relevant
tests/workflows. No `AGENTS.md` exists at this baseline. Existing Tauri commands,
the `ScriptotarApi` seam, on-disk persistence, the sidecar contract, Scriptotar
Classic, and release workflows remain unchanged by Agent 01.

The Cargo production dependency direction generally matches the documented
boundaries: `scriptotar-core` has no DB or Tauri dependency; `scriptotar-db`,
`scriptotar-jobs`, and the orchestration/research adapters depend inward on
domain contracts; `apps/desktop/src-tauri` composes the application. Test-only
dependencies from domain adapters to SQLite fixtures are deliberate.

## Confirmed findings

| Finding | Evidence | Severity | Disposition |
| --- | --- | --- | --- |
| Full-bootstrap project metrics repeatedly scan *all* jobs once **per project**. | `AppServices::bootstrap_for` previously ran `all_jobs.iter().filter(...)` inside `projects.iter().map(...)` in `apps/desktop/src-tauri/src/services.rs`. | Medium (scales with workspace size) | **Fixed in this branch**. Added a single linear aggregation in `services/project_jobs.rs`. |
| Full bootstrap materializes complete jobs, transcripts (including text/segments), research items, AI runs, creators, and watchlists, even when only a project header changes. | `AppServices::bootstrap_for` calls all `list_*` methods and serializes `BootstrapData`; `create_project`, `select_project`, and `save_watchlist` return that whole DTO. | High potential latency/memory cost for large libraries; **not measured** | **Open, cross-agent**. Needs DB-owned SQL summary/list pagination plus frontend/API contract negotiation. Do not silently shrink existing `bootstrap_app` payload. |
| Tauri IPC originally used synchronous handlers for large bootstrap reads and long-running network/provider calls. | The base SHA has `#[tauri::command] fn bootstrap_app`, `scan_creator`, and `run_ai` calling blocking service methods. | Medium potential latency / host thread pressure; **not measured** | **Partially fixed**. Eight expensive IPC commands now use an async adapter and `tauri::async_runtime::spawn_blocking`; other host commands and background cancellation remain future work. |
| The SQLite store opens/configures a connection per operation. | `SqliteStore::connection` in `crates/scriptotar-db/src/lib.rs`. | Opportunity; **not yet proven a bottleneck** | **DB owner** to profile connection initialization and decide whether connection reuse is justified. Do not introduce shared locks without evidence. |
| UI API release tests enumerate all Tauri commands and frontend methods. | `tests/test_api_surface_contract.py` requires exact command counts and matching `tauriClient`, `mockClient`, and invocation tests. | Compatibility safeguard | **Preserved**. Do not add unpaired IPC commands; coordinate any paginated interface with the UI agent. |

## Implemented improvement: linear project-job aggregation

A separate, small, pure helper, `summarize_project_jobs(&[Job])`, computes
`item_count` and the lexicographically latest `updated_at` for every project
with jobs **in one pass**. The original implementation filtered every job for
each project and allocated a temporary job-reference vector for every project.

- **Before:** O(projects × jobs) aggregation time, plus O(jobs) transient
  references per project.
- **After:** O(jobs + projects) time for aggregation plus UI project mapping,
  with O(projects-with-jobs) extra summary memory.
- No changes to the SQL result set, job state handling, serialization format,
  project ordering, latest-update comparison, empty-project fallback, or
  command registration.
- Regression tests cover empty workspaces, multiple projects, unsorted job
  timestamps, and project isolation.
- These are asymptotic bounds from inspected implementation, **not**
  measured runtime benchmarks. The remaining full-data fetch and transcript
  parsing still dominate some large-workspace cases.

## Async IPC service adapters

Eight handlers (`bootstrap_app`, `list_jobs`, `get_transcript`,
`select_project`, `create_project`, `save_watchlist`,
`scan_creator`, `run_ai`) keep validation and their public names,
arguments, return payloads, and errors while calling the blocking service
work on a Tauri background blocking worker. This prevents long disk,
subprocess, and AI/provider operations from executing inline in those IPC
handlers.

- `on_blocking_worker` preserves `Result<T, String>` and converts a
  panicked/cancelled worker join into a generic error rather than exposing
  arbitrary panic text as a user-visible error.
- The frontend API contract remains unchanged; the release-inventory regex
  now recognizes `#[tauri::command] async fn` in addition to sync handlers
  without reducing the expected number of commands.
- A Rust regression test checks off-caller-thread execution and correct
  success/error propagation.
- A service-level concurrency regression test starts three threads together
  against cloned `AppServices`: repeated selections, a concurrent project
  creation, and watchlist saves plus local Copy Prompt AI runs. It asserts
  valid project ownership for all watchlists/AI runs, matching persisted and
  in-memory active project after completion, stable response project IDs and
  no API-key leakage into saved prompts. This is deterministic local fixture
  coverage, not a network/provider or measured performance benchmark.
- Async scheduling **does not** change SQL access patterns or confer
  cancellation on running provider/subprocess tasks. Concurrent third-party
  AI calls and live research subprocesses remain outside this deterministic
  test and need further owner-specific stress/timeout/cancellation coverage.

## Future typed queries / pagination — coordination contract

Keep existing unbounded `bootstrap_app` semantics while introducing
**additive, opt-in** paged backend APIs. Proposed next step to Agent 02 (DB)
and the UI agent:

1. Define typed request/response DTOs for project-scoped job summaries and
   bounded transcript/search lists, including explicit stable sorting.
2. Implement **SQL-level** keyset pagination (or bounded, indexed OFFSET
   pagination where justified); do not fetch all rows then slice in Rust.
3. Add a compatible Tauri command and extend `ScriptotarApi`, Tauri and mock
   clients, invocation-contract tests, and existing command-inventory tests
   in the *same coordinated change*.
4. Keep full transcript text/segments behind `get_transcript` for new views
   while preserving the original legacy bootstrap API until consumers migrate.
5. Add large-library fixture tests, query-plan/index checks, and real CI
   timings before making performance claims.

This avoids cross-agent database and frontend conflicts and prevents an
unvalidated public API from bypassing release coverage.

The current change does not add any paginated public APIs: SQL-level backing
queries and paired frontend contract tests remain required before such APIs
can be honestly advertised as scalable.

## Validation obligations

The PR must run the existing **Rust foundation** workflow (Rustfmt, workspace
check, strict Clippy, workspace tests) and the repository **CI** workflow
including frontend API contracts. Agent 01 must inspect the final-commit
GitHub Actions job results before reporting DONE. The current implementation
does not require a migration or dependency/lockfile change.

## External CI blockers observed on 2026-11-05

The original exact-head checks on `e34ce42750f224b0ead62be5cea40a61efc23174`
passed Rust foundation (fmt, check, strict Clippy, tests), full CI/API
contracts, MCP E2E, security hygiene and the Windows installer. A successful
old-head run does **not** certify subsequent commits.

Two failures are outside Agent 01's service/IPC ownership and must remain
fail-closed until their respective owners remediate and revalidate:

- **Linux Tauri package**: `42083551028` failed in the Python runtime
  legal-bundle step. CPython 3.12.16 was outside the reviewed fallback
  runtime-license version list. Media/packaging owner must verify the
  exact license provenance and include it; never simply ignore the check.
- **Integration Supply chain**: `42083550770` failed `cargo audit` on
  `RUSTSEC-2026-0381`, identifying `webpki-roots 1.0.9` (bundled
  distrusted root intermediate). The advisory specifies `>=1.0.10`.
  This exact 1.0.9 version was already present in the baseline
  `517877b26df9028cdd32af69f6f5a88a68b948e9` `Cargo.lock`.
  Agent 01 received a cross-agent request from Agent 04 to resolve this
  shared Rust dependency blocker. The branch now updates **only** the
  `webpki-roots` 1.0.9 lockfile entry to 1.0.10, including the verified
  crates.io index checksum. Both releases have identical **normal**
  dependency requirements (`rustls-pki-types ^1.8`); no package manifest,
  security ignore rule, or runtime policy was changed. CI must still confirm
  the exact HEAD passes `cargo audit` and `cargo --locked` build/test gates.

Integrated Tauri compilation was still in progress at inspection time.
Agent 01 did **not** modify the Python sidecar, CPython licensing rules,
release script, or security policy. The one-package lockfile patch is a
security fix, not an advisory suppression. The PR remains unmerged and
blocked until exact-HEAD checks pass and independent QA provides review.

## Verified Rust dependency remediation

For `webpki-roots` 1.0.10, official `rust-lang/crates.io-index`
(`we/bp/webpki-roots`) records checksum
`1c1c8e20c8a38fc7d65abfe74eb7280e4aa667e8eec8bc84996d2874810748d9`.
The index declares the release non-yanked with minimum Rust 1.71,
and retains exactly the same normal dependency requirement as 1.0.9.
The security advisory `RUSTSEC-2026-0381` identifies 1.0.10 as the
minimum fixed release. `Cargo.lock` now uses this precise package
version and checksum, leaving other dependency entries untouched.

This is source/index verification, **not** substituted for
`cargo audit`, reproducible `--locked` builds or installer
acceptance evidence on the new exact PR HEAD.
