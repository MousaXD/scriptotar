# Agent 04 — Media pipeline audit and validation ledger

**Owner:** Agent 04 (media/transcription)
**Base:** `feature/chatgpt-mcp-ci-20261026` at `517877b26df9028cdd32af69f6f5a88a68b948e9`
**Working branch:** `agent-04/media-engine-overhaul`
**PR:** [#75](https://github.com/MousaXD/scriptotar/pull/75)
**Status:** IN PROGRESS — CI and installed-runtime evidence pending

## Scope inspected

- `crates/scriptotar-media/src/lib.rs`: command/event data types, supported models and extension validation, public protocol.
- `crates/scriptotar-orchestrator/src/lib.rs`: single-worker job queue, subprocess lifecycle, Rust-owned transitions/persistence and cancellation commands.
- `sidecars/transcription/scriptotar_sidecar/{engine,engine_worker,service,recovery,validation,protocol}.py`: yt-dlp, ffprobe, Faster Whisper, CPU/GPU selection, output finalization, cancellation, restart recovery, JSONL.
- `sidecars/transcription/tests/` and `.github/workflows/transcription-sidecar.yml`: existing mocks, protocol tests and dependency-import checks.

## Highest-risk findings and implemented remediation

| Area | Original hazard | Remediation | Regression evidence |
| --- | --- | --- | --- |
| Recovery | A matching checkpoint accepted any nonempty file at an arbitrary stored path, including changed bytes and out-of-directory files. | Bind checkpoint to streaming SHA-256 and size, require copied/downloaded media remain inside the job partial directory, and only permit explicit copy-free local source as an external path. Legacy unverified checkpoints restart from scratch. | `test_recovery_integrity.py` |
| Cancellation | The stdout reader could publish a terminal result after a concurrent cancel flag was set; a replaced worker could publish into a new worker's job. | Serialize cancel-state/worker identity validation, event publication and terminal-state reset behind the same lock. | `test_supervisor_races.py` |
| Windows cancellation | `Popen.terminate()` on Windows does not recursively kill launched descendant FFmpeg/yt-dlp processes. | Start the worker in a Windows process group and request `taskkill /PID <pid> /T /F`, with parent-only fallback when unavailable. POSIX process-group behavior retained. | `test_windows_cleanup.py` (mock contract only; real Windows descendant cleanup not yet certified) |
| URL duration | A missing or inaccurate extractor duration could bypass the configured limit after download. | Re-probe downloaded media with ffprobe and enforce the limit on the measured value where available. Reject empty acquired output. | `test_download_limits.py` |
| Model initialization | Any automatic-CUDA model initialization error triggered CPU retry, including unrelated model cache/disk errors. | Retry only for recognized CUDA runtime failures; preserve explicit-CUDA fail-closed behavior. | `test_device_fallback.py` |
| Source errors | Downloader/metadata failures could surface as generic Whisper engine failures with misleading model-download advice. | Report `SOURCE_ACCESS_FAILED` for yt-dlp `DownloadError` and nonzero status, without invoking authentication bypasses. | `test_source_diagnostics.py` |
| Duration serialization | Nonfinite durations could become JSON NaN/Infinity, incompatible with strict Rust JSON parsing. | Accept only finite positive durations from provider metadata and ffprobe. | `test_source_diagnostics.py` |

## Compatibility contract

No public protocol version, supported provider/domain, model choice (`small`, `medium`, `turbo`, `large-v3`), output format (text, cleaned text, timestamped text, SRT, VTT, JSON), language option, quality option, cookie source option or device setting has been removed. GPU/CPU execution is deliberately still serial: uncontrolled multi-model concurrency could exhaust RAM/VRAM. Rust still owns durable job state and installed-runtime licensing checks have not been altered. No MCP or general desktop UI files are touched.

## Tests and evidence boundaries

- Python 3.12 sidecar compile/unittest and pinned dependency import checks: `.github/workflows/transcription-sidecar.yml`.
- Rust media/orchestration and cancellation: repository CI and integration workflows; inspect actual run jobs before certification.
- Windows/Linux installers: respective Tauri workflows; success there does not by itself prove real Windows descendant teardown or CUDA inference.
- Live YouTube, Instagram, TikTok: separate E2E workflow; network restrictions, rate limits and provider authentication may prevent success. Never relabel a blocked provider test as product success.
- Faster Whisper CPU/CUDA inference, Arabic/English/mixed-language accuracy and GPU memory behavior: unit fixtures are **mocks**, not actual model inference; these checks require model downloads and appropriate hardware or explicit recorded fixtures.

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
