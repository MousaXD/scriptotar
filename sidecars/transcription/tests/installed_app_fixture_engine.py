from __future__ import annotations

import json
import sys
from pathlib import Path

TRANSCRIPT_TEXT = "installed fixture transcript survives restart"


def emit(event_type: str, **payload: object) -> None:
    print(json.dumps({"type": event_type, **payload}, separators=(",", ":")), flush=True)


def write_artifacts(job: dict[str, object]) -> tuple[Path, dict[str, str]]:
    output_root = Path(str(job["output_root"]))
    job_id = str(job["id"])
    output_dir = output_root / f"installed-e2e-{job_id}"
    output_dir.mkdir(parents=True, exist_ok=True)

    artifacts = {
        "text": output_dir / "transcript.txt",
        "clean_text": output_dir / "transcript_clean.txt",
        "timestamp_text": output_dir / "transcript_timestamps.txt",
        "srt": output_dir / "transcript.srt",
        "vtt": output_dir / "transcript.vtt",
        "json": output_dir / "transcript.json",
    }
    artifacts["text"].write_text(TRANSCRIPT_TEXT + "\n", encoding="utf-8")
    artifacts["clean_text"].write_text(TRANSCRIPT_TEXT + "\n", encoding="utf-8")
    artifacts["timestamp_text"].write_text(
        f"[00:00.000 --> 00:01.750] {TRANSCRIPT_TEXT}\n", encoding="utf-8"
    )
    artifacts["srt"].write_text(
        f"1\n00:00:00,000 --> 00:00:01,750\n{TRANSCRIPT_TEXT}\n", encoding="utf-8"
    )
    artifacts["vtt"].write_text(
        f"WEBVTT\n\n00:00.000 --> 00:01.750\n{TRANSCRIPT_TEXT}\n", encoding="utf-8"
    )
    artifacts["json"].write_text(
        json.dumps(
            {
                "text": TRANSCRIPT_TEXT,
                "language": "en",
                "duration_seconds": 1.75,
            },
            separators=(",", ":"),
        )
        + "\n",
        encoding="utf-8",
    )
    return output_dir, {name: str(path) for name, path in artifacts.items()}


def result_for(job: dict[str, object]) -> dict[str, object]:
    source = str(job["source"])
    output_dir, artifacts = write_artifacts(job)
    artifacts["media"] = source
    return {
        "source": {
            "title": "Installed E2E Fixture",
            "uploader": "Scriptotar CI",
            "source_url": source,
            "duration_seconds": 1.75,
            "extractor": "installed-e2e-fixture",
        },
        "transcript": {
            "text": TRANSCRIPT_TEXT,
            "clean_text": TRANSCRIPT_TEXT,
            "segments": [
                {
                    "index": 1,
                    "start": 0.0,
                    "end": 1.75,
                    "text": TRANSCRIPT_TEXT,
                    "words": [
                        {"word": "installed", "start": 0.0, "end": 0.35, "probability": 1.0},
                        {"word": "fixture", "start": 0.35, "end": 0.7, "probability": 1.0},
                        {"word": "transcript", "start": 0.7, "end": 1.05, "probability": 1.0},
                        {"word": "survives", "start": 1.05, "end": 1.4, "probability": 1.0},
                        {"word": "restart", "start": 1.4, "end": 1.75, "probability": 1.0},
                    ],
                }
            ],
            "words": [
                {"word": "installed", "start": 0.0, "end": 0.35, "probability": 1.0},
                {"word": "fixture", "start": 0.35, "end": 0.7, "probability": 1.0},
                {"word": "transcript", "start": 0.7, "end": 1.05, "probability": 1.0},
                {"word": "survives", "start": 1.05, "end": 1.4, "probability": 1.0},
                {"word": "restart", "start": 1.4, "end": 1.75, "probability": 1.0},
            ],
            "language": "en",
            "language_probability": 1.0,
            "duration_seconds": 1.75,
        },
        "artifacts": artifacts,
        "output_dir": str(output_dir),
    }


def main() -> int:
    emit("engine_ready")
    for line in sys.stdin:
        command = json.loads(line)
        command_type = command.get("type")
        if command_type == "shutdown":
            emit("engine_shutdown")
            return 0
        if command_type != "transcribe":
            continue

        job = command["job"]
        job_id = str(job["id"])
        emit("job_started", job_id=job_id)
        emit(
            "progress",
            job_id=job_id,
            stage="transcribing",
            percent=40.0,
            message="deterministic fixture transcription",
        )
        emit(
            "progress",
            job_id=job_id,
            stage="processing",
            percent=90.0,
            message="writing deterministic fixture artifacts",
        )
        emit("result", job_id=job_id, result=result_for(job))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
