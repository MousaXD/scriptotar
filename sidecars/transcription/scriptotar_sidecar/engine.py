from __future__ import annotations

import gc
import json
import os
import shutil
import subprocess
import sys
from datetime import datetime
from pathlib import Path
from typing import Any, Callable

from .errors import SidecarError
from .formatting import (
    captions_from_words,
    clean_transcript,
    human_time,
    render_srt,
    render_vtt,
    safe_name,
)
from .recovery import mark_media_ready, prepare_partial, write_private_json
from .validation import validate_supported_url
from .version import SIDECAR_VERSION

EventSink = Callable[[str, dict[str, Any]], None]


class WhisperCache:
    def __init__(self) -> None:
        self.model: Any = None
        self.key: tuple[str, str, str] | None = None
        self.batched_pipeline: Any = None

    @staticmethod
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
            except (OSError, subprocess.SubprocessError):
                pass
        return "cpu", "int8"

    def load(self, model_name: str, requested_device: str, use_batched: bool, emit: EventSink):
        from faster_whisper import BatchedInferencePipeline, WhisperModel

        device, compute = self.choose_device(requested_device)
        key = (model_name, device, compute)
        if self.model is not None and self.key == key:
            if use_batched and self.batched_pipeline is None:
                self.batched_pipeline = BatchedInferencePipeline(model=self.model)
            return self.model, self.batched_pipeline if use_batched else None, device, compute

        emit("progress", {"stage": "transcribing", "message": f"Loading Whisper model {model_name}."})
        try:
            model = WhisperModel(model_name, device=device, compute_type=compute)
        except Exception:
            if requested_device != "auto" or device != "cuda":
                raise
            device, compute = "cpu", "int8"
            key = (model_name, device, compute)
            model = WhisperModel(model_name, device=device, compute_type=compute)

        self.model = model
        self.key = key
        self.batched_pipeline = BatchedInferencePipeline(model=model) if use_batched else None
        gc.collect()
        return self.model, self.batched_pipeline if use_batched else None, device, compute


MODEL_CACHE = WhisperCache()


def probe_duration(path: Path) -> float | None:
    try:
        result = subprocess.run(
            [
                "ffprobe",
                "-v",
                "error",
                "-show_entries",
                "format=duration",
                "-of",
                "default=noprint_wrappers=1:nokey=1",
                str(path),
            ],
            check=True,
            capture_output=True,
            text=True,
            timeout=30,
        )
        return float(result.stdout.strip())
    except (OSError, ValueError, subprocess.SubprocessError):
        return None


def _quality_format(quality: str) -> tuple[str, str | None, list[str]]:
    if quality == "audio-only":
        return "ba/b", None, []
    sort_fields: list[str] = []
    if quality == "720p":
        sort_fields = ["res:720", "fps"]
    elif quality == "1080p":
        sort_fields = ["res:1080", "fps"]
    return "bv*+ba/b", "mp4", sort_fields


def _limited_metadata(info: dict[str, Any], fallback_url: str) -> dict[str, Any]:
    duration = info.get("duration")
    return {
        "title": str(info.get("title") or "video")[:500],
        "uploader": (str(info.get("uploader") or info.get("channel"))[:300] if info.get("uploader") or info.get("channel") else None),
        "source_url": str(info.get("webpage_url") or fallback_url),
        "duration_seconds": float(duration) if isinstance(duration, (int, float)) else None,
        "extractor": str(info.get("extractor_key") or info.get("extractor") or "")[:100] or None,
    }


def download_url(job: dict[str, Any], partial: Path, emit: EventSink) -> tuple[Path, dict[str, Any]]:
    import yt_dlp

    url = validate_supported_url(job["source"], cookies_browser=job.get("cookies", "none"))
    emit("progress", {"stage": "downloading", "percent": 0.0, "message": "Reading media metadata."})
    preflight_opts: dict[str, Any] = {
        "quiet": True,
        "no_warnings": True,
        "noplaylist": True,
        "socket_timeout": 30,
        "extract_flat": False,
    }
    if job.get("cookies") != "none":
        preflight_opts["cookiesfrombrowser"] = (job["cookies"],)
    with yt_dlp.YoutubeDL(preflight_opts) as ydl:
        info = ydl.extract_info(url, download=False)
    if not isinstance(info, dict):
        raise SidecarError("MEDIA_METADATA_FAILED", "The source returned no usable media metadata.", retryable=True)
    if info.get("is_live"):
        raise SidecarError("LIVE_STREAM_DISABLED", "Live streams are not supported. Save a recording first.")

    metadata = _limited_metadata(info, url)
    duration = metadata["duration_seconds"]
    max_seconds = int(job.get("max_duration_seconds") or 0)
    if duration and max_seconds and duration > max_seconds:
        raise SidecarError(
            "DURATION_LIMIT",
            f"Media duration {human_time(duration)} exceeds the configured {human_time(max_seconds)} limit.",
        )

    quality = job.get("quality", "720p")
    format_selector, merge, sort_fields = _quality_format(quality)
    output_template = str(partial / ("audio.%(ext)s" if quality == "audio-only" else "video.%(ext)s"))

    def hook(data: dict[str, Any]) -> None:
        if data.get("status") == "downloading":
            raw = str(data.get("_percent_str") or "").strip().replace("%", "")
            try:
                percent = max(0.0, min(100.0, float(raw)))
            except ValueError:
                percent = 0.0
            emit("progress", {"stage": "downloading", "percent": percent, "message": "Downloading media."})
        elif data.get("status") == "finished":
            emit("progress", {"stage": "preparing", "message": "Download complete; preparing media."})

    options: dict[str, Any] = {
        "format": format_selector,
        "outtmpl": output_template,
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
        options["merge_output_format"] = merge
    if sort_fields:
        options["format_sort"] = sort_fields
    if job.get("cookies") != "none":
        options["cookiesfrombrowser"] = (job["cookies"],)

    with yt_dlp.YoutubeDL(options) as ydl:
        ydl.download([url])

    candidates = [
        path
        for path in partial.iterdir()
        if path.is_file()
        and path.name not in {"checkpoint.json", "job.json", "error.json"}
        and path.suffix.lower() not in {".part", ".ytdl", ".json"}
    ]
    if not candidates:
        raise SidecarError("DOWNLOAD_OUTPUT_MISSING", "Download completed but no media file was produced.", retryable=True)
    media = max(candidates, key=lambda item: item.stat().st_size)
    if metadata["duration_seconds"] is None:
        metadata["duration_seconds"] = probe_duration(media)
    return media, metadata


def prepare_local(job: dict[str, Any], partial: Path, emit: EventSink) -> tuple[Path, dict[str, Any]]:
    source = Path(job["source"])
    duration = probe_duration(source)
    max_seconds = int(job.get("max_duration_seconds") or 0)
    if duration and max_seconds and duration > max_seconds:
        raise SidecarError(
            "DURATION_LIMIT",
            f"Media duration {human_time(duration)} exceeds the configured {human_time(max_seconds)} limit.",
        )
    if job.get("copy_source", True):
        destination = partial / f"video{source.suffix.lower()}"
        shutil.copy2(source, destination)
        media = destination
    else:
        media = source
    metadata = {
        "title": source.stem,
        "uploader": None,
        "source_url": str(source),
        "duration_seconds": duration,
        "extractor": "local-file",
    }
    emit("progress", {"stage": "preparing", "percent": 100.0, "message": "Local media is ready."})
    return media, metadata


def transcribe_media(job: dict[str, Any], media: Path, metadata: dict[str, Any], emit: EventSink) -> dict[str, Any]:
    model, batched, device, compute = MODEL_CACHE.load(
        job.get("model", "medium"), job.get("device", "auto"), bool(job.get("batched", False)), emit
    )
    language = None if job.get("language") == "auto" else job.get("language")
    task = "translate" if job.get("translate") else "transcribe"
    kwargs: dict[str, Any] = {
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

    duration = metadata.get("duration_seconds")
    segments: list[dict[str, Any]] = []
    words: list[dict[str, Any]] = []
    raw_lines: list[str] = []
    emit("progress", {"stage": "transcribing", "percent": 0.0, "message": "Transcribing speech."})
    for index, segment in enumerate(segments_iter, 1):
        text = (segment.text or "").strip()
        if text:
            raw_lines.append(text)
        segment_words: list[dict[str, Any]] = []
        for word in segment.words or []:
            item = {
                "start": round(float(word.start), 3) if word.start is not None else None,
                "end": round(float(word.end), 3) if word.end is not None else None,
                "word": str(word.word or ""),
                "probability": round(float(word.probability), 5) if word.probability is not None else None,
            }
            segment_words.append(item)
            words.append(item)
        segment_item = {
            "index": index,
            "start": round(float(segment.start), 3),
            "end": round(float(segment.end), 3),
            "text": text,
            "words": segment_words,
        }
        segments.append(segment_item)
        if isinstance(duration, (int, float)) and duration > 0:
            percent = max(0.0, min(100.0, float(segment.end) / float(duration) * 100.0))
            emit(
                "progress",
                {
                    "stage": "transcribing",
                    "percent": percent,
                    "message": f"Transcribed through {human_time(segment.end)}.",
                },
            )

    raw_text = "\n".join(line.strip() for line in raw_lines if line.strip()).strip()
    clean_text = clean_transcript(raw_lines)
    captions = captions_from_words(words) if words else [
        {"start": segment["start"], "end": segment["end"], "text": segment["text"]}
        for segment in segments
        if segment["text"]
    ]
    return {
        "text": raw_text,
        "clean_text": clean_text,
        "segments": segments,
        "words": words,
        "captions": captions,
        "language": getattr(info, "language", None),
        "language_probability": getattr(info, "language_probability", None),
        "device": device,
        "compute_type": compute,
        "task": task,
    }


def write_outputs(
    job: dict[str, Any], partial: Path, metadata: dict[str, Any], result: dict[str, Any]
) -> dict[str, str]:
    timestamp_lines = [
        f"[{human_time(segment['start'])} - {human_time(segment['end'])}] {segment['text']}"
        for segment in result["segments"]
        if segment["text"]
    ]
    text_files = {
        "text": ("transcript.txt", result["text"]),
        "clean_text": ("transcript_clean.txt", result["clean_text"]),
        "timestamp_text": ("transcript_timestamps.txt", "\n".join(timestamp_lines)),
        "srt": ("transcript.srt", render_srt(result["captions"])),
        "vtt": ("transcript.vtt", render_vtt(result["captions"])),
    }
    artifacts: dict[str, str] = {}
    for key, (filename, content) in text_files.items():
        path = partial / filename
        path.write_text(content + ("\n" if content and not content.endswith("\n") else ""), encoding="utf-8")
        artifacts[key] = filename

    transcript_json = {
        "app": "Scriptotar",
        "sidecar_version": SIDECAR_VERSION,
        "source": metadata,
        "model": job.get("model"),
        "device": result["device"],
        "compute_type": result["compute_type"],
        "task": result["task"],
        "detected_language": result["language"],
        "language_probability": result["language_probability"],
        "text": result["text"],
        "clean_text": result["clean_text"],
        "segments": result["segments"],
        "words": result["words"],
        "captions": result["captions"],
    }
    write_private_json(partial / "transcript.json", transcript_json)
    artifacts["json"] = "transcript.json"
    return artifacts


def final_directory(partial: Path, title: str) -> Path:
    stamp = datetime.now().strftime("%Y%m%d_%H%M%S")
    base = partial.parent / f"{stamp}_{safe_name(title)}"
    candidate = base
    suffix = 2
    while candidate.exists():
        candidate = partial.parent / f"{base.name}_{suffix}"
        suffix += 1
    return candidate


def run_job(job: dict[str, Any], emit: EventSink) -> dict[str, Any]:
    recovery = prepare_partial(job)
    partial = recovery.partial_dir
    write_private_json(partial / "job.json", {key: value for key, value in job.items() if key != "cookies"})

    try:
        if recovery.reused_media is not None and recovery.metadata is not None:
            media = recovery.reused_media
            metadata = recovery.metadata
            emit(
                "progress",
                {
                    "stage": "preparing",
                    "message": "Reusing a previously completed media artifact; transcription restarts from the beginning.",
                },
            )
        elif job["input_type"] == "url":
            media, metadata = download_url(job, partial, emit)
            mark_media_ready(job, partial, media, metadata)
        else:
            media, metadata = prepare_local(job, partial, emit)
            mark_media_ready(job, partial, media, metadata)

        result = transcribe_media(job, media, metadata, emit)
        emit("progress", {"stage": "processing", "message": "Writing transcript artifacts."})
        artifacts = write_outputs(job, partial, metadata, result)
        final = final_directory(partial, metadata["title"])
        media_inside = media.parent == partial
        partial.rename(final)
        resolved_artifacts = {key: str(final / path) for key, path in artifacts.items()}
        media_path = final / media.name if media_inside else media
        return {
            "source": metadata,
            "transcript": {
                "text": result["text"],
                "clean_text": result["clean_text"],
                "segments": result["segments"],
                "words": result["words"],
                "language": result["language"],
                "language_probability": result["language_probability"],
                "duration_seconds": metadata.get("duration_seconds"),
            },
            "artifacts": {**resolved_artifacts, "media": str(media_path)},
            "output_dir": str(final),
        }
    except BaseException as exc:
        if isinstance(exc, (KeyboardInterrupt, SystemExit)):
            raise
        if job.get("keep_failed"):
            try:
                write_private_json(
                    partial / "error.json",
                    {"type": type(exc).__name__, "message": str(exc)[:2000]},
                )
            except OSError:
                pass
        else:
            # Keep only crash/interruption leftovers. Controlled failures are cleaned unless requested.
            shutil.rmtree(partial, ignore_errors=True)
        raise
