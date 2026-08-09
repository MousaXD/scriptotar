from __future__ import annotations

import hashlib
import json
import os
import re
from pathlib import Path
from typing import Any
from urllib.parse import urlparse

from .errors import SidecarError

ALLOWED_DOMAINS = (
    "instagram.com",
    "tiktok.com",
    "youtube.com",
    "youtu.be",
)
VIDEO_EXTENSIONS = {".mp4", ".mkv", ".mov", ".webm", ".m4v", ".avi"}
ALLOWED_MODELS = {"small", "medium", "turbo", "large-v3"}
ALLOWED_DEVICES = {"auto", "cpu", "cuda"}
ALLOWED_QUALITIES = {"720p", "1080p", "best", "audio-only"}
ALLOWED_COOKIE_BROWSERS = {"none", "firefox", "chrome", "chromium", "brave", "edge"}
ALLOWED_LANGUAGES = {"auto", "ar", "en"}
JOB_ID_RE = re.compile(r"^[A-Za-z0-9][A-Za-z0-9._-]{0,127}$")


def _fail(code: str, message: str, **details: Any) -> SidecarError:
    return SidecarError(code, message, details=details or None)


def validate_job_id(value: Any) -> str:
    if not isinstance(value, str) or not JOB_ID_RE.fullmatch(value):
        raise _fail("INVALID_JOB_ID", "job_id must be 1-128 safe ASCII characters.")
    return value


def validate_supported_url(raw_url: Any, *, cookies_browser: str = "none") -> str:
    if not isinstance(raw_url, str):
        raise _fail("INVALID_URL", "URL input must be a string.")
    value = raw_url.strip()
    parsed = urlparse(value)
    if parsed.scheme not in {"http", "https"} or not parsed.hostname:
        raise _fail("INVALID_URL", "Only absolute http/https URLs are accepted.")
    if parsed.username or parsed.password:
        raise _fail("INVALID_URL", "Credentials embedded in URLs are not allowed.")
    host = parsed.hostname.lower().rstrip(".")
    if not any(host == domain or host.endswith("." + domain) for domain in ALLOWED_DOMAINS):
        raise _fail(
            "UNSUPPORTED_DOMAIN",
            "Supported URL domains are Instagram, TikTok, YouTube, and youtu.be.",
            host=host,
        )
    try:
        port = parsed.port
    except ValueError as exc:
        raise _fail("INVALID_URL", "URL port is malformed.") from exc
    if port not in {None, 80, 443}:
        raise _fail("INVALID_URL", "Non-standard URL ports are not allowed.")
    if cookies_browser != "none" and parsed.scheme != "https":
        raise _fail("INSECURE_COOKIE_URL", "Browser cookies may only be used with HTTPS URLs.")
    return value


def validate_local_input(raw_path: Any) -> Path:
    if not isinstance(raw_path, str) or not raw_path.strip():
        raise _fail("INVALID_INPUT", "Local input path must be a non-empty string.")
    path = Path(raw_path).expanduser().resolve()
    if not path.is_file():
        raise _fail("INPUT_NOT_FOUND", "Local input file does not exist.", path=str(path))
    if path.suffix.lower() not in VIDEO_EXTENSIONS:
        raise _fail(
            "UNSUPPORTED_FILE_TYPE",
            "Unsupported local media extension.",
            extension=path.suffix.lower(),
        )
    return path


def validate_output_root(raw_path: Any) -> Path:
    if not isinstance(raw_path, str) or not raw_path.strip():
        raise _fail("INVALID_OUTPUT", "output.root must be a non-empty path string.")
    path = Path(raw_path).expanduser()
    try:
        resolved = path.resolve(strict=False)
    except OSError as exc:
        raise _fail("INVALID_OUTPUT", f"Unable to resolve output path: {exc}") from exc
    if resolved.exists() and not resolved.is_dir():
        raise _fail("INVALID_OUTPUT", "Output root must be a directory.", path=str(resolved))
    parent = resolved
    while not parent.exists() and parent != parent.parent:
        parent = parent.parent
    if parent.exists() and not os.access(parent, os.W_OK | os.X_OK):
        raise _fail("OUTPUT_NOT_WRITABLE", "Output root is not writable.", path=str(resolved))
    return resolved


def _expect_bool(value: Any, name: str, default: bool) -> bool:
    if value is None:
        return default
    if not isinstance(value, bool):
        raise _fail("INVALID_OPTION", f"{name} must be boolean.")
    return value


def _expect_int(value: Any, name: str, default: int, minimum: int, maximum: int) -> int:
    if value is None:
        return default
    if isinstance(value, bool) or not isinstance(value, int) or not minimum <= value <= maximum:
        raise _fail("INVALID_OPTION", f"{name} must be an integer from {minimum} to {maximum}.")
    return value


def validate_job_payload(command: dict[str, Any]) -> dict[str, Any]:
    job_id = validate_job_id(command.get("job_id"))

    input_data = command.get("input")
    if not isinstance(input_data, dict):
        raise _fail("INVALID_INPUT", "input must be an object.")
    kind = input_data.get("kind")
    if kind not in {"url", "file"}:
        raise _fail("INVALID_INPUT", "input.kind must be 'url' or 'file'.")

    options = command.get("options") or {}
    if not isinstance(options, dict):
        raise _fail("INVALID_OPTION", "options must be an object.")
    allowed_option_keys = {
        "model",
        "device",
        "language",
        "quality",
        "cookies_browser",
        "max_duration_seconds",
        "copy_source",
        "translate",
        "batched",
        "batch_size",
        "keep_failed",
    }
    unknown = sorted(set(options) - allowed_option_keys)
    if unknown:
        raise _fail("INVALID_OPTION", "Unknown transcription option.", options=unknown)

    model = options.get("model", "medium")
    if model not in ALLOWED_MODELS:
        raise _fail("INVALID_MODEL", "Unsupported Whisper model.", model=model)
    device = options.get("device", "auto")
    if device not in ALLOWED_DEVICES:
        raise _fail("INVALID_OPTION", "device must be auto, cpu, or cuda.")
    language = options.get("language", "auto")
    if language not in ALLOWED_LANGUAGES:
        raise _fail("INVALID_OPTION", "language must be auto, ar, or en.")
    quality = options.get("quality", "720p")
    if quality not in ALLOWED_QUALITIES:
        raise _fail("INVALID_OPTION", "Unsupported media quality.", quality=quality)
    cookies_browser = options.get("cookies_browser", "none")
    if cookies_browser not in ALLOWED_COOKIE_BROWSERS:
        raise _fail("INVALID_OPTION", "Unsupported browser cookie source.")

    output = command.get("output")
    if not isinstance(output, dict):
        raise _fail("INVALID_OUTPUT", "output must be an object.")
    allowed_output_keys = {"root"}
    unknown_output = sorted(set(output) - allowed_output_keys)
    if unknown_output:
        raise _fail("INVALID_OUTPUT", "Unknown output option.", options=unknown_output)
    output_root = validate_output_root(output.get("root"))

    if kind == "url":
        source = validate_supported_url(input_data.get("value"), cookies_browser=cookies_browser)
    else:
        source = str(validate_local_input(input_data.get("value")))

    normalized = {
        "id": job_id,
        "input_type": kind,
        "source": source,
        "output_root": str(output_root),
        "model": model,
        "device": device,
        "language": language,
        "quality": quality,
        "cookies": cookies_browser,
        "max_duration_seconds": _expect_int(
            options.get("max_duration_seconds"), "max_duration_seconds", 3600, 0, 86_400
        ),
        "copy_source": _expect_bool(options.get("copy_source"), "copy_source", True),
        "translate": _expect_bool(options.get("translate"), "translate", False),
        "batched": _expect_bool(options.get("batched"), "batched", False),
        "batch_size": _expect_int(options.get("batch_size"), "batch_size", 8, 1, 64),
        "keep_failed": _expect_bool(options.get("keep_failed"), "keep_failed", False),
    }
    return normalized


def job_fingerprint(job: dict[str, Any]) -> str:
    material: dict[str, Any] = {
        "input_type": job["input_type"],
        "source": job["source"],
        "quality": job["quality"],
        "cookies": job["cookies"],
        "copy_source": job["copy_source"],
    }
    if job["input_type"] == "file":
        path = Path(job["source"])
        stat = path.stat()
        material["source_size"] = stat.st_size
        material["source_mtime_ns"] = stat.st_mtime_ns
    encoded = json.dumps(material, sort_keys=True, separators=(",", ":")).encode("utf-8")
    return hashlib.sha256(encoded).hexdigest()
