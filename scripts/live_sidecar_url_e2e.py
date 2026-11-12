#!/usr/bin/env python3
"""Check an exact URL through the real supervisor/worker JSON protocol.

Source and packaged runs are reported distinctly. No cookies, fixtures, URL
substitution, or model stubs; a source/access/model/protocol failure exits 1.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import queue
import subprocess
import sys
import threading
import time

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "sidecars/transcription"))
from whisper_runtime_canary import _next_event, _pump_stdout


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--url", required=True)
    parser.add_argument("--work-dir", type=Path, required=True)
    parser.add_argument("--runtime", type=Path)
    parser.add_argument("--engine", type=Path, help="Explicit replacement engine; recorded in the report")
    parser.add_argument("--model", default="small", choices=["small", "medium", "turbo", "large-v3"])
    parser.add_argument("--timeout-seconds", type=int, default=1200)
    args = parser.parse_args()
    work = args.work_dir.resolve()
    work.mkdir(parents=True, exist_ok=True)
    (work / "events.jsonl").write_text("", encoding="utf-8")
    env = os.environ.copy()
    # A source run must not accidentally inherit a fixture or packaged worker.
    for key in ("SCRIPTOTAR_SIDECAR_ENGINE_WORKER", "SCRIPTOTAR_SIDECAR_ENGINE_EXECUTABLE"):
        env.pop(key, None)
    env["HF_HOME"] = str(work / "models")
    report = {"url": args.url, "status": "failed", "model": args.model, "quality": "720p", "mode": "source"}
    if args.runtime:
        runtime = args.runtime.resolve()
        suffix = ".exe" if os.name == "nt" else ""
        command = [str(runtime / ("scriptotar-transcription" + suffix)), str(runtime / "sidecar.py")]
        env["SCRIPTOTAR_SIDECAR_ENGINE_EXECUTABLE"] = str(runtime / "engine" / ("scriptotar-engine" + suffix))
        env["PATH"] = str(runtime / "ffmpeg") + os.pathsep + env.get("PATH", "")
        report.update(mode="packaged", runtime=str(runtime))
    else:
        command = [sys.executable, str(ROOT / "sidecars/transcription/sidecar.py")]
        env["PYTHONPATH"] = str(ROOT / "sidecars/transcription")
    if args.engine:
        engine = args.engine.resolve()
        env["SCRIPTOTAR_SIDECAR_ENGINE_EXECUTABLE"] = str(engine)
        report.update(engine_override=str(engine), engine_sha256=hashlib.sha256(engine.read_bytes()).hexdigest())
    report["supervisor"] = command[0]
    started = time.monotonic()
    events = queue.Queue()
    stages = set()
    try:
        with (work / "stderr.log").open("w", encoding="utf-8") as stderr:
            proc = subprocess.Popen(command, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=stderr,
                                    env=env, text=True, bufsize=1)
            thread = threading.Thread(target=_pump_stdout, args=(proc.stdout, events), daemon=True)
            thread.start()
            deadline = started + args.timeout_seconds
            try:
                while True:
                    event = _next_event(events, proc, deadline=deadline, stderr_lines=[])
                    with (work / "events.jsonl").open("a", encoding="utf-8") as log:
                        log.write(json.dumps(event, ensure_ascii=False) + "\n")
                    if event.get("protocol") != 1:
                        raise RuntimeError("Wrong protocol version")
                    kind = event["type"]
                    if kind == "ready":
                        job = {"protocol": 1, "type": "transcribe", "job_id": "exact-url-canary",
                               "input": {"kind": "url", "value": args.url},
                               "output": {"root": str(work / "outputs")},
                               "options": {"model": args.model, "device": "cpu", "language": "auto",
                                           "quality": "720p", "cookies_browser": "none", "keep_failed": True}}
                        proc.stdin.write(json.dumps(job) + "\n")
                        proc.stdin.flush()
                    elif kind == "progress":
                        stage = event.get("stage")
                        if not isinstance(stage, str):
                            raise RuntimeError("Progress event is missing a stage")
                        if stage not in stages:
                            stages.add(stage)
                            print(f"stage={stage}", flush=True)
                    elif kind == "error":
                        raise RuntimeError(json.dumps(event["error"], ensure_ascii=False))
                    elif kind == "result":
                        result = event["result"]
                        artifacts = result["artifacts"]
                        expected = {"media", "text", "clean_text", "timestamp_text", "srt", "vtt", "json"}
                        if set(artifacts) != expected or not all(Path(p).is_file() for p in artifacts.values()):
                            raise RuntimeError("Missing transcript exports or downloaded media")
                        saved = json.loads(Path(artifacts["json"]).read_text(encoding="utf-8"))
                        if saved["device"] != "cpu" or saved["model"] != args.model:
                            raise RuntimeError("Unexpected inference model/device")
                        if "downloading" not in stages or "transcribing" not in stages:
                            raise RuntimeError("Required download/inference stages missing")
                        media = Path(artifacts["media"])
                        subprocess.run(["ffmpeg", "-v", "error", "-nostdin", "-i", str(media),
                                        "-map", "0:a:0", "-f", "null", "-"],
                                       env=env, check=True, capture_output=True, timeout=90)
                        report.update(status="passed", file_bytes=Path(artifacts["media"]).stat().st_size,
                                      segments=len(result["transcript"]["segments"]),
                                      duration_seconds=result["transcript"]["duration_seconds"],
                                      sidecar_version=saved["sidecar_version"], export_count=6, audio_decode="passed")
                        break
            finally:
                proc.stdin.close()
                try:
                    proc.wait(timeout=20)
                except subprocess.TimeoutExpired:
                    proc.kill()
                    proc.wait(timeout=5)
                thread.join(timeout=2)
                proc.stdout.close()
            if proc.returncode != 0:
                raise RuntimeError(f"Supervisor exited {proc.returncode}")
    except Exception as exc:
        report.update(status="failed", error=str(exc))
    report.update(elapsed_seconds=round(time.monotonic() - started, 2), stages=sorted(stages))
    (work / "summary.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(report, indent=2), flush=True)
    return 0 if report["status"] == "passed" else 1


if __name__ == "__main__":
    raise SystemExit(main())
