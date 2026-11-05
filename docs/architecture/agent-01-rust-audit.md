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
- Async scheduling **does not** change SQL access patterns or confer
  cancellation on running provider/subprocess tasks. Concurrent invocation
  behavior (particularly overlapping project selection and watchlist work)
  merits explicit end-to-end stress tests before expanding the offload scope.

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
