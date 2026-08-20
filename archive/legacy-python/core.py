#!/usr/bin/env python3
from __future__ import annotations

import re
import textwrap
from pathlib import Path
from urllib.parse import urlparse

ALLOWED_DOMAINS = (
    "instagram.com",
    "tiktok.com",
    "youtube.com",
    "youtu.be",
)
VIDEO_EXTS = {".mp4", ".mkv", ".mov", ".webm", ".m4v", ".avi"}


def safe_name(text: str | None, limit: int = 72) -> str:
    value = (text or "video").strip()
    value = re.sub(r"[^\w\-. ]+", "", value, flags=re.UNICODE)
    value = re.sub(r"\s+", "_", value).strip("._")
    return (value[:limit] or "video")


def human_time(seconds: float | int | None) -> str:
    total = int(max(0, float(seconds or 0)))
    hours, rem = divmod(total, 3600)
    minutes, secs = divmod(rem, 60)
    if hours:
        return f"{hours:02d}:{minutes:02d}:{secs:02d}"
    return f"{minutes:02d}:{secs:02d}"


def srt_time(seconds: float | int) -> str:
    millis = int(round(max(0.0, float(seconds)) * 1000))
    hours, rem = divmod(millis, 3_600_000)
    minutes, rem = divmod(rem, 60_000)
    secs, millis = divmod(rem, 1000)
    return f"{hours:02d}:{minutes:02d}:{secs:02d},{millis:03d}"


def vtt_time(seconds: float | int) -> str:
    return srt_time(seconds).replace(",", ".")


def validate_supported_url(raw_url: str) -> str:
    value = raw_url.strip()
    parsed = urlparse(value)
    if parsed.scheme not in {"http", "https"} or not parsed.hostname:
        raise ValueError("Enter a valid http/https URL.")
    host = parsed.hostname.lower().rstrip(".")
    if not any(host == d or host.endswith("." + d) for d in ALLOWED_DOMAINS):
        raise ValueError("Supported links: Instagram, TikTok, YouTube, and YouTube Shorts.")
    return value


def quality_format(quality: str) -> tuple[str, str | None]:
    if quality == "Audio only":
        return "ba/b", None
    return "bv*+ba/b", "mp4"


def quality_sort(quality: str) -> list[str]:
    # yt-dlp's `res` field uses the smallest dimension, so resolution limits
    # behave correctly for portrait Reels as well as landscape video.
    if quality == "720p":
        return ["res:720", "fps"]
    if quality == "1080p":
        return ["res:1080", "fps"]
    return []


def clean_transcript(lines: list[str]) -> str:
    out: list[str] = []
    previous = None
    for raw in lines:
        line = re.sub(r"\s+", " ", (raw or "").strip())
        if not line:
            continue
        key = line.casefold()
        if key == previous:
            continue
        out.append(line)
        previous = key
    return "\n".join(out).strip()


def wrap_caption(text: str, width: int = 42, max_lines: int = 2) -> str:
    text = re.sub(r"\s+", " ", text.strip())
    if not text:
        return ""
    lines = textwrap.wrap(
        text,
        width=width,
        break_long_words=False,
        break_on_hyphens=False,
    )
    if len(lines) <= max_lines:
        return "\n".join(lines)
    head = lines[: max_lines - 1]
    tail = " ".join(lines[max_lines - 1 :])
    if len(tail) > width * 2:
        tail = tail[: width * 2 - 1].rstrip() + "…"
    return "\n".join(head + [tail])


def captions_from_words(
    words: list[dict],
    max_chars: int = 76,
    max_duration: float = 6.0,
    max_gap: float = 1.2,
) -> list[dict]:
    """Group word timestamps into readable subtitle captions."""
    captions: list[dict] = []
    current: list[dict] = []

    def flush() -> None:
        nonlocal current
        if not current:
            return
        text = "".join(str(w.get("word", "")) for w in current).strip()
        if not text:
            current = []
            return
        captions.append(
            {
                "start": float(current[0].get("start") or 0),
                "end": float(current[-1].get("end") or current[-1].get("start") or 0),
                "text": wrap_caption(text),
            }
        )
        current = []

    for word in words:
        if word.get("start") is None or word.get("end") is None:
            continue
        if current:
            tentative = "".join(str(w.get("word", "")) for w in current + [word]).strip()
            duration = float(word["end"]) - float(current[0]["start"])
            gap = float(word["start"]) - float(current[-1]["end"])
            terminal = str(current[-1].get("word", "")).strip().endswith((".", "!", "?", "؟", "؛"))
            if len(tentative) > max_chars or duration > max_duration or gap > max_gap or (terminal and len(tentative) > 28):
                flush()
        current.append(word)
    flush()
    return captions


def write_srt(captions: list[dict]) -> str:
    blocks = []
    for idx, cap in enumerate(captions, 1):
        blocks.append(
            f"{idx}\n{srt_time(cap['start'])} --> {srt_time(cap['end'])}\n{cap['text']}\n"
        )
    return "\n".join(blocks)


def write_vtt(captions: list[dict]) -> str:
    blocks = ["WEBVTT", ""]
    for cap in captions:
        blocks.append(f"{vtt_time(cap['start'])} --> {vtt_time(cap['end'])}")
        blocks.append(cap["text"])
        blocks.append("")
    return "\n".join(blocks)


def local_video_ok(path: str) -> bool:
    p = Path(path).expanduser()
    return p.is_file() and p.suffix.lower() in VIDEO_EXTS
