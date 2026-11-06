from __future__ import annotations

import hashlib
import importlib.util
import io
import sys
import tempfile
import types
import unittest
import zipfile
from pathlib import Path
from unittest.mock import call, patch


class StaticFFmpegArchiveRetryTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        # The sidecar unit workflow intentionally installs no packaging extras.
        # Stub only the static_ffmpeg URL resolver, not the verified downloader.
        resolver = types.ModuleType("static_ffmpeg")
        resolver.run = types.SimpleNamespace(
            get_platform_key=lambda: "linux",
            get_platform_http_zip=lambda: "https://example.test/reviewed/linux.zip",
        )
        source = Path(__file__).resolve().parents[1] / "distribution_compliance.py"
        spec = importlib.util.spec_from_file_location("_scriptotar_distribution_test", source)
        assert spec is not None and spec.loader is not None
        module = importlib.util.module_from_spec(spec)
        with patch.dict(sys.modules, {"static_ffmpeg": resolver}):
            spec.loader.exec_module(module)
        cls.distribution = module

    def setUp(self):
        data = io.BytesIO()
        with zipfile.ZipFile(data, "w", compression=zipfile.ZIP_STORED) as output:
            output.writestr("linux/ffmpeg", b"reviewed-ffmpeg-fixture")
            output.writestr("linux/ffprobe", b"reviewed-ffprobe-fixture")
        self.reviewed = data.getvalue()
        self.spec = {
            "url": "https://example.test/reviewed/linux.zip",
            "sha256": hashlib.sha256(self.reviewed).hexdigest(),
            "size": len(self.reviewed),
            "git_lfs_pointer_blob": "fixture",
            "upstream_provider": "fixture",
            "upstream_provider_url": "https://example.test/",
            "upstream_selection": "fixture",
        }

    def test_transient_wrong_bytes_retries_same_url_and_requires_reviewed_digest(self):
        with tempfile.TemporaryDirectory() as tmp, (
            patch.object(self.distribution, "STATIC_FFMPEG_ARCHIVES", {"linux": self.spec}),
            patch.object(self.distribution.urllib.request, "urlopen", side_effect=[
                io.BytesIO(b"bad"), io.BytesIO(self.reviewed),
            ]) as urlopen,
            patch.object(self.distribution.time, "sleep") as sleep,
        ):
            ffmpeg, ffprobe, evidence = self.distribution.fetch_pinned_static_ffmpeg(Path(tmp))
            self.assertEqual(ffmpeg.read_bytes(), b"reviewed-ffmpeg-fixture")
            self.assertEqual(ffprobe.read_bytes(), b"reviewed-ffprobe-fixture")
            self.assertEqual(evidence["archive_sha256"], self.spec["sha256"])
            self.assertEqual(evidence["archive_size"], self.spec["size"])
            self.assertTrue(evidence["checksum_verified"])
        self.assertEqual(urlopen.call_count, 2)
        self.assertEqual(
            [item.args[0] for item in urlopen.call_args_list],
            [self.spec["url"], self.spec["url"]],
        )
        sleep.assert_called_once_with(1)

    def test_persistent_wrong_bytes_remain_hard_failure_and_delete_archive(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            with (
                patch.object(self.distribution, "STATIC_FFMPEG_ARCHIVES", {"linux": self.spec}),
                patch.object(self.distribution.urllib.request, "urlopen", side_effect=[
                    io.BytesIO(b"bad"), io.BytesIO(b"bad"), io.BytesIO(b"bad"),
                ]) as urlopen,
                patch.object(self.distribution.time, "sleep") as sleep,
            ):
                with self.assertRaisesRegex(RuntimeError, "checksum/size drift"):
                    self.distribution.fetch_pinned_static_ffmpeg(root)
                self.assertEqual(urlopen.call_count, 3)
                self.assertEqual(sleep.call_args_list, [call(1), call(2)])
            self.assertFalse((root / "linux.zip").exists())
            self.assertFalse((root / "extracted").exists())

    def test_oversized_archive_fails_immediately_without_retry(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            with (
                patch.object(self.distribution, "STATIC_FFMPEG_ARCHIVES", {"linux": self.spec}),
                patch.object(self.distribution.urllib.request, "urlopen",
                             return_value=io.BytesIO(b"x" * (self.spec["size"] + 1))) as urlopen,
                patch.object(self.distribution.time, "sleep") as sleep,
            ):
                with self.assertRaisesRegex(RuntimeError, "size drift"):
                    self.distribution.fetch_pinned_static_ffmpeg(root)
                urlopen.assert_called_once()
                sleep.assert_not_called()
            self.assertFalse((root / "linux.zip").exists())

    def test_resolver_url_drift_remains_fail_closed(self):
        with tempfile.TemporaryDirectory() as tmp, (
            patch.object(self.distribution, "STATIC_FFMPEG_ARCHIVES", {"linux": self.spec}),
            patch.object(self.distribution.static_ffmpeg_run, "get_platform_http_zip",
                         return_value="https://different.test/other.zip"),
            patch.object(self.distribution.urllib.request, "urlopen") as urlopen,
        ):
            with self.assertRaisesRegex(RuntimeError, "provider URL drifted"):
                self.distribution.fetch_pinned_static_ffmpeg(Path(tmp))
            urlopen.assert_not_called()


if __name__ == "__main__":
    unittest.main()
