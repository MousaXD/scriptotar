#!/usr/bin/env python3
from __future__ import annotations

import gc
import json
import os
import shutil
import subprocess
import sys
import traceback
from datetime import datetime
from pathlib import Path

import yt_dlp
from faster_whisper import BatchedInferencePipeline, WhisperModel

from core import (
    VIDEO_EXTS,
    captions_from_words,
    clean_transcript,
    human_time,
    quality_format,
    quality_sort,
    safe_name,
    validate_supported_url,
    write_srt,
    write_vtt,
)

APP_VERSION = "1.1.0"
CURRENT_MODEL = None
CURRENT_KEY = None
CURRENT_BATCHED = None


def emit(event_type: str, **payload) -> None:
    print(json.dumps({"type": event_type, **payload}, ensure_ascii=False), flush=True)


def probe_duration(path: Path) -> float | None:
    try:
        result = subprocess.run(
            [
                "ffprobe", "-v", "error", "-show_entries", "format=duration",
                "-of", "default=noprint_wrappers=1:nokey=1", str(path),
            ],
            check=True,
            capture_output=True,
            text=True,
        )
        return float(result.stdout.strip())
    except Exception:
        return None


def choose_device(requested: str) -> tuple[str, str]:
    if requested == "cpu":
        return "cpu", "int8"
    if requested == "cuda":
        return "cuda", "int8_float16"
    if shutil.which("nvidia-smi"):
        try:
            subprocess.run(
                ["nvidia-smi"],
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
                timeout=3,
                check=True,
            )
            return "cuda", "int8_float16"
        except Exception:
            pass
    return "cpu", "int8"


def load_model(model_name: str, requested_device: str, use_batched: bool):
    global CURRENT_MODEL, CURRENT_KEY, CURRENT_BATCHED
    device, compute = choose_device(requested_device)
    key = (model_name, device, compute)
    if CURRENT_MODEL is not None and CURRENT_KEY == key:
        emit("log", message=f"Reusing loaded Whisper model: {model_name} on {device}.")
        if use_batched and CURRENT_BATCHED is None:
            CURRENT_BATCHED = BatchedInferencePipeline(model=CURRENT_MODEL)
        return CURRENT_MODEL, CURRENT_BATCHED if use_batched else None, device, compute

    emit("status", message=f"Loading Whisper {model_name} on {device}...")
    emit("log", message=f"Loading model {model_name}; device={device}; compute={compute}.")
    try:
        model = WhisperModel(model_name, device=device, compute_type=compute)
    except Exception as exc:
        if requested_device == "auto" and device == "cuda":
            emit("log", message=f"CUDA initialization failed. Falling back to CPU: {exc}")
            device, compute = "cpu", "int8"
            key = (model_name, device, compute)
            model = WhisperModel(model_name, device=device, compute_type=compute)
        else:
            raise

    CURRENT_MODEL = model
    CURRENT_KEY = key
    CURRENT_BATCHED = BatchedInferencePipeline(model=model) if use_batched else None
    gc.collect()
    return CURRENT_MODEL, CURRENT_BATCHED, device, compute


def preflight_url(url: str, cookies_browser: str) -> dict:
    validate_supported_url(url)
    opts = {
        "quiet": True,
        "no_warnings": True,
        "noplaylist": True,
        "socket_timeout": 30,
        "extract_flat": False,
    }
    if cookies_browser != "none":
        opts["cookiesfrombrowser"] = (cookies_browser,)
    with yt_dlp.YoutubeDL(opts) as ydl:
        info = ydl.extract_info(url, download=False)
    if not info:
        raise RuntimeError("The site returned no media information.")
    if info.get("is_live"):
        raise RuntimeError("Live streams are disabled. Save a recording first, then transcribe the file.")
    return info


def make_partial_dir(root: str, job_id: str) -> Path:
    output_root = Path(root).expanduser()
    output_root.mkdir(parents=True, exist_ok=True)
    partial = output_root / f".scriptotar-{job_id}.partial"
    if partial.exists():
        shutil.rmtree(partial, ignore_errors=True)
    partial.mkdir(parents=True)
    return partial


def final_dir(partial: Path, title: str) -> Path:
    stamp = datetime.now().strftime("%Y%m%d_%H%M%S")
    base = partial.parent / f"{stamp}_{safe_name(title)}"
    candidate = base
    number = 2
    while candidate.exists():
        candidate = partial.parent / f"{base.name}_{number}"
        number += 1
    return candidate


def download_url(job: dict, partial: Path) -> tuple[Path, str, str | None, str, float | None]:
    url = validate_supported_url(job["source"])
    cookies = job.get("cookies", "none")
    emit("status", message="Reading video metadata...")
    info = preflight_url(url, cookies)
    title = info.get("title") or "video"
    uploader = info.get("uploader") or info.get("channel")
    source_url = info.get("webpage_url") or url
    duration = info.get("duration")
    max_seconds = int(job.get("max_duration_seconds") or 0)
    if duration and max_seconds and float(duration) > max_seconds:
        raise RuntimeError(
            f"Video is {human_time(duration)} long, above your {human_time(max_seconds)} safety limit. "
            "Raise the limit in Settings if this is intentional."
        )

    quality = job.get("quality", "720p")
    fmt, merge = quality_format(quality)
    sort_fields = quality_sort(quality)
    outtmpl = str(partial / ("audio.%(ext)s" if quality == "Audio only" else "video.%(ext)s"))

    def hook(data):
        if data.get("status") == "downloading":
            raw = (data.get("_percent_str") or "").strip().replace("%", "")
            try:
                pct = float(raw)
            except ValueError:
                pct = 0.0
            emit("progress", job_id=job["id"], value=min(32, pct * 0.32), message=f"Downloading... {pct:.1f}%")
        elif data.get("status") == "finished":
            emit("progress", job_id=job["id"], value=33, message="Download complete. Preparing media...")

    opts = {
        "format": fmt,
        "outtmpl": outtmpl,
        "noplaylist": True,
        "quiet": True,
        "no_warnings": True,
        "progress_hooks": [hook],
        "retries": 3,
        "fragment_retries": 3,
        "socket_timeout": 30,
        "overwrites": True,
    }
    if merge:
        opts["merge_output_format"] = merge
    if sort_fields:
        opts["format_sort"] = sort_fields
    if cookies != "none":
        opts["cookiesfrombrowser"] = (cookies,)

    with yt_dlp.YoutubeDL(opts) as ydl:
        ydl.download([url])

    candidates = [p for p in partial.iterdir() if p.is_file() and p.suffix.lower() not in {".part", ".ytdl", ".json"}]
    if not candidates:
        raise RuntimeError("Download finished but no media file was found.")
    media = max(candidates, key=lambda p: p.stat().st_size)
    return media, title, uploader, source_url, float(duration) if duration else probe_duration(media)


def prepare_local(job: dict, partial: Path) -> tuple[Path, str, None, str, float | None]:
    src = Path(job["source"]).expanduser().resolve()
    if not src.is_file() or src.suffix.lower() not in VIDEO_EXTS:
        raise RuntimeError("The selected local video does not exist or has an unsupported extension.")
    duration = probe_duration(src)
    max_seconds = int(job.get("max_duration_seconds") or 0)
    if duration and max_seconds and duration > max_seconds:
        raise RuntimeError(
            f"Video is {human_time(duration)} long, above your {human_time(max_seconds)} safety limit."
        )
    if job.get("copy_source", True):
        dest = partial / f"video{src.suffix.lower()}"
        shutil.copy2(src, dest)
        media = dest
    else:
        media = src
    emit("progress", job_id=job["id"], value=33, message="Local video ready.")
    return media, src.stem, None, str(src), duration


def transcribe(job: dict, media: Path, duration: float | None):
    model, batched, device, compute = load_model(
        job.get("model", "medium"),
        job.get("device", "auto"),
        bool(job.get("batched", False)),
    )
    language = {"Arabic": "ar", "English": "en", "auto": None}.get(job.get("language", "auto"))
    task = "translate" if job.get("translate") else "transcribe"
    kwargs = {
        "language": language,
        "task": task,
        "word_timestamps": True,
        "vad_filter": True,
        "vad_parameters": {"min_silence_duration_ms": 500},
    }
    if batched is not None:
        kwargs["batch_size"] = int(job.get("batch_size", 8))
        segments_iter, info = batched.transcribe(str(media), **kwargs)
    else:
        kwargs["beam_size"] = 5
        kwargs["condition_on_previous_text"] = True
        segments_iter, info = model.transcribe(str(media), **kwargs)

    segments = []
    words = []
    raw_lines = []
    emit("progress", job_id=job["id"], value=38, message="Transcribing speech...")
    for idx, segment in enumerate(segments_iter, 1):
        text = (segment.text or "").strip()
        if text:
            raw_lines.append(text)
        seg_words = []
        for word in (segment.words or []):
            item = {
                "start": round(float(word.start), 3) if word.start is not None else None,
                "end": round(float(word.end), 3) if word.end is not None else None,
                "word": word.word,
                "probability": round(float(word.probability), 5) if word.probability is not None else None,
            }
            seg_words.append(item)
            words.append(item)
        segments.append(
            {
                "index": idx,
                "start": round(float(segment.start), 3),
                "end": round(float(segment.end), 3),
                "text": text,
                "words": seg_words,
            }
        )
        if duration and duration > 0:
            value = 38 + min(58, (float(segment.end) / duration) * 58)
            emit(
                "progress",
                job_id=job["id"],
                value=value,
                message=f"Transcribing... {human_time(segment.end)} / {human_time(duration)}",
            )

    clean = clean_transcript(raw_lines)
    raw = "\n".join(line.strip() for line in raw_lines if line.strip()).strip()
    captions = captions_from_words(words) if words else [
        {"start": s["start"], "end": s["end"], "text": s["text"]}
        for s in segments if s["text"]
    ]
    return {
        "raw_text": raw,
        "clean_text": clean,
        "segments": segments,
        "words": words,
        "captions": captions,
        "language": getattr(info, "language", None),
        "language_probability": getattr(info, "language_probability", None),
        "device": device,
        "compute_type": compute,
        "task": task,
    }


def write_outputs(job: dict, partial: Path, media: Path, meta_base: dict, result: dict) -> None:
    (partial / "transcript.txt").write_text(result["raw_text"] + ("\n" if result["raw_text"] else ""), encoding="utf-8")
    (partial / "transcript_clean.txt").write_text(result["clean_text"] + ("\n" if result["clean_text"] else ""), encoding="utf-8")
    timestamp_lines = [
        f"[{human_time(s['start'])} - {human_time(s['end'])}] {s['text']}"
        for s in result["segments"] if s["text"]
    ]
    (partial / "transcript_timestamps.txt").write_text("\n".join(timestamp_lines) + ("\n" if timestamp_lines else ""), encoding="utf-8")
    (partial / "transcript.srt").write_text(write_srt(result["captions"]), encoding="utf-8")
    (partial / "transcript.vtt").write_text(write_vtt(result["captions"]), encoding="utf-8")
    metadata = {
        "app": "Scriptotar",
        "version": APP_VERSION,
        **meta_base,
        "model": job.get("model"),
        "device": result["device"],
        "compute_type": result["compute_type"],
        "task": result["task"],
        "detected_language": result["language"],
        "language_probability": result["language_probability"],
        "text": result["raw_text"],
        "clean_text": result["clean_text"],
        "segments": result["segments"],
        "words": result["words"],
        "captions": result["captions"],
    }
    (partial / "transcript.json").write_text(json.dumps(metadata, ensure_ascii=False, indent=2), encoding="utf-8")


def run_job(job: dict) -> None:
    job_id = job["id"]
    partial = make_partial_dir(job["output_root"], job_id)
    (partial / "job.json").write_text(json.dumps(job, ensure_ascii=False, indent=2), encoding="utf-8")
    try:
        emit("job_started", job_id=job_id)
        if job["input_type"] == "url":
            media, title, uploader, source, duration = download_url(job, partial)
        else:
            media, title, uploader, source, duration = prepare_local(job, partial)

        media_is_inside = media.parent == partial
        result = transcribe(job, media, duration)
        meta_base = {
            "title": title,
            "uploader": uploader,
            "source": source,
            "duration_seconds": duration,
            "media_file": media.name if media_is_inside else str(media),
        }
        write_outputs(job, partial, media, meta_base, result)
        final = final_dir(partial, title)
        partial.rename(final)
        final_media = final / media.name if media_is_inside else media
        emit(
            "progress",
            job_id=job_id,
            value=100,
            message="Finished",
        )
        emit(
            "result",
            job_id=job_id,
            output_dir=str(final),
            transcript=str(final / "transcript.txt"),
            clean_transcript=str(final / "transcript_clean.txt"),
            srt=str(final / "transcript.srt"),
            vtt=str(final / "transcript.vtt"),
            metadata=str(final / "transcript.json"),
            media=str(final_media),
            title=title,
            language=result["language"],
            language_probability=result["language_probability"],
        )
    except Exception as exc:
        keep_failed = bool(job.get("keep_failed", False))
        if keep_failed and partial.exists():
            failed = partial.with_name(partial.name.replace(".partial", ".failed"))
            try:
                partial.rename(failed)
                (failed / "ERROR.txt").write_text(f"{type(exc).__name__}: {exc}\n", encoding="utf-8")
            except Exception:
                pass
        else:
            shutil.rmtree(partial, ignore_errors=True)
        emit("error", job_id=job_id, message=f"{type(exc).__name__}: {exc}")


def main() -> int:
    emit("ready", version=APP_VERSION)
    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue
        try:
            message = json.loads(line)
            command = message.get("command")
            if command == "job":
                run_job(message["job"])
            elif command == "ping":
                emit("pong")
            elif command == "shutdown":
                emit("shutdown")
                return 0
            else:
                emit("error", job_id=message.get("job", {}).get("id"), message="Unknown worker command.")
        except Exception as exc:
            emit("error", message=f"Worker protocol error: {type(exc).__name__}: {exc}")
            traceback.print_exc(file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
