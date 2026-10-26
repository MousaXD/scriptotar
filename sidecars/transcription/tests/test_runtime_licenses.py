from __future__ import annotations

import unittest
from pathlib import Path

from runtime_licenses import LICENSE_METADATA_FALLBACKS, classify_ffmpeg_license, parse_pyav_ffmpeg_report


class RuntimeLicenseTests(unittest.TestCase):
    def test_tokenizers_requirement_matches_reviewed_license_fallback(self) -> None:
        reviewed = LICENSE_METADATA_FALLBACKS["tokenizers"]["version"]
        requirements = Path(__file__).resolve().parents[1] / "requirements-engine.txt"
        pinned = {
            line.split("==", 1)[0].strip(): line.split("==", 1)[1].strip()
            for line in requirements.read_text(encoding="utf-8").splitlines()
            if "==" in line and not line.strip().startswith("#")
        }
        self.assertEqual(pinned.get("tokenizers"), reviewed)

    def test_gpl_v3_build(self) -> None:
        self.assertEqual(
            classify_ffmpeg_license("configuration: --enable-gpl --enable-version3 --enable-libx264"),
            "GPL-3.0-or-later",
        )

    def test_gpl_v2_or_later_build(self) -> None:
        self.assertEqual(
            classify_ffmpeg_license("configuration: --enable-gpl --enable-libx264"),
            "GPL-2.0-or-later",
        )

    def test_lgpl_v3_build(self) -> None:
        self.assertEqual(
            classify_ffmpeg_license("configuration: --enable-version3 --enable-libvmaf"),
            "LGPL-3.0-or-later",
        )

    def test_default_lgpl_build(self) -> None:
        self.assertEqual(
            classify_ffmpeg_license("configuration: --disable-debug --enable-shared"),
            "LGPL-2.1-or-later",
        )

    def test_nonfree_build_is_rejected(self) -> None:
        with self.assertRaisesRegex(RuntimeError, "must not be redistributed"):
            classify_ffmpeg_license(
                "configuration: --enable-gpl --enable-version3 --enable-nonfree"
            )

    def test_pyav_lgpl_v3_report(self) -> None:
        output = """PyAV v18.0.0
library configuration: --disable-static --enable-shared --enable-version3
library license: LGPL version 3 or later
libavcodec     62. 11.100
libavformat    62.  3.100
"""
        report = parse_pyav_ffmpeg_report(output)
        self.assertEqual(report["pyav_version"], "18.0.0")
        groups = report["library_groups"]
        self.assertEqual(len(groups), 1)
        self.assertEqual(groups[0]["inferred_license"], "LGPL-3.0-or-later")
        self.assertEqual(len(groups[0]["libraries"]), 2)

    def test_pyav_gpl_drift_is_rejected(self) -> None:
        output = """PyAV v18.0.0
library configuration: --enable-gpl --enable-version3
library license: GPL version 3 or later
libavcodec     62. 11.100
"""
        with self.assertRaisesRegex(RuntimeError, "expected LGPL-3.0-or-later"):
            parse_pyav_ffmpeg_report(output)

    def test_pyav_nonfree_report_is_rejected(self) -> None:
        output = """PyAV v18.0.0
library configuration: --enable-version3 --enable-nonfree
library license: LGPL version 3 or later
libavcodec     62. 11.100
"""
        with self.assertRaisesRegex(RuntimeError, "must not be redistributed"):
            parse_pyav_ffmpeg_report(output)


if __name__ == "__main__":
    unittest.main()
