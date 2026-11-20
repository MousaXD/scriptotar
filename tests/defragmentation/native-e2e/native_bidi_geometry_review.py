#!/usr/bin/env python3
"""Read-only Arabic rail mirroring audit on original installed QA ZIPs.

Only records native AT-SPI/UIA coordinates and archived screenshot identities;
never assigns a full RTL, appearance, or 16-case PASS. No UI or SQLite writes.
"""
from __future__ import annotations

import argparse
import hashlib
import io
import json
import math
from pathlib import Path, PurePosixPath
import re
import zipfile

from visual_observations import sample_png_rgb

SHA = re.compile(r"[0-9a-f]{40}\Z")
RUN = re.compile(r"https://github\.com/MousaXD/scriptotar/actions/runs/[1-9][0-9]*\Z")
ARABIC_NAV = ("الاستوديو", "الطابور", "الإعدادات")
MAX_ARCHIVE_BYTES = 64 * 1024 * 1024
MAX_TOTAL_MEMBER_BYTES = 64 * 1024 * 1024
MAX_MEMBER_BYTES = 16 * 1024 * 1024
MAX_SCREENSHOT_BYTES = 8 * 1024 * 1024
MAX_JSON_BYTES = 1024 * 1024
MAX_MEMBERS = 128


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
    if any(type(bounds.get(field)) not in (int, float) or
           not math.isfinite(bounds[field]) for field in
           ("x", "y", "width", "height")):
        raise ValueError("Missing numeric native bounds")
    if (bounds["width"] <= 0 or bounds["height"] <= 0 or
            bounds["x"] < 0 or bounds["y"] < 0):
        raise ValueError("Invalid native navigation bounds")
    return bounds


def review(archive: Path, exact_sha: str) -> dict:
    if not SHA.fullmatch(exact_sha):
        raise ValueError("Expected exact source SHA")
    if not archive.is_file() or archive.is_symlink():
        raise ValueError("Evidence ZIP must be a bounded regular file")
    # Parse AND hash the same capped snapshot, never re-open a mutable path for
    # the archive checksum after inspecting its contents.
    with archive.open("rb") as stream:
        snapshot = stream.read(MAX_ARCHIVE_BYTES + 1)
    if len(snapshot) > MAX_ARCHIVE_BYTES:
        raise ValueError("Evidence ZIP exceeds bounded input size")
    with zipfile.ZipFile(io.BytesIO(snapshot)) as z:
        infos = z.infolist()
        names = [info.filename for info in infos]
        if len(names) > MAX_MEMBERS or len(names) != len(set(names)) or "result.json" not in names:
            raise ValueError("Duplicate, excessive or missing original evidence entry")
        if any(info.is_dir() or info.flag_bits & 1 or
               info.compress_type not in (zipfile.ZIP_STORED, zipfile.ZIP_DEFLATED) or
               info.file_size > MAX_MEMBER_BYTES or info.file_size < 0 or
               info.compress_size > MAX_ARCHIVE_BYTES or
               PurePosixPath(info.filename).is_absolute() or
               any(part in (".", "..") for part in info.filename.split("/"))
               for info in infos):
            raise ValueError("Unsafe or oversized evidence ZIP member")
        if sum(info.file_size for info in infos) > MAX_TOTAL_MEMBER_BYTES:
            raise ValueError("Excessive total decompressed evidence size")
        if z.getinfo("result.json").file_size > MAX_JSON_BYTES:
            raise ValueError("Oversized native result.json")
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
            arabic_names = {
                "arabic_studio": "arabic-locale.png",
                "arabic_queue": "arabic-queue.png",
                "arabic_settings": "arabic-settings.png",
            }
            english_name = "system-theme.png"
        elif platform.startswith("windows"):
            linux = False
            arabic_names = {
                "arabic_studio": "windows-arabic-locale.png",
                "arabic_queue": "windows-arabic-queue.png",
                "arabic_settings": "windows-arabic-settings.png",
            }
            english_name = "windows-theme-system.png"
        else:
            raise ValueError("Unknown native platform")
        required = list(arabic_names.values()) + [english_name]
        if any(name not in names for name in required):
            raise ValueError("Original Arabic or English screenshot missing")
        if any(z.getinfo(name).file_size > MAX_SCREENSHOT_BYTES for name in required):
            raise ValueError("Oversized installed screenshot evidence")
        images = {name: z.read(name) for name in required}
        dimensions = {name: sample_png_rgb(data)[1] for name, data in images.items()}
        if len(set(dimensions.values())) != 1:
            raise ValueError("Inconsistent installed display dimensions")
        width, height = dimensions[english_name]
        # Different real pages must have distinct native screenshots. Replayed
        # or relabeled same-page captures cannot establish three-page coverage.
        arabic_hashes = {name: hashlib.sha256(images[name]).hexdigest()
                         for name in arabic_names.values()}
        if len(set(arabic_hashes.values())) != len(arabic_hashes):
            raise ValueError("Duplicate Arabic screen capture across distinct pages")
        layouts = r.get("native_layout_observations", {})
        page_centers = {}
        for page in arabic_names:
            layout = layouts.get(page)
            if (not isinstance(layout, dict) or layout.get("missing_labels") or
                    layout.get("certifies_rtl_or_visual") is not False or
                    not all(name in layout.get("labels_requested", []) for name in ARABIC_NAV)):
                raise ValueError("Incomplete native Arabic navigation geometry: " + page)
            centers = {}
            for name in ARABIC_NAV:
                bounds = _nav(layout, name, linux=linux)
                if bounds["x"] + bounds["width"] > width or bounds["y"] + bounds["height"] > height:
                    raise ValueError("Native control lies outside screenshot display bounds: " + page)
                centers[name] = round((bounds["x"] + bounds["width"] / 2) / width, 4)
            page_centers[page] = centers
        english = _nav(layouts.get("theme_system", {}), "Settings", linux=linux)
        if (english["x"] + english["width"] > width or
                english["y"] + english["height"] > height):
            raise ValueError("English native control lies outside screenshot display bounds")
        right = page_centers["arabic_settings"]["الإعدادات"]
        left = (english["x"] + english["width"] / 2) / width
        # All three buttons form the right-side rail on every Arabic page.
        # A single mirrored Settings button does not prove the whole rail.
        mirrored = bool(left < .25 and all(
            value > .75 and value - left > .5
            for centers in page_centers.values() for value in centers.values()))
        return {
            "schema": "native_arabic_rail_geometry_noncertifying_v1",
            "source_sha": exact_sha,
            "run_url": r["run_url"],
            "platform": "linux" if linux else "windows",
            "archive_sha256": hashlib.sha256(snapshot).hexdigest(),
            "package_sha256": r["package_sha256"],
            "executable_sha256": r["executable_sha256"],
            "screenshot_sha256": {name: hashlib.sha256(data).hexdigest()
                                  for name, data in images.items()},
            "arabic_navigation_centers_by_page": page_centers,
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
