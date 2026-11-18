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


## Independent exact-head checkpoint: 2026-11-18 — SQL PASS, installed performance NOT TESTED

This section supplements, but does not change, the draft/NO-GO native verdict above.

- **Agent 08 branch source:** `90f603bc936d405b8fd5fffe872e250e8aa55619`, [completed exact-head SQL workflow #44314062220](https://github.com/MousaXD/scriptotar/actions/runs/44314062220). Linux `ubuntu-22.04` and Windows `windows-latest` **PASS** current Rust migrations plus synthetic 100/1,000/10,000 Study SQLite queries, and both publish provenance-bearing SQL-only JSON artifacts. Linux artifact **14432670100** (published ZIP digest `sha256:361a84b85398a366e50b09ab3a58ddfca69fe1c82515255a68280f8f2eb7dbcb`), Windows artifact **14430674367** (`sha256:e5e44825c6e2196d25d3915973fd4921cc0a1542a18f82ed114c907809f21ef8`). These digest values are GitHub artifact metadata, **not** an independently rehashed copy in this checkpoint. Neither artifact is a signed installed app or native performance trace.
- Other workflows at this exact source SHA: **PASS** general CI `44314062296`, Rust foundation `44314062185`, security hygiene `44314062175`, Linux Tauri package `44314062244`, Windows installer `44314062200`; **FAIL** Tauri migration integration `44314062399`. Packaging job success does not demonstrate measured, installed UI performance or clear the unrelated supply-chain release gate.
- **Most recent independently inspected installed Agent 07 run:** [#44313593525](https://github.com/MousaXD/scriptotar/actions/runs/44313593525) at **different** QA source `6981df8c1bf7592d083d9d3bd011f258755b8850`; both installed OS acceptance jobs **FAIL**, but both reach actual UI automation. Linux fails the unsupported Queue URL named AT-SPI alert assertion; Windows passes its negative-path UIA checks while five mandatory cases remain NOT TESTED. This run does **not** invoke `native_attach.py`, does not stage matching 100/1k/10k data into installed app, and has **zero** verified repeated native action or private-memory performance trials.
- The Windows UIA job log contains one `readiness.elapsed_seconds=4.1` value on its first successful `Document + Studio` poll (`poll_count=1`). Source inspection shows this is a coarse two-second-interval stopwatch probe with a basic Studio control, **not** confirmed actionable workspace readiness, not a cold/warm controlled trial, and not p50/p95. Do **not** promote this one observation into native performance certification or compare it against SQL timings.

**Next non-blocking coordination:** When Agent 07 has the native P0 QA pathway stable, collect independent AT-SPI/UIA event pairs for the requested journeys (installed launch; actionable Studio; project/Study/candidate navigation; save and read-back), and run the existing attach-only memory observer concurrently on the exact installed package. Agent 00 must confirm the disposable app-migrated database location and study counts. Require >=10 comparable successful repeat trials per platform/size/cache condition where feasible, monotonic raw samples and observed failures, package/executable/fixture SHA-256, OS/CPU/RAM/WebView runtime, and an explicit exact-source relationship. Accept authentic IPC distributions only when both command boundaries are instrumented; otherwise label IPC NOT TESTED. **No production optimization authorized; PR #127 stays DRAFT.**

## Fresh integration, independent SQL artifact verification and UI trace contract

Agent 08 integrated source 5ed385d09092d2c94a1bd86cb56888d96df0bb5e using the normal two-parent merge commit 432d7b157ab97cb4da0d9944d466bb06e9b0698c, preserving both Agent 08 history and integrated production fixes. Comparison from the common ancestor showed disjoint Agent 08 benchmark/handoff and Agent 00 production changes; no production files were overwritten.

### Independently checked complete SQL ZIPs, 2026-11-18

[Workflow #44332442668](https://github.com/MousaXD/scriptotar/actions/runs/44332442668) finished PASS on Linux and Windows at exact source 873da2395610f0b6fe3f862643db862ea8d0661c. Downloaded artifact ZIP bytes were independently SHA-256 matched to GitHub artifact metadata, parsed, and checked against all 11 raw trial samples per query/size/OS, recalculating medians and nearest-rank p95. Fixture/report checksums, Study counts and migration schema v12 were validated for all three sizes.

- Linux SQL-only artifact 14439517488, SHA-256 120828fe75f3c58905ea66e02c2d973db698df767794c5cf771d22e74fa35eb5. Python 3.12.16; SQLite 3.37.2; four logical CPUs.
- Windows SQL-only artifact 14439632301, SHA-256 26179a4e21227eda2836ac5bd6f0e9314e357ed3de90e9611a684bc2949de560. Python 3.12.10; SQLite 3.49.1; four logical CPUs.
- Database sizes at 100, 1,000, 10,000 synthetic Studies: 4.01, 14.23, 116.60 MiB.

| 10k synthetic Studies, warm Python SQL SELECT + fetchall, 11 trials | Linux p50/p95 ms | Windows p50/p95 ms |
| --- | ---: | ---: |
| Study search (200 metadata) | 20.7743 / 21.3528 | 12.735 / 13.007 |
| Transcript metadata (50) | 13.885 / 14.247 | 9.686 / 11.480 |

Study-search and transcript metadata plans record temporary B-tree ORDER BY operations. These are profiling candidates, NOT production regressions or native app timings. OS/runner/SQLite differences prohibit cross-platform speed conclusions. Prior artifact measurements at the earlier source SHA must not be conflated with these samples.

### New Agent 08 native UI timing-format validator

The benchmark-only tests/defragmentation/benchmarks/native_ui_trace.py parses JSONL exported by Agent 07's genuine installed native automation driver. A trace row has EXACTLY these fields:

- evidence_kind = externally-observed-installed-a11y-action
- source_sha, package_sha256, executable_sha256, fixture_sha256
- host_evidence_sha256 (SHA-256 of the exact native QA runner's host metadata file), native_qa_run_url (the exact GitHub installed QA run)
- session_id, trial_id, native_provider (linux-webkitgtk-atspi or windows-webview2-uia)
- study_count (100, 1000, 10000) and cache_state (cold-rebooted, cold-cache-dropped, warm-cache, unknown)
- action (cold_start_window, warm_start_window, studio_actionable, project_switch, study_open, candidate_typing, candidate_save, candidate_reopen, source_navigation, transcript_navigation, gui_interaction)
- start_monotonic_ns and completion_monotonic_ns as a same-clock ordered action/completion pair
- start_evidence_id and completion_evidence_id referencing Agent 07's independent UIA/AT-SPI observations.

It rejects unexpected secret fields, mock labels, invalid/mismatched hashes, forged or missing host evidence references, invalid run URLs, bad clocks, duplicate trial/action tuples, invalid cache/start conditions, wrong provider and Study count. It never pools different QA runs, host metadata digests, fixtures, actions, study counts, providers or cache states. Agent 07 must preserve the full OS build, CPU, RAM and WebView runtime metadata file whose SHA-256 is referenced by host_evidence_sha256; an unverified digest or GitHub URL is not a trusted host attestation. Default minimum is 10 independent successful comparable samples per group. Smaller groups return INSUFFICIENT_TRIALS and null p50/p95 rather than unsupported numbers. It preserves raw timestamp samples and uses nearest-rank p95.

Command with real independently attested Agent 07 input (the tool does NOT create traces):

    python3 -m tests.defragmentation.benchmarks.native_ui_trace --trace "$AGENT07_NATIVE_JSONL" --source-sha "$EXACT_SOURCE_SHA" --package-sha256 "$PACKAGE_SHA256" --executable-sha256 "$BIN_SHA256" --min-trials 10 --output "$FRESH_DIR/native-ui-summary.json"

The parser validates FORMAT and consistency only. Provider labels, evidence IDs and hashes supplied in JSON can be forged. Every result remains PENDING_INDEPENDENT_AGENT07_INSTALLED_QA until Agent 07 checks real window/accessible-control completion, package+binary identity, actual canonical app-migrated SQLite Study counts, cache state and raw event provenance. No real native UI data are included in this report.

### Unresolved dependencies and readiness

Agent 07 controls installed WebKitGTK/AT-SPI and Windows WebView2/UIA sessions; Agent 00 controls canonical app database layout and any authentic Tauri IPC instrumentation. Agent 08 can attach the existing read-only native_attach.py during a real Agent 07 installed session; it cannot substitute local Python process probes or mock/frontend traces. Await 10 comparable repeats per scenario, cold/warm control, authentic fixture and binary hash matches, native memory PSS/private working-set observations and native UI milestones. IPC latency remains NOT TESTED absent authentic command boundary instrumentation. Do not modify production solely due SQL temp sorting.

Branch instrumentation is independently testable; release-native performance certification remains BLOCKED / NOT TESTED until Agent 07 supplies installed-host evidence. PR #127 stays DRAFT; no merge to main, release, authorization, or paid egress.

## Source-derived safe synthetic installed database placement (new Agent 08 tooling)

Inspected integrated production source at 5ed385d09092d2c94a1bd86cb56888d96df0bb5e:

- apps/desktop/src-tauri/src/lib.rs:1541-1543 resolves an explicit SCRIPTOTAR_DATA_DIR or Tauri app.path().app_data_dir().
- apps/desktop/src-tauri/src/lib.rs:1558 passes that directory to AppServices::new_with_job_notifier.
- apps/desktop/src-tauri/src/services.rs:112-117 opens data_dir/scriptotar.sqlite3 and performs current integration migrations.
- The installed backend smoke also uses SCRIPTOTAR_DATA_DIR (lib.rs:1525-1534), but does not establish graphical WebView readiness.

**New benchmark-only tool:** tests/defragmentation/benchmarks/stage_installed_fixture.py verifies a source fixture and JSON manifest from the already existing app-migrated 100/1k/10k synthetic SQL generator. It demands matching exact source SHA and fixture digest/size, schema version 12, complete core/integration migration ledgers, correct canonical Study table counts, synthetic-only titles and project identity, valid foreign keys/SQLite integrity, and source + destination scratch directories each marked by a pre-existing .scriptotar-benchmark-only file containing SYNTHETIC_ONLY. It refuses symlinks, existing target databases/SQLite sidecars and writes an exclusive new 0600 destination data_dir/scriptotar.sqlite3. No ordinary user's database may be used. It preserves a separate staging JSON receipt; it does not start the app, invoke Tauri, claim real migrated Rust execution from a forged ledger or certify UI.

For each N=100/1000/10000, after the exact-head SQL generator has emitted scriptotar-synthetic-N.sqlite3 and its manifest, create a fresh **separate** destination directory with the scratch sentinel, then invoke the staging tool:

    python3 -m tests.defragmentation.benchmarks.stage_installed_fixture --fixture "$SQL_SCRATCH/scriptotar-synthetic-N.sqlite3" --manifest "$SQL_SCRATCH/scriptotar-synthetic-N.json" --data-dir "$FRESH_APP_DATA_DIR" --study-count N --source-sha "$EXACT_INSTALLED_SOURCE_SHA" --output "$FRESH_APP_DATA_DIR/staging-proof.json"

The *actual installed package* must run with SCRIPTOTAR_DATA_DIR pointing to FRESH_APP_DATA_DIR. Agent 07 must independently attest that live installed PID, package/executable/source hashes, real window/UIA/AT-SPI state, and read-only canonical Study counts/schema after app startup match the staged fixture; existence of the file and receipt are insufficient. Use a new app-data directory per size and trial where required to avoid cache/state contamination. The package's exact source SHA must match the fixture build source SHA; do not reuse older SQL artifact databases as exact-source native experiments.

The dedicated Linux/Windows migrated SQL smoke workflow now stages all three 100/1,000/10,000 datasets through this tool and uploads *receipt JSONs only*, deliberately excluding large SQLite bytes and never upgrading the SQL workflow to installed/native performance certification. Hermetic unit tests use deliberately simplified schemas to check safety behaviors only; **only the real Rust-migration workflow** can support app-migrated fixture claims.

Remaining external dependency: Agent 07 must run genuine installed .deb/NSIS native UI trials using this layout, add 10 comparable actual monotonic repetitions and concurrent attached private memory metrics; Agent 00 owns any real authenticated IPC timing hooks. All installed performance metrics continue to read BLOCKED/NOT TESTED until collected and independently validated. No production optimizations or release approvals have been requested.
