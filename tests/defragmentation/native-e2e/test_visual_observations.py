"""Pure-stdlib unit tests. Fixtures are synthetic PNGs, NEVER installed QA PASS."""
import json
import struct
import tempfile
import unittest
from unittest.mock import patch
import zipfile
import zlib
from pathlib import Path

from visual_observations import analyze, sample_png_rgb

SHA = "a" * 40


def make_png(rgb, *, mode=2, width=451, height=201):
    channels = 4 if mode == 6 else 3
    pixel = bytes(rgb) + (b"\xff" if mode == 6 else b"")
    raw = (b"\x00" + pixel * width) * height

    def chunk(kind, value):
        return (struct.pack(">I", len(value)) + kind + value +
                struct.pack(">I", zlib.crc32(kind + value) & 0xffffffff))

    return (b"\x89PNG\r\n\x1a\n" +
            chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, mode, 0, 0, 0)) +
            chunk(b"IDAT", zlib.compress(raw)) + chunk(b"IEND", b""))


def fixture_zip(path, platform="windows", *, source=SHA, omit=None,
                dark=(13, 17, 22), option=True):
    prefix = platform == "windows"
    themes = ([f"windows-theme-{x}.png" for x in
               ("system", "light", "dark", "dark-reopened")] if prefix else
              ["system-theme.png", "light-theme.png", "dark-theme.png",
               "dark-theme-reopened.png"])
    arabic = (["windows-arabic-locale.png", "windows-arabic-settings.png",
               "windows-arabic-queue.png"] if prefix else
              ["arabic-locale.png", "arabic-settings.png", "arabic-queue.png"])
    report = {
        "scope": "native-installed",
        "source_sha": source,
        "run_url": "https://github.com/MousaXD/scriptotar/actions/runs/44368820544",
        "platform": ("Windows WebView2 / UI Automation" if prefix else
                     "Linux WebKitGTK / AT-SPI"),
        "native_ui_probe": "installed-webview-automation",
        "package_sha256": "b" * 64,
        "executable_sha256": "c" * 64,
        "cases": {
            "installed_webview": {"status": "PASS"},
            "theme_mode_control": {"status": "PASS"},
            "arabic_locale_control": {"status": "PASS"},
            "light_dark_visual": {"status": "NOT TESTED"},
            "arabic_rtl": {"status": "NOT TESTED"},
        },
        "visual_capture": {"explicit_light_option": option},
        "verdict": "NOT TESTED",
    }
    with zipfile.ZipFile(path, "w") as archive:
        archive.writestr("result.json", json.dumps(report))
        for i, name in enumerate(themes + arabic):
            if name != omit:
                archive.writestr(name, make_png(
                    dark if i in (2, 3) else (245, 247, 248),
                    mode=6 if prefix else 2,
                ))


class NativeVisualNoncertifyingTests(unittest.TestCase):
    def test_png_rgb_rgba_filters_and_corruption(self):
        for mode in (2, 6):
            rgb, size = sample_png_rgb(make_png((13, 17, 22), mode=mode))
            self.assertEqual(rgb, (13, 17, 22))
            self.assertEqual(size, (451, 201))
        with self.assertRaises(ValueError):
            sample_png_rgb(make_png((13, 17, 22))[:-6])

    def test_png_inflate_budget_rejects_overlong_stream_without_unbounded_flush(self):
        # Synthetic input is intentionally compact (<20 KiB compressed). It
        # claims a 451x201 PNG but expands another 12 MiB after the valid rows.
        # No original installer/screenshot/source archive is read or trusted.
        width, height = 451, 201
        stride = width * 3 + 1
        expected_scan = stride * height
        original_scan = (b"\x00" + b"\x00\x00\x00" * width) * height
        self.assertEqual(len(original_scan), expected_scan)

        def chunk(kind, payload):
            return (struct.pack(">I", len(payload)) + kind + payload +
                    struct.pack(">I", zlib.crc32(kind + payload) & 0xffffffff))

        def mk_compressed(data):
            return (b"\x89PNG\r\n\x1a\n" +
                    chunk(b"IHDR", struct.pack(">IIBBBBB", width, height,
                                                8, 2, 0, 0, 0)) +
                    chunk(b"IDAT", data) + chunk(b"IEND", b""))

        bomb = mk_compressed(zlib.compress(original_scan + b"\x00" * (12 * 1024 * 1024), 9))
        self.assertLess(len(bomb), 20_000)
        import visual_observations as decoder
        original_decompressobj = zlib.decompressobj
        observations = {"flush_calls": 0, "max_decompress_bytes": 0,
                        "flush_materialized_bytes": 0}

        class CappedAuditInflater:
            def __init__(self, *args, **kwargs):
                self.actual = original_decompressobj(*args, **kwargs)

            def __getattr__(self, name):
                return getattr(self.actual, name)

            def decompress(self, data, max_length=0):
                value = self.actual.decompress(data, max_length)
                observations["max_decompress_bytes"] = max(
                    observations["max_decompress_bytes"], len(value))
                return value

            def flush(self, *args, **kwargs):
                observations["flush_calls"] += 1
                value = self.actual.flush(*args, **kwargs)
                observations["flush_materialized_bytes"] += len(value)
                return value

        with patch.object(decoder.zlib, "decompressobj",
                          side_effect=lambda *a, **k: CappedAuditInflater(*a, **k)):
            with self.assertRaisesRegex(ValueError, "PNG scanline length mismatch"):
                sample_png_rgb(bomb)
        self.assertEqual(observations["flush_calls"], 0,
                         "No unbounded zlib flush is safe for untrusted IDAT")
        self.assertEqual(observations["flush_materialized_bytes"], 0)
        self.assertLessEqual(observations["max_decompress_bytes"], expected_scan + 1)

    def test_png_rejects_truncated_trailing_and_overlong_zlib_streams(self):
        width, height = 451, 201
        raw = (b"\x00" + b"\x00\x00\x00" * width) * height

        def chunk(kind, payload):
            return (struct.pack(">I", len(payload)) + kind + payload +
                    struct.pack(">I", zlib.crc32(kind + payload) & 0xffffffff))

        def mk_png(payload):
            return (b"\x89PNG\r\n\x1a\n" +
                    chunk(b"IHDR", struct.pack(">IIBBBBB", width, height,
                                                8, 2, 0, 0, 0)) +
                    chunk(b"IDAT", payload) + chunk(b"IEND", b""))

        good = zlib.compress(raw, 9)
        self.assertEqual(sample_png_rgb(mk_png(good))[1], (width, height))
        # Recompute IDAT CRC so these truly test the zlib stream boundary,
        # not the earlier, unrelated PNG chunk CRC defense.
        for malicious in (
            good[:-1],                   # missing stream checksum trailer
            good + zlib.compress(b"x"), # appended second zlib stream
            good + b"trailing garbage", # unused compressed bytes
            zlib.compress(raw + b"x"),  # valid stream, too much decoded data
        ):
            with self.subTest(malicious_bytes=len(malicious)):
                with self.assertRaises(ValueError):
                    sample_png_rgb(mk_png(malicious))

    def test_theme_observations_never_certify_full_visual_or_rtl(self):
        with tempfile.TemporaryDirectory() as temp:
            for platform in ("windows", "linux"):
                path = Path(temp) / f"{platform}.zip"
                fixture_zip(path, platform)
                evidence = analyze(path, SHA)
                self.assertTrue(evidence["light_dark_and_persisted_dark_observed"])
                self.assertTrue(evidence["arabic_screen_captures_present"])
                self.assertEqual(evidence["native_16_case_verdict_unchanged"], "NOT TESTED")
                self.assertEqual(evidence["light_dark_visual_certification"], "NOT TESTED")
                self.assertEqual(evidence["arabic_rtl_certification"], "NOT TESTED")
                self.assertEqual(len(evidence["screenshots"]), 7)

    def test_reject_source_mismatch_missing_evidence_and_false_theme(self):
        with tempfile.TemporaryDirectory() as temp:
            path = Path(temp) / "sample.zip"
            fixture_zip(path, source="f" * 40)
            with self.assertRaisesRegex(ValueError, "identity"):
                analyze(path, SHA)
            fixture_zip(path, omit="windows-arabic-settings.png")
            with self.assertRaisesRegex(ValueError, "Required native screenshot"):
                analyze(path, SHA)
            fixture_zip(path, option=False)
            self.assertFalse(analyze(path, SHA)["light_dark_and_persisted_dark_observed"])
            fixture_zip(path, dark=(245, 247, 248))
            self.assertFalse(analyze(path, SHA)["light_dark_and_persisted_dark_observed"])


if __name__ == "__main__":
    unittest.main()
