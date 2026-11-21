"""Pure-stdlib unit tests. Fixtures are synthetic PNGs, NEVER installed QA PASS."""
import hashlib
import json
import os
import stat
import struct
import tempfile
import unittest
from unittest.mock import patch
import zipfile
import zlib
from pathlib import Path


class GuardedInflater:
    """Reject unbounded inflater operations, even if a decoder later errors."""

    def __init__(self, real, declared_scan_bytes):
        self._inner = real()
        self._limit = declared_scan_bytes + 1
        self.largest_response = 0

    def decompress(self, encoded, max_length=0):
        if not 0 < max_length <= self._limit:
            raise AssertionError("PNG inflate must have a declared output cap")
        decoded = self._inner.decompress(encoded, max_length)
        self.largest_response = max(self.largest_response, len(decoded))
        if len(decoded) > self._limit:
            raise AssertionError("PNG inflated beyond declared scanline budget")
        return decoded

    def flush(self, *args):
        raise AssertionError("Unbounded zlib flush is prohibited")

    def __getattr__(self, name):
        return getattr(self._inner, name)

import visual_observations
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

    def test_idat_overinflation_is_capped_before_allocation(self):
        width, height = 451, 201
        declared = (width * 3 + 1) * height
        # Malicious but CRC-valid IDAT contains many more decoded scanlines
        # than IHDR promises. The compressed stream remains tiny.
        malicious = zlib.compress(b"\x00" * (13 * 1024 * 1024), 9)

        def chunk(kind, payload):
            return (struct.pack(">I", len(payload)) + kind + payload +
                    struct.pack(">I", zlib.crc32(kind + payload) & 0xffffffff))

        ihdr = struct.pack(">IIBBBBB", width, height, 8, 2, 0, 0, 0)
        hostile_png = (b"\x89PNG\r\n\x1a\n" +
                       chunk(b"IHDR", ihdr) + chunk(b"IDAT", malicious) +
                       chunk(b"IEND", b""))
        valid_png = make_png((13, 17, 22), mode=2)
        original_decompressobj = zlib.decompressobj
        guards = []

        def guarded_decompressor(*args, **kwargs):
            guard = GuardedInflater(lambda: original_decompressobj(*args, **kwargs), declared)
            guards.append(guard)
            return guard

        with patch.object(zlib, "decompressobj", side_effect=guarded_decompressor):
            self.assertEqual(sample_png_rgb(valid_png)[0], (13, 17, 22))
            with self.assertRaisesRegex(ValueError, "PNG scanline length mismatch"):
                sample_png_rgb(hostile_png)
        self.assertEqual(len(guards), 2)
        self.assertTrue(all(g.largest_response <= declared + 1 for g in guards))

        # Stream integrity is checked, too: appended bytes and missing zlib
        # trailer must be rejected even if pixel count happens to match.
        raw = (b"\x00" + bytes((13, 17, 22)) * width) * height
        for encoded in (zlib.compress(raw) + b"TRAIL", zlib.compress(raw)[:-5]):
            forged = (b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", ihdr) +
                      chunk(b"IDAT", encoded) + chunk(b"IEND", b""))
            with self.assertRaisesRegex(ValueError, "PNG"):
                sample_png_rgb(forged)

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


    def test_visual_zip_rejects_oversized_compressed_members_before_inflation(self):
        def rewrite(path, changes):
            with zipfile.ZipFile(path) as source:
                originals = [(info.filename, source.read(info)) for info in source.infolist()]
            with zipfile.ZipFile(path, "w", compression=zipfile.ZIP_DEFLATED) as target:
                for name, data in originals:
                    target.writestr(name, changes.get(name, data))

        with tempfile.TemporaryDirectory() as temp:
            path = Path(temp) / "oversized.zip"
            fixture_zip(path)
            rewrite(path, {"result.json": b" " * (1024 * 1024 + 1)})
            self.assertLess(path.stat().st_size, 1024 * 1024)
            with self.assertRaisesRegex(ValueError, "Oversized native result.json"):
                analyze(path, SHA)

            fixture_zip(path)
            rewrite(path, {"windows-theme-dark.png": b"\x00" * (8 * 1024 * 1024 + 1)})
            self.assertLess(path.stat().st_size, 1024 * 1024)
            with self.assertRaisesRegex(ValueError, "Oversized native ZIP member"):
                analyze(path, SHA)

            # Sparse file checks the *archive* size without allocating RAM
            # or writing 65MiB of incompressible test data.
            with path.open("wb") as f:
                f.seek(64 * 1024 * 1024)
                f.write(b"x")
            with self.assertRaisesRegex(ValueError, "input size limit"):
                analyze(path, SHA)

    def test_visual_zip_rejects_ambiguous_json_in_both_orders(self):
        with tempfile.TemporaryDirectory() as temp:
            path = Path(temp) / "duplicates.zip"
            fixture_zip(path)
            with zipfile.ZipFile(path) as source:
                members = [(info.filename, source.read(info)) for info in source.infolist()]
            canonical = next(data for name, data in members if name == "result.json")
            doc = canonical.decode("utf-8")
            old_sha = '"source_sha": "' + SHA + '"'
            self.assertIn(old_sha, doc)
            foreign = '"source_sha": "' + "f" * 40 + '"'
            variants = [
                doc.replace(old_sha, old_sha + ", " + foreign),
                doc.replace(old_sha, foreign + ", " + old_sha),
                doc.replace(old_sha, old_sha + ', "sourc\\u0065_sha": "' + "f" * 40 + '"'),
                doc.replace('"cases": {', '"cases": {}, "cases": {'),
                doc.replace('"cases": {', '"cases": {"installed_webview": {}, "installed_webview": {},'),
                doc.replace('"verdict": "NOT TESTED"', '"verdict": NaN'),
                doc.replace('"verdict": "NOT TESTED"', '"verdict": Infinity'),
                doc.replace('"verdict": "NOT TESTED"', '"verdict": -Infinity'),
                doc.replace('"verdict": "NOT TESTED"', '"verdict": 1e9999'),
            ]
            for index, altered in enumerate(variants):
                with self.subTest(index=index):
                    with zipfile.ZipFile(path, "w", compression=zipfile.ZIP_DEFLATED) as target:
                        for name, payload in members:
                            target.writestr(
                                name, altered.encode("utf-8") if name == "result.json"
                                else payload
                            )
                    with self.assertRaisesRegex(ValueError, "Duplicate JSON|Nonfinite"):
                        analyze(path, SHA)

    def test_visual_zip_rejects_unsafe_names_modes_and_excessive_members(self):
        with tempfile.TemporaryDirectory() as temp:
            path = Path(temp) / "unsafe.zip"
            for name in ("../traversal", "/absolute", "nested//empty",
                         r"C:\windows\unsafe", "folder/./dot", "folder/../parent"):
                with self.subTest(name=name):
                    fixture_zip(path)
                    with zipfile.ZipFile(path, "a") as z:
                        z.writestr(name, b"bad")
                    with self.assertRaisesRegex(ValueError, "Unsafe or oversized"):
                        analyze(path, SHA)

            fixture_zip(path)
            with zipfile.ZipFile(path, "a") as z:
                z.writestr("result.json", b"{}")
            with self.assertRaisesRegex(ValueError, "Duplicate evidence ZIP"):
                analyze(path, SHA)

            fixture_zip(path)
            link = zipfile.ZipInfo("symlink-member")
            link.create_system = 3
            link.external_attr = (stat.S_IFLNK | 0o777) << 16
            with zipfile.ZipFile(path, "a") as z:
                z.writestr(link, b"another-file")
            with self.assertRaisesRegex(ValueError, "Unsafe or oversized"):
                analyze(path, SHA)

            fixture_zip(path)
            with zipfile.ZipFile(path, "a") as z:
                for i in range(129):
                    z.writestr(f"extra-{i}", b"x")
            # Reject the advertised count before ZipFile can allocate
            # a central-directory index, not merely at member preflight.
            with patch.object(visual_observations.zipfile, "ZipFile",
                              side_effect=AssertionError("ZipFile constructor reached")):
                with self.assertRaisesRegex(ValueError, "excessive native ZIP directory"):
                    analyze(path, SHA)

            if os.name != "nt":
                fixture_zip(path)
                symlink = Path(temp) / "alias.zip"
                symlink.symlink_to(path)
                with self.assertRaisesRegex(ValueError, "Symlinked"):
                    analyze(symlink, SHA)

    def test_zip_footer_rejects_malformed_metadata_before_indexing(self):
        with tempfile.TemporaryDirectory() as temp:
            path = Path(temp) / "footer.zip"
            fixture_zip(path)
            original = path.read_bytes()
            eocd = original.rfind(bytes.fromhex("504b0506"))
            self.assertGreater(eocd, 0)
            # Original ZIP footer is 22 bytes; counts/offsets are fixed-size
            # little-endian metadata. Never generate large hostile archives.
            cases = {
                "too_many_entries": (eocd + 10, 129),
                "zip64_marker": (eocd + 10, 65535),
                "multidisk": (eocd + 4, 1),
            }
            for label, (offset, value) in cases.items():
                with self.subTest(label=label):
                    mutated = bytearray(original)
                    struct.pack_into("<H", mutated, offset, value)
                    path.write_bytes(mutated)
                    with patch.object(visual_observations.zipfile, "ZipFile",
                                      side_effect=AssertionError("ZipFile constructor reached")):
                        with self.assertRaisesRegex(ValueError, "native ZIP directory"):
                            analyze(path, SHA)
            path.write_bytes(original + b"unauthorized-trailer")
            with patch.object(visual_observations.zipfile, "ZipFile",
                              side_effect=AssertionError("ZipFile constructor reached")):
                with self.assertRaisesRegex(ValueError, "native ZIP directory"):
                    analyze(path, SHA)

    def test_underreported_real_directory_cannot_allocate_zipinfo(self):
        # Original Agent10 negative: a 137-member archive can advertise
        # just one member in BOTH EOCD count fields. ZipFile would otherwise
        # construct 137 records *before* post-parse membership checks.
        with tempfile.TemporaryDirectory() as temp:
            path = Path(temp) / "forged-count.zip"
            fixture_zip(path)
            with zipfile.ZipFile(path, "a") as z:
                for i in range(129):
                    z.writestr(f"extra-{i}.txt", b"x")
            actual = path.read_bytes()
            eocd = actual.rfind(bytes.fromhex("504b0506"))
            self.assertGreater(eocd, 0)
            forged = bytearray(actual)
            struct.pack_into("<HH", forged, eocd + 8, 1, 1)
            path.write_bytes(forged)

            with patch.object(visual_observations.zipfile, "ZipFile",
                              side_effect=AssertionError("ZipFile indexed hostile entries")):
                with self.assertRaisesRegex(
                    ValueError, "central directory claimed count differs"
                ):
                    analyze(path, SHA)

    def test_directory_length_and_header_corruption_denied_before_index(self):
        with tempfile.TemporaryDirectory() as temp:
            path = Path(temp) / "corrupt-directory.zip"
            fixture_zip(path)
            original = path.read_bytes()
            eocd = original.rfind(bytes.fromhex("504b0506"))
            self.assertGreater(eocd, 0)
            directory_offset = struct.unpack_from("<I", original, eocd + 16)[0]
            for mutation in ("directory_length", "directory_offset",
                             "central_signature", "local_signature"):
                with self.subTest(mutation=mutation):
                    data = bytearray(original)
                    if mutation == "directory_length":
                        struct.pack_into("<I", data, eocd + 12, 2)
                    elif mutation == "directory_offset":
                        struct.pack_into("<I", data, eocd + 16, 2)
                    elif mutation == "central_signature":
                        data[directory_offset] = 0
                    else:
                        data[0] = 0
                    path.write_bytes(data)
                    with patch.object(visual_observations.zipfile, "ZipFile",
                                      side_effect=AssertionError("ZipFile indexed corruption")):
                        with self.assertRaisesRegex(ValueError, "(?:Native|native) ZIP"):
                            analyze(path, SHA)


    def test_zip_footer_allows_legitimate_native_evidence(self):
        with tempfile.TemporaryDirectory() as temp:
            path = Path(temp) / "valid.zip"
            fixture_zip(path)
            self.assertEqual(analyze(path, SHA)["source_sha"], SHA)


    def test_visual_zip_rejects_aggregate_inflated_size_and_unsupported_compression(self):
        with tempfile.TemporaryDirectory() as temp:
            path = Path(temp) / "aggregate.zip"
            fixture_zip(path)
            # 5 x 13MiB compressible members: a small archive with >64MiB
            # of claimed inflated content, refused by central-directory
            # metadata BEFORE any hostile member is extracted.
            chunk = b"X" * (13 * 1024 * 1024)
            with zipfile.ZipFile(path, "a", compression=zipfile.ZIP_DEFLATED) as z:
                for i in range(5):
                    z.writestr(f"aggregate-{i}.bin", chunk)
            self.assertLess(path.stat().st_size, 1024 * 1024)
            with self.assertRaisesRegex(ValueError, "Excessive total evidence ZIP"):
                analyze(path, SHA)

            fixture_zip(path)
            with zipfile.ZipFile(path, "a") as z:
                z.writestr("unsupported.bz2", b"x" * 200,
                           compress_type=zipfile.ZIP_BZIP2)
            with self.assertRaisesRegex(ValueError, "Unsafe or oversized"):
                analyze(path, SHA)

            fixture_zip(path)
            with zipfile.ZipFile(path, "a") as z:
                z.writestr("directory/", b"")
            with self.assertRaisesRegex(ValueError, "Unsafe or oversized"):
                analyze(path, SHA)


    def test_visual_zip_digest_is_bound_to_parsed_snapshot_not_swapped_path(self):
        with tempfile.TemporaryDirectory() as temp:
            path = Path(temp) / "mutable.zip"
            replacement = Path(temp) / "replacement.zip"
            fixture_zip(path)
            fixture_zip(replacement, source="f" * 40)
            original_snapshot = path.read_bytes()
            replacement_digest = hashlib.sha256(replacement.read_bytes()).hexdigest()
            original_zipfile = zipfile.ZipFile
            called = []

            def swap_after_snapshot(file, *args, **kwargs):
                if not called:
                    called.append(True)
                    os.replace(replacement, path)
                return original_zipfile(file, *args, **kwargs)

            with patch.object(visual_observations.zipfile, "ZipFile",
                              side_effect=swap_after_snapshot):
                result = analyze(path, SHA)
            self.assertEqual(result["source_sha"], SHA)
            self.assertEqual(result["archive_sha256"],
                             hashlib.sha256(original_snapshot).hexdigest())
            self.assertNotEqual(result["archive_sha256"], replacement_digest)
            self.assertTrue(called)

if __name__ == "__main__":
    unittest.main()
