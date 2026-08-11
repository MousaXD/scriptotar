# Scriptotar Next transcription sidecar

This directory contains the Python media/transcription boundary used by Scriptotar Next. Python owns media execution and Faster Whisper behavior; Rust owns application persistence, durable jobs, orchestration, AI/research state, and the desktop lifecycle.

The sidecar component version is currently `0.1.0`. Its public JSON Lines wire protocol is independently versioned as protocol `1`.

## Responsibilities

The transcription sidecar owns:

- yt-dlp media download for the supported platform allowlist;
- local media preparation;
- Faster Whisper model loading and transcription;
- segment and word timestamp generation;
- TXT, cleaned TXT, timestamp TXT, SRT, VTT, and JSON artifacts;
- conservative stage-scoped partial-artifact recovery;
- media-engine process isolation and cancellation.

It does **not** own:

- Scriptotar Next's SQLite application database;
- projects, creators, research/watchlist persistence, or job history;
- Tauri application lifecycle;
- AI provider credentials;
- frontend state.

See [`PROTOCOL.md`](./PROTOCOL.md) for the complete versioned JSONL contract.

## Layout

```text
sidecars/transcription/
  sidecar.py                      development/public supervisor entry point
  build_runtime.py                production runtime builder
  validate_runtime.py             packaged-runtime smoke test
  ytdlp_entry.py                  dedicated packaged research yt-dlp entry point
  engine_entry.py                 packaged heavy-engine entry point
  bootstrap.py                    development Python environment installer
  requirements-engine.txt         pinned engine dependencies
  requirements-bundle.txt         pinned packaging dependencies
  scriptotar_sidecar/
    version.py                    sidecar/protocol version sources
    protocol.py                   strict public JSONL parser/writer
    service.py                    persistent supervisor and cancellation
    engine_worker.py              private persistent engine process
    engine.py                     yt-dlp / Faster Whisper execution
    validation.py                 URL/path/model/options policy
    recovery.py                   conservative partial-artifact reuse
    formatting.py                 transcript/subtitle serialization
    capabilities.py               startup capability report
  tests/
```

The supervisor isolates the heavy engine in its own process group. Successful sequential jobs can reuse that engine and loaded Whisper model. Cancellation terminates the engine group and descendants; later work can start a fresh engine.

## Development run

The source entry point is:

```bash
python3 sidecars/transcription/sidecar.py
```

For a dedicated development environment without modifying system Python:

```bash
python3 sidecars/transcription/bootstrap.py --venv ~/.local/share/scriptotar/transcription-venv
~/.local/share/scriptotar/transcription-venv/bin/python sidecars/transcription/sidecar.py
```

`bootstrap.py` installs engine dependencies and verifies imports. It does **not** download a Whisper model. Faster Whisper obtains the selected model when an actual transcription requires an uncached model.

## Rust / Tauri host contract

The integrated Rust orchestrator spawns this boundary with piped stdin/stdout/stderr.

The host:

1. launches without a shell;
2. waits for `ready` and verifies protocol `1` compatibility;
3. treats stdout as protocol traffic and stderr as diagnostics;
4. persists the Rust-owned job before sending `transcribe`;
5. translates progress/result/error events into Rust service/repository updates;
6. sends `cancel` for user cancellation;
7. sends `shutdown` on orderly application exit with a host-side kill fallback;
8. treats unexpected sidecar death as interrupted/failed work, never successful completion.

The frontend never spawns this process directly. The sidecar never opens or mutates Scriptotar Next's application SQLite database.

## Packaged Next runtime

Windows and Linux Scriptotar Next packages do not require a separately installed Python interpreter.

`build_runtime.py` produces the `transcription-runtime/` resource tree used by the Tauri package workflows. It contains:

```text
scriptotar-transcription[.exe]   PyInstaller supervisor
sidecar.py                        compatibility/runtime marker
scriptotar-ytdlp[.exe]           dedicated research command
engine/scriptotar-engine[.exe]   isolated heavy engine + Python dependencies
ffmpeg/ffmpeg[.exe]
ffmpeg/ffprobe[.exe]
RUNTIME-VERSIONS.txt
```

Production Tauri startup resolves that resource directory and configures the Rust orchestrator through the `SCRIPTOTAR_SIDECAR_*` environment contract. Development/test overrides remain supported.

The packaged runtime includes Faster Whisper, yt-dlp, FFmpeg and ffprobe. Whisper model weights are intentionally downloaded and cached on first uncached use rather than embedded in every installer.

See `../../docs/NEXT_DISTRIBUTION.md` for package-level details.

## Dedicated creator-research command

The packaged `scriptotar-ytdlp` executable is separate from the transcription supervisor. Rust research services use it for creator/profile metadata retrieval instead of treating the supervisor as a general Python command runner.

This separation preserves a narrow public transcription protocol while allowing the package to remain self-contained for research dependencies.

## Security policy

- stdout is reserved for protocol JSON only;
- user-controlled subprocess values are arguments, never shell command strings;
- no `shell=True` is used;
- URL domains, schemes, ports, local extensions, model names, devices, qualities, languages, and cookie-browser selections are validated below the UI layer;
- browser-cookie use over plaintext HTTP is rejected;
- extractor metadata returned to Rust is reduced to an allowlist;
- `job.json` does not persist the cookie-browser selection;
- the sidecar applies restrictive POSIX file defaults for private partial state;
- public/internal protocol messages are size-bounded and malformed/out-of-order host events fail closed;
- AI API keys do not belong in the sidecar protocol.

## Recovery semantics

A complete downloaded/copied media artifact may be reused only when its checkpoint and fingerprint prove it belongs to the same job inputs.

Whisper inference is not described as resumable. Interrupted transcription restarts transcription from the beginning. Unproven partial download fragments are not promoted to completed media.

See `PROTOCOL.md` for the exact event and recovery rules.

## Tests

Fast tests use fixture/fake engine processes, so they do not contact social platforms or download Whisper models:

```bash
cd sidecars/transcription
PYTHONPATH=. python3 -m compileall -q .
PYTHONPATH=. python3 -m unittest discover -s tests -v
```

CI separately installs the real pinned engine dependencies, runs `pip check`, and verifies Faster Whisper/yt-dlp imports without downloading a model.

Package workflows additionally build the PyInstaller runtime and run:

```bash
python sidecars/transcription/validate_runtime.py <runtime-directory>
```

That smoke test validates packaged imports, private FFmpeg/ffprobe resolution, the dedicated yt-dlp command, and supervisor protocol startup/ping/shutdown.

## Classic boundary

The root-level `worker.py` and Classic Python/Tkinter application remain a separate supported product line. Next's packaged sidecar does not require deleting or rewriting that Classic runtime.
