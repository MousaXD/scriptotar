"""Source-only offline bundle checks; all audio bytes are fake unit data.

These tests deliberately bypass ffprobe through a stub. They NEVER make audio
fixtures for an installed package, set native verdicts or test live Whisper.
"""
from __future__ import annotations

import hashlib
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

from linux_real_media_attempt import NativeFailure, is_process_descendant, verify_prepared_originals
from real_source_fixtures import SOURCES
from real_study_lineage import LineageUnavailable


class OfflineMediaTrustUnitTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.base = Path(self.temporary.name)
        records = []
        for i, info in enumerate(SOURCES):
            label = info["source_id"].split(":", 1)[1]
            original = self.base / (label + ".ogg")
            encoded = self.base / (label + ".webm")
            original.write_bytes(b"OggS" + bytes([i + 1]) * 40000)
            encoded.write_bytes(bytes([i + 3]) * 40000)
            records.append({
                **info,
                "original_filename": original.name,
                "original_sha256": hashlib.sha256(original.read_bytes()).hexdigest(),
                "original_duration_seconds": 145.0,
                "webm_filename": encoded.name,
                "webm_sha256": hashlib.sha256(encoded.read_bytes()).hexdigest(),
                "webm_duration_seconds": 145.0,
                "media_provenance": "original-source-downloaded-and-ffprobe-verified",
            })
        self.manifest_path = self.base / "origin-manifest.json"
        self.manifest = {
            "purpose": "native-two-real-media-preparation-only",
            "installed_two_source_acceptance": "NOT TESTED",
            "sources": records,
        }
        self.write()

    def write(self):
        self.manifest_path.write_text(
            json.dumps(self.manifest, indent=2), encoding="utf-8",
        )
        self.pin = hashlib.sha256(self.manifest_path.read_bytes()).hexdigest()

    def verify(self, pin=None):
        with patch("linux_real_media_attempt.probe_audio", return_value=145.0):
            return verify_prepared_originals(
                self.base, self.pin if pin is None else pin, "fake-probe-stub",
            )

    def test_bundled_engine_requires_real_installed_pid_ancestry(self):
        parents = {
            100: 1, 101: 100, 102: 101, 103: 102,
            150: 1, 151: 150, 170: 171, 171: 170,
        }
        self.assertTrue(is_process_descendant(parents, 101, 100))
        self.assertTrue(is_process_descendant(parents, 103, 100))
        self.assertFalse(is_process_descendant(parents, 100, 100))
        self.assertFalse(is_process_descendant(parents, 151, 100))
        self.assertFalse(is_process_descendant(parents, 170, 100))
        self.assertFalse(is_process_descendant(parents, 999, 100))

    def test_correct_pinned_source_only_manifest_is_noncertifying(self):
        manifest, origins = self.verify()
        self.assertEqual(len(origins), 2)
        self.assertEqual(manifest["installed_two_source_acceptance"], "NOT TESTED")

    def test_manifest_pin_is_mandatory_and_invalidated_by_tampering(self):
        with self.assertRaisesRegex(NativeFailure, "pinned manifest"):
            self.verify("")
        pin_before = self.pin
        self.manifest["sources"][0]["title"] = "tampered"
        self.write()
        with self.assertRaisesRegex(NativeFailure, "differs"):
            self.verify(pin_before)
        with self.assertRaisesRegex(NativeFailure, "origin identity"):
            self.verify()

    def test_changed_webm_and_original_bytes_are_rejected(self):
        for name in ("webm_filename", "original_filename"):
            with self.subTest(name=name):
                row = self.manifest["sources"][0]
                path = self.base / row[name]
                before = path.read_bytes()
                path.write_bytes(before + b"different")
                with self.assertRaises(LineageUnavailable):
                    self.verify()
                path.write_bytes(before)

    def test_duplicate_origin_and_forged_success_rejected(self):
        self.manifest["sources"][1]["source_id"] = self.manifest["sources"][0]["source_id"]
        self.write()
        with self.assertRaisesRegex(NativeFailure, "approved"):
            self.verify()
        self.manifest["sources"][1]["source_id"] = SOURCES[1]["source_id"]
        self.manifest["installed_two_source_acceptance"] = "PASS"
        self.write()
        with self.assertRaisesRegex(NativeFailure, "must not claim"):
            self.verify()

    def test_symlink_bundle_and_files_are_rejected(self):
        link = self.base.parent / (self.base.name + "-link")
        link.symlink_to(self.base, target_is_directory=True)
        self.addCleanup(link.unlink)
        with patch("linux_real_media_attempt.probe_audio", return_value=145.0):
            with self.assertRaisesRegex(NativeFailure, "real directory"):
                verify_prepared_originals(link, self.pin, "fake-probe-stub")
        original = self.base / self.manifest["sources"][0]["original_filename"]
        original.rename(self.base / "relocated.ogg")
        original.symlink_to(self.base / "relocated.ogg")
        with self.assertRaisesRegex(NativeFailure, "symlinked"):
            self.verify()


if __name__ == "__main__":
    unittest.main()
