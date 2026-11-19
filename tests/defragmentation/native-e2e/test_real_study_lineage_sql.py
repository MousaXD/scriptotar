"""Synthetic SQL regression only; never an installed UI or transcription PASS.

These rows exist exclusively in a temporary unit-test DB to prove that the
read-only auditor catches missing/wrong lineage. Actual native acceptance
must produce its own independent jobs, real WebM media, sidecar output and DB.
"""
from __future__ import annotations

import hashlib
import json
from pathlib import Path
import sqlite3
import tempfile
import unittest

from real_study_lineage import LineageUnavailable, read_installed_lineage, verify_revocation


SCHEMA = """
CREATE TABLE content_studies (id TEXT, project_id TEXT, revision INTEGER, archived_at TEXT);
CREATE TABLE jobs (id TEXT, project_id TEXT, input_kind TEXT, input_value TEXT, state TEXT);
CREATE TABLE job_transcript_links (job_id TEXT, transcript_id TEXT);
CREATE TABLE transcripts (id TEXT, media_id TEXT, text TEXT, segments_json TEXT);
CREATE TABLE media (id TEXT, source_id TEXT);
CREATE TABLE sources (id TEXT, project_id TEXT, source_type TEXT);
CREATE TABLE study_sources (
 id TEXT, project_id TEXT, study_id TEXT, kind TEXT,
 reference_id TEXT, transcript_id TEXT, fingerprint TEXT, locator TEXT
);
CREATE TABLE study_evidence (id TEXT, study_source_id TEXT, claim TEXT);
CREATE TABLE draft_source_links (study_source_id TEXT, draft_id TEXT);
CREATE TABLE study_source_jobs (study_source_id TEXT, job_id TEXT);
CREATE TABLE creative_drafts (id TEXT, project_id TEXT, study_id TEXT, body TEXT);
CREATE TABLE draft_versions (id TEXT, draft_id TEXT, body TEXT);
"""


class SqlReadbackContractUnitTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.db = self.root / "scriptotar.sqlite3"
        self.manifest = self.root / "origin-manifest.json"
        self.project = "test-project"
        self.study = "test-study"
        sources = []
        with sqlite3.connect(self.db) as conn:
            conn.executescript(SCHEMA)
            conn.execute(
                "INSERT INTO content_studies VALUES (?,?,?,NULL)",
                (self.study, self.project, 3),
            )
            for n in (1, 2):
                path = self.root / ("origin" + str(n) + ".webm")
                # NOT a real audio file; unit tests must never call this a
                # native transcription or media fixture.
                path.write_bytes(bytes([n]) * 40000)
                original = self.root / ("origin" + str(n) + ".ogg")
                original.write_bytes(b"OggS" + bytes([n]) * 40000)
                key = str(n)
                sources.append({
                    "source_id": "commons:origin" + key,
                    "original_url": "https://upload.wikimedia.org/wikipedia/commons/"
                                    + key + "/source.ogg",
                    "original_filename": original.name,
                    "original_sha256": hashlib.sha256(original.read_bytes()).hexdigest(),
                    "webm_sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
                    "webm_filename": path.name,
                })
                conn.execute(
                    "INSERT INTO jobs VALUES (?,?,?,?,?)",
                    ("j" + key, self.project, "local_file", str(path), "completed"),
                )
                conn.execute(
                    "INSERT INTO sources VALUES (?,?,?)",
                    ("s" + key, self.project, "local_file"),
                )
                conn.execute("INSERT INTO media VALUES (?,?)", ("m" + key, "s" + key))
                conn.execute(
                    "INSERT INTO transcripts VALUES (?,?,?,?)",
                    ("t" + key, "unit test only " + key,
                     "This is unit test content only.",
                     json.dumps([{"text": "unit test only", "start_seconds": 0,
                                  "end_seconds": 2}])),
                )
                # Correct the intentionally positional media_id: it must
                # refer to the source's media row in the real schema.
                conn.execute(
                    "UPDATE transcripts SET media_id=? WHERE id=?",
                    ("m" + key, "t" + key),
                )
                conn.execute(
                    "INSERT INTO job_transcript_links VALUES (?,?)",
                    ("j" + key, "t" + key),
                )
                conn.execute(
                    "INSERT INTO study_sources VALUES (?,?,?,?,?,?,?,?)",
                    ("ss" + key, self.project, self.study, "transcript",
                     "t" + key, "t" + key, "fingerprint" + key, "locator" + key),
                )
                conn.execute(
                    "INSERT INTO study_evidence VALUES (?,?,?)",
                    ("e" + key, "ss" + key, "unit-only evidence"),
                )
            conn.execute(
                "INSERT INTO creative_drafts VALUES (?,?,?,?)",
                ("draft", self.project, self.study, "unchanged"),
            )
            conn.execute(
                "INSERT INTO draft_versions VALUES (?,?,?)",
                ("version", "draft", "unchanged"),
            )
        self.manifest.write_text(json.dumps({
            "purpose": "native-two-real-media-preparation-only",
            "sources": sources,
        }), encoding="utf-8")

    def read(self):
        return read_installed_lineage(
            self.db, self.manifest, self.root, self.project, self.study,
        )

    def test_readonly_sql_proves_joins_but_never_native_success(self):
        proof = self.read()
        self.assertEqual(len(proof["transcripts"]), 2)
        self.assertEqual(set(proof["bindings"]), {"t1", "t2"})
        self.assertEqual(proof["native_16_case_certification"], "NOT TESTED")
        self.assertEqual(proof["bindings"]["t1"]["history"]["study_evidence"]["rows"], 1)

    def test_real_schema_join_rejects_foreign_project_and_missing_segments(self):
        with sqlite3.connect(self.db) as conn:
            conn.execute("UPDATE sources SET project_id='foreign' WHERE id='s1'")
        with self.assertRaisesRegex(LineageUnavailable, "ownership"):
            self.read()
        with sqlite3.connect(self.db) as conn:
            conn.execute("UPDATE sources SET project_id=? WHERE id='s1'", (self.project,))
            conn.execute("UPDATE transcripts SET segments_json='[]' WHERE id='t1'")
        with self.assertRaisesRegex(LineageUnavailable, "segment"):
            self.read()

    def test_readback_before_after_revocation_preserves_provenance(self):
        before = self.read()
        with sqlite3.connect(self.db) as conn:
            conn.execute("UPDATE content_studies SET revision=4 WHERE id=?", (self.study,))
            conn.execute("UPDATE study_sources SET transcript_id=NULL WHERE id='ss1'")
        after = self.read()
        reopened = self.read()
        # This purely simulated update verifies auditor logic only; it is
        # explicitly NOT a native revocation action or authorization denial.
        proof = verify_revocation(before, after, "t1", reopened)
        self.assertEqual(proof["canonical_tombstone_and_history"], "OBSERVED")
        self.assertEqual(proof["source_revocation_native_certification"], "NOT TESTED")

    def test_prior_provenance_row_rewrite_fails_even_with_same_counts(self):
        before = self.read()
        with sqlite3.connect(self.db) as conn:
            conn.execute("UPDATE content_studies SET revision=4 WHERE id=?", (self.study,))
            conn.execute("UPDATE study_sources SET transcript_id=NULL WHERE id='ss1'")
            conn.execute("UPDATE study_evidence SET claim='tampered' WHERE id='e2'")
        after = self.read()
        with self.assertRaisesRegex(LineageUnavailable, "Historical"):
            verify_revocation(before, after, "t1")


if __name__ == "__main__":
    unittest.main()
