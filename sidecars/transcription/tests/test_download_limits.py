from __future__ import annotations

import sys
import tempfile
import types
import unittest
from pathlib import Path
from unittest.mock import patch

from scriptotar_sidecar import engine
from scriptotar_sidecar.errors import SidecarError


class DownloadLimitTests(unittest.TestCase):
    def run_fixture(self, *, declared_duration: float | None, measured_duration: float | None, media: bytes):
        class FakeYoutubeDL:
            def __init__(self, options):
                self.options = options

            def __enter__(self):
                return self

            def __exit__(self, *_args):
                return False

            def extract_info(self, _url, *, download):
                assert not download
                return {"title": "fixture", "duration": declared_duration}

            def download(self, _urls):
                output = Path(self.options["outtmpl"].replace("%(ext)s", "mp4"))
                output.write_bytes(media)
                return 0

        job = {
            "id": "duration-job",
            "source": "https://www.youtube.com/watch?v=fixture",
            "quality": "720p",
            "cookies": "none",
            "max_duration_seconds": 30,
        }
        with tempfile.TemporaryDirectory() as tmp:
            with (
                patch.dict(sys.modules, {"yt_dlp": types.SimpleNamespace(YoutubeDL=FakeYoutubeDL)}),
                patch.object(engine, "probe_duration", return_value=measured_duration),
            ):
                return engine.download_url(job, Path(tmp), lambda *_args: None)

    def test_missing_provider_duration_is_checked_after_download(self):
        with self.assertRaises(SidecarError) as raised:
            self.run_fixture(declared_duration=None, measured_duration=120, media=b"valid media")
        self.assertEqual(raised.exception.code, "DURATION_LIMIT")

    def test_underreported_provider_duration_cannot_bypass_limit(self):
        with self.assertRaises(SidecarError) as raised:
            self.run_fixture(declared_duration=10, measured_duration=120, media=b"valid media")
        self.assertEqual(raised.exception.code, "DURATION_LIMIT")

    def test_zero_length_download_cannot_be_successful(self):
        with self.assertRaises(SidecarError) as raised:
            self.run_fixture(declared_duration=10, measured_duration=None, media=b"")
        self.assertEqual(raised.exception.code, "DOWNLOAD_OUTPUT_MISSING")

    def test_media_within_limit_is_returned_and_actual_duration_reported(self):
        # TemporaryDirectory cleanup is inside run_fixture; only metadata survives.
        # Verify the measured duration replaces a misleading extractor estimate.
        _, metadata = self.run_fixture(declared_duration=25, measured_duration=20, media=b"media")
        self.assertEqual(metadata["duration_seconds"], 20)


if __name__ == "__main__":
    unittest.main()
