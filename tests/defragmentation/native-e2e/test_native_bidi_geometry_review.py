"""Synthetic-only rejection tests; never substitute for installed native proof."""
from __future__ import annotations

import hashlib
import json
import tempfile
import unittest
from unittest.mock import patch
import zipfile
from pathlib import Path

from native_bidi_geometry_review import review
from test_visual_observations import SHA, fixture_zip, make_png


def source_layout(linux: bool, arabic_x: int, english_x: int) -> dict:
    role = "push button" if linux else "ControlType.Button"

    def nav(name, x):
        return {"name": name, "role": role, "focusable": True,
                "showing": True, "offscreen": False, "nonpositive_bounds": False,
                "bounds": {"x": x, "y": 100, "width": 40, "height": 40}}

    arabic = {"missing_labels": [], "certifies_rtl_or_visual": False,
              "labels_requested": ["الاستوديو", "الطابور", "الإعدادات", "لغة الواجهة"],
              "matches": [nav("الاستوديو", arabic_x), nav("الطابور", arabic_x),
                          nav("الإعدادات", arabic_x)]}
    return {"arabic_studio": json.loads(json.dumps(arabic)),
            "arabic_queue": json.loads(json.dumps(arabic)),
            "arabic_settings": json.loads(json.dumps(arabic)),
            "theme_system": {"matches": [nav("Settings", english_x)]}}


def set_layout(path, layouts):
    with zipfile.ZipFile(path) as z:
        entries = {name: z.read(name) for name in z.namelist()}
    report = json.loads(entries["result.json"])
    report["native_layout_observations"] = layouts
    entries["result.json"] = json.dumps(report).encode()
    linux = str(report["platform"]).lower().startswith("linux")
    for index, label in enumerate(("arabic-locale.png", "arabic-queue.png", "arabic-settings.png")
                                  if linux else ("windows-arabic-locale.png", "windows-arabic-queue.png", "windows-arabic-settings.png")):
        entries[label] = make_png((245 - index, 247 - index, 248 - index),
                                  mode=2 if linux else 6)
    with zipfile.ZipFile(path, "w") as z:
        for name, payload in entries.items():
            z.writestr(name, payload)


class NativeBidiGeometryReviewTests(unittest.TestCase):
    def test_reject_duplicate_json_provenance_and_nested_status(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "windows.zip"
            for mutation in ("root", "escaped", "nested"):
                fixture_zip(path)
                set_layout(path, source_layout(False, 400, 10))
                with zipfile.ZipFile(path) as z:
                    entries = {name: z.read(name) for name in z.namelist()}
                report = entries["result.json"].decode("utf-8")
                if mutation == "root":
                    report = report.replace('"source_sha":', '"source_sha": "forged", "source_sha":', 1)
                elif mutation == "escaped":
                    # The JSON escape decodes to exactly the existing key.
                    report = report.replace('"source_sha":', '"source\\u005fsha": "forged", "source_sha":', 1)
                else:
                    report = report.replace('"status": "PASS"',
                                            '"status": "NOT TESTED", "status": "PASS"', 1)
                entries["result.json"] = report.encode("utf-8")
                with zipfile.ZipFile(path, "w") as z:
                    for name, payload in entries.items():
                        z.writestr(name, payload)
                with self.subTest(mutation=mutation):
                    with self.assertRaisesRegex(ValueError, "Duplicate JSON member"):
                        review(path, SHA)

    def test_archive_digest_is_from_the_only_parsed_byte_snapshot(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "windows.zip"
            fixture_zip(path)
            set_layout(path, source_layout(False, 400, 10))
            expected = hashlib.sha256(path.read_bytes()).hexdigest()
            # Old behavior parsed ZipFile(path) and then re-opened the
            # mutable path for its checksum. That made archive identity
            # vulnerable to a path-swap race; the reviewer must not do it.
            with patch.object(Path, "read_bytes", side_effect=AssertionError(
                    "Evidence path must not be reopened for the final digest")):
                proof = review(path, SHA)
            self.assertEqual(proof["archive_sha256"], expected)
            self.assertTrue(proof["right_to_left_navigation_rail_observed"])
            self.assertEqual(proof["arabic_rtl_certification"], "NOT TESTED")

    def test_mirrored_rail_is_only_a_noncertifying_observation(self):
        with tempfile.TemporaryDirectory() as tmp:
            for platform in ("linux", "windows"):
                path = Path(tmp) / f"{platform}.zip"
                fixture_zip(path, platform=platform)
                set_layout(path, source_layout(platform == "linux", 400, 10))
                result = review(path, SHA)
                self.assertTrue(result["right_to_left_navigation_rail_observed"])
                self.assertEqual(result["arabic_rtl_certification"], "NOT TESTED")
                self.assertEqual(result["appearance_accessibility_certification"], "NOT TESTED")
                self.assertEqual(result["native_16_case_verdict_unchanged"], "NOT TESTED")

    def test_reject_wrong_side_missing_semantics_and_source(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "windows.zip"
            fixture_zip(path)
            set_layout(path, source_layout(False, 10, 400))
            self.assertFalse(review(path, SHA)["right_to_left_navigation_rail_observed"])
            with self.assertRaisesRegex(ValueError, "identity"):
                review(path, "b" * 40)
            layout = source_layout(False, 400, 10)
            layout["arabic_settings"]["missing_labels"] = ["الإعدادات"]
            set_layout(path, layout)
            with self.assertRaisesRegex(ValueError, "Incomplete native Arabic"):
                review(path, SHA)

    def test_mirroring_requires_all_three_buttons_on_every_arabic_page(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "windows.zip"
            fixture_zip(path)
            layout = source_layout(False, 400, 10)
            layout["arabic_queue"]["matches"][1]["bounds"]["x"] = 10
            set_layout(path, layout)
            result = review(path, SHA)
            self.assertFalse(result["right_to_left_navigation_rail_observed"])
            self.assertEqual(result["arabic_navigation_centers_by_page"]["arabic_queue"]["الطابور"],
                             round(30 / result["display_dimensions"][0], 4))
            self.assertEqual(result["arabic_rtl_certification"], "NOT TESTED")

    def test_reject_offscreen_queue_nav_and_replayed_arabic_page(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "windows.zip"
            fixture_zip(path)
            layout = source_layout(False, 400, 10)
            layout["arabic_queue"]["matches"][0]["bounds"]["x"] = 500
            set_layout(path, layout)
            with self.assertRaisesRegex(ValueError, "outside screenshot"):
                review(path, SHA)
            set_layout(path, source_layout(False, 400, 10))
            with zipfile.ZipFile(path) as z:
                entries = {name: z.read(name) for name in z.namelist()}
            entries["windows-arabic-queue.png"] = entries["windows-arabic-locale.png"]
            with zipfile.ZipFile(path, "w") as z:
                for name, payload in entries.items():
                    z.writestr(name, payload)
            with self.assertRaisesRegex(ValueError, "Duplicate Arabic screen capture"):
                review(path, SHA)

    def test_reject_oversized_member_and_path_traversal(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "windows.zip"
            fixture_zip(path)
            set_layout(path, source_layout(False, 400, 10))
            with zipfile.ZipFile(path, "a") as z:
                z.writestr("../injected.txt", b"never extracted")
            with self.assertRaisesRegex(ValueError, "Unsafe or oversized"):
                review(path, SHA)
            fixture_zip(path)
            set_layout(path, source_layout(False, 400, 10))
            with zipfile.ZipFile(path, "a", compression=zipfile.ZIP_DEFLATED) as z:
                z.writestr("oversized.txt", b"x" * (16 * 1024 * 1024 + 1))
            with self.assertRaisesRegex(ValueError, "Unsafe or oversized"):
                review(path, SHA)

    def test_reject_nonfinite_or_negative_native_coordinates(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "windows.zip"
            fixture_zip(path)
            layout = source_layout(False, 400, 10)
            layout["arabic_settings"]["matches"][2]["bounds"]["x"] = float("nan")
            set_layout(path, layout)
            with self.assertRaisesRegex(ValueError, "Missing numeric native bounds"):
                review(path, SHA)
            layout = source_layout(False, 400, 10)
            layout["arabic_settings"]["matches"][2]["bounds"]["y"] = -1
            set_layout(path, layout)
            with self.assertRaisesRegex(ValueError, "Invalid native navigation bounds"):
                review(path, SHA)

    def test_reject_unproven_native_locale_before_reading_geometry(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "windows.zip"
            fixture_zip(path)
            set_layout(path, source_layout(False, 400, 10))
            with zipfile.ZipFile(path) as z:
                entries = {name: z.read(name) for name in z.namelist()}
            report = json.loads(entries["result.json"])
            report["cases"]["arabic_locale_control"]["status"] = "NOT TESTED"
            entries["result.json"] = json.dumps(report).encode()
            with zipfile.ZipFile(path, "w") as z:
                for name, payload in entries.items():
                    z.writestr(name, payload)
            with self.assertRaisesRegex(ValueError, "prerequisites are not PASS"):
                review(path, SHA)

    def test_reject_offscreen_or_unnamed_nav(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "linux.zip"
            fixture_zip(path, platform="linux")
            layout = source_layout(True, 400, 10)
            layout["arabic_settings"]["matches"][2]["showing"] = False
            set_layout(path, layout)
            with self.assertRaisesRegex(ValueError, "Missing or ambiguous"):
                review(path, SHA)


if __name__ == "__main__":
    unittest.main()
