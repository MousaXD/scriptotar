from __future__ import annotations

import subprocess
import sys
import tempfile
import types
import unittest
from pathlib import Path
from unittest.mock import patch

from scriptotar_sidecar import engine
from scriptotar_sidecar.errors import SidecarError


class SourceDiagnosticsTests(unittest.TestCase):
    def job(self):
        return {
            "id": "provider-fixture",
            "source": "https://www.youtube.com/watch?v=fixture",
            "quality": "720p",
            "cookies": "none",
            "max_duration_seconds": 60,
        }

    def fake_provider(self, fail_at: str):
        class DownloadError(Exception):
            pass

        class YoutubeDL:
            def __init__(self, options):
                self.options = options

            def __enter__(self):
                return self

            def __exit__(self, *_args):
                return False

            def extract_info(self, _url, *, download):
                if fail_at == "metadata":
                    raise DownloadError("provider denied this request")
                return {"title": "fixture", "duration": 20}

            def download(self, _urls):
                if fail_at == "download":
                    raise DownloadError("provider stream unavailable")
                return 1

        return types.SimpleNamespace(
            YoutubeDL=YoutubeDL,
            utils=types.SimpleNamespace(DownloadError=DownloadError),
        )

    def test_metadata_access_failure_is_not_mislabeled_as_whisper_engine_failure(self):
        with tempfile.TemporaryDirectory() as tmp:
            with patch.dict(sys.modules, {"yt_dlp": self.fake_provider("metadata")}):
                with self.assertRaises(SidecarError) as raised:
                    engine.download_url(self.job(), Path(tmp), lambda *_args: None)
        self.assertEqual(raised.exception.code, "SOURCE_ACCESS_FAILED")
        self.assertNotIn("model", raised.exception.message.lower())

    def test_download_access_failure_is_actionable(self):
        with tempfile.TemporaryDirectory() as tmp:
            with patch.dict(sys.modules, {"yt_dlp": self.fake_provider("download")}):
                with self.assertRaises(SidecarError) as raised:
                    engine.download_url(self.job(), Path(tmp), lambda *_args: None)
        self.assertEqual(raised.exception.code, "SOURCE_ACCESS_FAILED")

    def test_nonzero_downloader_status_is_not_success(self):
        with tempfile.TemporaryDirectory() as tmp:
            with patch.dict(sys.modules, {"yt_dlp": self.fake_provider("status")}):
                with self.assertRaises(SidecarError) as raised:
                    engine.download_url(self.job(), Path(tmp), lambda *_args: None)
        self.assertEqual(raised.exception.code, "SOURCE_ACCESS_FAILED")

    def test_invalid_duration_metadata_cannot_leak_nan_to_protocol(self):
        for duration in (float("nan"), float("inf"), float("-inf"), -1, 0):
            with self.subTest(duration=duration):
                metadata = engine._limited_metadata({"duration": duration}, "https://www.youtube.com")
                self.assertIsNone(metadata["duration_seconds"])

    def test_ffprobe_ignores_nonfinite_durations(self):
        for duration in ("nan", "inf", "0", "-7"):
            with self.subTest(duration=duration):
                completed = subprocess.CompletedProcess(["ffprobe"], 0, stdout=duration)
                with patch.object(engine.subprocess, "run", return_value=completed):
                    self.assertIsNone(engine.probe_duration(Path("/tmp/fake.mp4")))


if __name__ == "__main__":
    unittest.main()
