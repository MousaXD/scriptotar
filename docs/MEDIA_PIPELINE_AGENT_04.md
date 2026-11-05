# Agent 04 — Media pipeline audit and validation ledger

**Owner:** Agent 04 (media/transcription)
**Base:** `feature/chatgpt-mcp-ci-20261026` at `517877b26df9028cdd32af69f6f5a88a68b948e9`
**Working branch:** `agent-04/media-engine-overhaul`
**PR:** [#75](https://github.com/MousaXD/scriptotar/pull/75)
**Status:** IN PROGRESS — current licensing-integrity and Windows descendant-test changes require exact-HEAD CI; Rust security advisory and live YouTube access remain unresolved

## Scope inspected

- `crates/scriptotar-media/src/lib.rs`: command/event data types, supported models and extension validation, public protocol.
- `crates/scriptotar-orchestrator/src/lib.rs`: single-worker job queue, subprocess lifecycle, Rust-owned transitions/persistence and cancellation commands.
- `sidecars/transcription/scriptotar_sidecar/{engine,engine_worker,service,recovery,validation,protocol}.py`: yt-dlp, ffprobe, Faster Whisper, CPU/GPU selection, output finalization, cancellation, restart recovery, JSONL.
- `sidecars/transcription/tests/` and `.github/workflows/transcription-sidecar.yml`: existing mocks, protocol tests and dependency-import checks.

## Highest-risk findings and implemented remediation

| Area | Original hazard | Remediation | Regression evidence |
| --- | --- | --- | --- |
| Recovery | A matching checkpoint accepted any nonempty file at an arbitrary stored path, including changed bytes and out-of-directory files. | Bind checkpoint to streaming SHA-256 and size, require copied/downloaded media remain inside the job partial directory, and only permit explicit copy-free local source as an external path. Legacy unverified checkpoints restart from scratch. | `test_recovery_integrity.py` |
| Predictable partial path | A pre-existing symlink at `.scriptotar-<job>.partial` could redirect checkpoint writes into an unrelated directory. | Remove only the symlink before reading or writing the job checkpoint; never follow the symlink. | `test_recovery_integrity.py` |
| Cancellation | The stdout reader could publish a terminal result after a concurrent cancel flag was set; a replaced worker could publish into a new worker's job. | Serialize cancel-state/worker identity validation, event publication and terminal-state reset behind the same lock. | `test_supervisor_races.py` |
| Windows cancellation | `Popen.terminate()` on Windows does not recursively kill launched descendant FFmpeg/yt-dlp processes. | Start the worker in a Windows process group and request `taskkill /PID <pid> /T /F`, with parent-only fallback when unavailable. POSIX process-group behavior retained. | `test_windows_cleanup.py` (mock contract only; real Windows descendant cleanup not yet certified) |
| URL duration | A missing or inaccurate extractor duration could bypass the configured limit after download. | Re-probe downloaded media with ffprobe and enforce the limit on the measured value where available. Reject empty acquired output. | `test_download_limits.py` |
| Model initialization | Any automatic-CUDA model initialization error triggered CPU retry, including unrelated model cache/disk errors. | Retry only for recognized CUDA runtime failures; preserve explicit-CUDA fail-closed behavior. | `test_device_fallback.py` |
| Source errors | Downloader/metadata failures could surface as generic Whisper engine failures with misleading model-download advice. | Report `SOURCE_ACCESS_FAILED` for yt-dlp `DownloadError` and nonzero status, without invoking authentication bypasses. | `test_source_diagnostics.py` |
| Duration serialization | Nonfinite durations could become JSON NaN/Infinity, incompatible with strict Rust JSON parsing. | Accept only finite positive durations from provider metadata and ffprobe. | `test_source_diagnostics.py` |

## Compatibility contract

No public protocol version, supported provider/domain, model choice (`small`, `medium`, `turbo`, `large-v3`), output format (text, cleaned text, timestamped text, SRT, VTT, JSON), language option, quality option, cookie source option or device setting has been removed. GPU/CPU execution is deliberately still serial: uncontrolled multi-model concurrency could exhaust RAM/VRAM. Rust still owns durable job state and installed-runtime license validation has been strengthened with a copied-file SHA-256 check. No MCP or general desktop UI files are touched.

## Tests and evidence boundaries

- Python 3.12 sidecar compile/unittest and pinned dependency import checks: `.github/workflows/transcription-sidecar.yml`.
- Rust media/orchestration and cancellation: repository CI and integration workflows; inspect actual run jobs before certification.
- Windows/Linux installers: respective Tauri workflows; success there does not by itself prove real Windows descendant teardown or CUDA inference.
- Live YouTube, Instagram, TikTok: separate E2E workflow; network restrictions, rate limits and provider authentication may prevent success. Never relabel a blocked provider test as product success.
- Faster Whisper CPU/CUDA inference, Arabic/English/mixed-language accuracy and GPU memory behavior: unit fixtures are **mocks**, not actual model inference; these checks require model downloads and appropriate hardware or explicit recorded fixtures.

## Current CI findings (2026-11-05, exact pre-fix SHA `eccf55561dd1f72295a4c98c4aae1ab5fd50baba`)

- [Transcription Sidecar](https://github.com/MousaXD/scriptotar/actions/runs/42084349548): **SUCCESS** — Python protocol/unit tests and actual pinned dependency import checks.
- [Windows Tauri Installer](https://github.com/MousaXD/scriptotar/actions/runs/42084349484): **SUCCESS** — installer build/smoke job. This does **not** certify native Windows descendant termination, which is separately mocked.
- [MCP E2E](https://github.com/MousaXD/scriptotar/actions/runs/42084349758) and [Security hygiene](https://github.com/MousaXD/scriptotar/actions/runs/42084349684): **SUCCESS**.
- [Tauri migration integration](https://github.com/MousaXD/scriptotar/actions/runs/42084349565): Rust workspace and Rust↔sidecar integration jobs **SUCCESS**; Rust advisories job **FAILED** at `webpki-roots 1.0.9`, `RUSTSEC-2026-0381` (upgrade to >=1.0.10). Agent 01 notified on PR #73; no advisory bypass or Rust dependency modification in Agent 04's branch.
- [Linux Tauri Package](https://github.com/MousaXD/scriptotar/actions/runs/42084349473): **FAILED** during packaged runtime licensing because the GitHub runner updated to CPython **3.12.16**, whereas the existing reviewed exact-version list ended at 3.12.15. Upstream `https://raw.githubusercontent.com/python/cpython/v3.12.16/LICENSE` and the previous pinned `v3.12.14/LICENSE` were compared and are text-identical (13,936 characters); `runtime_licenses.py` now admits exactly 3.12.16 while retaining the same pinned license SHA-256 verification, and tests protect against admitting a future 3.12.17 by accident. **Follow-up Linux CI result still required on the new SHA.** Agent 07 notified on PR #76.
- [Real YouTube Instagram TikTok E2E](https://github.com/MousaXD/scriptotar/actions/runs/42084349607): **FAILED overall** because the exact YouTube URL was challenged for bot sign-in on the hosted runner (`SOURCE_ACCESS_FAILED`); Instagram's real download and Faster Whisper excerpt transcription **PASSED** (3 segments, ~15 s excerpt); TikTok's real download and inference **PASSED** (0 speech segments for its 10.8 s excerpt). These real provider observations do **not** certify Arabic/English/mixed-language accuracy or all accounts/regions. The job stays a strict failure; no cookies bypass, alternate fixture, or skip.
- CI aggregate and integrated Tauri build had not completed when first inspected; **do not imply a pass without the final run's status**.

## Follow-up remediation and boundaries

The narrowly scoped CPython 3.12.16 review and regression test are committed on the Agent 04 branch, without changing upstream license content, legal bundle files, runtime validation, allowed algorithms, or trusted hashes. Subsequent release packaging must prove the legal bundle is generated and validated successfully. Agents 01/07 own the cross-branch Rust dependency advisory and release gate, respectively. No performance or GPU benchmark was measured.

## Prior fully observed media-code CI (`2e0bb15882dd24ee4421a995a9de5f5ea507c210`)

- [Python Transcription Sidecar](https://github.com/MousaXD/scriptotar/actions/runs/42088502570): **SUCCESS** (sidecar tests and pinned dependency checks; mocks in unit tests are not hardware inference).
- [CI](https://github.com/MousaXD/scriptotar/actions/runs/42088502251): job `test` **SUCCESS**; workflow was still finalizing when checked.
- [Linux Tauri Package](https://github.com/MousaXD/scriptotar/actions/runs/42088502477): **SUCCESS**, including the installed runtime/compliance validation. This confirms the narrow reviewed CPython 3.12.16 license fix resolved the prior packaging failure.
- [Windows Tauri Installer](https://github.com/MousaXD/scriptotar/actions/runs/42088502232): **SUCCESS**, but does not certify native Windows descendant termination.
- [Security hygiene](https://github.com/MousaXD/scriptotar/actions/runs/42088502383) and [MCP E2E](https://github.com/MousaXD/scriptotar/actions/runs/42088502220): **SUCCESS**.
- [Tauri migration integration](https://github.com/MousaXD/scriptotar/actions/runs/42088502418): Rust workspace, Svelte, Python sidecar and Rust↔sidecar integration jobs **SUCCESS**; integrated Tauri build still **IN PROGRESS**; Rust advisories job **FAILED** on `RUSTSEC-2026-0381` (`webpki-roots 1.0.9`), a Rust dependency issue already escalated to Agent 01 and Agent 07. No suppression permitted.
- [Live platform E2E](https://github.com/MousaXD/scriptotar/actions/runs/42088502215): **FAILED BEFORE PROVIDER TESTS**, during Ubuntu `apt-get install ffmpeg`: both bounded install/update attempts exhausted their 180-second limits, ending with exit code 124 ([job 127459982490](https://github.com/MousaXD/scriptotar/actions/runs/42088502215/job/127459982490)). **No live provider or model inference result from this run.** Earlier `eccf55561d` run observed real Instagram/TikTok inference success and YouTube bot challenge; do not extrapolate those observations to this SHA.

**Status:** IN PROGRESS. Exact HEAD has not passed every required gate. No hardware throughput benchmark, GPU inference, multilingual accuracy measurement, or native Windows process-tree termination test was performed. Keep PR #75 draft, do not merge.

## New certification evidence and follow-up changes (2026-11-05)

### Exact pre-follow-up HEAD: `51988102acc26af58a0a141f520cfa7e96271b65`

All eight GitHub Actions runs for this 40-character SHA have completed. Results:

| Required check | Evidence | Result |
| --- | --- | --- |
| CI | [42093221109](https://github.com/MousaXD/scriptotar/actions/runs/42093221109) | **SUCCESS** |
| Python Transcription Sidecar | [42093221115](https://github.com/MousaXD/scriptotar/actions/runs/42093221115) | **SUCCESS**, including protocol/unit tests and pinned dependency imports |
| Rust workspace and Rust ↔ sidecar integration | [42093221235](https://github.com/MousaXD/scriptotar/actions/runs/42093221235) | **SUCCESS** individual jobs |
| Integrated Tauri build | [42093221235](https://github.com/MousaXD/scriptotar/actions/runs/42093221235) | **SUCCESS** individual job |
| Linux .deb and installed-runtime legal/compliance checks | [42093221077](https://github.com/MousaXD/scriptotar/actions/runs/42093221077) | **SUCCESS** |
| Windows NSIS installer and packaged-runtime smoke | [42093220994](https://github.com/MousaXD/scriptotar/actions/runs/42093220994) | **SUCCESS**; native Windows descendant termination not exercised |
| MCP E2E and security hygiene | [42093220983](https://github.com/MousaXD/scriptotar/actions/runs/42093220983), [42093221015](https://github.com/MousaXD/scriptotar/actions/runs/42093221015) | **SUCCESS** |
| Rust advisories | [42093221235, job 127477063329](https://github.com/MousaXD/scriptotar/actions/runs/42093221235/job/127477063329) | **FAILED**: `webpki-roots 1.0.9` has `RUSTSEC-2026-0381`, fixed in >=1.0.10. Agent 01 owns dependency remediation; do not suppress |
| Live YouTube / Instagram / TikTok | [42093220987, job 127476426666](https://github.com/MousaXD/scriptotar/actions/runs/42093220987/job/127476426666) | **FAILED overall**: YouTube bot-sign-in challenge. Instagram **real download + 15-second Whisper inference passed** (3 segments). TikTok **real download + inference passed** (0 speech segments). No authentication bypass, no mocks or alternate provider URL used |

### Additional changes after this verified HEAD

1. `runtime_licenses.py` records the SHA-256 of the **copied Python runtime license bytes** in `RUNTIME-LICENSES.json` as `python_runtime.license_sha256`.
2. `validate_runtime.py` fails closed if an installed runtime lacks the checksum or if the shipped Python license file does not match it. `test_runtime_licenses.py` uses deterministic offline fixtures to test correct bytes, mutated bytes and missing digest. The reviewed CPython 3.12.16 provenance/version fallback and its pinned upstream hash remain in force.
3. `test_subprocess.py` adds a Windows-only **real child-process** cancellation test. It obtains an OS process handle before termination, waits for the descendant to exit, and verifies the next job works. This is **not** a mock, but **still requires an actual Windows CI runner** to execute; the Linux sidecar test workflow skips Windows-only tests. Agents 01 and 07 were notified through PR #73 and PR #76 respectively to coordinate runner coverage.

A subsequent CI run on the **new final HEAD** must certify these changes, including installed Linux and Windows runtime licensing. An integrated release candidate must additionally resolve or explicitly block on the Rust advisory and live YouTube provider test. Do not mistake real Instagram/TikTok inference for Arabic/English/mixed-language accuracy validation or CUDA inference.

**Benchmarks:** No CPU/GPU speed, memory, throughput, or checkpoint-hashing runtime benchmark has been measured. The recovery integrity check adds bounded 1 MiB streaming I/O and no uncontrolled inference concurrency.

## Performance tradeoffs / open questions

- Recovery integrity intentionally reads the completed media file once to hash it and once upon attempted restart reuse. This is bounded-memory O(media size) I/O; no throughput benchmark has been measured. Avoid claiming performance gains without measurements.
- Downloaded outputs now incur an ffprobe call to enforce true duration. ffprobe already has a 30-second subprocess timeout. No numerical wall-clock benchmark exists yet.
- The sequential engine design protects typical desktops against unbounded CPU/GPU concurrency; resource-aware parallelism requires profiling, memory budgets and explicit orchestration agreement.
- A child that is already orphaned before the supervisor attempts Windows `taskkill` may evade its process-tree walk; OS job-object supervision would provide stronger guarantees but needs real Windows validation.
- End-to-end per-job watchdog timeouts for a hung Whisper runtime remain to be evaluated against legitimate very long transcriptions.
- Model-cache cleanup, provider access restrictions, and installed-app sidecar licensing/compliance need continued targeted certification.

## Agent coordination

- Agent 02: no database or job-state persistence schema changes were made; shared recovery implications should be reviewed during PR integration.
- Agent 01: Rust public IPC and architecture untouched.
- Agent 07: request explicit installed Windows/Linux runtime checks once CI is green.

**Merge policy:** Draft PR only. Agent 00 decides review and integration; Agent 04 must not self-merge.
