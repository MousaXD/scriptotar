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

## Certification continuation — exact 2026-11-05 SHA `ca2fe9d8527804a5ed268bd88e4b32a9f8a987e1`

- [Transcription Sidecar 42098689298](https://github.com/MousaXD/scriptotar/actions/runs/42098689298): **SUCCESS** — actual pinned dependency imports and Python protocol/unit suite (the real Windows descendant test is **SKIPPED** on Linux).
- [CI 42098689261](https://github.com/MousaXD/scriptotar/actions/runs/42098689261): **SUCCESS**.
- [Linux Tauri Package 42098689195](https://github.com/MousaXD/scriptotar/actions/runs/42098689195): **SUCCESS** — package build, installed runtime/attribution smoke and legal inventory checks including Python runtime license checksum.
- [Windows Tauri Installer 42098689211](https://github.com/MousaXD/scriptotar/actions/runs/42098689211): **SUCCESS** — installer smoke. This does **not** run the new native Windows child-process test; Agent 01 was asked to add Windows runner coverage in workflow ownership.
- [Security hygiene 42098689188](https://github.com/MousaXD/scriptotar/actions/runs/42098689188): **SUCCESS**.
- [Tauri migration integration 42098689219](https://github.com/MousaXD/scriptotar/actions/runs/42098689219): **FAILED as aggregate** because [Rust advisories job 127495428177](https://github.com/MousaXD/scriptotar/actions/runs/42098689219/job/127495428177) finds `RUSTSEC-2026-0381` in `webpki-roots 1.0.9` (fixed in >=1.0.10). Rust workspace, Svelte, Python, Rust↔sidecar and integrated Tauri build jobs individually **SUCCESS**. Do **not** relax or suppress the advisory, owned by Rust/release agents.
- [MCP E2E 42098689289](https://github.com/MousaXD/scriptotar/actions/runs/42098689289): **FAILED in Ubuntu desktop-dependency apt install** after package mirrors repeatedly stalled and the step exited 124 ([job 127495431211](https://github.com/MousaXD/scriptotar/actions/runs/42098689289/job/127495431211)). Earlier individual MCP tests passed within that workflow before the apt step. This is a runner/package-network blocker and not proof of a functional MCP regression. No workflow or dependency changes were made by Agent 04; escalate to the appropriate owners.
- [Real YouTube Instagram TikTok E2E 42098689240](https://github.com/MousaXD/scriptotar/actions/runs/42098689240): **FAILED overall**. [Job 127495984915](https://github.com/MousaXD/scriptotar/actions/runs/42098689240/job/127495984915) reports YouTube bot-sign-in request and `SOURCE_ACCESS_FAILED`, with no YouTube download/inference; real Instagram download and 15-second Whisper inference **PASSED** (3 segments, 16.56 seconds reported elapsed), real TikTok download and inference **PASSED** (0 segments, 2.75 seconds elapsed). These are per-case **observed wall-clock timings**, **not** a controlled performance benchmark, speedup, CUDA test or language-accuracy certification.

### Follow-up media correctness hardening

Independent code audit identified an additional terminal-event race: `EngineSupervisor._abort_engine_protocol` and the unexpected-worker-exit path cleared `active_job_id` under the start/cancel lock but emitted the fatal error after unlocking. A concurrently accepted successor job could then appear before the previous job's failure event. Agent 04 changed both paths to emit while still holding the shared arbitration lock, preserving the existing diagnostic code and retryable flag. Two deterministic adversarial tests in `test_supervisor_races.py` launch a competing thread inside the event-write callback and assert it cannot acquire the supervisor state lock during either fatal emission. **These follow-up changes require green CI on their new exact HEAD**. The new test is a concurrency fixture, not real model inference.

**Compatibility and limits:** no models, formats, providers, device modes, locale options or application-owned security checks were removed. GPU performance, mixed Arabic/English recognition accuracy, and actual Windows descendant cleanup remain unmeasured/unverified. No self-merge.

## Certification checkpoint — 2026-11-06, pre-ledger code HEAD `6af12f28a35daf38ffe91b0e2f3eb05bd1855b7c`

**STATUS: IN PROGRESS; READY FOR AGENT 00 REVIEW: NO.** This checkpoint records the observed *code SHA*. Updating this ledger advances the PR HEAD and requires a new exact-SHA CI pass before any final sign-off.

| Gate | Exact-SHA workflow or job | Verified result |
| --- | --- | --- |
| Python sidecar | [Transcription Sidecar 42104436633](https://github.com/MousaXD/scriptotar/actions/runs/42104436633), [unit job 127515424217](https://github.com/MousaXD/scriptotar/actions/runs/42104436633/job/127515424217) | **SUCCESS** — 69 Python tests, 1 Windows-only test skipped; includes both new fatal-error event-order tests and license-byte integrity fixtures. Import check success is separate from model inference |
| General CI | [CI 42104436574](https://github.com/MousaXD/scriptotar/actions/runs/42104436574) | **SUCCESS** |
| Security hygiene | [Security hygiene 42104436830](https://github.com/MousaXD/scriptotar/actions/runs/42104436830) | **SUCCESS** |
| Windows NSIS installer | [Windows Tauri Installer 42104436475](https://github.com/MousaXD/scriptotar/actions/runs/42104436475), [job 127515427029](https://github.com/MousaXD/scriptotar/actions/runs/42104436475/job/127515427029) | **SUCCESS**. Installer and packaged-runtime smoke test; does **not** exercise the Windows-only actual child-process handle regression. Agent 01/07 asked to arrange that runner invocation |
| Linux .deb + installed-runtime validation | [Linux Tauri Package 42104436908](https://github.com/MousaXD/scriptotar/actions/runs/42104436908), [job 127515422150](https://github.com/MousaXD/scriptotar/actions/runs/42104436908/job/127515422150) | **IN PROGRESS** while installing Tauri system dependencies; no pass or failure claimed |
| Tauri / Rust ↔ sidecar integration | [Tauri migration integration 42104436990](https://github.com/MousaXD/scriptotar/actions/runs/42104436990) | **IN PROGRESS/QUEUED as aggregate**. Rust workspace job **SUCCESS**, Rust↔sidecar, Python sidecar and Svelte frontend jobs **QUEUED** when inspected; [Supply-chain job 127515424444](https://github.com/MousaXD/scriptotar/actions/runs/42104436990/job/127515424444) **FAILED** on pre-existing `webpki-roots 1.0.9` / `RUSTSEC-2026-0381` (>=1.0.10 required). Rust dependency owner Agent 01, release review Agent 07; no weakening or ignore |
| MCP E2E | [MCP E2E 42104436548](https://github.com/MousaXD/scriptotar/actions/runs/42104436548) | **CANCELLED**, not a pass, no new functional regression established |
| Live three-provider E2E | [Real YouTube Instagram TikTok E2E 42104436580](https://github.com/MousaXD/scriptotar/actions/runs/42104436580), [job 127515695009](https://github.com/MousaXD/scriptotar/actions/runs/42104436580/job/127515695009) | **FAILED overall**: YouTube was challenged for bot verification; correctly returned `SOURCE_ACCESS_FAILED`, download/inference not run. Instagram **real download and Faster Whisper inference PASSED** (3 segments; 15-second clip; observed elapsed 15.68 seconds). TikTok **real download/inference PASSED** (0 speech segments; 10.8-second clip; observed elapsed 4.02 seconds). No source/authentication bypass, substitute or skip |

**Known limitations:** Windows real-descendant cancellation remains only mocked in the general unit suite because its native counterpart is skipped on Linux and has not been invoked in the Windows installer workflow. Native GPU CUDA inference, Arabic/mixed-language accuracy, large-model throughput, and systematic memory/performance benchmarks remain unverified. The observed live excerpt elapsed times above are **per-run diagnostics**, not repeatable controlled benchmarks or speedups. Recovery uses bounded-memory streaming SHA-256 and ffprobe incurs bounded additional disk/process work.

**Feature compatibility:** all existing model, provider, output format, language, quality, cookie-source, and CPU/GPU choices are preserved; security and licensing checks remain fail-closed. No changes to Rust dependency lockfiles, branch protection, external authentication, CI workflows, base branch or `main` by Agent 04.

## 2026-11-06 continued certification — exact code/ledger HEAD `77fd7162f6c16da80748f003c1f37f2b986f0c44`

- [Python Transcription Sidecar 42107804077](https://github.com/MousaXD/scriptotar/actions/runs/42107804077), [job 127527145269](https://github.com/MousaXD/scriptotar/actions/runs/42107804077/job/127527145269): **SUCCESS**, 69 Python tests with 1 Windows-only native process-tree test skipped. Real dependency imports also **SUCCESS**. Existing fatal worker-crash and protocol-error ordering tests and bundled Python-license digest tests **PASSED**; they use deterministic fixtures, not live model inference.
- [CI 42107804214](https://github.com/MousaXD/scriptotar/actions/runs/42107804214), [Windows installer 42107804157](https://github.com/MousaXD/scriptotar/actions/runs/42107804157), [MCP E2E 42107804301](https://github.com/MousaXD/scriptotar/actions/runs/42107804301), [security hygiene 42107804221](https://github.com/MousaXD/scriptotar/actions/runs/42107804221): all **SUCCESS** on this code SHA.
- [Linux Tauri Package 42107804299](https://github.com/MousaXD/scriptotar/actions/runs/42107804299): **IN PROGRESS** at inspection; [job 127527389985](https://github.com/MousaXD/scriptotar/actions/runs/42107804299/job/127527389985) was executing repeated installed-app IPC/persistence E2E. No Linux completion is claimed.
- [Tauri migration integration 42107804115](https://github.com/MousaXD/scriptotar/actions/runs/42107804115): Rust workspace, Svelte, Python sidecar, Rust↔sidecar individual jobs **SUCCESS**. [Supply chain job 127527153592](https://github.com/MousaXD/scriptotar/actions/runs/42107804115/job/127527153592) **FAILED**: `webpki-roots 1.0.9`, `RUSTSEC-2026-0381`, update >=1.0.10 required; coordinated to Rust/QA owners. Integrated Tauri build was queued during inspection, subsequently the aggregate was **CANCELLED** after newer commits; not an aggregate pass.
- [Live YouTube/Instagram/TikTok E2E 42107804328](https://github.com/MousaXD/scriptotar/actions/runs/42107804328), [job 127527147207](https://github.com/MousaXD/scriptotar/actions/runs/42107804328/job/127527147207): **FAILED overall** because YouTube access required bot verification and produced `SOURCE_ACCESS_FAILED` before download/transcription. Instagram real download and 15-second Faster Whisper excerpt inference **PASSED** (3 speech segments; 15.18 s observed elapsed); TikTok real download/inference **PASSED** (0 speech segments; 3.21 s observed elapsed). These are individual diagnostic times, **not controlled benchmarks**, speedup claims, or GPU/multilingual accuracy checks. Access safeguards and real-provider test failure remain visible.

### Additional Agent 04 bug fixed after that SHA

Independent inspection found that a normal `cancel()` acknowledgement had the same event-order race previously repaired for worker crashes: the supervisor cleared its job/cancel state under `RLock`, released it, then published `cancelled`. A concurrent incoming job could be accepted before the prior job's cancellation event. The cancelled acknowledgement is now published under the same arbitration lock, and still **is not emitted if process termination raises**. Regression cases `test_cancelled_acknowledgement_precedes_new_job_acceptance` and `test_cancel_does_not_acknowledge_termination_failure` prove these boundaries with deterministic competing-thread/failed-termination fixtures. This does not change process-group termination, provider access, failure codes, output formats, supported models, hardware fallback, or current protocol. No benchmark was measured. Native Windows descendant cleanup still needs the Windows-only test to run on Windows CI. This code change must pass its *own* exact-SHA CI before any certification.

**Status: IN PROGRESS; READY FOR AGENT 00 REVIEW: NO.** Audit-documentation commit itself advances HEAD again, so certification must always reference the final exact commit. No merges to base or main.

## Certification checkpoint — 2026-11-06 01:29 GST, code HEAD `8e5a030a578ec13c1586db9ca17e1685b8c890a6`

**STATUS: IN PROGRESS; READY FOR AGENT 00 REVIEW: NO.** The code HEAD passed [Transcription Sidecar 42111000726](https://github.com/MousaXD/scriptotar/actions/runs/42111000726): [unit job 127538202760](https://github.com/MousaXD/scriptotar/actions/runs/42111000726/job/127538202760) ran **71 tests, 1 skipped** (Windows-only real descendant cleanup). The two newly added cancellation-ordering and failed-termination tests **PASSED**. [Real dependency imports job 127538203232](https://github.com/MousaXD/scriptotar/actions/runs/42111000726/job/127538203232) **PASSED**. These tests use deterministic mocks/fixtures and real imports, **not** a live GPU or model benchmark.

- [MCP E2E 42111001096](https://github.com/MousaXD/scriptotar/actions/runs/42111001096) **SUCCESS**.
- [General CI 42111000821](https://github.com/MousaXD/scriptotar/actions/runs/42111000821) **IN PROGRESS** (installing Linux Tauri native libraries; no final result).
- [Windows Tauri Installer 42111000490](https://github.com/MousaXD/scriptotar/actions/runs/42111000490) **IN PROGRESS** (bundling NSIS installer). This packaging workflow is not a native descendant-process termination test.
- [Linux Tauri Package 42111000519](https://github.com/MousaXD/scriptotar/actions/runs/42111000519) **IN PROGRESS** (installing pinned Tauri CLI).
- [Tauri migration integration 42111000417](https://github.com/MousaXD/scriptotar/actions/runs/42111000417) **QUEUED/IN PROGRESS** as aggregate: Rust↔sidecar integration and Svelte jobs **SUCCESS** individually; Rust workspace, Python sidecar, supply-chain jobs **QUEUED** at inspection. Prior supply-chain failure on `webpki-roots 1.0.9` / `RUSTSEC-2026-0381` remains unresolved in previously inspected SHA; do not assert result of this queued job.
- [Security hygiene 42111000919](https://github.com/MousaXD/scriptotar/actions/runs/42111000919) **PENDING**.
- [Real YouTube Instagram TikTok E2E 42111000447](https://github.com/MousaXD/scriptotar/actions/runs/42111000447), [job 127538374606](https://github.com/MousaXD/scriptotar/actions/runs/42111000447/job/127538374606) **FAILED overall** because YouTube required bot sign-in (`SOURCE_ACCESS_FAILED`, no download or inference). Instagram **real download + Faster Whisper inference PASSED** (15 s excerpt, 3 segments, 35.48 s observed elapsed); TikTok **real download + inference PASSED** (10.8 s excerpt, 0 segments, 4.87 s observed elapsed). These are per-run diagnostics, not controlled benchmarks or claims about multilingual accuracy, CUDA, or speedup.

All supported models, formats, providers, language, quality and device options remain intact. Authentication, runtime licensing, and fail-closed behavior were not weakened. Native Windows process-tree cleanup remains unverified on Windows. The documentation-only commit recording this checkpoint advances the SHA and **requires fresh exact-SHA validation**. No base/main writes or self-merge.

## Exact-SHA certification checkpoint — 2026-11-06 01:51 GST, pre-documentation HEAD `b5eae7d59bab2305062c7d167181a16b01fe6e7c`

**STATUS: IN PROGRESS; READY FOR AGENT 00 REVIEW: NO.** Exact-SHA results:
- [Transcription Sidecar 42113176987](https://github.com/MousaXD/scriptotar/actions/runs/42113176987), [unit job 127545829271](https://github.com/MousaXD/scriptotar/actions/runs/42113176987/job/127545829271): **SUCCESS**, 71 tests, 1 skipped Windows-only native descendant cleanup. Both new cancellation/failed-termination regressions **PASSED**. Real dependency imports **SUCCESS** (not live inference).
- [CI 42113178938](https://github.com/MousaXD/scriptotar/actions/runs/42113178938), [Windows installer 42113177233](https://github.com/MousaXD/scriptotar/actions/runs/42113177233), [Linux installed package 42113177150](https://github.com/MousaXD/scriptotar/actions/runs/42113177150), [MCP E2E 42113176842](https://github.com/MousaXD/scriptotar/actions/runs/42113176842), [Security hygiene 42113177624](https://github.com/MousaXD/scriptotar/actions/runs/42113177624): **SUCCESS**. Packaging success does not execute the skipped Windows-native child termination test.
- [Tauri migration integration 42113177040](https://github.com/MousaXD/scriptotar/actions/runs/42113177040): **IN PROGRESS** as aggregate; Rust↔sidecar, Svelte and Python sidecar individual jobs **SUCCESS**; Rust workspace **IN PROGRESS** installing native dependencies. [Supply-chain job 127545837663](https://github.com/MousaXD/scriptotar/actions/runs/42113177040/job/127545837663) **FAILED**: `webpki-roots 1.0.9` / `RUSTSEC-2026-0381`, fixed >=1.0.10. Agent 01/07 own remediation and release gate; no ignore/suppression.
- [Real provider E2E 42113176840](https://github.com/MousaXD/scriptotar/actions/runs/42113176840), [job 127545828991](https://github.com/MousaXD/scriptotar/actions/runs/42113176840/job/127545828991): **FAILED overall** because YouTube required bot sign-in, correctly reported `SOURCE_ACCESS_FAILED`, no YouTube download or inference. Instagram real download + 15 s Faster Whisper inference **PASSED** (3 segments, 17.97 s observed elapsed); TikTok real download + inference **PASSED** (0 segments, 2.88 s observed elapsed). These are unbenchmarked per-run diagnostics, not hardware throughput or multilingual accuracy evidence.

**Compatibility:** All providers, models, languages, output formats, device choices, local input, authentication boundaries and license integrity checks remain preserved. **Unverified:** native Windows descendant termination, CUDA, Arabic/mixed-language accuracy, repeatable CPU/GPU/memory benchmarks. No new media defect was established by these failures; no speculative code or workflow changes. This ledger commit will itself advance HEAD, so fresh exact-HEAD workflows are needed before certification. Never merge to base/main.

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
