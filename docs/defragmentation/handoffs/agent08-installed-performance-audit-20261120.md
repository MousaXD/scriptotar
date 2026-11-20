# Agent 08 — exact-integration installed performance audit (20 November 2026)

**Independent verdict: `BLOCKED / NOT CERTIFIED`.** This is an evidence audit and a concrete Agent07 runner hook request, **not** a measured installed performance report, release approval, or claim of no regression.

## Exact production candidate and scope

- Repository: [MousaXD/scriptotar](https://github.com/MousaXD/scriptotar).
- Inspected integration commit / draft PR [#114](https://github.com/MousaXD/scriptotar/pull/114): `514ec3ee571c4309e5b7473b8a65ab4fb73f497a`.
- Merged QA contribution: [#131](https://github.com/MousaXD/scriptotar/pull/131), merged by two-parent integration commit `34efd849f79bf8b62f9665481449a870ae0e3a8d`.
- Verified performance infrastructure contributions **already merged**: #147 (installed-artifact hash guard), #148 (native timestamp replay guard), #151 (attached-memory preflight), #154 (terminal PID-identity recheck), #156 (historic evidence report). Do **not** recreate existing synthetic SQL infrastructure.
- Original exact-integration installed [workflow #44616608421](https://github.com/MousaXD/scriptotar/actions/runs/44616608421): Linux WebKitGTK/AT-SPI job [135886944032](https://github.com/MousaXD/scriptotar/actions/runs/44616608421/job/135886944032) `failure`; Windows WebView2/UIA job [135886943041](https://github.com/MousaXD/scriptotar/actions/runs/44616608421/job/135886943041) `failure`. Both `result.json` entries: **11 PASS, 0 FAIL, 5 NOT TESTED** out of 16 native functional cases. Functional tests are not repeated performance trials.

## Original artifact provenance — independently rehashed ZIP bytes

| Platform | Original small review artifact | ZIP SHA-256 independently computed | Actual installer SHA-256 in runner `result.json` | Installed exe SHA-256 in runner `result.json` |
|---|---|---|---|---|
| Linux | [14575791759](https://github.com/MousaXD/scriptotar/actions/runs/44616608421/artifacts/14575791759) | `2e98e842c4170af129b4658bf01d23fd3814175c7832448b97df47c589a2707c` | `ee9e9d8f84640561a0b02dab7f6376048fa823c3e734d10d90a252753d8263e6` | `7a9bb99d4ebeb0717191a485a72194df2b747cef5535ccb472a7c90f0f08a91e` |
| Windows | [14576187802](https://github.com/MousaXD/scriptotar/actions/runs/44616608421/artifacts/14576187802) | `9aed936141795b0b150f55cee8bc403c00191558abc503fbc35f298714ae66fb` | `8528e5942b8551ecedf9588e5b3cf5695f8aa5e5913fb1785c60597708823d23` | `4302dd1d396a20d9e7e19c3967de1782b4a0e88047b607b1d2f0f53f023cc8ae` |

The original Linux review ZIP contains 35 entries and Windows 32 entries. Both independently checked ZIPs have the exact integration source SHA in their embedded `result.json`; neither includes original `native_attach` sample JSON, `attached_memory_gate` preflight, independent `native_ui_trace` timestamp JSONL, or genuine installed 100/1,000/10,000-Study cohort. The installer/exe SHA values above are **runner assertions**, not independent byte-rehashes of the actual original installer and executable. Full package artifact links: [Linux 14576761357](https://github.com/MousaXD/scriptotar/actions/runs/44616608421/artifacts/14576761357), [Windows 14576477403](https://github.com/MousaXD/scriptotar/actions/runs/44616608421/artifacts/14576477403). Neither large package ZIP was independently rehashed in this audit.

Archived historical PID: Linux `27136` in a bounded native layout observation; Windows `1144` as source process in `readiness`. These processes belonged to finished CI jobs and cannot be attached to now. Windows `readiness.elapsed_seconds=3.5` and `poll_count=1` is **one functional UIA polling observation**, **not** a startup latency trial. Original Linux host metadata: `Linux-6.8.0-1070-azure-x86_64-with-glibc2.35`, `x86_64`, `MemTotal=16372452 KiB`. Original Windows native report: `Microsoft Windows NT 10.0.26100.0`, `AMD64 Family 25 Model 17 Stepping 1`; RAM not established as a comparable benchmark condition.

### Independent native performance matrix

| Requirement | Linux | Windows |
|---|---|---|
| Live installed root PID + kernel birth + complete WebView descendants, source/package/exe start/end hash binding | NOT TESTED | NOT TESTED |
| Actual read-only attached per-sample PSS/private resident / private working-set and commit; CPU if available | NOT TESTED, n=0 | NOT TESTED, n=0 |
| Cold/warm installed launch, useful Studio, Study open, edit/save, project/candidate navigation, search, quit/reopen | NOT TESTED, n=0, p50/p95 N/A | NOT TESTED, n=0, p50/p95 N/A |
| Genuine installed 100 / 1,000 / 10,000 Study UI open/edit/save/relaunch + durable SQLite verification | NOT TESTED | NOT TESTED |
| Hardware-/cache-/workload-matched baseline regression comparison | INDETERMINATE | INDETERMINATE |

Do not convert UIA polling seconds or CI job duration into latency. Do not convert source-only serial memory samples into independent UI trials. `n=0` means zero **verified benchmark** trials, not zero functional app activity or zero memory consumption. No performance regression is either proved or disproved.

## 2026-11-20 Agent08 read-only CPU source instrumentation (NOT installed evidence)

Following the exact-integration audit, Agent08 added **source-only P2 measurement
correctness changes** in the same draft PR. `native_attach.py` now optionally
collects **cumulative root process CPU time** from Linux `/proc/<pid>/stat`
user+system ticks or Windows read-only `GetProcessTimes` counters. Each value is
collected within the existing root process-image and kernel-birth brackets, and
missing permissions remain `null`, not zero.

`attached_memory_gate.py` treats this signal as optional for older raw reports,
but **rejects partially present, malformed, negative, or decreasing counters**
for a purported single root PID birth. When every sample is valid, it emits
only `ROOT_ONLY_UNCERTIFIED` aggregate core-equivalents over the observed
monotonic interval. These values exclude WebKitGTK/WebView2 helper CPU, ignore
reparented helpers, and do not prove native GUI performance. The new synthetic
source tests check good/missing/tampered samples and a genuine local *test runner*
PID; **testing a Python runner PID is never an installed Scriptotar benchmark**.

The historical installed workflow artifacts still contain **zero live root CPU
or complete WebView tree CPU measurements**; no measured CPU percentage, p50/p95,
peak memory, native 100/1k/10k cohort or regression verdict can be published.
Do not treat the synthetic CPU test, helper-code change, or any source CI PASS as
recovery of the external Agent07 concurrent live-runner requirement. Agent07
must still coordinate optional inside-driver capture on both OSes and Agent08
must review original exact-source artifacts.

## 20 November follow-up: installed post-PR158 integration and ten-trial floor

Agent00 merged P0 [PR158](https://github.com/MousaXD/scriptotar/pull/158)
as two-parent integration `50ed95608fc742db4a7cf1bdd2ac04fa8c5457e6`,
**after** the original `514ec3...` report above. Its newly source-exact
[installed Linux/Windows #44736655696](https://github.com/MousaXD/scriptotar/actions/runs/44736655696)
was **FAIL** on both platforms (installed GUI test steps failed closed).
Original native runner logs show the unchanged five functional cases
`wrong_project_ipc_denial`, `multi_source_transcript`, `source_revocation`,
`arabic_rtl` and `light_dark_visual` as NOT TESTED on Linux; Windows
`result.json` also has `verdict=NOT TESTED`. The Linux separate
two-source attempt remains INCOMPLETE without a lawfully sourced, authenticated
offline real-media bundle. No concurrent Agent08 root/child process sampler,
ten-trial native latency JSONL or genuine installed Study-scale dataset
measurements were reported in the native job logs. Do not carry prior
source-package hashes forward to this new SHA.

New original GitHub Actions review ZIP metadata, **provider-reported upload
digests (NOT independently rehashed local ZIP bytes)**:
- Linux `native-webkit-review` artifact `14629767447`,
  SHA-256 `3b94d161d3b27352ea043d60eb11d25527b1ccc0738af1c91a6a7d6c438b3553`.
- Windows `native-webview2-review` artifact `14628883692`,
  SHA-256 `3dd8edda9210727ba1b31a4d005959bdb84b29ea45f77fba1cf8252d2cea0164`.
- Complete Linux bundle artifact `14629438135`,
  SHA-256 `f27c73684d6484b0096da89a391c6900bba4dcb5ee78327d58f662176b5dea6f`.
- Complete Windows bundle artifact `14628774023`,
  SHA-256 `c3b6b9ceee5ac9acaeaa5522aa9e1f7ad5b346a0dc14ebee7c1f60d788b34fa6`.

P2 source-only trace parser `native_ui_trace.py` previously accepted
`--min-trials 3`, even though the native performance requirement is at
least **ten independent trials per exact source/platform/host/cache/workload
bucket**. The Agent08 draft now rejects any `min_trials` below 10,
non-integer or above 100, preventing underpowered p50/p95 output even if
a caller overrides the CLI flag. Source-only negative tests cover low
thresholds and 3-vs-10 observations. This is a format-policy guard
**only**, not proof that any installed driver actually generated ten
genuine event pairs. Do not infer that earlier 3-trial synthetic test
fixtures were performance measurements; they never were.

The P0 PR158 source guard remains pinned to the original `514ec3...`
reviewed tree with a two-YAML-only diff allowance. The new Agent08
Python modifications are therefore **not yet mergeable into owner
integration without an explicit independently reviewed source-baseline
rollforward**. Do not weaken this prebuild guard or merge PR159
speculatively. The latest draft PR source CI must be evaluated at its
exact head and re-reviewed by Agent00/Agent10 where security policy
requires. Original root performance trials remain `n=0` both OS;
native certification remains BLOCKED.

## Subsequent Agent08 hostile-input boundary and independent staging results

The subsequent Agent08 draft `native_ui_trace.py` source-only validator also
caps an untrusted external JSONL input at **8 MiB raw bytes**, **4096
characters per line**, and **10000 parsed events** before accepting a new
row; it now reads at most `MAX_TRACE_BYTES + 1` from disk, refuses oversized
files and lines or invalid UTF-8 rather than blindly loading an arbitrary
full trace with `Path.read_text()`. The added deterministic negative source
tests cover all three limits, malformed UTF-8 and a normal ten-event cohort.
This is solely evidence-parser resource governance, not an installed
performance observation, replay-proof provenance or an approval to ingest
private transcript payloads. **The exact-head CI of these newest changes must
pass independently before declaring the new code source-test ready.**

The original independent Agent10 [PR166](https://github.com/MousaXD/scriptotar/pull/166)
strict [#44741737474](https://github.com/MousaXD/scriptotar/actions/runs/44741737474)
at reviewed baseline stage `c11eef988c9c2ee86fdd50c547ba4a1b3554cbd2`
**completed SUCCESS**, including 14 adversarial static and nine committed
Git-tree negative mutation controls; this is **not an approval of this newer
Agent08 six-file source**. The original stage
[Linux+Windows installed #44741358875](https://github.com/MousaXD/scriptotar/actions/runs/44741358875)
**completed FAILURE** on both OS native GUI acceptance steps. Stage CI,
Windows NSIS, two-platform P2 migrated synthetic smoke and other
source-only checks succeeded. The stage remains DRAFT, with original
installed native benchmark evidence `n=0`. Do not claim that source
security checks, package builds or synthetic SQL make a native UI PASS.

## Original synthetic SQL artifacts (separate evidence class)

Original P2 [workflow #44559483015](https://github.com/MousaXD/scriptotar/actions/runs/44559483015) at *older* exact source `f0daf2606dba09240c8a269b3110e135d570cf02` completed Linux/Windows migration and SQL-only runs with 11 query iterations for each of 100, 1k, 10k Study rows. Both original ZIPs were downloaded and their bytes independently hashed: [Linux 14546612494](https://github.com/MousaXD/scriptotar/actions/runs/44559483015/artifacts/14546612494), `40c1ee3682032b01c377380af27df581efb5b4875ec02b98e03aaa14760650bf`; [Windows 14549115338](https://github.com/MousaXD/scriptotar/actions/runs/44559483015/artifacts/14549115338), `af238e5959f54dadab7c6276e699587214032d3d2a66e0d961bbc1c747509e41`. Read original `sql-100.json`, `sql-1000.json`, `sql-10000.json` and validated source and sample provenance. These are **Python SQLite/fetchall** latencies under a production-migrated schema, not current integration Rust IPC or native GUI timing.

| Study metadata search p50 / nearest-rank p95, ms | 100 rows | 1,000 rows | 10,000 rows |
|---|---|---|---|
| Linux SQLite 3.37.2 | 0.2870 / 0.3209 | 1.8599 / 1.9197 | 18.0577 / 20.4699 |
| Windows SQLite 3.49.1 | 0.2217 / 0.2624 | 1.4043 / 1.6150 | 13.8249 / 15.3084 |

This query uses `idx_content_studies_project_updated` and a temporary ORDER BY B-tree; profile it further **only if source-exact installed UI evidence demonstrates an actual matching performance problem**.

## Source-inspected instrumentation and real dependency

Existing P2 tools at integration SHA: `native_attach.py` (read-only PID/image/birth-checked memory sampler), `native_memory.py` (Linux `smaps_rollup` and Windows EX2 process counters), `attached_memory_gate.py` (fail-closed original sample preflight), `native_ui_trace.py` (strict external native-driver timestamp format and >=10-trial summaries), `stage_installed_fixture.py` and `study_scale.py` (synthetic migrated fixture preparation), `native_trials.py` (noncertifying process-only trials). These are adequate **format and structural validators**, but no current native driver invokes them on a *live* installed window within its controlled process lifetime. Reparented WebKit/WebView2 helper children also need explicit verification; the snapshot's `tree_memory_complete` flag only covers descendants visible to the current parent map, not independent proof that all helpers were counted.

Agent07's Linux `tests/defragmentation/native-e2e/linux_webkit.py` starts the installed app in `Session.start()` via `subprocess.Popen`; its `exercise()` stops or restarts the app before it exits. Agent07's Windows `tests/defragmentation/native-e2e/windows_webview2.ps1` starts via `Start-InstalledDesktop` and later stops/uninstalls the app in its `finally`. The shared `.github/workflows/installed-webview-acceptance.yml` currently invokes each driver synchronously and uploads evidence **only after the driver exits**. Therefore a subsequent CI step cannot attach to the historical PID. This is the **specific missing in-session instrumentation**.

### Concrete Agent07 runner change requested (not Agent08-owned)

1. Make opt-in perf captures a nonprivileged, **inside-driver** feature: record immutable source/workflow URL, real launch PID and kernel-birth ID, original installed package/executable hashes, run-scoped host metadata and fresh isolated profile. Start read-only existing `native_attach` *concurrently while the installed GUI is still alive*; join and preserve raw status and JSON **before each real shutdown/relaunch**. Label failures, early death, invalid birth/image and missing WebView descendants explicitly; do not make later CI uploads stand in for live sampling.
2. Capture action start and actual actionable AT-SPI/UIA completion from the **same driver's monotonic clock**, each with distinct native evidence IDs, trial/session IDs and source/package/exe/fixture/host hashes. Run existing `native_ui_trace` on untouched external JSONL. Collect at least 10 independent genuine same-platform, comparable-host, cache-and-workload trials per declared action. Mark cold-cache unverified without a proven purge/reboot.
3. Agent00 has authorized **test-only**, separate, new, uniquely marked, canonical, non-symlink runner-owned temporary `SCRIPTOTAR_DATA_DIR` locations for 100/1k/10k cohorts ([original approval](https://github.com/MousaXD/scriptotar/pull/114#issuecomment-7014172646)). Do not touch existing user data, credentials, or paid-provider egress. Require installed GUI actually to open the staged canonical project and dataset, navigate/edit/save, quit/reopen, and independently inspect migrated SQLite durable state, with original stdout/stderr and failure evidence retained.
4. Archive raw sampler output, attached preflight, independent UI JSONL, summarized format validation, real SHA-256 proof, platform/runtime/hardware, complete captured action evidence and canonical post-reopen SQLite readback. Agent08 must review the **original** published artifacts before assigning any PASS or quantified metric.

Coordination sent to [Agent07 PR131 comment #7038310688](https://github.com/MousaXD/scriptotar/pull/131#issuecomment-7038310688). Agent07 owns genuine AT-SPI/UIA driver code and the native workflow; Agent08 owns read-only evidence validation and performance reporting; Agent00 exclusively owns integration and production modifications.

**Final Agent08 status: `BLOCKED` on independent live runner/driver event provenance.** No measured p50, p95, process memory peak, CPU distribution, 100/1k/10k installed-app result, or regression classification exists. No main merge, publication, paid egress, release GO or functional-case certification is authorized by this report.
