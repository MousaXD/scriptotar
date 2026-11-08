"""Offline checks for the real-provider canary itself; never touch providers."""
from __future__ import annotations

import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / 'scripts'))
sys.path.insert(0, str(ROOT / 'sidecars/transcription'))
import live_media_e2e as live


class LiveMediaContractTests(unittest.TestCase):
    def test_public_sources_are_exactly_three_and_user_youtube_is_preserved(self):
        live.validate_cases()
        self.assertEqual([case.platform for case in live.CASES], ['youtube','instagram','tiktok'])
        self.assertIn('2PuFyjAs7JA', live.CASES[0].url)

    def test_real_probe_requires_audio(self):
        with patch.object(live.subprocess, 'run', return_value=subprocess.CompletedProcess(
            args=['ffprobe'], returncode=0, stdout=json.dumps({'format': {'duration': '12'}, 'streams': []})
        )):
            with self.assertRaisesRegex(live.CanaryFailure, 'audio'):
                live.probe_media(Path('/fixture.mp4'))

    def test_small_download_is_validated_as_media_instead_of_rejected_by_size(self):
        from scriptotar_sidecar import engine

        cases = (
            ('small_valid_audio', 5286, (11.021, 'opus'), 'passed'),
            ('small_invalid_audio', 5286, live.CanaryFailure('no decodable audio'), 'failed'),
            ('empty', 0, (11.021, 'opus'), 'failed'),
            ('oversized', 300 * 1024 * 1024 + 1, (11.021, 'opus'), 'failed'),
        )
        for name, size, probe_result, expected in cases:
            with self.subTest(name=name), tempfile.TemporaryDirectory() as tmp:
                def download(job, partial, emit):
                    media = partial / 'audio.webm'
                    with media.open('wb') as stream:
                        stream.truncate(size)
                    return media, {'extractor': 'Youtube'}

                with (
                    patch.object(engine, 'download_url', side_effect=download),
                    patch.object(live, 'probe_media') as probe,
                    patch.object(engine, 'transcribe_media') as transcribe,
                ):
                    if isinstance(probe_result, Exception):
                        probe.side_effect = probe_result
                    else:
                        probe.return_value = probe_result
                    result = live.check_case(live.CASES[0], Path(tmp), transcribe=False)

                self.assertEqual(result['status'], expected, result)
                transcribe.assert_not_called()
                if size == 5286:
                    probe.assert_called_once()
                    if expected == 'passed':
                        self.assertEqual(result['download'], 'passed')
                        self.assertEqual(result['file_bytes'], 5286)
                        self.assertEqual(result['codec'], 'opus')
                    else:
                        self.assertIn('no decodable audio', result['error'])
                else:
                    probe.assert_not_called()
                    self.assertIn('size outside accepted bounds', result['error'])

    def test_github_summary_is_explicit_on_download_failure(self):
        report = {
            'app_version':'1.3.0',
            'results':[
                {'platform':'youtube','download':'passed','transcription':'passed','status':'passed'},
                {'platform':'instagram','download':'not_run','transcription':'not_run','status':'failed',
                 'error_class':'DownloadError','error':'HTTP 403 login required'},
            ],
        }
        markdown = live.summary_markdown(report)
        self.assertIn('HTTP 403', markdown)
        self.assertIn('| instagram | not_run | not_run | failed |', markdown)
        self.assertIn('no mocks, skips, or alternate URLs', markdown)

    def test_ci_installs_native_media_tools_before_live_download(self):
        workflow = (ROOT / '.github/workflows/live-platform-e2e.yml').read_text()
        self.assertIn('apt-get install -y --no-install-recommends ffmpeg', workflow)
        self.assertIn('timeout 180', workflow)
        self.assertIn('command -v ffprobe', workflow)
        self.assertIn('python3 scripts/live_media_e2e.py', workflow)
        self.assertNotIn('continue-on-error:', workflow)

    def test_not_a_mock_media_provider(self):
        source = (ROOT / 'scripts/live_media_e2e.py').read_text()
        self.assertIn('download_url(job, partial, sink)', source)
        self.assertIn('transcribe_media(job, excerpt,', source)
        self.assertNotIn('return fake_download', source)


if __name__ == '__main__':
    unittest.main()
