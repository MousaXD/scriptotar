"""Offline checks for the real-provider canary itself; never touch providers."""
from __future__ import annotations

import json
from pathlib import Path
import subprocess
import sys
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

    def test_github_summary_is_explicit_on_download_failure(self):
        report = {
            'app_version':'1.1.0',
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
        self.assertIn('sudo apt-get install -y --no-install-recommends ffmpeg', workflow)
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
