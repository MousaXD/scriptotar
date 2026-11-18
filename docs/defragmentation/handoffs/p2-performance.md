# Agent 08 — P2 performance handoff (updated 2026-11-18)

**Branch:** `agent-08/p2-native-performance-20261116`
**Synchronized integration baseline:** `7af95c574643c14565a5c10d0be2cc78c5e57254` via history-preserving merge commit `1223d81d4894561e5fb44e9ba75624083d4166df`.
**Integration:** draft PR #127 remains open; issue #100 remains open. The original six benchmark files already exist byte-identically in integration; do not double-merge or squash historical commits. New work is isolated to benchmark tooling, this handoff, and one separately scoped performance SQL CI workflow; no production code, release flag or native acceptance workflow was edited.

## Delivered

- `study_scale.py`: adds N=100, 1,000, 10,000 synthetic canonical Content Studies, StudySource links, briefs, draft candidates, immutable draft versions, and one ~1 MiB transcript to the existing guarded scratch-only legacy fixture generator. Requires an **actual empty app-migrated v6+** SQLite template and records resulting DB SHA-256/size.
- `query_suite.py`: profiles 11 representative warm read-only SQL shapes (Study search/exact open, brief, sources, draft selector/body/history, research, jobs, transcript metadata/full body). Raw trial timings, p50/p95 nearest-rank, `EXPLAIN QUERY PLAN`, row count, approximate fetched text bytes and DB SHA-256. This is NOT Rust/Tauri or IPC benchmarking.
- `native_trials.py`: repeats the existing installed-executable Linux process sampler, computes distribution package and executable SHA-256, machine CPU/logical-core/RAM metadata, separate declared cold/warm modes, per-trial raw RSS and p50/p95. Requires an isolated scratch `SCRIPTOTAR_DATA_DIR` with a `.scriptotar-benchmark-only` sentinel containing `SYNTHETIC_ONLY`. A native UI-driver/operator marker may supply time to useful workspace; otherwise it remains null.
- `ipc_trace.py`: validates externally collected, explicit installed-WebView IPC JSONL events and computes raw samples and per-command p50/p95 for duration, request bytes and response bytes. Rejects mock provenance and SHA mismatches. This does **not independently authenticate** a purported native trace.
- `test_performance_suite.py`: hermetic synthetic isolation/validation, query and hash, percentile, and fail-closed IPC trace tests.

## 2026-11-18 migration validation and independently repeatable SQL run

The current `scriptotar-db::LATEST_SCHEMA_VERSION` is **12**, not v9. The previous unit tests use a deliberately simplified v9 **stub**, so those five tests cannot validate the current migration, indexes, guards or triggers. Two new, non-production tools address the gap:

- `crates/scriptotar-db/examples/make_blank_perf_template.rs` creates a **new file only** within a sentinel-marked scratch directory using the canonical `SqliteStore::open` and `run_integration_migrations` code paths. It refuses preexisting output, an unmarked directory, and non-absolute output paths.
- `tests/defragmentation/benchmarks/run_migrated_scale.py` requires a clean exact-SHA checkout, invokes that Rust example, seeds all three sizes, runs the 11 warm read-only query shapes, verifies fixture hash stability and writes source/host/schema metadata. It refuses overwrites and cannot upgrade SQL data to native performance certification.
- `.github/workflows/p2-migrated-sql-performance.yml` is a **separate synthetic-SQL-only** Linux/Windows workflow. Its green result would establish that the current migration/schema and tooling execute on both OS runners; it would **not** certify installed WebKitGTK or WebView2 performance. Check exact-head run status before claiming it passed.

Recommended one-command migration smoke from a clean checkout (requires Rust, Cargo, Python 3.12+; produce a new scratch folder each run):

```bash
export SRC_SHA="$(git rev-parse HEAD)"
export PERF_DIR="$(mktemp -d /tmp/scriptotar-perf-qa.XXXXXX)"
printf 'SYNTHETIC_ONLY\\n' > "$PERF_DIR/.scriptotar-benchmark-only"
python3 -m tests.defragmentation.benchmarks.run_migrated_scale \\
  --out-dir "$PERF_DIR" --source-sha "$SRC_SHA" \\
  --sizes 100 1000 10000 --iterations 21
```

Alternatively, create just a canonical blank v12 schema template using:

```bash
cargo run --locked --release -p scriptotar-db --example make_blank_perf_template \\
  -- "$PERF_DIR/blank-migrated.sqlite3"
```

The existing manual `study_scale.py` / `query_suite.py` commands below then accept that template. Use a different scratch directory for each complete run; never reuse the ordinary app data location. The tool returns explicit failure rather than silently falling back to the simplified test schema.

## Native certification and baseline controls still required

**No installed native benchmark evidence has been collected by these changes.** A complete Agent 07-attested run still needs independently observed UI milestones (window visible, Studio usable, project switch completed, draft saved/updated), command-level WebView2/WebKitGTK IPC trace provenance, precise source+package+binary hashes and both-platform installed application logs. Prior installed acceptance artifacts prove functionality, **not** these latency distributions. Do not count process alive, a self-declared marker or a headless/mock driver as native UI readiness.

Capture cold and warm startup separately, with cache treatments described precisely, minimum 10 measured native runs per platform/size where practical (report sample count for every excluded condition), raw p50/p95 distributions, resident/private memory caveats, and matched hardware details. Compare the **same released package** baseline against the proposed build under the same host/OS and measurement method. Use alternating A/B trial order where possible; report percent deltas alongside raw observations and do not infer an optimization from environment noise. The SQL-only workflow is not this A/B test.

**Decision as of this update:** **NO-GO for native performance certification.** The executable/IPC/UI timing and Windows/WebView2 memory measurements remain unverified. Keep PR draft; Agent 00 owns any production code changes after an evidenced bottleneck and separate approval.

## Reproducible commands

Requires Python 3.12+ and a **blank schema generated by Scriptotar's real current (v12) migrations**. No user data may be used. Generate separately from the installed application into a new scratch location; the app's production data directory is never a valid template.

```bash
export SRC_SHA="$(git rev-parse HEAD)"
mkdir -p /tmp/scriptotar-perf-qa
python3 -m tests.defragmentation.benchmarks.study_scale \
  --template /absolute/path/blank-migrated.sqlite3 \
  --out-dir /tmp/scriptotar-perf-qa --source-sha "$SRC_SHA"
for N in 100 1000 10000; do
  python3 -m tests.defragmentation.benchmarks.query_suite \
    --db "/tmp/scriptotar-perf-qa/scriptotar-synthetic-$N.sqlite3" \
    --source-sha "$SRC_SHA" --iterations 21 \
    --output "/tmp/scriptotar-perf-qa/sql-$N.json"
done
python3 -m unittest tests.defragmentation.benchmarks.test_performance_suite -v
```

**Installed process measurements** (Linux): use the exact packaged binary and corresponding .deb, not a debug build. Place the synthetic app DB in its correct **current** `SCRIPTOTAR_DATA_DIR` location before running. An empty or wrongly named database means the package is not measuring the seeded workload. Do not claim process liveness equals workspace readiness.

```bash
mkdir -p /tmp/scriptotar-perf-qa/ready /tmp/scriptotar-perf-qa/app-data
printf 'SYNTHETIC_ONLY\n' > /tmp/scriptotar-perf-qa/app-data/.scriptotar-benchmark-only
python3 -m tests.defragmentation.benchmarks.native_trials \
  --package /absolute/path/to/real-package.deb \
  --executable /absolute/path/to/installed/scriptotar-binary \
  --source-sha "$SRC_SHA" --mode warm --cache-state warm-cache \
  --trials 5 --seconds 30 --ready-dir /tmp/scriptotar-perf-qa/ready \
  --data-dir /tmp/scriptotar-perf-qa/app-data \
  --output /tmp/scriptotar-perf-qa/native-warm.json
# Run cold separately, declare and document a defensible cache state.
```

A unique ready-marker path is printed for each trial. Only create it after an independently observed *useful Studio workspace*, not the splash screen or process start. Cache labels are operator assertions; page-cache clearing, actual installation, and real UI provenance need independent review. Linux summed process-tree RSS double-counts shared pages; Windows child RSS is not implemented.

**External native IPC trace format** (each line JSON; never include raw requests/responses, transcript text, headers or secrets):

```json
{"evidence_kind":"installed-tauri-webview-ipc","source_sha":"<40 hex>","package_sha256":"<64 hex>","executable_sha256":"<64 hex>","session_id":"opaque-session-id","command":"get_studio_study","duration_ms":12.5,"request_bytes":42,"response_bytes":2400}
```

```bash
python3 -m tests.defragmentation.benchmarks.ipc_trace \
  --trace /path/to/independent-installed-driver.jsonl \
  --source-sha "$SRC_SHA" --package-sha256 "$PKG_SHA256" \
  --executable-sha256 "$BIN_SHA256" \
  --output /tmp/scriptotar-perf-qa/native-ipc.json
```

Agent 07 must attest exact installed WebView identity, UI readiness, editor scrolling/input, draft-candidate switching, research view interactions, and whether timestamps/bytes cover both sides of the native IPC boundary. JSONL merely declaring native origin is not proof.

## Evidence and candidates for *future* optimization

**Local tooling smoke only:** Five hermetic unit tests passed using a minimal local test schema and a local mirror of the existing seed generator. All three sizes (100, 1,000, 10,000) generated and completed 11 SQL query shapes on that minimal schema. Those data are **NOT** a real application baseline, not installed native package measurements, and not exact-head GitHub CI. The new tests remain to be run against the real repository and migrated schema in CI.

**Inspection hypotheses (not measured bottlenecks):** `study_gateway.rs::list_studio_studies` makes up to 200 separate `get_creative_brief` calls after metadata search; `studies.rs::search_studies` uses a leading-wildcard `LIKE` query; `read_studio_bundle_for` loads all selected draft versions even though the candidate selector is metadata-only. Compare genuine native traces and SQL plans before altering any of these.

## Agent 00 decisions / remaining gates

1. The blank app-migrated v12 template creation is now reproducible via the Rust example. Still confirm the actual installed-app DB filename/placement before attempting native workload measurements.
2. Coordinate with Agent 07 on a real Linux/Windows installed WebView driver that emits attested useful-workspace, IPC and editor-response telemetry. No browser-mock metrics are acceptable as native.
3. Grant specific shared-file ownership **only after** real p95/EXPLAIN/RSS evidence isolates a bottleneck. No performance improvements are claimed by this PR.
4. Require updated exact-commit Rust/frontend/security/MCP/package CI plus independent installed native QA and release approval.

**Status:** DRAFT / NO-GO. No final native p50/p95 results, real binary hashes, package hashes, verified hardware baseline or post-fix speedups are available here. Issue #100 remains open.
