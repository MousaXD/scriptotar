# Agent 08 — Native Performance Certification Report (2026-11-19)

**Verdict: NOT CERTIFIED / installed performance NOT TESTED.** This is a source-exact evidence register and reproducibility handoff, not a benchmark result. No app performance regression or performance release GO can be inferred. Agent 00 alone owns integration and release.

## Checked GitHub source and original CI evidence

- **Agent08 PR #151**: source `d7645ffac26633524a2e91891ca34bcd549cec42` on `agent-08/p2-strict-attached-memory-preflight-20261119`; source-only gate implementation in `tests/defragmentation/benchmarks/attached_memory_gate.py`. PR #147 and #148 already merged into integration and not modified.
- Original two-platform performance-source run: https://github.com/MousaXD/scriptotar/actions/runs/44532447911; Linux job [135589028084](https://github.com/MousaXD/scriptotar/actions/runs/44532447911/job/135589028084), Windows job [135589027362](https://github.com/MousaXD/scriptotar/actions/runs/44532447911/job/135589027362), both **SUCCESS** at this source SHA, 24/24 Python unit/source tests per OS. This explicitly includes four **synthetic** attached-memory-gate tests. Real Rust production SQLite schema migrations (v12) were used for the **synthetic SQL-only** 100, 1,000, 10,000-Study scenarios; staging a fixture is not opening it through the installed app.
- Other completed **PR151 exact-head** workflows: [main CI 44532447956](https://github.com/MousaXD/scriptotar/actions/runs/44532447956), [GUI QA 44532447969](https://github.com/MousaXD/scriptotar/actions/runs/44532447969), [security hygiene 44532447704](https://github.com/MousaXD/scriptotar/actions/runs/44532447704), [MCP E2E 44532447771](https://github.com/MousaXD/scriptotar/actions/runs/44532447771): all **SUCCESS**; no installed performance was measured by them.
- **Agent07 installed QA PR #131**: source `9f4930f1c0b692252d97479da266d094666b168c`; original installed run https://github.com/MousaXD/scriptotar/actions/runs/44532618321, Linux job [135589636534](https://github.com/MousaXD/scriptotar/actions/runs/44532618321/job/135589636534) and Windows job [135589641269](https://github.com/MousaXD/scriptotar/actions/runs/44532618321/job/135589641269) both **FAIL** on the incomplete independent P0 16-case acceptance rubric. Native launch/functional interactions occurred and source packages were built/installed; this does **not** establish usable performance sample cohorts.
- Agent07 original artifact IDs on that run: Linux package `14535916084`, Linux QA review `14536415682`, Windows installer `14535488079`, Windows QA review `14536850727`. Original attached performance-memory raw JSON and 10-trial native UI JSONL are **absent**. Never transfer source/artifact identities between the two different source SHAs.
- **Direct inspection of downloaded ORIGINAL Agent07 QA review ZIP bytes:** Linux review artifact `14536415682` ZIP SHA-256 `5d9031177d369e68c94f2918860244dd09a91ea1abb0bcb44560aa9d029952b9`; embedded `result.json` and `SHA256SUMS.txt` report actual package SHA-256 `883b9049a21d1c4e74d59bf71ecc7f0019a1d9652b525110a30bf39d5bd65277` and installed executable `7458ddbdcceda5c51d40d99460c2fa5db8d207411618b26d093b18ed5a55695e`. Windows QA review artifact `14536850727` ZIP SHA-256 `a96eb86309158fb363e3b2d91e6e84b8798c548b8f6530fce9b6a7cb1e794458`; embedded `result.json` reports installer `484814dfd5c46bed0f256ccd933e368e34bc636c613070221a72dd14f3a9a785` and installed executable `f3c10d6b74f758df4cd615358f613dc4b926d79b9214c1f198c44768745b87c9`. The original ZIP bytes were independently SHA-256 checked; the package/binary byte hashes here are **embedded operator/runner reports**, not independently recomputed from their separately uploaded package artifacts.
- Native QA original **11 PASS / 0 FAIL / 5 NOT TESTED per OS** (read from both `result.json`): remaining five are wrong-project mutating IPC, multi-source real transcription, source revocation, independent Arabic RTL and independent light/dark visual checks. These 11 functional PASS cases are **not performance trials**.

## Results with correct evidence classes

| Platform / workload | Actual installed process/UI? | Samples / independent trials | p50 | p95 | Process memory | Status |
| --- | --- | ---: | --- | --- | --- | --- |
| Linux WebKitGTK — cold launch | Native QA did launch, no benchmark | 0 | N/A | N/A | PSS/RSS not collected for benchmark | NOT TESTED |
| Linux — warm launch, Studio readiness, editor/Study open/save/navigation/reopen | Native UI behavior exercised without timing trace | 0 | N/A | N/A | PSS/private resident not collected | NOT TESTED |
| Windows WebView2 — cold launch | Native QA did launch, no benchmark | 0 | N/A | N/A | Private working set/commit not collected | NOT TESTED |
| Windows — warm launch, Studio readiness, editor/Study open/save/navigation/reopen | Native UI behavior exercised without timing trace | 0 | N/A | N/A | Private working set/commit not collected | NOT TESTED |
| Linux + Windows — v12 SQL-only, 100 / 1,000 / 10,000 rows | NO, synthetic SQL profiling only | 11 SQL samples per query shape per scale in source CI | Do not convert to native p50 | Do not convert to native p95 | N/A | SYNTHETIC |
| Linux + Windows — installed app with 100 / 1,000 / 10,000 persisted canonical Studies | No independently proven installed app-opened cohort | 0 | N/A | N/A | N/A | NOT TESTED |
| PR151 memory preflight consistency / negative tests | No installed app | 4 synthetic gate unit tests per OS (24 full-suite tests) | N/A | N/A | Synthetic fixture values only | VERIFIED SOURCE TEST |

**Hardware and OS:** P2 source CI uses GitHub Actions `ubuntu-22.04` and `windows-latest`, validating script portability only. **Observed in original native QA metadata, not a performance run:** Linux `Linux-6.8.0-1070-azure-x86_64-with-glibc2.35`, CPU architecture `x86_64`, `ram_kib=16372448` (~15.6 GiB reported); Windows `Microsoft Windows NT 10.0.26100.0`, `AMD64 Family 25 Model 1 Stepping 1, AuthenticAMD`, RAM not reported, 6 WebView2 processes observed in session. Windows `readiness.elapsed_seconds=9.1` was one UIA-polling observation (`poll_count=2`), **NOT a calibrated cold/warm startup trial or p50/p95**. Exact display server, versioned WebKitGTK/WebView2 and native resource limits have not been established for any repeatable performance cohort. Reporting invented specifications or performance values is prohibited.

## Independently reproducible installed sampling contract

1. Build/package and install the **same exact** source SHA being timed. Before launch, bind the original installer package and resolved installed binary to independent SHA-256, GitHub workflow run URL and the source SHA. Record host hardware/OS/GUI runtime metadata. Do not use the detached PR151 source-test SHA as the installed application's identity when the packaged app comes from another commit.
2. Agent07 alone instruments the **real installed** AT-SPI/UIA driver and launches the actual packaged app. Capture root PID from the launch operation plus kernel-reported process image and process birth time; immediately sample with the already-existing read-only `native_attach.py` sidecar **while that PID remains alive and native actions are genuinely executed**. Sample **per distinct launch**, not self PID, orphaned process, fake executable or CI step duration. Preserve full stdout/stderr, exit code, sampler implementation SHA/version, original report, package/binary SHA at start and finish, raw per-sample monotonic brackets and process tree completeness. Treat unreadable or reparented helper processes as unresolved coverage.
3. For example, from a source-exact checkout on the installed runner, with real approved paths and environment (do not substitute placeholders silently):

   ```bash
   python -m tests.defragmentation.benchmarks.native_attach \
     --pid "$LIVE_INSTALLED_ROOT_PID" --package "$ORIGINAL_PACKAGE" \
     --executable "$RESOLVED_INSTALLED_EXE" --source-sha "$APP_SOURCE_SHA" \
     --run-url "$ACTIONS_RUN_URL" --seconds 30 --interval-seconds 0.25 \
     --study-count "$APP_VERIFIED_STUDY_COUNT" --output "$NEW_RAW_MEMORY_JSON"

   python -m tests.defragmentation.benchmarks.attached_memory_gate \
     --attached-json "$NEW_RAW_MEMORY_JSON" --package "$ORIGINAL_PACKAGE" \
     --executable "$RESOLVED_INSTALLED_EXE" --source-sha "$APP_SOURCE_SHA" \
     --run-url "$ACTIONS_RUN_URL" \
     --package-sha256 "$INDEPENDENT_PACKAGE_SHA256" \
     --executable-sha256 "$INDEPENDENT_EXE_SHA256" \
     --output "$NEW_PREFLIGHT_JSON"
   ```

   The output is only **ELIGIBLE FOR INDEPENDENT REVIEW**, not a certificate. It rejects absent/changed originals, bad package/source/run labels, missing birth identity, early termination, malformed/replayed/overlapping monotonic sample brackets, incomplete trees, and missing Linux PSS/private resident or Windows EX2 private working-set/private-commit measures. Its JSON values are not cryptographically authenticated.
4. Capture **independently observed** native action-start and accessibility-actionable completion events in the **same native driver monotonic clock** with unique session/trial/evidence IDs. Cold vs warm cache state requires *real cache provenance*, not just a process restart. Keep action, OS, host, source SHA, package/executable SHA, approved fixture SHA, QA run ID and cache regime separate. Existing `native_ui_trace.py` accepts strict external JSONL; run it only on real native-driver events, with `--min-trials 10`. Its output is **UNVERIFIED EXTERNAL TRACE** until original native QA corroborates raw driver and UI observations. Never combine Linux and Windows timing domains, cross-run observations, or serial memory samples into independent UI trials.
5. Agent00 approval is required for an isolated **sentinel-marked** `SCRIPTOTAR_DATA_DIR` per 100 / 1,000 / 10,000 Study fixture. Use production v12 migrations and fixture/manifest hashes, no real user DB or private documents, no silent data overwrite. Confirm the installed GUI actually opens the *same* canonical project and counts, lists sources, edits, saves, navigates, exits and relaunches. Canonical SQLite readback must verify persisted results and nonmutation outside intended scope. The staged SQL-only fixture does not satisfy this.
6. Repeat **at least 10 independent genuine UI trials** for each platform × workload × action × proven cache state, preserving failures, timeouts and unavailable cases. Report sample count, median (p50), nearest-rank p95, max/peak sample memory and device specs from original artifacts. Serial memory p50/p95 are sampling-window distributions, not independent startup/action latency percentiles. Compare source-exact baseline and candidate *within matched hardware/OS/workload conditions* before asserting regression.

## Open blockers and release interpretation

- No live installed WebKitGTK/WebView2 runner PID is available from completed Actions jobs; connector-only access to historic logs/artifacts cannot retroactively attach to a terminated process.
- Agent07 native instrumentation, original timing JSONL and raw memory samples are not yet supplied; requested via [PR131 coordination](https://github.com/MousaXD/scriptotar/pull/131#issuecomment-7014045763).
- Agent00 sentinel-marked isolated app-data approval and canonical 100/1,000/10,000 installed-app Study readback remain pending; requested via [PR114 handoff](https://github.com/MousaXD/scriptotar/pull/114#issuecomment-7014046869).
- No matched baseline/candidate native measurements means regression verdict **INDETERMINATE**, not 'no regression'.
- PR151 can be **reviewed for its narrow source-tested preflight** independently. It cannot be represented as installed-native benchmark completion or as release security/QA certification. PR remains unmerged; Agent00 alone integrates.
