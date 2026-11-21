"""Read-only visual observations from real installed native QA evidence ZIPs.

Noncertifying: screenshot pixels do not prove RTL, keyboard focus, contrast
throughout the application, or all mandatory cases. Never change result.json.
"""
from __future__ import annotations
import argparse
import hashlib
import io
import json
import math
import os
import re
import stat
import struct
import zlib
import zipfile
from pathlib import Path

PNG_SIGNATURE = b"\x89PNG\r\n\x1a\n"
SAMPLE_X, SAMPLE_Y = 450, 200
HEX_SHA = re.compile(r"[0-9a-f]{40}\Z")
RUN_URL = re.compile(r"https://github\.com/MousaXD/scriptotar/actions/runs/[1-9][0-9]*\Z")
MAX_ARCHIVE_BYTES = 64 * 1024 * 1024
MAX_TOTAL_MEMBER_BYTES = 64 * 1024 * 1024
MAX_MEMBER_BYTES = 16 * 1024 * 1024
MAX_SCREENSHOT_BYTES = 8 * 1024 * 1024
MAX_JSON_BYTES = 1024 * 1024
MAX_MEMBERS = 128


def _read_archive_snapshot(archive: Path) -> bytes:
    """Open once, cap before allocation, hash exactly the bytes later parsed."""
    if archive.is_symlink():
        raise ValueError("Symlinked evidence ZIP is not accepted")
    flags = os.O_RDONLY | getattr(os, "O_BINARY", 0) | getattr(os, "O_NOFOLLOW", 0)
    with os.fdopen(os.open(archive, flags), "rb") as stream:
        before = os.fstat(stream.fileno())
        if not stat.S_ISREG(before.st_mode):
            raise ValueError("Evidence ZIP is not a regular file")
        if before.st_size > MAX_ARCHIVE_BYTES:
            raise ValueError("Evidence ZIP exceeds input size limit")
        snapshot = stream.read(MAX_ARCHIVE_BYTES + 1)
        after = os.fstat(stream.fileno())
    if (len(snapshot) > MAX_ARCHIVE_BYTES or
            len(snapshot) != before.st_size or
            after.st_size != before.st_size or
            after.st_mtime_ns != before.st_mtime_ns):
        raise ValueError("Evidence ZIP changed during bounded snapshot read")
    return snapshot


def _unique_json_pairs(pairs: list[tuple[str, object]]) -> dict:
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError("Duplicate JSON member in native evidence")
        result[key] = value
    return result


def _reject_nonfinite(value: str) -> object:
    raise ValueError("Nonfinite JSON constant in native evidence: " + value)


def _finite_float(value: str) -> float:
    parsed = float(value)
    if not math.isfinite(parsed):
        raise ValueError("Nonfinite JSON number in native evidence")
    return parsed


def _preflight_zip_footer(snapshot: bytes) -> None:
    """Reject excessive ZIP directory counts BEFORE ZipFile creates ZipInfo.

    The bounded outer snapshot is already read, but ZipFile's constructor
    materializes central-directory records before _preflight_members() can
    inspect them. Limit the advertised count at the original EOCD footer.
    ZIP64/multidisk/ambiguous footers are not required for these small native
    evidence archives and fail closed rather than creating a large index.
    """
    # End Of Central Directory is 22 bytes plus at most 65535 comment bytes.
    offset = snapshot.rfind(b"PK\x05\x06", max(0, len(snapshot) - 65557))
    if offset < 0 or len(snapshot) - offset < 22:
        raise ValueError("Missing native evidence ZIP end directory")
    (disk, directory_disk, disk_entries, total_entries, directory_bytes,
     directory_offset, comment_bytes) = struct.unpack_from("<HHHHIIH", snapshot, offset + 4)
    if (offset + 22 + comment_bytes != len(snapshot) or
            disk != 0 or directory_disk != 0 or
            disk_entries != total_entries or
            total_entries == 0 or total_entries > MAX_MEMBERS or
            directory_bytes == 0 or directory_bytes == 0xffffffff or
            directory_offset == 0xffffffff or
            total_entries == 0xffff):
        raise ValueError("Unsafe or excessive native ZIP directory metadata")


def _preflight_members(z: zipfile.ZipFile) -> dict[str, zipfile.ZipInfo]:
    """Deny hostile ZIP metadata before opening or inflating any member."""
    entries = z.infolist()
    if not 1 <= len(entries) <= MAX_MEMBERS:
        raise ValueError("Evidence ZIP has excessive or missing members")
    names = {}
    total = 0
    for info in entries:
        name = info.filename
        original = getattr(info, "orig_filename", name)
        parts = name.split("/")
        unix_mode = (info.external_attr >> 16) if info.create_system == 3 else 0
        if (not name or len(name.encode("utf-8")) > 300 or
                "\x00" in original or "\\" in name or ":" in name or
                name.startswith("/") or any(part in ("", ".", "..") for part in parts) or
                info.is_dir() or
                (stat.S_IFMT(unix_mode) not in (0, stat.S_IFREG)) or
                (info.flag_bits & 1) or
                info.compress_type not in (zipfile.ZIP_STORED, zipfile.ZIP_DEFLATED) or
                info.file_size < 0 or info.file_size > MAX_MEMBER_BYTES or
                info.compress_size < 0 or info.compress_size > MAX_ARCHIVE_BYTES):
            raise ValueError("Unsafe or oversized evidence ZIP member")
        if name in names:
            raise ValueError("Duplicate evidence ZIP name")
        total += info.file_size
        if total > MAX_TOTAL_MEMBER_BYTES:
            raise ValueError("Excessive total evidence ZIP inflated size")
        names[name] = info
    if "result.json" not in names:
        raise ValueError("Missing native result.json")
    if names["result.json"].file_size > MAX_JSON_BYTES:
        raise ValueError("Oversized native result.json")
    return names


def _read_member(z: zipfile.ZipFile, info: zipfile.ZipInfo, limit: int) -> bytes:
    """Read at most limit+1 bytes, including when central sizes are dishonest."""
    if info.file_size > limit:
        raise ValueError("Oversized native ZIP member: " + info.filename)
    with z.open(info, "r") as stream:
        payload = stream.read(limit + 1)
        if len(payload) > limit or len(payload) != info.file_size or stream.read(1):
            raise ValueError("Native ZIP member exceeds declared bounded length")
    return payload


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
    # Never use zlib.decompressobj().flush() on an untrusted PNG: it
    # unconditionally expands the remaining IDAT bytes with NO output cap.
    # A CRC-valid small IDAT can contain far more pixels than IHDR declares.
    expected_scan_bytes = (stride + 1) * height
    inflater = zlib.decompressobj()
    scan = bytearray()
    for offset in range(0, len(compressed), 64 * 1024):
        pending = bytes(compressed[offset:offset + 64 * 1024])
        while pending:
            # The +1 detects an overlong stream before materializing an
            # unbounded decompression bomb, including on the last IDAT block.
            remaining = expected_scan_bytes + 1 - len(scan)
            if remaining <= 0:
                raise ValueError("PNG scanline length mismatch")
            output = inflater.decompress(pending, remaining)
            scan.extend(output)
            if len(scan) > expected_scan_bytes:
                raise ValueError("PNG scanline length mismatch")
            if inflater.unused_data:
                raise ValueError("PNG zlib stream has trailing compressed data")
            next_pending = inflater.unconsumed_tail
            if len(next_pending) == len(pending) and not output:
                raise ValueError("PNG inflater made no forward progress")
            pending = next_pending
        if inflater.eof and offset + 64 * 1024 < len(compressed):
            raise ValueError("PNG zlib stream has trailing compressed data")
    if not inflater.eof or len(scan) != expected_scan_bytes:
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
    snapshot = _read_archive_snapshot(archive)
    _preflight_zip_footer(snapshot)
    with zipfile.ZipFile(io.BytesIO(snapshot)) as z:
        names = _preflight_members(z)
        report = json.loads(
            _read_member(z, names["result.json"], MAX_JSON_BYTES),
            object_pairs_hook=_unique_json_pairs,
            parse_constant=_reject_nonfinite,
            parse_float=_finite_float,
        )
        if not isinstance(report, dict):
            raise ValueError("Native result.json must be a JSON object")
        if (report.get("scope") != "native-installed" or
                report.get("source_sha") != exact_sha or
                not isinstance(report.get("run_url"), str) or
                not RUN_URL.fullmatch(report["run_url"])):
            raise ValueError("Installed run/source identity not matched")
        if report.get("native_ui_probe") != "installed-webview-automation":
            raise ValueError("Not the installed native probe")
        if any(not isinstance(report.get(key), str) or
               not re.fullmatch(r"[0-9a-f]{64}", report[key])
               for key in ("package_sha256", "executable_sha256")):
            raise ValueError("Installed package/executable identity missing")
        if not isinstance(report.get("platform"), str):
            raise ValueError("Native platform must be a string")
        platform_name = report["platform"].lower()
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
            image = _read_member(z, names[name], MAX_SCREENSHOT_BYTES)
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
        if not isinstance(report.get("cases"), dict):
            raise ValueError("Native result cases must be an object")
        if not isinstance(report.get("visual_capture"), dict):
            raise ValueError("Native visual capture must be an object")
        cases = {k: (v.get("status") if isinstance(v, dict) else v)
                 for k, v in report["cases"].items()}
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
            "archive_sha256": hashlib.sha256(snapshot).hexdigest(),
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
