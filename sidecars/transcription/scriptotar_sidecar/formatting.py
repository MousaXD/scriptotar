from __future__ import annotations

import re
import textwrap
from typing import Any


def safe_name(text: str | None, limit: int = 72) -> str:
    value = (text or "video").strip()
    value = re.sub(r"[^\w\-. ]+", "", value, flags=re.UNICODE)
    value = re.sub(r"\s+", "_", value).strip("._")
    return value[:limit] or "video"


def human_time(seconds: float | int | None) -> str:
    total = int(max(0, float(seconds or 0)))
    hours, rem = divmod(total, 3600)
    minutes, secs = divmod(rem, 60)
    if hours:
        return f"{hours:02d}:{minutes:02d}:{secs:02d}"
    return f"{minutes:02d}:{secs:02d}"


def subtitle_time(seconds: float | int, decimal: str) -> str:
    millis = int(round(max(0.0, float(seconds)) * 1000))
    hours, rem = divmod(millis, 3_600_000)
    minutes, rem = divmod(rem, 60_000)
    secs, millis = divmod(rem, 1000)
    return f"{hours:02d}:{minutes:02d}:{secs:02d}{decimal}{millis:03d}"


def clean_transcript(lines: list[str]) -> str:
    result: list[str] = []
    previous: str | None = None
    for raw in lines:
        line = re.sub(r"\s+", " ", (raw or "").strip())
        if not line:
            continue
        key = line.casefold()
        if key == previous:
            continue
        result.append(line)
        previous = key
    return "\n".join(result).strip()


def wrap_caption(text: str, width: int = 42, max_lines: int = 2) -> str:
    value = re.sub(r"\s+", " ", text.strip())
    if not value:
        return ""
    lines = textwrap.wrap(value, width=width, break_long_words=False, break_on_hyphens=False)
    if len(lines) <= max_lines:
        return "\n".join(lines)
    head = lines[: max_lines - 1]
    tail = " ".join(lines[max_lines - 1 :])
    if len(tail) > width * 2:
        tail = tail[: width * 2 - 1].rstrip() + "…"
    return "\n".join(head + [tail])


def captions_from_words(words: list[dict[str, Any]]) -> list[dict[str, Any]]:
    captions: list[dict[str, Any]] = []
    current: list[dict[str, Any]] = []

    def flush() -> None:
        nonlocal current
        if not current:
            return
        text = "".join(str(word.get("word", "")) for word in current).strip()
        if text:
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
            tentative = "".join(str(item.get("word", "")) for item in current + [word]).strip()
            duration = float(word["end"]) - float(current[0]["start"])
            gap = float(word["start"]) - float(current[-1]["end"])
            terminal = str(current[-1].get("word", "")).strip().endswith((".", "!", "?", "؟", "؛"))
            if len(tentative) > 76 or duration > 6.0 or gap > 1.2 or (terminal and len(tentative) > 28):
                flush()
        current.append(word)
    flush()
    return captions


def render_srt(captions: list[dict[str, Any]]) -> str:
    blocks = []
    for index, caption in enumerate(captions, 1):
        blocks.append(
            f"{index}\n{subtitle_time(caption['start'], ',')} --> {subtitle_time(caption['end'], ',')}\n{caption['text']}\n"
        )
    return "\n".join(blocks)


def render_vtt(captions: list[dict[str, Any]]) -> str:
    blocks = ["WEBVTT", ""]
    for caption in captions:
        blocks.extend(
            [
                f"{subtitle_time(caption['start'], '.')} --> {subtitle_time(caption['end'], '.')}",
                caption["text"],
                "",
            ]
        )
    return "\n".join(blocks)
