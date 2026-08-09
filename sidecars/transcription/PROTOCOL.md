# Scriptotar Transcription Sidecar Protocol

The public host/sidecar protocol is **JSON Lines over stdin/stdout**. One JSON object is written per line. The current protocol version is defined only in `scriptotar_sidecar/version.py` and is currently `1`.

The sidecar is deliberately not an application server. It does not own Scriptotar's SQLite database, projects, creator analytics, AI provider keys, or Tauri lifecycle. Rust remains responsible for durable application state and interprets sidecar events into persisted job transitions.

## Transport rules

- Host writes UTF-8 JSON Lines to sidecar stdin.
- Sidecar writes UTF-8 JSON Lines to stdout.
- **stdout is protocol-only.** Library diagnostics and tracebacks are routed to stderr.
- The host must treat an invalid stdout line as a protocol violation rather than human console text.
- The sidecar handles one transcription job at a time and accepts multiple sequential jobs without restarting the process.
- Unknown object fields are rejected so accidental schema drift is visible.

Every public command and event contains:

```json
{"protocol":1,"type":"..."}
```

## Startup and capability negotiation

On startup the sidecar emits:

```json
{
  "protocol": 1,
  "type": "ready",
  "capabilities": {
    "sidecar_version": "0.1.0",
    "protocol_versions": [1],
    "features": {
      "url_input": true,
      "local_input": true,
      "cancellation": true,
      "persistent_engine": true,
      "word_timestamps": true,
      "formats": ["txt", "clean_txt", "timestamp_txt", "srt", "vtt", "json"]
    },
    "binaries": {"ffmpeg": true, "ffprobe": true},
    "python_packages": {"yt_dlp": "...", "faster_whisper": "..."},
    "supported_models": ["large-v3", "medium", "small", "turbo"],
    "supported_domains": ["instagram.com", "tiktok.com", "youtube.com", "youtu.be"]
  }
}
```

A missing dependency is reported as `null` in the capability report. Dependency imports themselves are validated in CI without downloading a Whisper model.

## Ping

Command:

```json
{"protocol":1,"type":"ping","request_id":"optional-correlation-id"}
```

Event:

```json
{"protocol":1,"type":"pong","request_id":"optional-correlation-id","sidecar_version":"0.1.0"}
```

## Transcribe

Command:

```json
{
  "protocol": 1,
  "type": "transcribe",
  "request_id": "optional-correlation-id",
  "job_id": "rust-owned-job-id",
  "input": {
    "kind": "file",
    "value": "/absolute/or/resolvable/path/video.mp4"
  },
  "output": {
    "root": "/home/user/Videos/Scriptotar"
  },
  "options": {
    "model": "medium",
    "device": "auto",
    "language": "auto",
    "quality": "720p",
    "cookies_browser": "none",
    "max_duration_seconds": 3600,
    "copy_source": true,
    "translate": false,
    "batched": false,
    "batch_size": 8,
    "keep_failed": false
  }
}
```

For URL input:

```json
"input":{"kind":"url","value":"https://www.youtube.com/watch?v=..."}
```

Accepted values are intentionally bounded:

- `model`: `small`, `medium`, `turbo`, `large-v3`
- `device`: `auto`, `cpu`, `cuda`
- `language`: `auto`, `ar`, `en`
- `quality`: `720p`, `1080p`, `best`, `audio-only`
- `cookies_browser`: `none`, `firefox`, `chrome`, `chromium`, `brave`, `edge`
- local extensions: `.mp4`, `.mkv`, `.mov`, `.webm`, `.m4v`, `.avi`
- URL domains: Instagram, TikTok, YouTube, and youtu.be subdomains only

The sidecar rejects embedded URL credentials, non-standard ports, unsupported domains, arbitrary model names, unknown options, and browser-cookie use over plaintext HTTP.

After validation and handoff to the engine:

```json
{"protocol":1,"type":"accepted","job_id":"rust-owned-job-id"}
```

Progress is stage-oriented. `percent` is optional because some work does not expose trustworthy precision:

```json
{"protocol":1,"type":"job_started","job_id":"..."}
{"protocol":1,"type":"progress","job_id":"...","stage":"downloading","percent":42.5,"message":"Downloading media."}
{"protocol":1,"type":"progress","job_id":"...","stage":"transcribing","message":"Loading Whisper model medium."}
{"protocol":1,"type":"progress","job_id":"...","stage":"processing","message":"Writing transcript artifacts."}
```

Successful result:

```json
{
  "protocol": 1,
  "type": "result",
  "job_id": "...",
  "result": {
    "source": {
      "title": "Example",
      "uploader": "Creator",
      "source_url": "https://...",
      "duration_seconds": 31.2,
      "extractor": "Youtube"
    },
    "transcript": {
      "text": "raw segment text",
      "clean_text": "cleaned text",
      "segments": [
        {"index":1,"start":0.0,"end":2.1,"text":"...","words":[]}
      ],
      "words": [
        {"start":0.0,"end":0.4,"word":" Hello","probability":0.99}
      ],
      "language": "en",
      "language_probability": 0.99,
      "duration_seconds": 31.2
    },
    "artifacts": {
      "text": "/.../transcript.txt",
      "clean_text": "/.../transcript_clean.txt",
      "timestamp_text": "/.../transcript_timestamps.txt",
      "srt": "/.../transcript.srt",
      "vtt": "/.../transcript.vtt",
      "json": "/.../transcript.json",
      "media": "/.../video.mp4"
    },
    "output_dir": "/.../20260809_Example"
  }
}
```

The extractor result is deliberately reduced to a small metadata allowlist. Raw yt-dlp extractor dictionaries are not returned to the host.

## Errors

Errors are structured:

```json
{
  "protocol": 1,
  "type": "error",
  "job_id": "optional-job-id",
  "error": {
    "code": "INVALID_MODEL",
    "message": "Unsupported Whisper model.",
    "retryable": false,
    "details": {"model":"..."}
  }
}
```

Protocol errors do not terminate the sidecar. A heavy-engine crash is isolated and reported as `ENGINE_CRASHED`; the supervisor can start a fresh engine for the next job.

## Cancellation

Command:

```json
{"protocol":1,"type":"cancel","job_id":"..."}
```

Event:

```json
{"protocol":1,"type":"cancelled","job_id":"...","reason":"user_requested"}
```

The protocol supervisor and transcription engine are separate processes. The engine starts in a dedicated process group on POSIX systems. Cancellation terminates that entire group, including descendant FFmpeg/yt-dlp processes, and a later job gets a fresh engine process. A successful, uncancelled engine stays alive between jobs so Faster Whisper can reuse a loaded model.

Cancellation is therefore **not** a pause/resume operation.

## Interrupted artifact recovery

Each in-progress job uses:

```text
<output-root>/.scriptotar-<job-id>.partial/
```

The partial directory contains a restricted-permission `checkpoint.json` with a fingerprint of the media-affecting inputs.

Recovery rules are intentionally conservative:

1. If no valid checkpoint exists, the partial directory is discarded and the job restarts from preparation/downloading.
2. If the checkpoint proves `media_ready`, the complete media artifact may be reused when the job fingerprint still matches.
3. Partial transcript/SRT/VTT/JSON files are deleted before retry. Faster Whisper transcription restarts from the beginning.
4. `.part`/fragment state is not called resumable by Scriptotar. Unproven partial downloads are discarded rather than advertised as safe resume.
5. Controlled failures are cleaned unless `keep_failed=true`. Unexpected process death naturally leaves the `.partial` checkpoint for a later retry.

This is stage reuse, not transcript resume.

## Shutdown and EOF

Graceful command:

```json
{"protocol":1,"type":"shutdown","request_id":"optional"}
```

The sidecar stops its engine and replies:

```json
{"protocol":1,"type":"shutdown","request_id":"optional"}
```

Then it exits with code `0`. EOF on stdin also shuts down the engine and exits cleanly. `Ctrl+C` exits `130`. An unexpected fatal supervisor exception exits `70`. A transcription engine crash does **not** crash the public protocol process; it becomes an `ENGINE_CRASHED` event.

## Protocol versioning

`scriptotar_sidecar/version.py` is the single source for both `SIDECAR_VERSION` and `PROTOCOL_VERSION`. Do not duplicate them in worker modules or transcript writers.

A breaking command/event schema change must increment `PROTOCOL_VERSION`. Additive capability values that old hosts can safely ignore do not require a protocol bump, but public object fields are otherwise strict to prevent silent drift.
