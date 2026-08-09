from __future__ import annotations

import json
import os
import subprocess
import sys
import time
from pathlib import Path


def emit(event_type: str, **payload):
    print(json.dumps({"type": event_type, **payload}), flush=True)


def result_for(job: dict) -> dict:
    source = job["source"]
    return {
        "source": {
            "title": Path(source).stem,
            "uploader": None,
            "source_url": source,
            "duration_seconds": 1.25,
            "extractor": "fake",
        },
        "transcript": {
            "text": "hello world",
            "clean_text": "hello world",
            "segments": [{"index": 1, "start": 0.0, "end": 1.25, "text": "hello world", "words": []}],
            "words": [],
            "language": "en",
            "language_probability": 0.99,
            "duration_seconds": 1.25,
        },
        "artifacts": {
            "text": "/tmp/transcript.txt",
            "clean_text": "/tmp/transcript_clean.txt",
            "timestamp_text": "/tmp/transcript_timestamps.txt",
            "srt": "/tmp/transcript.srt",
            "vtt": "/tmp/transcript.vtt",
            "json": "/tmp/transcript.json",
            "media": source,
        },
        "output_dir": "/tmp/fake-output",
    }


def main() -> int:
    emit("engine_ready")
    for line in sys.stdin:
        command = json.loads(line)
        if command.get("type") == "shutdown":
            emit("engine_shutdown")
            return 0
        if command.get("type") != "transcribe":
            continue
        job = command["job"]
        job_id = job["id"]
        emit("job_started", job_id=job_id)
        name = Path(job["source"]).name
        if "stderr" in name:
            print("library diagnostic that must never reach public stdout", file=sys.stderr, flush=True)
        if "crash" in name:
            os._exit(23)
        if "fail" in name:
            emit(
                "error",
                job_id=job_id,
                error={"code": "FAKE_FAILURE", "message": "fake failure", "retryable": False},
            )
            continue
        if "spawn-child" in name:
            child = subprocess.Popen([sys.executable, "-c", "import time; time.sleep(60)"])
            pid_file = os.environ.get("SCRIPTOTAR_TEST_CHILD_PID_FILE")
            if pid_file:
                Path(pid_file).write_text(str(child.pid), encoding="utf-8")
            emit("progress", job_id=job_id, stage="transcribing", percent=10.0, message="blocked")
            child.wait()
            continue
        emit("progress", job_id=job_id, stage="transcribing", percent=50.0, message="halfway")
        time.sleep(0.03)
        emit("result", job_id=job_id, result=result_for(job))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
