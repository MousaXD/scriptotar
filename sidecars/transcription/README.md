# Scriptotar Transcription Sidecar

This directory is the Tauri-era boundary around Scriptotar's existing media and Faster Whisper behavior. It deliberately keeps Python responsible for media execution/transcription while Rust owns application persistence and orchestration.

## Responsibilities

The sidecar owns:

- yt-dlp media download for the supported platform allowlist;
- local media preparation;
- Faster Whisper model loading and transcription;
- segment and word timestamp generation;
- TXT, cleaned TXT, timestamp TXT, SRT, VTT, and JSON artifacts;
- stage-scoped partial artifact recovery;
- media-engine process isolation and cancellation.

It does **not** own:

- Scriptotar's SQLite database;
- projects, creators, research/watchlist persistence, or job history;
- Tauri application lifecycle;
- AI provider credentials;
- frontend state.

See [`PROTOCOL.md`](./PROTOCOL.md) for the complete versioned JSONL contract.

## Layout

```text
sidecars/transcription/
  sidecar.py                      public stdin/stdout supervisor
  bootstrap.py                    Python environment installer only
  requirements-engine.txt         pinned engine dependencies
  scriptotar_sidecar/
    version.py                    single sidecar/protocol version source
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

The supervisor isolates the heavy engine in its own process group. Successful sequential jobs reuse that engine and therefore reuse loaded Whisper models. Cancellation kills the engine group and descendants, then the next job starts a fresh engine.

## Development run

The public entry point is:

```bash
python3 sidecars/transcription/sidecar.py
```

For a dedicated environment without modifying system Python:

```bash
python3 sidecars/transcription/bootstrap.py --venv ~/.local/share/scriptotar/transcription-venv
~/.local/share/scriptotar/transcription-venv/bin/python sidecars/transcription/sidecar.py
```

`bootstrap.py` installs Python dependencies and verifies imports. It does **not** download a Whisper model. Faster Whisper downloads/uses the selected model when an actual transcription asks for it.

## Rust / Tauri host contract

Agent 1's Rust host should spawn the sidecar with piped stdin/stdout/stderr. The recommended development command is:

```text
<sidecar-venv-python> <repo-or-bundle>/sidecars/transcription/sidecar.py
```

Recommended environment:

```text
PYTHONUNBUFFERED=1
```

The parent should:

1. spawn the sidecar without a shell and with piped stdin/stdout/stderr;
2. wait for `ready` and verify protocol `1` is supported;
3. keep stderr as diagnostics only;
4. persist the Rust-owned job before sending `transcribe`;
5. translate progress/result/error events into Rust service/repository updates;
6. send `cancel` for user cancellation;
7. send `shutdown` on orderly application exit and enforce a host-side timeout/kill fallback;
8. treat unexpected sidecar process death as an interrupted Rust job, not as successful completion.

The frontend must never spawn this Python process directly. The sidecar must never open or mutate Scriptotar's application SQLite database.

For a packaged Tauri build, the exact bundled executable/venv strategy can change without changing protocol v1. The transport contract is stdin/stdout/stderr, not a hard-coded filesystem layout.

## Security policy

- stdout is reserved for protocol JSON only;
- user-controlled subprocess values are always argument values, never shell command strings;
- no `shell=True` is used;
- URL domains, schemes, ports, local extensions, model names, devices, qualities, languages, and cookie-browser selections are validated below the UI layer;
- browser-cookie use over plaintext HTTP is rejected;
- extractor metadata returned to Rust is reduced to an allowlist;
- `job.json` does not persist the cookie-browser selection;
- the sidecar applies a restrictive `umask` on POSIX and writes partial metadata with owner-only permissions;
- no AI API key belongs in this sidecar protocol.

## Recovery semantics

A complete downloaded/copied media artifact may be reused only when its checkpoint and fingerprint prove it belongs to the same job inputs. Whisper output is never described as resumable: interrupted transcription restarts transcription from the beginning. See `PROTOCOL.md` for the exact rules.

## Tests

Fast tests use a fake engine process, so they do not contact platforms or download models:

```bash
cd sidecars/transcription
PYTHONPATH=. python3 -m compileall -q .
PYTHONPATH=. python3 -m unittest discover -s tests -v
```

Coverage includes protocol parsing, ready/ping, malformed and unsupported protocol commands, URL/local validation, cancellation and descendant cleanup, sequential jobs, failed-then-successful jobs, engine crashes, interrupted partial state, transcript serialization, stdout/stderr separation, and clean shutdown.

CI separately installs the real pinned `yt-dlp` and `faster-whisper` dependencies and imports them. CI intentionally does not download a Whisper model and does not use live Instagram/TikTok/YouTube tests.

## Known boundary

The legacy root-level `worker.py` remains untouched so the existing Tkinter application keeps working during migration. Agent 4 can later switch the Rust host to this sidecar after Agent 1's job/persistence layer and Agent 2's frontend are integrated.
