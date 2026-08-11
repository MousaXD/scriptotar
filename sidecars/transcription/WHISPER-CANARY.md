# Real Whisper runtime canary

`.github/workflows/whisper-runtime-canary.yml` is the heavyweight health check for Scriptotar Next's real packaged transcription path.

It is intentionally separate from normal pull-request CI. The final workflow runs weekly and supports `workflow_dispatch` for manual diagnosis.

## What it proves

The job builds the same self-contained transcription runtime used by the Tauri package workflows, validates its bundled executables, and then runs `sidecars/transcription/whisper_runtime_canary.py`.

The canary:

1. starts with a new empty Hugging Face cache;
2. generates a deterministic English speech fixture locally with `espeak-ng`;
3. converts that fixture with the packaged FFmpeg and verifies it with the packaged ffprobe;
4. starts the packaged `scriptotar-transcription` supervisor;
5. sends a normal protocol `transcribe` job using the production-supported `small` model on CPU;
6. exercises the packaged engine and normal `run_job` path, including faster-whisper/CTranslate2 inference and transcript artifact generation;
7. requires a non-empty transcript with at least 75% unique-word overlap with `The quick brown fox jumps over the lazy dog.`;
8. verifies that the initially empty model cache became meaningfully populated;
9. terminates the first packaged process;
10. starts a fresh packaged process with `HF_HUB_OFFLINE=1` and requires the same real inference path to succeed from that cache.

The second run is deliberately a fresh process so an in-memory `WhisperModel` instance cannot masquerade as disk-cache reuse.

## Model and cache

`small` is used because it is the smallest model currently accepted by Scriptotar's production sidecar validation. The canary does not add a special CI-only model to the product's supported model list.

The workflow uses a fresh cache beneath `${RUNNER_TEMP}/whisper-canary-work/hf-cache` on every run. It does not restore a GitHub Actions model cache before the first inference, so successful first-use inference demonstrates real model acquisition. The second inference reuses the same on-disk cache with Hugging Face offline mode enabled.

## Failure behavior

The workflow is not `continue-on-error`. Build, runtime validation, model download, inference, transcript matching, or offline-cache failures make the canary red.

The script classifies failures containing strong network/provider indicators as `provider-or-network`; other failures are classified as `runtime-or-inference`. The classification is diagnostic only and never converts a failure into success.

A JSON summary, the canary log, and `RUNTIME-VERSIONS.txt` are uploaded as short-lived diagnostics when available. The model cache itself is not uploaded.
