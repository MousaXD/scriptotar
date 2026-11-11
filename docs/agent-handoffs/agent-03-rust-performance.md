# Agent 03 — Rust bootstrap / SQLite performance handoff

**Branch:** `agent-03/rust-bootstrap-optimization`
**PR:** https://github.com/MousaXD/scriptotar/pull/92 (draft until exact-HEAD certification)
**Baseline:** `f741244634b5143602a21a554094a498a0005b11`
**Latest implementation and benchmark SHA before this report revision:** `e0cae0e3a6d57605fcfc09a6331ddfca7e13faa0`
**Handoff status:** Rust implementation independently code-reviewed and exact-implementation-HEAD Rust gates passed; frontend integration, supply-chain remediation and independent release QA remain pending. The exact final documentation-only HEAD is recorded in the PR conversation after this update.

## Findings and changes

1. **Transcript bloat.** The full `bootstrap_for` previously selected `t.text`, `t.segments_json`, and `t.words_json` for every transcript and then parsed segments into UI objects, even for the dashboard. New `SqliteStore::list_transcript_metadata(project_id)` selects only IDs, source metadata, language, duration and timestamp. It enforces project isolation inside SQL and preserves `ORDER BY t.created_at DESC`.
2. **All-project job scan.** Bootstrap previously fetched **all** job rows for every project, even though the UI needs only the active project's jobs and project counters. The revised query fetches active-project jobs and `project_job_summaries()` computes counts and lexicographic `MAX(updated_at)` with SQL `GROUP BY`. No index or migration was added.
3. **Duplicated Tauri startup work.** The Tauri `.setup` path and installed-backend smoke previously opened SQLite, ran integration migrations, and recovered interrupted watchlist refreshes before constructing `AppServices`, which independently performed those steps. Both now rely on `AppServices` to perform these safety checks once; the operational Tauri state shares a cloned handle to that recovered database. This preserves the schema, recovery, and permission behavior without redundant startup I/O.
4. **API compatibility.** `bootstrap_app` with **no arguments** still returns the original full `BootstrapData`, including complete text and parsed segments. `bootstrap_app({summary:true})` returns a typed `BootstrapData<UiTranscriptSummary>` with **no** text/segments properties. Both retain the same project, creator, research, job, AI, library, settings, watchlist-status and migration-status behavior at the TypeScript seam. `get_transcript({transcriptId})` continues to return full detail with the existing active-project guard.
5. **Frontend seam.** `ScriptotarApi.bootstrapSummary()` calls the opt-in mode and the mock implements the same metadata-only representation. `tauriClient.test.ts` verifies both old and new invocations. The API inventory test count is updated from 35 to 36 methods; the count of registered Rust commands stays **34**, avoiding a new privileged IPC entry point.
6. **Diagnostics.** Setting `SCRIPTOTAR_PROFILE_STARTUP=1` emits database-open, integration-migration, recovery, service initialization, startup total, bootstrap preparation/assembly, JSON encoding time, and encoded-byte counts. Diagnostic output does not print stored content. `database_open_ms` includes initial core migration/file preparation; `query_ms` is an application-phase wall clock, not an isolated SQLite query microbenchmark. Serialized JSON byte count is a proxy for the wire payload, not an actual WebView transfer-time measurement.

## Tests and reproducible profiling

- Rust service fixture `summary_bootstrap_preserves_metadata_and_project_isolation_without_large_bodies` builds 2- and 128-transcript disposable SQLite workspaces with long text and segments. It prints full/summary wall time and JSON byte counts, checks IDs/order, library parity, absence of body/segments in summary JSON, significant payload reduction and project-isolated detail retrieval.
- `grouped_sql_job_summary_matches_full_history_and_active_project_order` checks the new grouped SQL output against the prior Rust aggregation for multiple projects.
- The existing integrated DB test also checks transcript metadata projections and isolation.
- The Tauri initial setup and installed-backend smoke now construct the already-validating `AppServices` without pre-running the same migrations and recovery a second time. Dedicated installed smoke/integration CI must verify this change.
- Frontend contract test checks `bootstrap_app` and `bootstrap_app({summary:true})` arguments, with the public API inventory mapping all commands and implementations.
- An independent source-level contract check verified 34 registered/implemented/wired Rust commands and 36 public frontend methods with bindings/mock/invocation evidence. This is not equivalent to executing Python, TypeScript or Rust tests.

Reproduce with:

```sh
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --locked -- --nocapture
python -m unittest discover -s tests -p 'test_api_surface_contract.py'
cd apps/desktop-ui && npm ci && npm run check && npm run test && npm run build
# Optional installed/local diagnostic:
SCRIPTOTAR_PROFILE_STARTUP=1 # set in the launching process environment
```

### Measured, reproducible SQL-only before/after

An independent Python 3 SQLite `SELECT + fetchall` microbenchmark was executed against **disposable, synthetic** local databases containing the same relevant table structures and original indexes. Each project has 6,000-word transcript text and 500 repeated segment phrases per transcript, two projects total, and four jobs per transcript in each project. Times are medians of eight **warm** runs; the first iteration is excluded. The Python source for reproducing the methodology is in `crates/scriptotar-db/benchmarks/bootstrap_sql_profile.py` (stdlib only).

| Transcripts per project | Full transcript SQL median | Metadata-only SQL median | Full string data fetched | Metadata string data fetched | Old all-jobs SQL median | New grouped-jobs SQL median |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 2 | 0.039 ms | 0.008 ms | 123,408 characters | 136 characters | 0.022 ms | 0.006 ms |
| 128 | 9.519 ms | 1.185 ms | 7,898,988 characters | 9,142 characters | 1.283 ms | 0.200 ms |
| 1,024 | 93.239 ms | 12.557 ms | 63,196,668 characters | 75,518 characters | 12.122 ms | 1.915 ms |

**Interpretation and limits:** This is a controlled SQLite/Python SQL-path comparison, **not** timings of Rust Tauri bootstrap, on-disk migrations, recovery, actual WebView IPC, or user data. The production Rust integration fixture also records wall times and serialized JSON bytes (2/128 transcripts), but those numbers are captured in test stdout and need a `--nocapture` execution before publication. SQLite `EXPLAIN QUERY PLAN` for both transcript queries uses `idx_sources_project_created`, `idx_media_source` and `idx_transcripts_media`; both currently use a temporary B-tree for `ORDER BY t.created_at DESC`. The grouped-jobs query scans the pre-existing `idx_jobs_project_created`. No index is added without a measured improvement for this query plan.

### Exact-HEAD CI status and defects identified during self-audit

At the time of this report revision the current source HEAD was `e0cae0e3a6d57605fcfc09a6331ddfca7e13faa0`, with eight GitHub Actions workflows started or queued. Completed outcomes on **older commits do not certify this HEAD**:

- Earlier `c658d5f7f46265a275d4e5c14e19c5547b986e70`: Rust foundation failed rustfmt (two whitespace-formatting diffs) and CI/MCP/Linux installed smoke failed `E0716` because a test borrowed from a temporary `Vec<Job>`. Both defects were corrected without suppressing checks.
- Later `b2e168d1e0d4cdcf02e75fb4217d1d80c9c90e92`: `CI`, `Desktop UI` and `Security hygiene` completed successfully; Rust foundation passed rustfmt + workspace cargo check but failed strict Clippy on an `ordinal % 2 == 0` test fixture. That fixture now uses `ordinal.is_multiple_of(2)`. Other concurrent runs were cancelled on newer pushes and cannot count as passes.
- An older Tauri supply-chain job failed `cargo audit` on `RUSTSEC-2026-0459`, `libsqlite3-sys 0.30.1` / bundled SQLite CVE-2026-119375. Dependency lockfile is outside Agent 03 exclusive ownership; no advisory suppression or unapproved dependency change was made. Agent 00/dependency owner must triage and obtain an exact-integrated-SHA security PASS.
- The current container checkout is not a Git worktree and has no Rust toolchain or GitHub DNS access. Rustfmt, Cargo check/Clippy and tests are executed by GitHub Actions for the branch. The Python/SQLite microbenchmark was run locally using disposable fixtures. A source-level recheck confirmed the expected 34 registered/dispatched/wired native IPC handlers and 36 public TypeScript methods, with no missing binding/mock/test evidence.

**Current readiness:** **BLOCKED** pending an exact-final-head full CI pass, integrated UI adoption, agent-owned shared test review, and Agent 00/08 certification. No production user dataset was touched.

## Safety and compatibility notes

No changes to migration definitions, SQL durability settings, project selection, recovery transitions, foreign keys, MCP, media orchestration, authorization or credential storage. Only redundant invocations of the **same** already-enforced migration and recovery operations were removed from the outer startup wrapper. Full `get_transcript` remains available and fails for transcripts outside the active project. The summary query filters project ownership **in SQL** rather than filtering retrieved full bodies.

**Agent 02 / Agent 06:** Adopt `api.bootstrapSummary()` for first-screen startup, use `api.getTranscript(id)` on opening the viewer and preserve loading/error/project-switch behavior. No views or `App.svelte` were modified here. Consider summary variants for project selection/watchlist mutations only after coordinated frontend changes.

**Agent 08:** Independently inspect the shared, one-line `tests/test_api_surface_contract.py` inventory update, validate frontend and Rust contracts, check source-level and installed-app startup scenarios, and capture exact-SHA before/after data. This is the sole change outside strictly agent-owned sources.

**Remaining work:** End-to-end UI adoption, native Rust fixture numbers with `--nocapture`, SQL-level paginated research/library/job lists, connection/migration timings on realistic user-sized fixtures, vulnerability-owner disposition, exact-current-head CI and integrated release certification. The synthetic SQL-only `EXPLAIN QUERY PLAN` and medians above are available but do not replace these gates.

**Release gate:** NO-GO until all applicable exact-commit workflow jobs pass and Agent 00 / independent QA certify the combined branch.
 

## Final Agent 03 remediation and scoped handoff — 2026-11-11

**Existing PR:** https://github.com/MousaXD/scriptotar/pull/92 (kept draft, unmerged).  
**Exact validated implementation commit:** `6332854a0f0aca988c47e4ede0a98e18e01efe45`.  
**Original failing commit:** `6a255efc81e2c3cd663de7b15562a4d2229a2379`.  
**Final branch/doc commit:** recorded by its full SHA in PR #92's final Agent 03 comment; the documentation commit cannot embed its own SHA. Branch ref: https://github.com/MousaXD/scriptotar/commits/agent-03/rust-bootstrap-optimization.

### Final code change and verification

- Fixed exactly the Rustfmt-reported `if ordinal.is_multiple_of(2)` layout in the `seed_transcript_row` **test fixture**, `apps/desktop/src-tauri/src/services.rs`. No behavior, assertions, migrations or security policy changed. Evidence: https://github.com/MousaXD/scriptotar/actions/runs/42985754898.
- Relative to `main`, the implementation PR changes 13 existing/new paths: `apps/desktop-ui/src/api/{client.ts,mockClient.ts,tauriClient.test.ts,tauriClient.ts}`; `apps/desktop-ui/src/types.ts`; `apps/desktop/src-tauri/src/{dto.rs,lib.rs,services.rs}`; `crates/scriptotar-db/{benchmarks/bootstrap_sql_profile.py,src/integration.rs,src/lib.rs}`; this handoff; `tests/test_api_surface_contract.py`. The final remediation code commit only changed `services.rs`, formatting-only.
- **Rust foundation on implementation commit: PASS**, https://github.com/MousaXD/scriptotar/actions/runs/43016307429. This ran, and passed, `cargo fmt --all -- --check`, `cargo check --workspace --all-targets --locked`, `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`, and `cargo test --workspace --locked`. Every Rust test group completed with zero test failures (one pre-existing ignored test in a group of 57).
- **Main CI: PASS**, https://github.com/MousaXD/scriptotar/actions/runs/43016307563, including Python tests (97 passed) and Rust workspace tests. **Desktop UI: PASS**, https://github.com/MousaXD/scriptotar/actions/runs/43016307313. **Security hygiene: PASS**, https://github.com/MousaXD/scriptotar/actions/runs/43016307303. **MCP E2E: PASS**, https://github.com/MousaXD/scriptotar/actions/runs/43016307471. Tauri migration integration's Rust workspace, Rust↔sidecar, Python, and Svelte jobs also passed on implementation SHA; overall completion and packaging/supply-chain outcomes must be checked again on the final documentation-only SHA.
- API seam audit: `bootstrap_app` with omitted `summary` still returns the **full** `BootstrapData<UiTranscript>`; `bootstrapSummary()` explicitly sends `{summary:true}`; `get_transcript` reads full persisted detail and rejects other-project transcripts; existing transcript view calls it and preserves TXT, timestamped TXT, SRT, VTT and JSON exports. No new privileged Rust command was added. The shared API inventory change is **35→36 frontend methods**, not 34→35 Rust commands, and awaits Agent 08's independent test-ownership approval.
- DB audit: metadata query filters `s.project_id = ?1` in SQLite, fetches no transcript body, words or segments, and retains existing `ORDER BY t.created_at DESC`. The existing source locator is deliberately retained (for local sources it can be a file path); the separate media local path is not selected. SQL `GROUP BY project_id` counters and string `MAX(updated_at)` match the old in-memory comparator. Active-project job row ordering remains unchanged. Recovery/migrations continue inside `AppServices` and are no longer duplicated by Tauri setup.
- Profiling is explicitly opt-in via `SCRIPTOTAR_PROFILE_STARTUP`; timings are aggregate wall-clock segments and the serialized JSON length, **not** an instrumented WebView native startup or IPC benchmark; diagnostics do not log transcript content.

### Performance evidence, carefully scoped

The previously captured disposable, **synthetic Python SQLite SQL-only benchmark** (eight warm runs, medians; not real native Rust/Tauri startup) reported:

| Transcripts/project | Full transcript query | Metadata-only query | Full string chars | Metadata string chars | All jobs query | Grouped counters |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 2 | 0.039 ms | 0.008 ms | 123,408 | 136 | 0.022 ms | 0.006 ms |
| 128 | 9.519 ms | 1.185 ms | 7,898,988 | 9,142 | 1.283 ms | 0.200 ms |
| 1,024 | 93.239 ms | 12.557 ms | 63,196,668 | 75,518 | 12.122 ms | 1.915 ms |

No **real native cold-start, WebView/IPC timing or end-user startup improvement** has been measured or claimed. The Rust fixture emits native process benchmark-like diagnostic lines under `--nocapture`, but the current CI runs `cargo test --workspace --locked` with captured output. Agent 00/QA should gather packaged-app cold-start and IPC timings with `SCRIPTOTAR_PROFILE_STARTUP=1` before performance release claims.

### Remaining external integration dependencies and verdict

1. **Agent 00 (frontend integration)** must opt into `bootstrapSummary()` in coordinated view and App changes and retain full detail-on-demand, project switching and AI Studio data contracts. This branch deliberately leaves those frontend views unchanged.
2. **Security/dependency owner** resolves https://github.com/MousaXD/scriptotar/issues/97 (`RUSTSEC-2026-0459` / `libsqlite3-sys 0.30.1`) and validates all integrations with updated manifests/lockfile. No RustSec advisory was ignored or suppressed.
3. **Agent 08 / Agent 00** independently validate combined-source API inventory, exact integrated CI, migration/recovery and installed Windows/Linux startup, and real native performance. Any remaining GitHub Actions status after this handoff must be assessed for the exact **final** PR SHA, not the earlier implementation SHA.

**SCOPED RUST PERFORMANCE VERDICT: GO at validated code commit `6332854a...`**, subject to final documentation-only HEAD CI. **OVERALL RELEASE / MERGE VERDICT: BLOCKED** pending dependent security remediation, frontend summary adoption and integration certification. Do not merge to main from Agent 03.
