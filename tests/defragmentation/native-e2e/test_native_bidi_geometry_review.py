"""Synthetic-only rejection tests; never substitute for installed native proof."""
from __future__ import annotations

import json
import tempfile
import unittest
import zipfile
from pathlib import Path

from native_bidi_geometry_review import review
from test_visual_observations import SHA, fixture_zip


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
    return {"arabic_studio": arabic, "arabic_queue": arabic,
            "arabic_settings": arabic,
            "theme_system": {"matches": [nav("Settings", english_x)]}}


def set_layout(path, layouts):
    with zipfile.ZipFile(path) as z:
        entries = {name: z.read(name) for name in z.namelist()}
    report = json.loads(entries["result.json"])
    report["native_layout_observations"] = layouts
    entries["result.json"] = json.dumps(report).encode()
    with zipfile.ZipFile(path, "w") as z:
        for name, payload in entries.items():
            z.writestr(name, payload)


class NativeBidiGeometryReviewTests(unittest.TestCase):
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
