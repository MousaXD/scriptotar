"""Agent10 external negative regressions on Agent07 revised raw ZIP pre-index walk.

No native installed testing; only standard-library synthetic archives.
"""
import struct
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch
import zipfile

import visual_observations
from test_visual_observations import SHA, fixture_zip


class Agent10RawZipPreindexTests(unittest.TestCase):
    def build(self, path):
        fixture_zip(path, "windows")
        with zipfile.ZipFile(path, "a", compression=zipfile.ZIP_STORED) as archive:
            for i in range(129):
                archive.writestr(f"extra-{i:03}.txt", b"x")
        data = bytearray(path.read_bytes())
        eocd = data.rfind(b"PK\x05\x06")
        self.assertGreater(eocd, 0)
        actual_count = struct.unpack_from("<H", data, eocd + 10)[0]
        self.assertEqual(actual_count, 137)
        return data, eocd

    def test_underreported_directory_counts_denied_before_zipfile_index(self):
        with tempfile.TemporaryDirectory() as temp:
            path = Path(temp) / "forged.zip"
            original, end = self.build(path)
            for claimed in (1, 2, 127, 128):
                with self.subTest(claimed=claimed):
                    data = bytearray(original)
                    struct.pack_into("<HH", data, end + 8, claimed, claimed)
                    path.write_bytes(data)
                    # Constructor MUST not allocate any ZipInfo. In this
                    # test an attempted call raises instead of indexing.
                    with patch.object(
                        visual_observations.zipfile, "ZipFile",
                        side_effect=AssertionError("ZipFile allocated before actual-count check"),
                    ):
                        with self.assertRaisesRegex(
                            ValueError, "central directory claimed count differs"
                        ):
                            visual_observations.analyze(path, SHA)
            print("AGENT10_RAW_DIRECTORY_UNDERCOUNT_REJECTED_BEFORE_ZIPFILE")

    def test_honest_oversized_count_denied_before_zipfile_index(self):
        with tempfile.TemporaryDirectory() as temp:
            path = Path(temp) / "oversized.zip"
            self.build(path)
            with patch.object(
                visual_observations.zipfile, "ZipFile",
                side_effect=AssertionError("ZipFile allocated on rejected oversized count"),
            ):
                with self.assertRaisesRegex(ValueError, "excessive native ZIP directory"):
                    visual_observations.analyze(path, SHA)

    def test_raw_directory_length_and_local_signature_corruption_denied(self):
        with tempfile.TemporaryDirectory() as temp:
            path = Path(temp) / "corruption.zip"
            fixture_zip(path, "windows")
            data = path.read_bytes()
            eocd = data.rfind(b"PK\x05\x06")
            offset = struct.unpack_from("<I", data, eocd + 16)[0]
            for bad in ("short_directory", "bad_central", "bad_local"):
                with self.subTest(bad=bad):
                    altered = bytearray(data)
                    if bad == "short_directory":
                        struct.pack_into("<I", altered, eocd + 12, 1)
                    elif bad == "bad_central":
                        altered[offset:offset + 4] = b"FAKE"
                    else:
                        altered[:4] = b"FAKE"
                    path.write_bytes(altered)
                    with patch.object(
                        visual_observations.zipfile, "ZipFile",
                        side_effect=AssertionError("ZipFile called on corrupt metadata"),
                    ):
                        with self.assertRaisesRegex(ValueError, "native ZIP|Native ZIP"):
                            visual_observations.analyze(path, SHA)

    def test_legitimate_eight_member_native_fixture_still_loads(self):
        with tempfile.TemporaryDirectory() as temp:
            path = Path(temp) / "legitimate.zip"
            fixture_zip(path, "windows")
            result = visual_observations.analyze(path, SHA)
            self.assertEqual(result["source_sha"], SHA)
            self.assertEqual(len(result["screenshots"]), 7)
            self.assertEqual(result["light_dark_visual_certification"], "NOT TESTED")
            self.assertEqual(result["arabic_rtl_certification"], "NOT TESTED")


if __name__ == "__main__":
    unittest.main()
