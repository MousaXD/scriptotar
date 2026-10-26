from __future__ import annotations

import ast
import unittest
from pathlib import Path

from runtime_licenses import LICENSE_METADATA_FALLBACKS, classify_ffmpeg_license, parse_pyav_ffmpeg_report


class RuntimeLicenseTests(unittest.TestCase):
    def test_curl_cffi_native_wheel_matches_reviewed_recipe(self) -> None:
        root = Path(__file__).resolve().parents[1]
        requirements = root / "requirements-engine.txt"
        declared = [
            line.strip().split("==", 1)[1]
            for line in requirements.read_text(encoding="utf-8").splitlines()
            if line.startswith("curl-cffi==")
        ]
        # Parse the compliance constants without importing bundler-only
        # dependencies (static_ffmpeg must not be needed by sidecar unit tests).
        source = ast.parse((root / "distribution_compliance.py").read_text(encoding="utf-8"))
        assignments = {
            target.id: ast.literal_eval(node.value)
            for node in source.body
            if isinstance(node, ast.Assign)
            for target in node.targets
            if isinstance(target, ast.Name)
            and target.id in {"CURL_CFFI_VERSION", "CURL_CFFI_NATIVE_RECIPE"}
        }
        reviewed = assignments["CURL_CFFI_VERSION"]
        self.assertEqual(declared, [reviewed])
        self.assertEqual(
            assignments["CURL_CFFI_NATIVE_RECIPE"]["curl_cffi_source"],
            f"https://github.com/lexiforest/curl_cffi/tree/v{reviewed}",
        )

    def test_ytdlp_bundle_label_matches_locked_runtime_requirement(self) -> None:
        requirement = Path(__file__).resolve().parents[1] / "requirements-engine.txt"
        source = Path(__file__).resolve().parents[1] / "build_runtime.py"
        pinned = [line.strip() for line in requirement.read_text().splitlines() if line.startswith("yt-dlp[")]
        self.assertEqual(pinned, ["yt-dlp[default,curl-cffi]==2026.10.21"])
        self.assertIn("yt-dlp=2026.10.21 (dedicated scriptotar-ytdlp executable)", source.read_text())

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
