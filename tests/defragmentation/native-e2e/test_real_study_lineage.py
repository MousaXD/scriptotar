"""Fail-closed unit tests; synthetic dictionaries are NEVER native acceptance evidence."""
from __future__ import annotations

from copy import deepcopy
import hashlib
import json
from pathlib import Path
import tempfile
import unittest

from real_study_lineage import (
    LineageUnavailable, digest_rows, expected_media,
    read_installed_lineage, verify_revocation,
)


def snapshots():
    history = {
        table: {"rows": 0, "sha256": "a" * 64}
        for table in ("study_evidence", "draft_source_links", "study_source_jobs")
    }
    before = {
        "evidence_kind": "installed-canonical-db-readback-only",
        "native_16_case_certification": "NOT TESTED",
        "project_id_sha256": "b" * 64,
        "study_id_sha256": "c" * 64,
        "revision": 3,
        "transcripts": [{"transcript_id": "first"}, {"transcript_id": "second"}],
        "bindings": {
            x: {
                "study_source_id": "source-" + x,
                "fingerprint": "fingerprint-" + x,
                "locator_sha256": x[0] * 64,
                "available": True,
                "tombstoned": False,
                "history": deepcopy(history),
            } for x in ("first", "second")
        },
        "drafts": {"rows": 1, "sha256": "d" * 64},
        "draft_versions": {"rows": 2, "sha256": "e" * 64},
    }
    after = deepcopy(before)
    after["revision"] += 1
    after["bindings"]["first"]["available"] = False
    after["bindings"]["first"]["tombstoned"] = True
    return before, after


class NativeReadOnlyLineageUnitTests(unittest.TestCase):
    def test_row_digests_ignore_order_without_leaking_text(self):
        a = digest_rows([("source-1", "private transcript"), ("source-2", "other")])
        b = digest_rows([("source-2", "other"), ("source-1", "private transcript")])
        self.assertEqual(a, b)
        self.assertEqual(a["rows"], 2)
        self.assertNotIn("private transcript", json.dumps(a))

    def test_only_canonical_tombstone_preserves_unrelated_source_and_history(self):
        before, after = snapshots()
        proof = verify_revocation(before, after, "first", deepcopy(after))
        self.assertEqual(proof["canonical_tombstone_and_history"], "OBSERVED")
        self.assertEqual(proof["reopen_comparison"], "OBSERVED")
        self.assertEqual(proof["stale_native_action_denial"], "NOT TESTED")
        self.assertEqual(proof["source_revocation_native_certification"], "NOT TESTED")

    def test_forged_revision_or_wrong_tombstone_is_rejected(self):
        before, after = snapshots()
        for mutate in (
            lambda x: x.update(revision=3),
            lambda x: x["bindings"]["first"].update(tombstoned=False),
            lambda x: x["bindings"]["first"].update(available=True),
            lambda x: x["bindings"]["second"].update(available=False),
            lambda x: x["bindings"]["second"].update(fingerprint="changed"),
            lambda x: x["bindings"]["first"]["history"]["study_evidence"].update(rows=4),
            lambda x: x["draft_versions"].update(rows=0),
            lambda x: x.update(study_id_sha256="e" * 64),
            lambda x: x["transcripts"].append({"transcript_id": "third"}),
        ):
            bad = deepcopy(after)
            mutate(bad)
            with self.subTest(mutate=mutate), self.assertRaises(LineageUnavailable):
                verify_revocation(before, bad, "first")

    def test_reopen_resurrection_is_rejected(self):
        before, after = snapshots()
        reopened = deepcopy(after)
        reopened["bindings"]["first"]["available"] = True
        with self.assertRaisesRegex(LineageUnavailable, "Reopened"):
            verify_revocation(before, after, "first", reopened)

    def test_missing_installed_profile_can_never_be_certified(self):
        with tempfile.TemporaryDirectory() as directory:
            base = Path(directory)
            manifest = {
                "purpose": "native-two-real-media-preparation-only",
                "sources": [],
            }
            source = base / "origin-manifest.json"
            source.write_text(json.dumps(manifest), encoding="utf-8")
            with self.assertRaisesRegex(LineageUnavailable, "two independent"):
                read_installed_lineage(base / "missing.sqlite3", source, base, "A", "study")

    def test_duplicate_media_sha_and_tampered_bytes_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            base = Path(directory)
            origins = []
            for i in (1, 2):
                path = base / ("source" + str(i) + ".webm")
                path.write_bytes(bytes([i]) * 40000)
                sha = hashlib.sha256(path.read_bytes()).hexdigest()
                origins.append({
                    "source_id": "commons:origin-" + str(i),
                    "original_url": "https://upload.wikimedia.org/wikipedia/commons/"
                                    + str(i) + "/origin.ogg",
                    "original_sha256": hashlib.sha256(bytes([i])).hexdigest(),
                    "webm_sha256": sha,
                    "webm_filename": path.name,
                })
            manifest_path = base / "origin-manifest.json"
            payload = {
                "purpose": "native-two-real-media-preparation-only",
                "sources": origins,
            }
            manifest_path.write_text(json.dumps(payload), encoding="utf-8")
            # Only a fixture-byte preflight; arbitrary unit-test bytes do NOT
            # establish real media or pass any installed application scenario.
            self.assertEqual(len(expected_media(manifest_path, base)), 2)
            origins[1]["webm_sha256"] = origins[0]["webm_sha256"]
            manifest_path.write_text(json.dumps(payload), encoding="utf-8")
            with self.assertRaises(LineageUnavailable):
                expected_media(manifest_path, base)
            origins[1]["webm_sha256"] = hashlib.sha256(
                (base / "source2.webm").read_bytes()).hexdigest()
            manifest_path.write_text(json.dumps(payload), encoding="utf-8")
            (base / "source2.webm").write_bytes(b"changed" * 6000)
            with self.assertRaisesRegex(LineageUnavailable, "differ"):
                expected_media(manifest_path, base)


if __name__ == "__main__":
    unittest.main()
