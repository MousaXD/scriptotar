from __future__ import annotations

import importlib.metadata
import shutil
from typing import Any

from .validation import ALLOWED_DOMAINS, ALLOWED_MODELS
from .version import PROTOCOL_VERSION, SIDECAR_VERSION


def _package_version(name: str) -> str | None:
    try:
        return importlib.metadata.version(name)
    except importlib.metadata.PackageNotFoundError:
        return None


def capability_report() -> dict[str, Any]:
    return {
        "sidecar_version": SIDECAR_VERSION,
        "protocol_versions": [PROTOCOL_VERSION],
        "features": {
            "url_input": True,
            "local_input": True,
            "cancellation": True,
            "persistent_engine": True,
            "word_timestamps": True,
            "formats": ["txt", "clean_txt", "timestamp_txt", "srt", "vtt", "json"],
        },
        "binaries": {
            "ffmpeg": shutil.which("ffmpeg") is not None,
            "ffprobe": shutil.which("ffprobe") is not None,
        },
        "python_packages": {
            "yt_dlp": _package_version("yt-dlp"),
            "faster_whisper": _package_version("faster-whisper"),
        },
        "supported_models": sorted(ALLOWED_MODELS),
        "supported_domains": list(ALLOWED_DOMAINS),
    }
