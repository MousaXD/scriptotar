#!/usr/bin/env python3
"""Strict real-provider download + small-Whisper transcription E2E, in isolated CI.

This is intentionally NOT a mocked extractor test. Real platform bot/login/region
failures are reported per platform and make the workflow fail (never skip or
pretend success). No browser cookies, accounts, proxy bypass or test-only URL
fallback is used. All files stay inside a disposable CI directory.
"""
from __future__ import annotations

import argparse
from dataclasses import dataclass
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time
from typing import Any
from urllib.parse import urlsplit


@dataclass(frozen=True)
class LiveCase:
    platform: str
    url: str
    extractor_token: str


# YouTube: exact user-supplied URL. Instagram: an existing public Scriptotar
# research URL (historically downloaded before a CUDA transcription failure).
# TikTok: official developer documentation's embedded public video example.
CASES = (
    LiveCase('youtube', 'https://youtu.be/2PuFyjAs7JA?si=yqHYehEncAJH67oB', 'youtube'),
    LiveCase('instagram', 'https://www.instagram.com/reel/DaLCUltyZ7p/?hl=en', 'instagram'),
    LiveCase('tiktok', 'https://www.tiktok.com/@scout2015/video/6718335390845095173', 'tiktok'),
)


class CanaryFailure(RuntimeError):
    pass


def validate_cases() -> None:
    from scriptotar_sidecar.validation import validate_supported_url

    assert {case.platform for case in CASES} == {'youtube', 'instagram', 'tiktok'}
    assert len({case.url for case in CASES}) == len(CASES)
    assert CASES[0].url == 'https://youtu.be/2PuFyjAs7JA?si=yqHYehEncAJH67oB'
    expected_hosts = {'youtube': {'youtu.be', 'youtube.com'},
                      'instagram': {'instagram.com'}, 'tiktok': {'tiktok.com'}}
    for case in CASES:
        parsed = urlsplit(validate_supported_url(case.url, cookies_browser='none'))
        assert parsed.scheme == 'https', case
        host = (parsed.hostname or '').lower()
        assert any(host == allowed or host.endswith('.' + allowed)
                   for allowed in expected_hosts[case.platform]), case
        assert parsed.username is None and parsed.password is None, case


def probe_media(path: Path) -> tuple[float, str]:
    result = subprocess.run(
        ['ffprobe', '-v', 'error', '-show_format', '-show_streams', '-of', 'json', str(path)],
        check=True, capture_output=True, text=True, timeout=30,
    )
    metadata = json.loads(result.stdout)
    tracks = metadata.get('streams', [])
    audio_tracks = [s for s in tracks if s.get('codec_type') == 'audio']
    if not audio_tracks:
        raise CanaryFailure('Downloaded file has no decodable audio track')
    seconds = float(metadata.get('format', {}).get('duration') or 0)
    if not 0 < seconds <= 86400:
        raise CanaryFailure(f'ffprobe found invalid audio duration: {seconds}')
    return seconds, str(audio_tracks[0].get('codec_name', 'unknown'))


def clip_audio(source: Path, destination: Path) -> tuple[float, str]:
    subprocess.run(
        [
            'ffmpeg', '-hide_banner', '-loglevel', 'error', '-nostdin', '-y',
            '-i', str(source), '-t', '15', '-vn', '-ar', '16000', '-ac', '1',
            '-c:a', 'pcm_s16le', str(destination),
        ],
        check=True, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, timeout=90,
    )
    if not destination.is_file() or destination.stat().st_size < 2048:
        raise CanaryFailure('15-second transcription excerpt was not written')
    return probe_media(destination)


def check_case(case: LiveCase, root: Path, transcribe: bool) -> dict[str, Any]:
    from scriptotar_sidecar.engine import download_url, transcribe_media, write_outputs
    from scriptotar_sidecar.validation import validate_job_payload

    directory = root / case.platform
    directory.mkdir(mode=0o700)
    partial = directory / 'media'
    partial.mkdir(mode=0o700)
    output_root = directory / 'outputs'
    started = time.monotonic()
    events: list[dict[str, Any]] = []

    def sink(kind: str, payload: dict[str, Any]) -> None:
        # The retained CI report includes counters, not private URLs or cookies.
        if kind == 'progress':
            events.append({'stage': payload.get('stage'), 'percent': payload.get('percent')})

    command = {
        'protocol': 1, 'type': 'transcribe', 'job_id': f'ci-live-{case.platform}',
        'input': {'kind': 'url', 'value': case.url},
        'output': {'root': str(output_root)},
        'options': {
            'model': 'small', 'device': 'cpu', 'language': 'auto',
            'quality': 'audio-only', 'cookies_browser': 'none',
            'max_duration_seconds': 3600, 'keep_failed': True,
        },
    }
    job = validate_job_payload(command)
    result: dict[str, Any] = {
        'platform': case.platform, 'source_url': case.url,
        'download': 'not_run', 'transcription': 'not_run', 'status': 'failed',
    }
    try:
        media, metadata = download_url(job, partial, sink)
        if not media.is_file() or not media.resolve().is_relative_to(partial.resolve()):
            raise CanaryFailure('Downloader wrote media outside its dedicated directory')
        size = media.stat().st_size
        if not 8192 <= size <= 300 * 1024 * 1024:
            raise CanaryFailure(f'Download size outside accepted bounds: {size} bytes')
        reported_extractor = str(metadata.get('extractor') or '').lower()
        if case.extractor_token not in reported_extractor:
            raise CanaryFailure(f'Wrong provider extractor returned: {reported_extractor!r}')
        seconds, audio_codec = probe_media(media)
        result.update(download='passed', file_bytes=size, duration_seconds=round(seconds, 2),
                      codec=audio_codec, extractor=metadata['extractor'],
                      download_seconds=round(time.monotonic() - started, 2))
        if transcribe:
            excerpt = directory / 'speech-sample.wav'
            clip_seconds, _ = clip_audio(media, excerpt)
            inference = transcribe_media(job, excerpt,
                                         {**metadata, 'duration_seconds': clip_seconds}, sink)
            if inference.get('device') != 'cpu' or inference.get('compute_type') != 'int8':
                raise CanaryFailure('Production Whisper did not perform CPU int8 inference')
            if not isinstance(inference.get('segments'), list) or not isinstance(inference.get('text'), str):
                raise CanaryFailure('Invalid Whisper inference response')
            exports = directory / 'exports'
            exports.mkdir(mode=0o700)
            files = write_outputs(job, exports, metadata, inference)
            if set(files) != {'text', 'clean_text', 'timestamp_text', 'srt', 'vtt', 'json'}:
                raise CanaryFailure('Some transcription export types were not produced')
            if not all((exports / relative).is_file() for relative in files.values()):
                raise CanaryFailure('Transcription output files are missing')
            saved = json.loads((exports / files['json']).read_text(encoding='utf-8'))
            if saved['sidecar_version'] != '1.1.0' or saved['device'] != 'cpu':
                raise CanaryFailure('Saved transcript identity or device is incorrect')
            result['transcription'] = 'passed'
            result['segment_count'] = len(inference['segments'])
            result['clip_seconds'] = round(clip_seconds, 2)
        result.update(status='passed', event_count=len(events),
                      elapsed_seconds=round(time.monotonic() - started, 2))
    except Exception as exc:
        result.update(status='failed', error_class=type(exc).__name__,
                      error=str(exc)[:1100], elapsed_seconds=round(time.monotonic() - started, 2))
    return result


def summary_markdown(report: dict[str, Any]) -> str:
    lines = [f"## Scriptotar {report['app_version']} real platform E2E", '',
             '| Platform | Download | Real Whisper (15 s excerpt) | Result |',
             '| --- | --- | --- | --- |']
    for row in report.get('results', []):
        lines.append(f"| {row['platform']} | {row['download']} | {row['transcription']} | {row['status']} |")
        if row.get('error'):
            detail = str(row['error']).replace('|', '/').replace('\n', ' ')[:400]
            lines.append(f"\n**{row['platform']} blocker:** `{row['error_class']}` — {detail}\n")
    lines.append('\nReal source failures fail this workflow; no mocks, skips, or alternate URLs are accepted.')
    return '\n'.join(lines) + '\n'


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--platform', choices=['all', 'youtube', 'instagram', 'tiktok'], default='all')
    parser.add_argument('--summary-json', type=Path)
    parser.add_argument('--validate-only', action='store_true')
    parser.add_argument('--download-only', action='store_true', help='Only for triage; full CI always runs real Whisper')
    args = parser.parse_args()
    validate_cases()
    if args.validate_only:
        print('PASS: all three pinned public video URLs are recognized and exact user YouTube URL is preserved')
        return 0
    from scriptotar_sidecar.version import SIDECAR_VERSION
    report: dict[str, Any] = {'app_version': SIDECAR_VERSION, 'status': 'failed', 'results': []}
    cases = CASES if args.platform == 'all' else tuple(c for c in CASES if c.platform == args.platform)
    with tempfile.TemporaryDirectory(prefix='scriptotar-live-e2e-') as tmp:
        for case in cases:
            result = check_case(case, Path(tmp), transcribe=not args.download_only)
            report['results'].append(result)
            print(json.dumps(result, sort_keys=True), flush=True)
    report['status'] = 'passed' if all(row['status'] == 'passed' for row in report['results']) else 'failed'
    if args.summary_json:
        args.summary_json.parent.mkdir(parents=True, exist_ok=True)
        args.summary_json.write_text(json.dumps(report, indent=2) + '\n', encoding='utf-8')
    print(summary_markdown(report))
    return 0 if report['status'] == 'passed' else 1


if __name__ == '__main__':
    sys.exit(main())
