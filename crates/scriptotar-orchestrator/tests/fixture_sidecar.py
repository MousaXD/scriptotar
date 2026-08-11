from __future__ import annotations

import json
import os
import sys
import time
from pathlib import Path

PROTOCOL = 1


def emit(event_type: str, *, protocol: int = PROTOCOL, **payload: object) -> None:
    print(json.dumps({"protocol": protocol, "type": event_type, **payload}), flush=True)


def result_for(source: str, input_kind: str) -> dict[str, object]:
    return {
        "source": {
            "title": Path(source).stem,
            "uploader": None,
            "source_url": source,
            "duration_seconds": 1.0,
            "extractor": "fixture",
        },
        "transcript": {
            "text": "fixture transcript",
            "clean_text": "fixture transcript",
            "segments": [{"index": 1, "start": 0.0, "end": 1.0, "text": "fixture transcript", "words": []}],
            "words": [],
            "language": "en",
            "language_probability": 1.0,
            "duration_seconds": 1.0,
        },
        "artifacts": {
            "text": None,
            "clean_text": None,
            "timestamp_text": None,
            "srt": None,
            "vtt": None,
            "json": None,
            "media": "/tmp/downloaded-fixture.mp4" if input_kind == "url" else source,
        },
        "output_dir": "/tmp/scriptotar-fixture",
    }


def main() -> int:
    emit(
        "ready",
        capabilities={
            "sidecar_version": "fixture",
            "protocol_versions": [PROTOCOL],
        },
    )
    for raw in sys.stdin:
        command = json.loads(raw)
        command_type = command.get("type")
        if command_type == "shutdown":
            emit("shutdown", request_id=command.get("request_id"))
            return 0
        if command_type != "transcribe":
            continue

        job_id = command["job_id"]
        input_kind = command["input"]["kind"]
        source = command["input"]["value"]
        name = Path(source).name
        emit("accepted", job_id=job_id, request_id=command.get("request_id"))
        emit("job_started", job_id=job_id)

        if "malformed" in name:
            print('{"protocol":1,"type":"result"', flush=True)
            continue
        if "wrong-protocol" in name:
            emit("result", protocol=99, job_id=job_id, result=result_for(source, input_kind))
            continue
        if "wrong-job-id" in name:
            emit(
                "result",
                job_id="definitely-not-the-active-job",
                result=result_for(source, input_kind),
            )
            continue
        if "sidecar-exit" in name:
            os._exit(23)

        if input_kind == "url":
            emit("progress", job_id=job_id, stage="downloading", percent=40.0, message="fixture download")
            # Keep the transient state observable even on heavily loaded CI
            # runners. This is test-fixture latency only; production code is
            # unaffected.
            time.sleep(0.5)
        emit("progress", job_id=job_id, stage="transcribing", percent=50.0, message="fixture")
        if "progress-hold" in name:
            time.sleep(0.15)
        emit("result", job_id=job_id, result=result_for(source, input_kind))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())