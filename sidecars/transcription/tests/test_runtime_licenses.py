from __future__ import annotations

import unittest

from runtime_licenses import classify_ffmpeg_license, parse_pyav_ffmpeg_report


class RuntimeLicenseTests(unittest.TestCase):
    def test_gpl_v3_build(self) -> None:
        output = "configuration: --enable-gpl --enable-version3 --enable-libx264"
        self.assertEqual(classify_ffmpeg_license(output), "GPL-3.0-or-later")

    def test_gpl_v2_or_later_build(self) -> None:
        output = "configuration: --enable-gpl --enable-libx264"
        self.assertEqual(classify_ffmpeg_license(output), "GPL-2.0-or-later")

    def test_lgpl_v3_build(self) -> None:
        output = "configuration: --enable-version3 --enable-libvmaf"
        self.assertEqual(classify_ffmpeg_license(output), "LGPL-3.0-or-later")

    def test_default_lgpl_build(self) -> None:
        output = "configuration: --disable-debug --enable-shared"
        self.assertEqual(classify_ffmpeg_license(output), "LGPL-2.1-or-later")

    def test_nonfree_build_is_rejected(self) -> None:
        output = "configuration: --enable-gpl --enable-version3 --enable-nonfree"
        with self.assertRaisesRegex(RuntimeError, "must not be redistributed"):
            classify_ffmpeg_license(output)

    def test_pyav_ffmpeg_gpl_v3_report(self) -> None:
        output = """PyAV v18.0.0
library configuration: --disable-static --enable-shared --enable-gpl --enable-version3 --enable-libx264
library license: GPL version 3 or later
libavcodec     62. 11.100
libavformat    62.  3.100
"""
        report = parse_pyav_ffmpeg_report(output)
        self.assertEqual(report["pyav_version"], "18.0.0")
        groups = report["library_groups"]
        self.assertEqual(len(groups), 1)
        self.assertEqual(groups[0]["inferred_license"], "GPL-3.0-or-later")
        self.assertEqual(len(groups[0]["libraries"]), 2)

    def test_pyav_nonfree_report_is_rejected(self) -> None:
        output = """PyAV v18.0.0
library configuration: --enable-gpl --enable-version3 --enable-nonfree
library license: GPL version 3 or later
libavcodec     62. 11.100
"""
        with self.assertRaisesRegex(RuntimeError, "must not be redistributed"):
            parse_pyav_ffmpeg_report(output)


if __name__ == "__main__":
    unittest.main()
