"""Agent 10 test-only negative: forged EOCD count bypasses pre-allocation cap.

This intentionally confirms a resource-boundary failure in the unchanged
PR198 source at a frozen exact SHA. It does NOT install the application,
assign native QA PASS, or process untrusted real-world archives.
"""
import struct
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch
import zipfile

import visual_observations
from test_visual_observations import SHA, fixture_zip


class Agent10ZipDirectoryCountNegative(unittest.TestCase):
    def test_forged_eocd_claims_one_but_zipfile_indexes_over_budget(self):
        with tempfile.TemporaryDirectory() as temp:
            path = Path(temp) / "forged-eocd.zip"
            fixture_zip(path, "windows")
            with zipfile.ZipFile(path, "a", compression=zipfile.ZIP_STORED) as out:
                for i in range(129):
                    out.writestr(f"safe-extra-{i:03}.bin", b"x")

            snapshot = bytearray(path.read_bytes())
            marker = snapshot.rfind(b"PK\x05\x06")
            self.assertGreater(marker, 0)
            original_total = struct.unpack_from("<H", snapshot, marker + 10)[0]
            self.assertGreater(original_total, visual_observations.MAX_MEMBERS)
            self.assertLess(len(snapshot), 1024 * 1024)

            # EOCD positions: +8 = entries on disk; +10 = total entries.
            # Leave the actual central-directory bytes and its size unchanged.
            struct.pack_into("<H", snapshot, marker + 8, 1)
            struct.pack_into("<H", snapshot, marker + 10, 1)
            path.write_bytes(snapshot)

            # Current PR198 "before ZipFile" footer preflight trusts the
            # advertised count and accepts this false 1-entry footer.
            visual_observations._preflight_zip_footer(bytes(snapshot))
            original_zipfile = zipfile.ZipFile
            observed_indexed = []

            def witness(*args, **kwargs):
                opened = original_zipfile(*args, **kwargs)
                observed_indexed.append(len(opened.infolist()))
                return opened

            with patch.object(
                visual_observations.zipfile, "ZipFile", side_effect=witness
            ):
                with self.assertRaisesRegex(
                    ValueError, "excessive or missing members"
                ):
                    visual_observations.analyze(path, SHA)

            self.assertEqual(len(observed_indexed), 1)
            self.assertEqual(observed_indexed[0], original_total)
            self.assertGreater(
                observed_indexed[0],
                visual_observations.MAX_MEMBERS,
                "ZIP constructor did not actually exceed preflight budget",
            )
            print(
                "AGENT10_EOCD_UNDERREPORT_CONFIRMED "
                f"claimed=1 indexed={observed_indexed[0]} "
                f"limit={visual_observations.MAX_MEMBERS}"
            )

    def test_honest_count_is_rejected_before_zip_constructor(self):
        with tempfile.TemporaryDirectory() as temp:
            path = Path(temp) / "honest-too-many.zip"
            fixture_zip(path, "windows")
            with zipfile.ZipFile(path, "a") as out:
                for i in range(129):
                    out.writestr(f"safe-extra-{i:03}.bin", b"x")
            with patch.object(
                visual_observations.zipfile, "ZipFile",
                side_effect=AssertionError("unsafe constructor reached"),
            ):
                with self.assertRaisesRegex(
                    ValueError, "excessive native ZIP directory"
                ):
                    visual_observations.analyze(path, SHA)


if __name__ == "__main__":
    unittest.main()
