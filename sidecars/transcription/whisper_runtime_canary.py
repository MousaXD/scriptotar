from __future__ import annotations

import argparse
import json
import os
import queue
import re
import shutil
import subprocess
import sys
import threading
import time
from pathlib import Path
from typing import Any, TextIO

EXPECTED_PHRASE = "The quick brown fox jumps over the lazy dog."
PROTOCOL_VERSION = 1
PROVIDER_ERROR_MARKERS = (
    "huggingface",
    "hf.co",
    "http error",
    "httpsconnection",
    "connection error",
    "connection refused",
    "connection reset",
    "name resolution",
    "temporary failure in name resolution",
    "timed out",
    "timeout",
    "ssl error",
    "status code: 429",
    "status code: 500",
    "status code: 502",
    "status code: 503",
    "status code: 504",
)


class CanaryFailure(RuntimeError):
    pass


def _exe_name(stem: str) -> str:
    return f"{stem}.exe" if os.name == "nt" else stem


def _runtime_env(runtime: Path, cache: Path, *, offline: bool) -> dict[str, str]:
    env = os.environ.copy()
    env["SCRIPTOTAR_SIDECAR_ENGINE_EXECUTABLE"] = str(
        runtime / "engine" / _exe_name("scriptotar-engine")
    )
    env["SCRIPTOTAR_YTDLP_EXECUTABLE"] = str(runtime / _exe_name("scriptotar-ytdlp"))
    env["PATH"] = str(runtime / "ffmpeg") + os.pathsep + env.get("PATH", "")
    env["PYTHONUNBUFFERED"] = "1"
    env["HF_HOME"] = str(cache)
    env["HF_HUB_CACHE"] = str(cache / "hub")
    env["XDG_CACHE_HOME"] = str(cache / "xdg")
    if offline:
        env["HF_HUB_OFFLINE"] = "1"
    else:
        env.pop("HF_HUB_OFFLINE", None)
    return env


def _run_checked(args: list[str], *, env: dict[str, str] | None = None, timeout: int = 60) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        args,
        check=True,
        capture_output=True,
        text=True,
        env=env,
        timeout=timeout,
    )


def generate_fixture(runtime: Path, work_dir: Path) -> tuple[Path, float]:
    espeak = shutil.which("espeak-ng") or shutil.which("espeak")
    if not espeak:
        raise CanaryFailure("espeak-ng/espeak is required to generate the deterministic speech fixture")

    wav = work_dir / "canary-source.wav"
    media = work_dir / "canary-speech.mkv"
    _run_checked([espeak, "-v", "en-us", "-s", "145", "-w", str(wav), EXPECTED_PHRASE])

    ffmpeg = runtime / "ffmpeg" / _exe_name("ffmpeg")
    ffprobe = runtime / "ffmpeg" / _exe_name("ffprobe")
    if not ffmpeg.is_file() or not ffprobe.is_file():
        raise CanaryFailure("packaged ffmpeg/ffprobe executables are missing")

    _run_checked(
        [
            str(ffmpeg),
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-i",
            str(wav),
            "-ar",
            "16000",
            "-ac",
            "1",
            "-c:a",
            "pcm_s16le",
            str(media),
        ],
        timeout=60,
    )
    probe = _run_checked(
        [
            str(ffprobe),
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "default=noprint_wrappers=1:nokey=1",
            str(media),
        ],
        timeout=30,
    )
    try:
        duration = float(probe.stdout.strip())
    except ValueError as exc:
        raise CanaryFailure(f"ffprobe returned an invalid duration: {probe.stdout!r}") from exc
    if duration <= 0:
        raise CanaryFailure(f"ffprobe reported a non-positive duration: {duration}")
    return media, duration


def _pump_stdout(stream: TextIO, output: queue.Queue[str | None]) -> None:
    try:
        for raw in stream:
            output.put(raw)
    finally:
        output.put(None)


def _pump_stderr(stream: TextIO, lines: list[str]) -> None:
    for raw in stream:
        line = raw.rstrip()
        lines.append(line)
        print(f"[sidecar stderr] {line}", file=sys.stderr, flush=True)


def _next_event(
    output: queue.Queue[str | None],
    proc: subprocess.Popen[str],
    *,
    deadline: float,
    stderr_lines: list[str],
) -> dict[str, Any]:
    while True:
        remaining = deadline - time.monotonic()
        if remaining <= 0:
            raise CanaryFailure("timed out waiting for packaged sidecar protocol output")
        try:
            raw = output.get(timeout=min(1.0, remaining))
        except queue.Empty:
            if proc.poll() is not None:
                tail = "\n".join(stderr_lines[-30:])
                raise CanaryFailure(
                    f"packaged sidecar exited with code {proc.returncode} before completing the canary"
                    + (f"\nstderr tail:\n{tail}" if tail else "")
                )
            continue
        if raw is None:
            tail = "\n".join(stderr_lines[-30:])
            raise CanaryFailure(
                "packaged sidecar closed stdout before completing the canary"
                + (f"\nstderr tail:\n{tail}" if tail else "")
            )
        line = raw.strip()
        if not line:
            continue
        try:
            event = json.loads(line)
        except json.JSONDecodeError as exc:
            raise CanaryFailure(f"packaged sidecar emitted malformed JSON: {line[:1000]}") from exc
        if not isinstance(event, dict) or not isinstance(event.get("type"), str):
            raise CanaryFailure(f"packaged sidecar emitted an invalid event: {event!r}")
        return event


def _log_event(event: dict[str, Any]) -> None:
    event_type = event["type"]
    if event_type == "progress":
        print(
            "[sidecar progress] "
            f"stage={event.get('stage')} percent={event.get('percent')} message={event.get('message')}",
            flush=True,
        )
    elif event_type in {"ready", "accepted", "job_started", "shutdown"}:
        print(f"[sidecar] {event_type}", flush=True)


def transcribe_with_packaged_runtime(
    runtime: Path,
    media: Path,
    output_root: Path,
    cache: Path,
    *,
    model: str,
    offline: bool,
    timeout_seconds: int,
) -> dict[str, Any]:
    supervisor = runtime / _exe_name("scriptotar-transcription")
    marker = runtime / "sidecar.py"
    if not supervisor.is_file() or not marker.is_file():
        raise CanaryFailure("packaged transcription supervisor or sidecar marker is missing")

    output_root.mkdir(parents=True, exist_ok=True)
    env = _runtime_env(runtime, cache, offline=offline)
    proc = subprocess.Popen(
        [str(supervisor), str(marker)],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        bufsize=1,
        env=env,
    )
    assert proc.stdin is not None
    assert proc.stdout is not None
    assert proc.stderr is not None

    stdout_queue: queue.Queue[str | None] = queue.Queue()
    stderr_lines: list[str] = []
    stdout_thread = threading.Thread(target=_pump_stdout, args=(proc.stdout, stdout_queue), daemon=True)
    stderr_thread = threading.Thread(target=_pump_stderr, args=(proc.stderr, stderr_lines), daemon=True)
    stdout_thread.start()
    stderr_thread.start()

    result: dict[str, Any] | None = None
    failure: CanaryFailure | None = None
    deadline = time.monotonic() + timeout_seconds
    job_id = "weekly-whisper-canary-offline" if offline else "weekly-whisper-canary-download"

    try:
        while True:
            event = _next_event(stdout_queue, proc, deadline=deadline, stderr_lines=stderr_lines)
            _log_event(event)
            if event["type"] == "ready":
                break
            if event["type"] == "error":
                raise CanaryFailure(f"sidecar failed before ready: {event.get('error')!r}")

        command = {
            "protocol": PROTOCOL_VERSION,
            "type": "transcribe",
            "request_id": job_id,
            "job_id": job_id,
            "input": {"kind": "file", "value": str(media.resolve())},
            "output": {"root": str(output_root.resolve())},
            "options": {
                "model": model,
                "device": "cpu",
                "language": "en",
                "copy_source": True,
                "translate": False,
                "batched": False,
                "max_duration_seconds": 30,
                "keep_failed": True,
            },
        }
        proc.stdin.write(json.dumps(command, separators=(",", ":")) + "\n")
        proc.stdin.flush()

        while True:
            event = _next_event(stdout_queue, proc, deadline=deadline, stderr_lines=stderr_lines)
            _log_event(event)
            event_type = event["type"]
            if event_type == "error":
                failure = CanaryFailure(
                    "packaged transcription failed: "
                    + json.dumps(event.get("error"), ensure_ascii=False, sort_keys=True)
                )
                break
            if event_type == "result":
                payload = event.get("result")
                if not isinstance(payload, dict):
                    failure = CanaryFailure(f"result event had invalid payload: {event!r}")
                else:
                    result = payload
                break
    finally:
        try:
            proc.stdin.close()
        except OSError:
            pass
        try:
            proc.wait(timeout=20)
        except subprocess.TimeoutExpired:
            proc.kill()
            proc.wait(timeout=5)
        stdout_thread.join(timeout=2)
        stderr_thread.join(timeout=2)

    if failure is not None:
        tail = "\n".join(stderr_lines[-30:])
        if tail:
            raise CanaryFailure(f"{failure}\nstderr tail:\n{tail}")
        raise failure
    if result is None:
        raise CanaryFailure("packaged transcription completed without a result event")
    if proc.returncode != 0:
        tail = "\n".join(stderr_lines[-30:])
        raise CanaryFailure(
            f"packaged sidecar exited with code {proc.returncode} after result"
            + (f"\nstderr tail:\n{tail}" if tail else "")
        )
    return result


def normalized_words(text: str) -> list[str]:
    return re.findall(r"[a-z]+", text.lower())


def assert_reasonable_match(actual: str) -> float:
    actual_words = normalized_words(actual)
    if not actual_words:
        raise CanaryFailure("Whisper produced an empty transcript")
    expected = set(normalized_words(EXPECTED_PHRASE))
    observed = set(actual_words)
    overlap = expected & observed
    score = len(overlap) / len(expected)
    if score < 0.75:
        raise CanaryFailure(
            "Whisper transcript did not reasonably match the fixture speech: "
            f"score={score:.3f} expected={EXPECTED_PHRASE!r} actual={actual!r}"
        )
    return score


def cache_stats(cache: Path) -> tuple[int, int]:
    files = [path for path in cache.rglob("*") if path.is_file()]
    return len(files), sum(path.stat().st_size for path in files)


def classify_failure(exc: BaseException) -> str:
    text = str(exc).lower()
    if any(marker in text for marker in PROVIDER_ERROR_MARKERS):
        return "provider-or-network"
    return "runtime-or-inference"


def write_summary(path: Path | None, payload: dict[str, Any]) -> None:
    if path is None:
        return
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def run(runtime: Path, work_dir: Path, *, model: str, timeout_seconds: int) -> dict[str, Any]:
    started = time.monotonic()
    runtime = runtime.resolve()
    work_dir = work_dir.resolve()
    if work_dir.exists():
        shutil.rmtree(work_dir)
    work_dir.mkdir(parents=True)

    cache = work_dir / "hf-cache"
    cache.mkdir()
    if any(cache.iterdir()):
        raise CanaryFailure("canary model cache was not empty before the first transcription")

    media, duration = generate_fixture(runtime, work_dir)
    print(f"CANARY_FIXTURE_DURATION_SECONDS={duration:.3f}", flush=True)

    first_started = time.monotonic()
    first = transcribe_with_packaged_runtime(
        runtime,
        media,
        work_dir / "output-download",
        cache,
        model=model,
        offline=False,
        timeout_seconds=timeout_seconds,
    )
    first_seconds = time.monotonic() - first_started
    first_text = str(first.get("transcript", {}).get("text") or "").strip()
    first_match = assert_reasonable_match(first_text)

    cache_files, cache_bytes = cache_stats(cache)
    if cache_files == 0 or cache_bytes < 10 * 1024 * 1024:
        raise CanaryFailure(
            "model inference succeeded but the configured Hugging Face cache was not meaningfully populated: "
            f"files={cache_files} bytes={cache_bytes}"
        )

    offline_started = time.monotonic()
    second = transcribe_with_packaged_runtime(
        runtime,
        media,
        work_dir / "output-offline",
        cache,
        model=model,
        offline=True,
        timeout_seconds=timeout_seconds,
    )
    offline_seconds = time.monotonic() - offline_started
    second_text = str(second.get("transcript", {}).get("text") or "").strip()
    second_match = assert_reasonable_match(second_text)

    total_seconds = time.monotonic() - started
    summary = {
        "status": "passed",
        "model": model,
        "fixture_text": EXPECTED_PHRASE,
        "fixture_duration_seconds": round(duration, 3),
        "first_transcript": first_text,
        "offline_transcript": second_text,
        "first_match_score": round(first_match, 3),
        "offline_match_score": round(second_match, 3),
        "cache": str(cache),
        "cache_files": cache_files,
        "cache_bytes": cache_bytes,
        "download_and_first_inference_seconds": round(first_seconds, 3),
        "offline_cached_inference_seconds": round(offline_seconds, 3),
        "total_seconds": round(total_seconds, 3),
        "proof": [
            "packaged-supervisor",
            "packaged-engine",
            "packaged-ffmpeg",
            "packaged-ffprobe",
            "fresh-model-download",
            "real-faster-whisper-inference",
            "ctranslate2-execution",
            "offline-cache-reuse-in-fresh-process",
        ],
    }
    print(f"CANARY_MODEL={model}", flush=True)
    print(f"CANARY_CACHE={cache}", flush=True)
    print(f"CANARY_CACHE_FILES={cache_files}", flush=True)
    print(f"CANARY_CACHE_BYTES={cache_bytes}", flush=True)
    print(f"CANARY_FIRST_SECONDS={first_seconds:.3f}", flush=True)
    print(f"CANARY_OFFLINE_SECONDS={offline_seconds:.3f}", flush=True)
    print(f"CANARY_TOTAL_SECONDS={total_seconds:.3f}", flush=True)
    print(f"CANARY_FIRST_TRANSCRIPT={first_text}", flush=True)
    print(f"CANARY_OFFLINE_TRANSCRIPT={second_text}", flush=True)
    print("CANARY_PROOF=" + ",".join(summary["proof"]), flush=True)
    return summary


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Exercise the real packaged Scriptotar Whisper runtime with download and offline cache reuse."
    )
    parser.add_argument("--runtime", type=Path, required=True)
    parser.add_argument("--work-dir", type=Path, required=True)
    parser.add_argument("--model", default="small")
    parser.add_argument("--timeout-seconds", type=int, default=1200)
    parser.add_argument("--summary-json", type=Path)
    args = parser.parse_args()

    try:
        summary = run(
            args.runtime,
            args.work_dir,
            model=args.model,
            timeout_seconds=args.timeout_seconds,
        )
    except Exception as exc:
        failure_kind = classify_failure(exc)
        payload = {
            "status": "failed",
            "model": args.model,
            "failure_kind": failure_kind,
            "error": str(exc),
        }
        write_summary(args.summary_json, payload)
        print(f"CANARY_FAILURE_KIND={failure_kind}", file=sys.stderr, flush=True)
        print(f"Whisper runtime canary failed: {exc}", file=sys.stderr, flush=True)
        return 1

    write_summary(args.summary_json, summary)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
