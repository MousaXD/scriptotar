#!/usr/bin/env python3
"""Agent10 independent bounded PNG inflater negative for native review ZIPs.

The fixed-size original fixture has a tiny IDAT byte stream but a malformed
overlong expanded scan. Instrument only zlib.decompressobj().flush() to record
expanded data; limit the fixture to 12 MiB excess so this test itself is safe.
Do not use real user ZIPs, app runtimes, or network source data.
"""
from __future__ import annotations

import argparse
import importlib.util
import struct
import zlib
from pathlib import Path

TARGET_SOURCE_SHA = "94f598d87543c9333dff6f489219cf314d77259f"
VISUAL_BLOB = "7b3dc4e12b04acc0f052403c705e24e117f8d968"
REVIEW_BLOB = "d025ee5d12485dca7bbc4c9586a6e31c94532a7b"
PNG_SIGNATURE = b"\x89PNG\r\n\x1a\n"
WIDTH, HEIGHT = 640, 480
EXPAND_EXCESS = 12 * 1024 * 1024


def png_chunk(kind: bytes, value: bytes) -> bytes:
    return (
        struct.pack(">I", len(value))
        + kind
        + value
        + struct.pack(">I", zlib.crc32(kind + value) & 0xFFFFFFFF)
    )


def sample_png(*, extra_bytes: int) -> bytes:
    row_bytes = WIDTH * 3 + 1
    normal_scan_bytes = row_bytes * HEIGHT
    # This all-zero scan is a valid PNG when extra_bytes == 0.
    # Extra >0 is a syntactically valid PNG whose IDAT payload decompresses
    # beyond dimensions claimed by IHDR. A secure parser rejects this with
    # limited work, not after materializing the entire bomb.
    content = b"\x00" * (normal_scan_bytes + extra_bytes)
    header = struct.pack(">IIBBBBB", WIDTH, HEIGHT, 8, 2, 0, 0, 0)
    return (
        PNG_SIGNATURE
        + png_chunk(b"IHDR", header)
        + png_chunk(b"IDAT", zlib.compress(content, level=9))
        + png_chunk(b"IEND", b"")
    )


def load_target(source: Path):
    assert source.is_file(), "Pinned original native decoder missing"
    spec = importlib.util.spec_from_file_location(
        "agent10_exact_native_visual_png_decoder", source
    )
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def test_decoder(path: Path, review_source: Path) -> None:
    # Refuse a test that accidentally stops exercising the real Agent07 call
    # path and its unchanged shared native PNG decoder.
    assert "from visual_observations import sample_png_rgb" in review_source.read_text(encoding="utf-8")
    assert "sample_png_rgb(data)" in review_source.read_text(encoding="utf-8")
    module = load_target(path)

    benign = sample_png(extra_bytes=0)
    assert len(benign) < 30_000, "Benign sample no longer small"
    pixel, size = module.sample_png_rgb(benign)
    assert size == (WIDTH, HEIGHT) and pixel == (0, 0, 0), (size, pixel)
    print("PASS: clean synthetic 640x480 screenshot and observed pixel", flush=True)

    malicious = sample_png(extra_bytes=EXPAND_EXCESS)
    assert len(malicious) < 30_000, "Bomb fixture must remain compact"
    expected_scan = (WIDTH * 3 + 1) * HEIGHT
    observed = {"max_flush_bytes": 0, "flush_calls": 0}
    original_decompressobj = module.zlib.decompressobj

    class InstrumentedInflater:
        def __init__(self, *args, **kwargs):
            self._original = original_decompressobj(*args, **kwargs)

        def __getattr__(self, key):
            return getattr(self._original, key)

        def decompress(self, *args, **kwargs):
            return self._original.decompress(*args, **kwargs)

        def flush(self, *args, **kwargs):
            # Intentionally count output *as actually produced by the
            # candidate*. The 12MiB cap is only on this fixture. A source
            # fix should never get here with unbounded excess pending.
            observed["flush_calls"] += 1
            result = self._original.flush(*args, **kwargs)
            observed["max_flush_bytes"] = max(
                observed["max_flush_bytes"], len(result)
            )
            return result

    module.zlib.decompressobj = lambda *args, **kwargs: InstrumentedInflater(*args, **kwargs)
    rejected = False
    try:
        try:
            module.sample_png_rgb(malicious)
        except (ValueError, zlib.error) as exc:
            rejected = True
            print("PASS: malformed overlong PNG eventually rejected: " + str(exc), flush=True)
    finally:
        module.zlib.decompressobj = original_decompressobj

    assert rejected, "FAIL OPEN: malformed oversized expanded PNG accepted"
    count = observed["max_flush_bytes"]
    print(
        f"EVIDENCE: PNG {len(malicious)} compressed bytes, "
        f"declared scan {expected_scan} bytes, "
        f"unbounded flush emitted {count} bytes in "
        f"{observed['flush_calls']} calls",
        flush=True,
    )
    assert count <= expected_scan + 1, (
        "RESOURCE VETO: decoder materialized "
        f"{count} bytes via unrestricted inflater.flush() "
        f"for PNG claiming only {expected_scan} scan bytes"
    )
    print("PASS: oversize IDAT denied without unbounded inflated allocation", flush=True)


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--candidate", type=Path, default=Path(".agent10-pr161-data"))
    args = ap.parse_args()
    root = args.candidate.resolve(strict=True)
    test_decoder(
        root / "tests/defragmentation/native-e2e/visual_observations.py",
        root / "tests/defragmentation/native-e2e/native_bidi_geometry_review.py",
    )


if __name__ == "__main__":
    main()
