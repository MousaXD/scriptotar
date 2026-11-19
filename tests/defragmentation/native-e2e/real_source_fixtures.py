#!/usr/bin/env python3
"""Prepare TWO genuine public media fixtures; NEVER certify native ingestion.

This optional QA-only tool fetches original LibriVox recordings through their
Wikimedia Commons *original binary* URLs and genuinely transcodes the audio
into a supported .webm container. It neither invents transcript text nor
writes Scriptotar's canonical SQLite. Installed UI ingest, sidecar transcripts,
source permissions and the 16-case matrix remain independently NOT TESTED.
"""
from __future__ import annotations

import argparse
from datetime import datetime, timezone
import hashlib
import json
import math
from pathlib import Path
import subprocess
import urllib.request


MAX_DOWNLOAD_BYTES = 8 * 1024 * 1024
SOURCES = (
    {
        "source_id": "commons:shortstory025_reflection",
        "title": "A Reflection (Kate Chopin)",
        "narrator": "Alan Davis-Drake",
        "page": "https://commons.wikimedia.org/wiki/File:Shortstory025_reflection.ogg",
        "original_url": "https://upload.wikimedia.org/wikipedia/commons/4/4f/Shortstory025_reflection.ogg",
    },
    {
        "source_id": "commons:charon_dunsany_sc",
        "title": "Charon (Lord Dunsany)",
        "narrator": "Steven Collins",
        "page": "https://commons.wikimedia.org/wiki/File:Charon_dunsany_sc.ogg",
        "original_url": "https://upload.wikimedia.org/wikipedia/commons/6/6d/Charon_dunsany_sc.ogg",
    },
)


def digest(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as file:
        for chunk in iter(lambda: file.read(65536), b""):
            h.update(chunk)
    return h.hexdigest()


def validate_distinct(records: list[dict]) -> None:
    """Never present renamed, copied or transcoded duplicate audio as two origins."""
    if len(records) != 2:
        raise ValueError("Exactly two independently acquired media origins are required")
    for key in ("source_id", "original_url", "original_sha256", "webm_sha256"):
        values = [row.get(key) for row in records]
        if any(not isinstance(value, str) or not value for value in values):
            raise ValueError(f"Missing source provenance field: {key}")
        if len(set(values)) != 2:
            raise ValueError(f"Duplicate media source provenance: {key}")


class RejectRedirects(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        return None


def download_original(url: str, target: Path) -> None:
    """Bounded original Ogg transfer; disallow redirects to unreviewed origins."""
    if url not in {row["original_url"] for row in SOURCES}:
        raise ValueError("Download URL is not an approved original Wikimedia file")
    opener = urllib.request.build_opener(RejectRedirects())
    request = urllib.request.Request(
        url, headers={"User-Agent": "Scriptotar-Independent-Native-QA/1.0"}
    )
    total = 0
    with opener.open(request, timeout=35) as response, target.open("xb") as out:
        if response.geturl() != url:
            raise ValueError("Original download origin changed")
        while True:
            chunk = response.read(65536)
            if not chunk:
                break
            total += len(chunk)
            if total > MAX_DOWNLOAD_BYTES:
                raise ValueError("Original media exceeds bounded download limit")
            out.write(chunk)
    if total < 32768 or target.open("rb").read(4) != b"OggS":
        raise ValueError("Original downloaded file is not a plausible Ogg recording")


def probe_audio(path: Path, ffprobe: str, codec: str | None = None) -> float:
    command = [
        ffprobe, "-v", "error", "-select_streams", "a:0",
        "-show_entries", "stream=codec_name:format=duration", "-of", "json",
        str(path),
    ]
    result = subprocess.run(
        command, check=True, capture_output=True, text=True, timeout=35
    )
    info = json.loads(result.stdout)
    streams = info.get("streams", [])
    if len(streams) != 1:
        raise ValueError("Source must have exactly one probed first audio stream")
    if codec is not None and streams[0].get("codec_name") != codec:
        raise ValueError(f"Unexpected transcoded audio codec: {streams[0]}")
    duration = float(info.get("format", {}).get("duration", 0))
    if not math.isfinite(duration) or not 10 <= duration <= 600:
        raise ValueError("Media has invalid or unreasonable real duration")
    return duration


def prepare(output: Path, ffmpeg: str = "ffmpeg",
            ffprobe: str = "ffprobe") -> dict:
    if output.exists() and any(output.iterdir()):
        raise ValueError("Use a NEW empty isolated QA directory; do not reuse artifacts")
    output.mkdir(parents=True, exist_ok=True)
    records = []
    for source in SOURCES:
        name = source["source_id"].split(":", 1)[1]
        original = output / (name + ".ogg")
        converted = output / (name + ".webm")
        download_original(source["original_url"], original)
        duration = probe_audio(original, ffprobe)
        subprocess.run(
            [ffmpeg, "-nostdin", "-hide_banner", "-loglevel", "error", "-y",
             "-i", str(original), "-map", "0:a:0", "-vn",
             "-c:a", "libopus", "-b:a", "64k", "-f", "webm",
             str(converted)],
            check=True, capture_output=True, timeout=150,
        )
        transcoded_duration = probe_audio(converted, ffprobe, codec="opus")
        if abs(duration - transcoded_duration) > 2:
            raise ValueError("Transcoding changed source duration unexpectedly")
        if converted.stat().st_size < 32768:
            raise ValueError("Transcoded .webm media is empty or truncated")
        records.append({
            **source,
            "original_filename": original.name,
            "original_sha256": digest(original),
            "original_size_bytes": original.stat().st_size,
            "webm_filename": converted.name,
            "webm_sha256": digest(converted),
            "webm_size_bytes": converted.stat().st_size,
            "original_duration_seconds": duration,
            "webm_duration_seconds": transcoded_duration,
            "media_provenance": "original-source-downloaded-and-ffprobe-verified",
            "installed_import": "NOT TESTED",
            "transcript_provenance": "NOT TESTED",
            "study_attachment": "NOT TESTED",
        })
    validate_distinct(records)
    manifest = {
        "purpose": "native-two-real-media-preparation-only",
        "created_at_utc": datetime.now(timezone.utc).isoformat(),
        "sources": records,
        "installed_two_source_acceptance": "NOT TESTED",
        "note": "Only a real installed Queue/sidecar workflow and canonical DB can certify transcripts.",
    }
    (output / "origin-manifest.json").write_text(
        json.dumps(manifest, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
    )
    return manifest


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--ffmpeg", default="ffmpeg")
    parser.add_argument("--ffprobe", default="ffprobe")
    parser.add_argument(
        "--prepare", action="store_true",
        help="Explicit consent for ephemeral public media downloads and transcode",
    )
    args = parser.parse_args()
    if not args.prepare:
        parser.error("--prepare required: downloads must never run on import or by default")
    manifest = prepare(args.output, args.ffmpeg, args.ffprobe)
    print(json.dumps({
        "origins": [source["source_id"] for source in manifest["sources"]],
        "native_acceptance": manifest["installed_two_source_acceptance"],
    }, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
