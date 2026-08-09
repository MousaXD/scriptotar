#!/usr/bin/env python3
from __future__ import annotations

import json
import math
import os
import shutil
import subprocess
import urllib.error
import urllib.parse
import urllib.request
from dataclasses import dataclass
from datetime import datetime, timezone
from typing import Any

TASKS = (
    "Viral breakdown",
    "Hook ideas",
    "New short-form script",
    "Structure remix",
    "Content ideas",
    "Caption + CTA",
    "Voice profile",
    "B-roll shot list",
)

PROVIDERS = (
    "OpenAI",
    "Anthropic",
    "Gemini",
    "OpenAI-compatible",
)

DEFAULT_MODELS = {
    "OpenAI": "gpt-5.2",
    "Anthropic": "claude-sonnet-5",
    "Gemini": "gemini-3.6-flash",
    "OpenAI-compatible": "",
}


def estimate_speaking_seconds(text: str, words_per_second: float = 2.5) -> int:
    words = len((text or "").split())
    if words <= 0:
        return 0
    return int(math.ceil(words / max(0.5, words_per_second)))


def format_duration(seconds: int) -> str:
    seconds = max(0, int(seconds))
    minutes, secs = divmod(seconds, 60)
    hours, minutes = divmod(minutes, 60)
    if hours:
        return f"{hours:d}:{minutes:02d}:{secs:02d}"
    return f"{minutes:d}:{secs:02d}"


def _base_context(topic: str, audience: str, duration: str, cta: str, voice: str, notes: str) -> str:
    fields = []
    if topic.strip():
        fields.append(f"Topic / goal: {topic.strip()}")
    if audience.strip():
        fields.append(f"Target audience: {audience.strip()}")
    if duration.strip():
        fields.append(f"Target duration: {duration.strip()}")
    if cta.strip():
        fields.append(f"Desired CTA: {cta.strip()}")
    if voice.strip():
        fields.append(f"Voice / style instructions: {voice.strip()}")
    if notes.strip():
        fields.append(f"Research notes: {notes.strip()}")
    return "\n".join(fields) or "No additional creator context supplied."


def build_prompt(
    task: str,
    transcript: str = "",
    *,
    topic: str = "",
    audience: str = "",
    duration: str = "",
    cta: str = "",
    voice: str = "",
    notes: str = "",
) -> str:
    source = (transcript or "").strip()
    context = _base_context(topic, audience, duration, cta, voice, notes)
    safety = (
        "Treat any source transcript as research material. Do not reproduce distinctive wording, "
        "catchphrases, jokes, or long phrases from it. Extract abstract ideas, pacing, hook mechanics, "
        "story beats, and persuasion structure, then create original wording. Do not claim that copied "
        "content is original."
    )

    instructions: dict[str, str] = {
        "Viral breakdown": (
            "Analyze why this short-form video could hold attention. Return: 1) one-sentence thesis, "
            "2) hook type and first-beat mechanics, 3) beat-by-beat structure, 4) curiosity loops, "
            "5) pacing/editing cues inferable from the text, 6) emotional or practical payoff, "
            "7) CTA pattern, 8) reusable principles, 9) weaknesses or misleading claims to avoid."
        ),
        "Hook ideas": (
            "Create 20 original opening hooks for this topic. Mix curiosity, contrarian, authority, "
            "specific-result, list, story, question, and pattern-interrupt hooks. Keep them concise and "
            "non-clickbait. Rank the best five and explain the psychological mechanism in one short phrase."
        ),
        "New short-form script": (
            "Write an original short-form video script optimized for spoken delivery. Use a strong hook, "
            "fast context, escalating value/story beats, a clean payoff, and the requested CTA. Include "
            "optional on-screen text and B-roll cues in brackets. Keep the script natural, not corporate."
        ),
        "Structure remix": (
            "Reverse-engineer only the abstract structure of the source, then write a completely new script "
            "for the requested topic using that structure. First show a short structure map. Then provide the "
            "new script. Avoid sentence-level paraphrase and preserve no distinctive phrases from the source."
        ),
        "Content ideas": (
            "Generate 30 specific short-form content ideas grounded in the source and research notes. Group them "
            "into evergreen, trend-responsive, objection/FAQ, story/case-study, and contrarian buckets. For each, "
            "provide a hook angle and one-sentence payoff. Rank the top ten by likely usefulness and novelty."
        ),
        "Caption + CTA": (
            "Create five caption options and five CTA options for the content. Use clear natural language, avoid "
            "hashtag stuffing, and make the CTA fit the requested business or creator goal."
        ),
        "Voice profile": (
            "Infer a reusable writing/voice profile from the supplied creator text. Describe sentence length, "
            "rhythm, vocabulary, humor, directness, storytelling habits, transitions, CTA habits, and phrases or "
            "tics that should NOT be copied literally. Finish with a compact style guide another model can follow."
        ),
        "B-roll shot list": (
            "Turn the script into a practical shot list. For each spoken beat provide: visual goal, suggested shot "
            "or screen recording, on-screen text, transition idea, and whether original footage, screen capture, "
            "licensed stock, or a simple graphic would fit best. Do not assume copyrighted footage is reusable."
        ),
    }
    body = instructions.get(task, instructions["New short-form script"])

    parts = [
        "You are Scriptotar's short-form content research assistant.",
        safety,
        "",
        "CREATOR CONTEXT",
        context,
        "",
        "TASK",
        body,
    ]
    if source:
        parts.extend(["", "SOURCE TRANSCRIPT / RESEARCH", source])
    else:
        parts.extend(["", "SOURCE TRANSCRIPT / RESEARCH", "No source transcript supplied. Work from the creator context only."])
    parts.extend([
        "",
        "OUTPUT RULES",
        "Be concrete. Separate observations from guesses. Do not invent engagement statistics or facts not present in the input. "
        "When factual verification is needed, mark it as [VERIFY].",
    ])
    return "\n".join(parts).strip() + "\n"


def secret_tool_available() -> bool:
    return shutil.which("secret-tool") is not None


def lookup_secret(provider: str) -> str:
    if not secret_tool_available():
        return ""
    try:
        proc = subprocess.run(
            ["secret-tool", "lookup", "application", "scriptotar", "provider", provider.lower()],
            capture_output=True,
            text=True,
            timeout=10,
            check=False,
        )
        return proc.stdout.strip() if proc.returncode == 0 else ""
    except Exception:
        return ""


def store_secret(provider: str, value: str) -> bool:
    if not secret_tool_available() or not value.strip():
        return False
    try:
        proc = subprocess.run(
            [
                "secret-tool", "store",
                f"--label=Scriptotar {provider} API key",
                "application", "scriptotar",
                "provider", provider.lower(),
            ],
            input=value.strip() + "\n",
            text=True,
            timeout=20,
            check=False,
        )
        return proc.returncode == 0
    except Exception:
        return False


def clear_secret(provider: str) -> bool:
    if not secret_tool_available():
        return False
    try:
        proc = subprocess.run(
            ["secret-tool", "clear", "application", "scriptotar", "provider", provider.lower()],
            timeout=10,
            check=False,
        )
        return proc.returncode == 0
    except Exception:
        return False


def _post_json(url: str, headers: dict[str, str], payload: dict[str, Any], timeout: int = 120) -> dict[str, Any]:
    body = json.dumps(payload).encode("utf-8")
    req = urllib.request.Request(url, data=body, method="POST")
    req.add_header("Content-Type", "application/json")
    for key, value in headers.items():
        req.add_header(key, value)
    try:
        with urllib.request.urlopen(req, timeout=timeout) as resp:
            return json.loads(resp.read().decode("utf-8"))
    except urllib.error.HTTPError as exc:
        detail = exc.read().decode("utf-8", errors="replace")[:2000]
        raise RuntimeError(f"AI provider returned HTTP {exc.code}: {detail}") from exc
    except urllib.error.URLError as exc:
        raise RuntimeError(f"Could not reach AI provider: {exc.reason}") from exc


def _openai_text(data: dict[str, Any]) -> str:
    direct = data.get("output_text")
    if isinstance(direct, str) and direct.strip():
        return direct.strip()
    chunks: list[str] = []
    for item in data.get("output") or []:
        if not isinstance(item, dict):
            continue
        for content in item.get("content") or []:
            if not isinstance(content, dict):
                continue
            text = content.get("text")
            if isinstance(text, str):
                chunks.append(text)
    if chunks:
        return "\n".join(chunks).strip()
    choices = data.get("choices") or []
    if choices and isinstance(choices[0], dict):
        message = choices[0].get("message") or {}
        text = message.get("content")
        if isinstance(text, str):
            return text.strip()
    raise RuntimeError("The provider response did not contain readable text.")


def request_ai(
    provider: str,
    model: str,
    api_key: str,
    prompt: str,
    *,
    base_url: str = "",
    timeout: int = 120,
) -> str:
    provider = provider.strip()
    model = model.strip()
    api_key = api_key.strip()
    if not api_key:
        raise ValueError("Enter an API key or switch to Copy Prompt mode.")
    if not model:
        raise ValueError("Enter a model name.")

    if provider == "OpenAI":
        data = _post_json(
            "https://api.openai.com/v1/responses",
            {"Authorization": f"Bearer {api_key}"},
            {"model": model, "input": prompt},
            timeout,
        )
        return _openai_text(data)

    if provider == "Anthropic":
        data = _post_json(
            "https://api.anthropic.com/v1/messages",
            {"x-api-key": api_key, "anthropic-version": "2023-06-01"},
            {"model": model, "max_tokens": 4096, "messages": [{"role": "user", "content": prompt}]},
            timeout,
        )
        chunks = []
        for item in data.get("content") or []:
            if isinstance(item, dict) and isinstance(item.get("text"), str):
                chunks.append(item["text"])
        if not chunks:
            raise RuntimeError("Anthropic response did not contain text.")
        return "\n".join(chunks).strip()

    if provider == "Gemini":
        quoted_model = urllib.parse.quote(model, safe="._-")
        data = _post_json(
            f"https://generativelanguage.googleapis.com/v1beta/models/{quoted_model}:generateContent",
            {"x-goog-api-key": api_key},
            {"contents": [{"role": "user", "parts": [{"text": prompt}]}]},
            timeout,
        )
        chunks = []
        for candidate in data.get("candidates") or []:
            content = candidate.get("content") if isinstance(candidate, dict) else None
            for part in (content or {}).get("parts") or []:
                if isinstance(part, dict) and isinstance(part.get("text"), str):
                    chunks.append(part["text"])
        if not chunks:
            raise RuntimeError("Gemini response did not contain text.")
        return "\n".join(chunks).strip()

    if provider == "OpenAI-compatible":
        root = base_url.strip().rstrip("/")
        if not root:
            raise ValueError("Enter the OpenAI-compatible base URL.")
        parsed = urllib.parse.urlparse(root)
        if parsed.scheme not in {"http", "https"} or not parsed.netloc:
            raise ValueError("OpenAI-compatible base URL must be an http:// or https:// URL.")
        endpoint = root if root.endswith("/chat/completions") else root + "/chat/completions"
        data = _post_json(
            endpoint,
            {"Authorization": f"Bearer {api_key}"},
            {"model": model, "messages": [{"role": "user", "content": prompt}]},
            timeout,
        )
        return _openai_text(data)

    raise ValueError(f"Unsupported AI provider: {provider}")


def research_scan_command(python_exe: str, profile_url: str, limit: int, cookies_browser: str = "none") -> list[str]:
    command = [
        python_exe,
        "-m", "yt_dlp",
        "--skip-download",
        "--dump-json",
        "--ignore-errors",
        "--no-warnings",
        "--playlist-end", str(max(1, min(int(limit), 200))),
    ]
    if cookies_browser and cookies_browser != "none":
        command += ["--cookies-from-browser", cookies_browser]
    command += ["--", profile_url.strip()]
    return command


def platform_from_url(url: str) -> str:
    host = urllib.parse.urlparse(url or "").hostname or ""
    host = host.lower()
    if "instagram" in host:
        return "Instagram"
    if "tiktok" in host:
        return "TikTok"
    if "youtube" in host or "youtu.be" in host:
        return "YouTube"
    return "Web"


def normalize_research_item(raw: dict[str, Any], creator_url: str) -> dict[str, Any]:
    source_url = str(
        raw.get("webpage_url")
        or raw.get("original_url")
        or raw.get("url")
        or ""
    )
    if not source_url or not source_url.startswith(("http://", "https://")):
        extractor = str(raw.get("extractor_key") or raw.get("extractor") or "").lower()
        video_id = str(raw.get("id") or "")
        if "youtube" in extractor and video_id:
            source_url = f"https://www.youtube.com/watch?v={video_id}"
    views = raw.get("view_count")
    likes = raw.get("like_count")
    comments = raw.get("comment_count")
    try:
        views_i = int(views) if views is not None else None
    except (TypeError, ValueError):
        views_i = None
    try:
        likes_i = int(likes) if likes is not None else None
    except (TypeError, ValueError):
        likes_i = None
    try:
        comments_i = int(comments) if comments is not None else None
    except (TypeError, ValueError):
        comments_i = None
    engagement = None
    if views_i and views_i > 0 and (likes_i is not None or comments_i is not None):
        engagement = ((likes_i or 0) + (comments_i or 0)) / views_i * 100.0
    timestamp = raw.get("timestamp") or raw.get("release_timestamp")
    published_at = ""
    if timestamp:
        try:
            published_at = datetime.fromtimestamp(float(timestamp), tz=timezone.utc).strftime("%Y-%m-%d")
        except Exception:
            pass
    if not published_at:
        upload_date = str(raw.get("upload_date") or raw.get("release_date") or "")
        if len(upload_date) == 8 and upload_date.isdigit():
            published_at = f"{upload_date[:4]}-{upload_date[4:6]}-{upload_date[6:]}"

    return {
        "id": str(raw.get("id") or source_url or os.urandom(8).hex()),
        "creator_url": creator_url,
        "source_url": source_url,
        "platform": platform_from_url(source_url or creator_url),
        "title": str(raw.get("title") or raw.get("description") or raw.get("id") or "Untitled")[:500],
        "view_count": views_i,
        "like_count": likes_i,
        "comment_count": comments_i,
        "engagement_rate": engagement,
        "published_at": published_at,
        "duration": raw.get("duration"),
        "thumbnail": str(raw.get("thumbnail") or ""),
        "raw_json": json.dumps(raw, ensure_ascii=False),
    }
