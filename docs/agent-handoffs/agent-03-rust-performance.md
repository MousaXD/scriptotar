# Agent 03 — Rust bootstrap / SQLite performance handoff

**Branch:** `agent-03/rust-bootstrap-optimization`
**PR:** https://github.com/MousaXD/scriptotar/pull/92 (draft until exact-HEAD certification)
**Baseline:** `f741244634b5143602a21a554094a498a0005b11`
**Latest implementation and benchmark SHA before this report revision:** `e0cae0e3a6d57605fcfc09a6331ddfca7e13faa0`
**Handoff status:** Implementation pushed; CI certification and frontend adoption pending. The PR head is the authoritative final SHA after documentation edits.

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
