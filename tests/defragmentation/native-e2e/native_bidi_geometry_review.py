#!/usr/bin/env python3
"""Read-only Arabic rail mirroring audit on original installed QA ZIPs.

Only records native AT-SPI/UIA coordinates and archived screenshot identities;
never assigns a full RTL, appearance, or 16-case PASS. No UI or SQLite writes.
"""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import re
import zipfile

from visual_observations import sample_png_rgb

SHA = re.compile(r"[0-9a-f]{40}\Z")
RUN = re.compile(r"https://github\.com/MousaXD/scriptotar/actions/runs/[1-9][0-9]*\Z")
ARABIC_NAV = ("الاستوديو", "الطابور", "الإعدادات")


def _nav(layout: dict, name: str, *, linux: bool) -> dict:
    expected_role = "push button" if linux else "ControlType.Button"
    matches = [item for item in layout.get("matches", [])
               if item.get("name") == name and item.get("role") == expected_role
               and item.get("focusable") is True
               and item.get("nonpositive_bounds") is False
               and (item.get("showing") is True if linux
                    else item.get("offscreen") is False)]
    if len(matches) != 1:
        raise ValueError(f"Missing or ambiguous native accessible navigation button: {name}")
    bounds = matches[0].get("bounds", {})
    if any(type(bounds.get(field)) not in (int, float) for field in
           ("x", "y", "width", "height")):
        raise ValueError("Missing numeric native bounds")
    if bounds["width"] <= 0 or bounds["height"] <= 0 or bounds["x"] < 0:
        raise ValueError("Invalid native navigation bounds")
    return bounds


def review(archive: Path, exact_sha: str) -> dict:
    if not SHA.fullmatch(exact_sha):
        raise ValueError("Expected exact source SHA")
    with zipfile.ZipFile(archive) as z:
        names = z.namelist()
        if len(names) != len(set(names)) or "result.json" not in names:
            raise ValueError("Duplicate or missing original evidence entry")
        r = json.loads(z.read("result.json"))
        if (r.get("scope") != "native-installed" or
                r.get("native_ui_probe") != "installed-webview-automation" or
                r.get("source_sha") != exact_sha or
                not RUN.fullmatch(str(r.get("run_url")))):
            raise ValueError("Installed evidence identity mismatch")
        cases = r.get("cases", {})
        if any(not isinstance(cases.get(key), dict) or
               cases[key].get("status") != "PASS"
               for key in ("installed_webview", "arabic_locale_control",
                           "theme_mode_control")):
            raise ValueError("Installed native locale/appearance prerequisites are not PASS")
        for key in ("package_sha256", "executable_sha256"):
            if not re.fullmatch(r"[0-9a-f]{64}", str(r.get(key))):
                raise ValueError("Missing hashed installed package/executable identity")
        platform = str(r.get("platform", "")).lower()
        if platform.startswith("linux"):
            linux = True
            arabic_name, english_name = "arabic-settings.png", "system-theme.png"
        elif platform.startswith("windows"):
            linux = False
            arabic_name, english_name = "windows-arabic-settings.png", "windows-theme-system.png"
        else:
            raise ValueError("Unknown native platform")
        if arabic_name not in names or english_name not in names:
            raise ValueError("Original Arabic or English screenshot missing")
        ar_image, en_image = z.read(arabic_name), z.read(english_name)
        _, ar_dimensions = sample_png_rgb(ar_image)
        _, en_dimensions = sample_png_rgb(en_image)
        if ar_dimensions != en_dimensions:
            raise ValueError("Inconsistent installed display dimensions")
        width, height = ar_dimensions
        layouts = r.get("native_layout_observations", {})
        for page in ("arabic_studio", "arabic_queue", "arabic_settings"):
            layout = layouts.get(page)
            if (not isinstance(layout, dict) or layout.get("missing_labels") or
                    layout.get("certifies_rtl_or_visual") is not False or
                    not all(name in layout.get("labels_requested", []) for name in ARABIC_NAV)):
                raise ValueError("Incomplete native Arabic navigation geometry: " + page)
            for name in ARABIC_NAV:
                _nav(layout, name, linux=linux)
        arabic = _nav(layouts["arabic_settings"], "الإعدادات", linux=linux)
        english = _nav(layouts.get("theme_system", {}), "Settings", linux=linux)
        for bounds in (arabic, english):
            if (bounds["x"] + bounds["width"] > width or
                    bounds["y"] + bounds["height"] > height):
                raise ValueError("Native control lies outside screenshot display bounds")
        right = (arabic["x"] + arabic["width"] / 2) / width
        left = (english["x"] + english["width"] / 2) / width
        mirrored = bool(right > .75 and left < .25 and right - left > .5)
        return {
            "schema": "native_arabic_rail_geometry_noncertifying_v1",
            "source_sha": exact_sha,
            "run_url": r["run_url"],
            "platform": "linux" if linux else "windows",
            "archive_sha256": hashlib.sha256(archive.read_bytes()).hexdigest(),
            "package_sha256": r["package_sha256"],
            "executable_sha256": r["executable_sha256"],
            "screenshot_sha256": {arabic_name: hashlib.sha256(ar_image).hexdigest(),
                                  english_name: hashlib.sha256(en_image).hexdigest()},
            "display_dimensions": [width, height],
            "arabic_nav_pages_with_named_focusable_buttons": 3,
            "arabic_settings_nav_center_fraction": round(right, 4),
            "english_settings_nav_center_fraction": round(left, 4),
            "right_to_left_navigation_rail_observed": mirrored,
            "native_16_case_verdict_unchanged": r.get("verdict"),
            "arabic_rtl_certification": "NOT TESTED",
            "appearance_accessibility_certification": "NOT TESTED",
            "limitations": ("Archived geometry and pixels establish only observed placement; "
                            "not mixed-script order, dialog geometry, focus sequence, "
                            "screen-reader announcements, narrow layouts, system theme "
                            "changes, or full contrast. Verify ZIP provenance independently."),
        }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--archive", required=True, type=Path)
    parser.add_argument("--exact-sha", required=True)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    data = json.dumps(review(args.archive, args.exact_sha), indent=2, sort_keys=True) + "\n"
    if args.output:
        args.output.write_text(data, encoding="utf-8")
    else:
        print(data, end="")


if __name__ == "__main__":
    main()
