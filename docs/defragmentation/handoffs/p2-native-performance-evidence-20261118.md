# Agent 08 — independent native performance evidence report (2026-11-18)

**Decision: DRAFT / NATIVE CERTIFICATION NO-GO.** This is an evidence audit and collection contract, **not** a report of new installed desktop measurements. Do not publish synthetic SQL timing or GitHub job wall time as Studio/WebView latency. No production Rust, Tauri, Svelte or SQLite application code was changed in this measurement work.

## Verified provenance and available raw evidence

- Source for migrated SQL smoke: `01d915574cb117e3bbc47480efb96030097e735f`, [exact workflow 44299533922](https://github.com/MousaXD/scriptotar/actions/runs/44299533922). Jobs **PASS** on `ubuntu-22.04` and `windows-latest`. Each executed current Scriptotar Rust migrations followed by synthetic 100, 1,000, 10,000 Study SQLite query profiles. This verifies the SQL fixture generator and query tools, not installed app throughput.
- Linux SQL-only artifact **14425855103**, ZIP digest `sha256:13a578b3e6cefb29d3e46c23b1b1f344a430ca3ae53d54cbdc46f794a8f06a50`. Windows SQL-only artifact **14423834139**, ZIP digest `sha256:2d450a4d7dcaf8ffb2cf1ec7b7c4d44b133a1e16ad5372c8abea62f77d61176e`. Both are attached to the exact workflow; their ZIP SHA-256 digests are **not** package or executable checksums.
- Real installed native QA source: `9c272a92de8df42a1ecfee5211a4f7ea3bb74e44`, [Agent 07 workflow 44302808654](https://github.com/MousaXD/scriptotar/actions/runs/44302808654). Linux WebKitGTK and Windows WebView2 acceptance both concluded **FAILURE**. Its artifacts prove that installed-package GUI functional testing was attempted; they have **not** been adjudicated as independent quantitative native performance samples. Linux installed artifact ID **14427273474**, Windows installed artifact ID **14427229971**; smaller review archives **14427142757** and **14428460649**.
- The SQL run and native QA run use **different source SHAs**. No direct package-to-package comparative speedup claim is possible. No native benchmark raw JSONL or package/executable hash-matched latency distribution has been verified here.

## Independent raw SQL p50/p95 appendix (synthetic; not installed native)

The original green Linux/Windows migrated SQLite artifacts are now independently downloaded, ZIP SHA-256 verified and analyzed at all 100/1,000/10,000 scales. See [SQL artifact analysis with raw p50/p95, source hashes and EXPLAIN plans](p2-sql-artifact-analysis-20261118.md). At 10,000 synthetic Studies, warm Python SQLite Study search takes **18.0529 / 18.1564 ms** Linux p50/p95 and **22.3999 / 27.4326 ms** Windows; transcript metadata **11.7849 / 12.0149 ms** Linux and **17.6649 / 18.3786 ms** Windows (11 samples/query). Both queries require temporary B-tree ORDER BY according to archived query plans. These are **not** installed GUI/IPC latencies, and OS hardware/SQLite versions are not matched.

## Independent native measurement matrix

| Measurement | Linux WebKitGTK | Windows WebView2 | Missing direct hook / evidence |
| --- | --- | --- | --- |
| Cold app launch to visible window | NOT TESTED | NOT TESTED | Independent native window observation with monotonic launch time, explicit cache treatment |
| Warm app launch to visible window | NOT TESTED | NOT TESTED | Repeated native/window observations on unchanged package and profile |
| Useful Studio readiness | NOT TESTED | NOT TESTED | Native accessibility verifies actual Studio content and actionable controls, not a process-ready marker |
| Project switching | NOT TESTED | NOT TESTED | Per-action native input timestamp and verified target dashboard render |
| Opening Study / 100, 1k, 10k | NOT TESTED | NOT TESTED | Native action completion on installed app with verified seeded canonical DB and exact item count |
| Draft edit, save, close, reopen | NOT TESTED | NOT TESTED | Native editor interaction + canonical read-only save verification |
| Source/transcript navigation | NOT TESTED | NOT TESTED | Genuine source fixture, real WebView action completion, no forged provider content |
| Tauri IPC per-command latency/payloads | NOT TESTED | NOT TESTED | Trusted instrumentation/QA export with actual command boundary timestamps and payload lengths |
| Linux resident PSS / private memory | NOT TESTED | N/A | Newly added `smaps_rollup` read-only sampler must run against installed process and descendants |
| Windows private working set / private commit | N/A | NOT TESTED | Newly added Win32 `PROCESS_MEMORY_COUNTERS_EX2` sampler must run against installed WebView2 process tree |
| Comparable installed A/B trial p50/p95 | NOT TESTED | NOT TESTED | Same hardware, workload, cache state, OS build, measurement hook and package provenance |

**No native p50/p95 or native memory values are available.** Do not fill these cells with synthetic SQL numbers, zeros or mocked browser results. Native sample count for every listed metric is currently **0 verified samples**.

## New independent collection implementation

- `tests/defragmentation/benchmarks/native_memory.py`: Linux reads `/proc/<pid>/smaps_rollup` PSS and private-clean/private-dirty bytes on current descendants, and Windows uses Toolhelp32 process enumeration plus PSAPI `PROCESS_MEMORY_COUNTERS_EX2` for private working set (resident) and private commit (not the same thing). Unknown, failed access, unsupported Windows builds or missing children produce `null` tree totals, not partial sums disguised as accurate memory.
- `native_session.py`: preserves legacy shared-inclusive RSS but also writes **raw timestamped memory observations**, sample completeness, peak main/tree PSS and private resident/commit measures. It is still process sampling, not UI-attested proof.
- `native_trials.py`: aggregates trial p50/p95 for new memory metrics without silently coercing unavailable results to zero.
- `test_performance_suite.py`: tests unit conversion, process ancestry, incomplete-tree rejection and explicitly non-certified process snapshots.
- This instrument must be executed as part of an **independently attested installed-app session**; code presence and hermetic unit tests establish neither correct Windows telemetry on a host nor actual installed app memory values. Windows EX2 requires a supported patched OS (see [Microsoft PSAPI EX2 reference](https://learn.microsoft.com/en-us/windows/win32/api/psapi/ns-psapi-process_memory_counters_ex2)).

## Attaching memory sampling to Agent 07's real installed app (new)

`native_attach.py` is benchmark-only. Agent 07 must obtain the PID **directly from the installed native driver** after launching the real signed/packaged executable and keep the GUI session open while the sampler runs. It does not spawn/kill Scriptotar, drive the UI, alter the database, or certify a window. It validates the kernel-reported running image against the path supplied for the installed executable, with no permission to label an arbitrary process or debug build as certified.

Linux example (run in parallel with independent AT-SPI UI interaction after Agent 07 has installed the exact .deb):

```bash
python3 -m tests.defragmentation.benchmarks.native_attach \
  --pid "$INSTALLED_APP_PID_FROM_AGENT07" \
  --executable "$INSTALLED_APP_BINARY" --package "$EXACT_DEB" \
  --source-sha "$EXACT_HEAD_SHA" --study-count 100 \
  --run-url "$EXACT_NATIVE_QA_RUN_URL" \
  --seconds 30 --interval-seconds 0.25 \
  --output "$FRESH_EVIDENCE_DIR/native-100-memory.json"
```

Windows: the same command is supported under Python 3.12+ using `--pid` from UI Automation and the exact NSIS-installer/installed executable paths. No Linux /proc assumptions are applied on Windows.

This stores kernel-probed image identity, process-tree memory observations, package/binary digests, raw samples, p50/p95, omissions and sample completeness. A complete process-tree total requires **every observed descendant** to have readable data. When a child is inaccessible, that total is null, not a partial sum. Results remain `PENDING_INDEPENDENT_NATIVE_QA` and must be tied to Agent 07 UI milestone evidence and a read-only verification that the running app uses the provided Study count. Trials measured with an attached package whose source does not match the UI package are invalid.

These are `process memory` measurements only. Process-tree ancestry can miss orphaned/reparented WebView helpers. If that occurs, Agent 07 must inventory/identify WebView helper PIDs and explain whether OS-level accounting is complete; do not silently count only the parent.

## Exact collection contract for Agents 07 and 00

1. Agent 07 owns installed native driver interaction. Supply **real** monotonic input/completion UI event pairs for open/usable Studio, select project, open Study, edit/save/reopen candidate and source navigation. Assert actual native WebKitGTK AT-SPI or Windows UI Automation document and render state for each observation. Record any skipped case as `NOT TESTED`.
2. Agent 00 must verify the canonical installed app DB filename/path and approval of a disposable, explicitly synthetic seeded `SCRIPTOTAR_DATA_DIR`. Confirm that the installed process truly opens the 100, 1,000 or 10,000 Study **app-migrated** fixture, with read-only row-count/schema checks and hash evidence. A separate SQL fixture existing on disk does not establish a native workload.
3. Record OS/version, machine CPU, physical RAM, WebView runtime, package SHA-256, installed executable SHA-256, exact source SHA, app build flags, cache policy, synthetic fixture hashes, full input timestamps and raw outcomes. Run at least 10 comparable successful trials per platform/size/cache condition where feasible. Record warm and cold separately. Use `native_trials.py` for process/memory data; do not treat its optional operator marker as independently verified useful UI readiness.
4. For IPC, Agent 07/00 must establish trustworthy event provenance. Existing `ipc_trace.py` checks syntax/hash consistency and computes p50/p95, but its JSONL `evidence_kind` field is **self-asserted**, not cryptographic or UI proof. Do not log transcript/secret content, tokens, headers or raw IPC payload.
5. Preserve all raw per-trial samples and failures. Summarize p50 and nearest-rank p95 and sample sizes; prefer paired, alternated A/B on the same hardware with identical package version and instrumentation. Distinguish performance improvement from differences in cache, runner machine class, CI load or source SHA.
6. If required hooks cannot be produced on a real installed app, leave metrics `NOT TESTED`, state the blocker, and keep PR #127 draft.

## Profiling hypotheses; not measured regressions

- **First to inspect if Studio-open p95 is high:** `study_gateway.rs::list_studio_studies` can fetch individual briefs after listing study metadata, creating a potential query fan-out. Check exact native IPC duration and `EXPLAIN QUERY PLAN` before a batching proposal.
- **Next if search p95 grows with scale:** `studies.rs::search_studies` contains a leading-wildcard `LIKE` shape. Use real query plans and selectivity on app-migrated fixtures to determine whether indexing/search design matters.
- **Next if candidate switching or reopening p95 is high:** `read_studio_bundle_for` may load draft versions not needed by selector views. Confirm actual IPC response byte distributions and render profiles before proposing pagination/lazy loading.
- **Memory-specific:** verify installed WebView/WebKit helper process ancestry and distinguish private resident bytes from private commit; shared-inclusive RSS can inflate naive process-tree totals.

**Production optimization requests: none.** These are inspection targets only, not established bottlenecks. Agent 00 owns production modifications and merge approval. Preserve every individual commit; no squash or release merge while certification is incomplete.
