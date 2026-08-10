from __future__ import annotations

import unittest

from runtime_licenses import classify_ffmpeg_license


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


if __name__ == "__main__":
    unittest.main()
