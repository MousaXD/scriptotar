from __future__ import annotations

import json
import os
import shutil
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from .validation import job_fingerprint

CHECKPOINT_NAME = "checkpoint.json"
TRANSCRIPT_FILES = {
    "transcript.txt",
    "transcript_clean.txt",
    "transcript_timestamps.txt",
    "transcript.srt",
    "transcript.vtt",
    "transcript.json",
}


@dataclass(slots=True)
class RecoveryDecision:
    partial_dir: Path
    reused_media: Path | None
    metadata: dict[str, Any] | None
    restarted_stage: str


def partial_directory(output_root: Path, job_id: str) -> Path:
    return output_root / f".scriptotar-{job_id}.partial"


def write_private_json(path: Path, payload: dict[str, Any]) -> None:
    path.write_text(json.dumps(payload, ensure_ascii=False, indent=2), encoding="utf-8")
    try:
        os.chmod(path, 0o600)
    except OSError:
        pass


def _load_checkpoint(partial: Path) -> dict[str, Any] | None:
    try:
        payload = json.loads((partial / CHECKPOINT_NAME).read_text(encoding="utf-8"))
        return payload if isinstance(payload, dict) else None
    except (OSError, json.JSONDecodeError):
        return None


def _clear_transcript_artifacts(partial: Path) -> None:
    for name in TRANSCRIPT_FILES:
        try:
            (partial / name).unlink()
        except FileNotFoundError:
            pass


def prepare_partial(job: dict[str, Any]) -> RecoveryDecision:
    output_root = Path(job["output_root"])
    output_existed = output_root.exists()
    output_root.mkdir(parents=True, exist_ok=True, mode=0o700)
    if not output_existed:
        try:
            os.chmod(output_root, 0o700)
        except OSError:
            pass
    partial = partial_directory(output_root, job["id"])
    fingerprint = job_fingerprint(job)

    if partial.exists():
        checkpoint = _load_checkpoint(partial)
        if checkpoint and checkpoint.get("fingerprint") == fingerprint and checkpoint.get("stage") == "media_ready":
            media_value = checkpoint.get("media")
            metadata = checkpoint.get("metadata")
            if isinstance(media_value, str) and isinstance(metadata, dict):
                media = Path(media_value)
                if not media.is_absolute():
                    media = partial / media
                try:
                    valid_media = media.is_file() and media.stat().st_size > 0
                except OSError:
                    valid_media = False
                if valid_media:
                    _clear_transcript_artifacts(partial)
                    return RecoveryDecision(partial, media, metadata, "transcribing")
        shutil.rmtree(partial, ignore_errors=True)

    partial.mkdir(parents=True, mode=0o700)
    try:
        os.chmod(partial, 0o700)
    except OSError:
        pass
    write_private_json(
        partial / CHECKPOINT_NAME,
        {"stage": "preparing", "fingerprint": fingerprint},
    )
    return RecoveryDecision(partial, None, None, "preparing")


def mark_media_ready(
    job: dict[str, Any], partial: Path, media: Path, metadata: dict[str, Any]
) -> None:
    try:
        relative = media.relative_to(partial)
        media_value = str(relative)
    except ValueError:
        media_value = str(media)
    write_private_json(
        partial / CHECKPOINT_NAME,
        {
            "stage": "media_ready",
            "fingerprint": job_fingerprint(job),
            "media": media_value,
            "metadata": metadata,
        },
    )
