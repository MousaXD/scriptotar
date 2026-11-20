"""Read-only visual observations from real installed native QA evidence ZIPs.

Noncertifying: screenshot pixels do not prove RTL, keyboard focus, contrast
throughout the application, or all mandatory cases. Never change result.json.
"""
from __future__ import annotations
import argparse
import hashlib
import json
import re
import struct
import zlib
import zipfile
from pathlib import Path

PNG_SIGNATURE = b"\x89PNG\r\n\x1a\n"
SAMPLE_X, SAMPLE_Y = 450, 200
HEX_SHA = re.compile(r"[0-9a-f]{40}\Z")
RUN_URL = re.compile(r"https://github\.com/MousaXD/scriptotar/actions/runs/[1-9][0-9]*\Z")


def _paeth(a: int, b: int, c: int) -> int:
    p = a + b - c
    d = (abs(p - a), abs(p - b), abs(p - c))
    return (a, b, c)[d.index(min(d))]


def sample_png_rgb(png: bytes, x: int = SAMPLE_X, y: int = SAMPLE_Y) -> tuple[tuple[int, int, int], tuple[int, int]]:
    """CRC-checked, stdlib-only 8-bit noninterlaced RGB/RGBA PNG pixel."""
    if not png.startswith(PNG_SIGNATURE):
        raise ValueError("Not a PNG screenshot")
    offset = len(PNG_SIGNATURE)
    width = height = channels = None
    compressed = bytearray()
    ended = False
    while offset + 12 <= len(png):
        length = struct.unpack_from(">I", png, offset)[0]
        offset += 4
        if length > len(png) - offset - 8:
            raise ValueError("Truncated PNG chunk")
        kind = png[offset:offset+4]
        value = png[offset+4:offset+4+length]
        offset += 4 + length
        checksum = struct.unpack_from(">I", png, offset)[0]
        offset += 4
        if zlib.crc32(kind+value) & 0xffffffff != checksum:
            raise ValueError("Corrupt PNG chunk checksum")
        if kind == b"IHDR":
            if width is not None or length != 13:
                raise ValueError("Malformed PNG header")
            width, height, depth, mode, compress, filtering, interlace = struct.unpack(">IIBBBBB", value)
            if (depth != 8 or mode not in (2, 6) or
                    (compress, filtering, interlace) != (0, 0, 0)):
                raise ValueError("Unsupported PNG screenshot encoding")
            channels = 3 if mode == 2 else 4
            if x >= width or y >= height or width * height > 8_000_000:
                raise ValueError("Invalid PNG size or sample coordinate")
        elif kind == b"IDAT":
            compressed.extend(value)
        elif kind == b"IEND":
            ended = True
            break
    if not ended or width is None or not compressed:
        raise ValueError("Incomplete PNG")
    stride = width * channels
    # Do not call inflater.flush() on untrusted IDAT: it ignores the output
    # bound passed to decompress() and can allocate vastly more bytes than
    # IHDR promises before the later length check rejects the image.
    expected_scan = (stride + 1) * height
    inflater = zlib.decompressobj()
    try:
        # One overflow-detection byte is sufficient; max_length limits the
        # actual output allocation even for tiny, extreme-ratio IDAT streams.
        scan = inflater.decompress(bytes(compressed), expected_scan + 1)
    except zlib.error as exc:
        raise ValueError("Corrupt PNG compressed scanlines") from exc
    if (len(scan) != expected_scan or not inflater.eof or
            inflater.unconsumed_tail or inflater.unused_data):
        raise ValueError("PNG scanline length mismatch")
    last = bytearray(stride)
    for row in range(y+1):
        pos = row * (stride+1)
        f = scan[pos]
        source = scan[pos+1:pos+1+stride]
        if f > 4:
            raise ValueError("Unsupported PNG scanline filter")
        current = bytearray(stride)
        for k, byte in enumerate(source):
            left = current[k-channels] if k >= channels else 0
            up = last[k]
            upperleft = last[k-channels] if k >= channels else 0
            if f == 0:
                num = byte
            elif f == 1:
                num = byte + left
            elif f == 2:
                num = byte + up
            elif f == 3:
                num = byte + (left+up)//2
            else:
                num = byte + _paeth(left, up, upperleft)
            current[k] = num & 255
        last = current
    return tuple(last[x*channels:x*channels+3]), (width, height)


def analyze(archive: Path, exact_sha: str) -> dict:
    """Observe real native screenshots without manufacturing any PASS verdict."""
    if not HEX_SHA.fullmatch(exact_sha):
        raise ValueError("Invalid 40-character exact source SHA")
    with zipfile.ZipFile(archive) as z:
        names = z.namelist()
        if len(set(names)) != len(names) or "result.json" not in names:
            raise ValueError("Duplicate evidence name or missing native result.json")
        report = json.loads(z.read("result.json"))
        if (report.get("scope") != "native-installed" or
                report.get("source_sha") != exact_sha or
                not RUN_URL.fullmatch(str(report.get("run_url")))):
            raise ValueError("Installed run/source identity not matched")
        if report.get("native_ui_probe") != "installed-webview-automation":
            raise ValueError("Not the installed native probe")
        if any(not re.fullmatch(r"[0-9a-f]{64}", str(report.get(key)))
               for key in ("package_sha256", "executable_sha256")):
            raise ValueError("Installed package/executable identity missing")
        platform_name = str(report.get("platform", "")).lower()
        if platform_name.startswith("windows"):
            platform = "windows"
            theme = [f"windows-theme-{v}.png" for v in ("system", "light", "dark", "dark-reopened")]
            arabic = ["windows-arabic-locale.png", "windows-arabic-settings.png",
                      "windows-arabic-queue.png"]
        elif platform_name.startswith("linux"):
            platform = "linux"
            theme = ["system-theme.png", "light-theme.png", "dark-theme.png", "dark-theme-reopened.png"]
            arabic = ["arabic-locale.png", "arabic-settings.png", "arabic-queue.png"]
        else:
            raise ValueError("Unknown installed platform")
        required = theme + arabic
        if any(name not in names for name in required):
            raise ValueError("Required native screenshot missing")
        screenshots = {}
        sizes = set()
        for name in required:
            image = z.read(name)
            if len(image) < 100:
                raise ValueError("Native screenshot is implausibly small")
            rgb, dimensions = sample_png_rgb(image)
            sizes.add(dimensions)
            screenshots[name] = {
                "sha256": hashlib.sha256(image).hexdigest(),
                "dimensions": dimensions,
                "sample_rgb": rgb,
                "relative_luma": round(sum(a*b for a, b in
                    zip(rgb, (.2126, .7152, .0722)))/255, 4),
            }
        if len(sizes) != 1:
            raise ValueError("Inconsistent native screenshot dimensions")
        cases = {k: (v.get("status") if isinstance(v, dict) else v)
                 for k, v in report.get("cases", {}).items()}
        real_ui = (cases.get("installed_webview") == "PASS" and
                   cases.get("theme_mode_control") == "PASS" and
                   report.get("visual_capture", {}).get("explicit_light_option") is True)
        l = [screenshots[name]["relative_luma"] for name in theme]
        appearance = (l[0] > .8 and l[1] > .8 and l[2] < .25 and l[3] < .25 and
                      l[0] - l[2] > .5 and abs(l[2] - l[3]) < .12)
        return {
            "schema": "native_visual_review_noncertifying_v1",
            "source_sha": exact_sha,
            "run_url": report["run_url"],
            "platform": platform,
            "archive_sha256": hashlib.sha256(archive.read_bytes()).hexdigest(),
            "screenshots": screenshots,
            "native_theme_ui_gate_passed": bool(real_ui),
            "light_dark_and_persisted_dark_observed": bool(appearance and real_ui),
            "arabic_screen_captures_present": cases.get("arabic_locale_control") == "PASS",
            "native_16_case_verdict_unchanged": report.get("verdict"),
            "light_dark_visual_certification": "NOT TESTED",
            "arabic_rtl_certification": "NOT TESTED",
            "limits": ("Screenshots/pixels are NOT proof of full-product contrast, "
                       "dialogs, keyboard focus, bidirectional text or the 16-case matrix. "
                       "Native artifact metadata is not a signature; verify GitHub artifact provenance independently."),
        }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--archive", required=True, type=Path)
    parser.add_argument("--exact-sha", required=True)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    data = analyze(args.archive, args.exact_sha)
    serialized = json.dumps(data, indent=2, sort_keys=True, ensure_ascii=False) + "\n"
    if args.output:
        args.output.write_text(serialized, encoding="utf-8")
    else:
        print(serialized)


if __name__ == "__main__":
    main()
