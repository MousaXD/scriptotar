# Agent 03 — Rust bootstrap / SQLite performance handoff

**Branch:** `agent-03/rust-bootstrap-optimization`
**PR:** https://github.com/MousaXD/scriptotar/pull/92 (draft until exact-HEAD certification)
**Baseline:** `f741244634b5143602a21a554094a498a0005b11`
**Last implementation SHA before this report:** `f566e69e844df3cb1c786887df3e748050c87020`
**Handoff status:** Implementation pushed; CI certification and frontend adoption pending. The PR head is the authoritative final SHA after documentation edits.

## Findings and changes

1. **Transcript bloat.** The full `bootstrap_for` previously selected `t.text`, `t.segments_json`, and `t.words_json` for every transcript and then parsed segments into UI objects, even for the dashboard. New `SqliteStore::list_transcript_metadata(project_id)` selects only IDs, source metadata, language, duration and timestamp. It enforces project isolation inside SQL and preserves `ORDER BY t.created_at DESC`.
2. **All-project job scan.** Bootstrap previously fetched **all** job rows for every project, even though the UI needs only the active project's jobs and project counters. The revised query fetches active-project jobs and `project_job_summaries()` computes counts and lexicographic `MAX(updated_at)` with SQL `GROUP BY`. No index or migration was added.
3. **API compatibility.** `bootstrap_app` with **no arguments** still returns the original full `BootstrapData`, including complete text and parsed segments. `bootstrap_app({summary:true})` returns a typed `BootstrapData<UiTranscriptSummary>` with **no** text/segments properties. Both retain the same project, creator, research, job, AI, library, settings, watchlist-status and migration-status behavior at the TypeScript seam. `get_transcript({transcriptId})` continues to return full detail with the existing active-project guard.
4. **Frontend seam.** `ScriptotarApi.bootstrapSummary()` calls the opt-in mode and the mock implements the same metadata-only representation. `tauriClient.test.ts` verifies both old and new invocations. The API inventory test count is updated from 35 to 36 methods; the count of registered Rust commands stays **34**, avoiding a new privileged IPC entry point.
5. **Diagnostics.** Setting `SCRIPTOTAR_PROFILE_STARTUP=1` emits database-open, integration-migration, recovery, service initialization, startup total, bootstrap preparation/assembly, JSON encoding time, and encoded-byte counts. Diagnostic output does not print stored content. `database_open_ms` includes initial core migration/file preparation; `query_ms` is an application-phase wall clock, not an isolated SQLite query microbenchmark. Serialized JSON byte count is a proxy for the wire payload, not an actual WebView transfer-time measurement.

## Tests and reproducible profiling

- Rust service fixture `summary_bootstrap_preserves_metadata_and_project_isolation_without_large_bodies` builds 2- and 128-transcript disposable SQLite workspaces with long text and segments. It prints full/summary wall time and JSON byte counts, checks IDs/order, library parity, absence of body/segments in summary JSON, significant payload reduction and project-isolated detail retrieval.
- `grouped_sql_job_summary_matches_full_history_and_active_project_order` checks the new grouped SQL output against the prior Rust aggregation for multiple projects.
- The existing integrated DB test also checks transcript metadata projections and isolation.
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

**Observed before/after timings:** Not yet verified. CI was queued when this report was created, and the current environment has no Rust toolchain or network git clone access. Do not invent measured speed-ups or treat queued CI as PASS; use the exact-head Actions logs and fixture output when workers execute.

## Safety and compatibility notes

No changes to migrations, SQL durability settings, project selection, recovery transitions, foreign keys, MCP, media orchestration, authorization or credential storage. Full `get_transcript` remains available and fails for transcripts outside the active project. The summary query filters project ownership **in SQL** rather than filtering retrieved full bodies.

**Agent 02 / Agent 06:** Adopt `api.bootstrapSummary()` for first-screen startup, use `api.getTranscript(id)` on opening the viewer and preserve loading/error/project-switch behavior. No views or `App.svelte` were modified here. Consider summary variants for project selection/watchlist mutations only after coordinated frontend changes.

**Agent 08:** Independently inspect the shared, one-line `tests/test_api_surface_contract.py` inventory update, validate frontend and Rust contracts, check source-level and installed-app startup scenarios, and capture exact-SHA before/after data. This is the sole change outside strictly agent-owned sources.

**Remaining work:** End-to-end adoption, actual CI benchmark output, SQLite `EXPLAIN QUERY PLAN` evidence for large datasets, SQL-level paginated research/library/job lists, connection/migration profiling on realistic user-sized fixtures, and exact integrated release certification. No claim that these are complete.

**Release gate:** NO-GO until all applicable exact-commit workflow jobs pass and Agent 00 / independent QA certify the combined branch.
