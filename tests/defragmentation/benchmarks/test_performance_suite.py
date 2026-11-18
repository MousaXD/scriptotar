"""Offline hermetic tests; test migration stubs are NOT app-migrated baselines."""
from __future__ import annotations

import json
from contextlib import closing
from pathlib import Path
import sqlite3
import tempfile
import unittest

from .study_scale import build
from .query_suite import profile, percentile
from .ipc_trace import summarize_trace
from .native_trials import summary
from .native_memory import parse_smaps_rollup, descendants, aggregate_samples, memory_snapshot

SHA = "3" * 40
HASH = "a" * 64

# Minimal DDL exercises fixture FKs; real benchmark templates must come from
# scriptotar-db migrations (including indexes/triggers), not these test stubs.
SCHEMA = """
PRAGMA user_version=9;
CREATE TABLE schema_migrations(version INTEGER PRIMARY KEY,name TEXT,applied_at TEXT);
CREATE TABLE integration_schema_migrations(version INTEGER PRIMARY KEY,name TEXT,applied_at TEXT);
CREATE TABLE projects(id TEXT PRIMARY KEY,name TEXT NOT NULL UNIQUE,created_at TEXT NOT NULL);
CREATE TABLE creators(id TEXT PRIMARY KEY);
CREATE TABLE sources(id TEXT PRIMARY KEY,project_id TEXT REFERENCES projects(id),source_type TEXT,locator TEXT,title TEXT,created_at TEXT);
CREATE TABLE media(id TEXT PRIMARY KEY,source_id TEXT REFERENCES sources(id),local_path TEXT,created_at TEXT);
CREATE TABLE transcripts(id TEXT PRIMARY KEY,media_id TEXT REFERENCES media(id),language TEXT,text TEXT,created_at TEXT,updated_at TEXT);
CREATE TABLE research_items(id TEXT PRIMARY KEY,project_id TEXT REFERENCES projects(id),source_url TEXT,platform TEXT,title TEXT,view_count INTEGER,like_count INTEGER,comment_count INTEGER,scanned_at TEXT);
CREATE TABLE analyses(id TEXT PRIMARY KEY);
CREATE TABLE ai_runs(id TEXT PRIMARY KEY,project_id TEXT REFERENCES projects(id),task TEXT,mode TEXT,prompt TEXT,result TEXT,created_at TEXT);
CREATE TABLE jobs(id TEXT PRIMARY KEY,project_id TEXT REFERENCES projects(id),input_kind TEXT,input_value TEXT,state TEXT,created_at TEXT,updated_at TEXT);
CREATE TABLE content_studies(id TEXT PRIMARY KEY,project_id TEXT REFERENCES projects(id),title TEXT,goal TEXT,status TEXT,revision INTEGER,created_at TEXT,updated_at TEXT,archived_at TEXT);
CREATE TABLE study_sources(id TEXT PRIMARY KEY,project_id TEXT REFERENCES projects(id),study_id TEXT REFERENCES content_studies(id),kind TEXT,reference_id TEXT,transcript_id TEXT REFERENCES transcripts(id),locator TEXT,fingerprint TEXT,collected_at TEXT,created_at TEXT);
CREATE TABLE study_briefs(study_id TEXT PRIMARY KEY REFERENCES content_studies(id),project_id TEXT REFERENCES projects(id),audience TEXT,goal TEXT,language TEXT,platform TEXT,tone TEXT,duration_seconds INTEGER,objective TEXT,constraints_json TEXT,revision INTEGER,updated_at TEXT);
CREATE TABLE creative_drafts(id TEXT PRIMARY KEY,project_id TEXT REFERENCES projects(id),study_id TEXT REFERENCES content_studies(id),title TEXT,body TEXT,revision INTEGER,created_at TEXT,updated_at TEXT,archived_at TEXT);
CREATE TABLE draft_versions(id TEXT PRIMARY KEY,draft_id TEXT REFERENCES creative_drafts(id),version INTEGER,title TEXT,body TEXT,saved_at TEXT);
"""


def blank(path: Path) -> None:
    with closing(sqlite3.connect(path)) as db, db:
        db.executescript(SCHEMA)
        db.executemany("INSERT INTO schema_migrations VALUES (?,?,?)",
                       [(i, f"migration-{i}", "2026-01-01") for i in range(1, 10)])
        names = ("legacy_import_support", "job_transcript_lineage",
                 "safe_job_transcript_backfill", "job_transcript_project_guard",
                 "parent_lineage_reassignment_guards")
        db.executemany("INSERT INTO integration_schema_migrations VALUES (?,?,?)",
                       [(i, name, "2026-01-01") for i, name in enumerate(names, 1)])


class StudyPerformanceSuiteTests(unittest.TestCase):
    def test_fixture_and_profiles_do_not_modify_template(self):
        with tempfile.TemporaryDirectory() as folder:
            base = Path(folder) / "template.db"
            generated = Path(folder) / "fixture.db"
            blank(base)
            before = base.read_bytes()
            result = build(base, generated, 100, SHA)
            self.assertEqual(result["schema_version"], 9)
            self.assertEqual(result["per_collection"], 100)
            self.assertEqual(before, base.read_bytes())
            with closing(sqlite3.connect(generated)) as db, db:
                for table in ("content_studies", "study_sources", "study_briefs", "creative_drafts", "draft_versions"):
                    self.assertEqual(db.execute(f"SELECT COUNT(*) FROM {table}").fetchone()[0], 100)
                self.assertEqual(db.execute("PRAGMA foreign_key_check").fetchall(), [])
            report = profile(generated, SHA, 3)
            self.assertEqual(report["certification"], "NOT_NATIVE_CERTIFIED")
            self.assertEqual(report["query_profiles"]["study_search_200_metadata"]["rows_returned"], 100)
            self.assertGreater(report["query_profiles"]["transcript_full_body_1mb"]["fetched_field_bytes_estimate"], 1_000_000)
            self.assertEqual(len(report["query_profiles"]["draft_selected_body"]["samples_ms"]), 3)
            self.assertEqual(result["db_sha256"], report["database_sha256"])
            with self.assertRaisesRegex(ValueError, "output must not exist"):
                build(base, generated, 100, SHA)

    def test_never_seed_private_data_or_without_studies(self):
        with tempfile.TemporaryDirectory() as folder:
            base = Path(folder) / "template.db"
            generated = Path(folder) / "fixture.db"
            blank(base)
            with closing(sqlite3.connect(base)) as db, db:
                db.execute("INSERT INTO projects VALUES (?,?,?)", ("secret-id", "SECRET", "2026"))
            with self.assertRaisesRegex(ValueError, "template is not empty"):
                build(base, generated, 100, SHA)
            self.assertFalse(generated.exists())
            with closing(sqlite3.connect(base)) as db, db:
                db.execute("DELETE FROM projects")
                db.execute("DROP TABLE content_studies")
            with self.assertRaisesRegex(ValueError, "requires actual Scriptotar v6"):
                build(base, generated, 100, SHA)
            self.assertFalse(generated.exists())

    def test_migrated_integration_ledger_is_allowed_but_forgery_is_refused(self):
        with tempfile.TemporaryDirectory() as folder:
            base = Path(folder) / "template.db"
            first = Path(folder) / "first.db"
            second = Path(folder) / "second.db"
            blank(base)
            result = build(base, first, 100, SHA)
            self.assertEqual(result["schema_version"], 9)
            with closing(sqlite3.connect(base)) as db, db:
                db.execute("UPDATE integration_schema_migrations SET name='forged' WHERE version=3")
            with self.assertRaisesRegex(ValueError, "integration migration ledger"):
                build(base, second, 100, SHA)
            self.assertFalse(second.exists())

    def test_query_refuses_nonfixture_and_bad_sha(self):
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder) / "blank.db"
            blank(path)
            with self.assertRaisesRegex(ValueError, "non-synthetic"):
                profile(path, SHA, 3)
            with self.assertRaisesRegex(ValueError, "40-hex"):
                profile(path, "INVALID", 3)

    def test_percentiles_and_missing_rss_are_explicit(self):
        self.assertEqual(percentile([10, 7, 5, 3, 1], 0.95), 10.0)
        self.assertEqual(summary([None, None]), {"valid_trials": 0, "p50": None, "p95_nearest_rank": None})

    def test_memory_rollup_bytes_and_fail_closed_shared_accounting(self):
        observed = parse_smaps_rollup(
            "Rss: 50 kB\nPss: 30 kB\nPrivate_Clean: 7 kB\nPrivate_Dirty: 9 kB\n"
        )
        self.assertEqual(observed, {"pss_bytes": 30 * 1024,
                                    "private_resident_bytes": 16 * 1024})
        self.assertIsNone(parse_smaps_rollup("Rss: 10 kB\n")["pss_bytes"])
        self.assertIsNone(parse_smaps_rollup("Pss: 10 kB\n")["private_resident_bytes"])
        pids = descendants(10, {10: 1, 11: 10, 12: 11, 88: 3})
        self.assertEqual(pids, {10, 11, 12})
        samples = {10: {"pss_bytes": 9000}, 11: {"pss_bytes": 2000},
                   12: {"pss_bytes": 3000}}
        self.assertEqual(aggregate_samples(samples, pids, "pss_bytes"), 14000)
        samples[11]["pss_bytes"] = None
        self.assertIsNone(aggregate_samples(samples, pids, "pss_bytes"))
        self.assertIsNone(aggregate_samples(samples, set(), "pss_bytes"))

    def test_local_process_memory_snapshot_is_not_native_evidence(self):
        import os
        import platform
        record = memory_snapshot(os.getpid())
        self.assertEqual(record["provider"].startswith("linux-") if platform.system() == "Linux"
                         else record["provider"].startswith("windows-")
                         if platform.system() == "Windows" else record["provider"] == "unsupported", True)
        self.assertNotIn("certification", record)
        self.assertNotIn("studio_ready_ms", record)

    def test_mock_ipc_is_rejected_and_only_declared_native_input_is_summarized(self):
        with tempfile.TemporaryDirectory() as folder:
            trace = Path(folder) / "trace.jsonl"
            observations = []
            for index in range(3):
                observations.append({"evidence_kind": "chromium-mock", "source_sha": SHA,
                                     "package_sha256": HASH, "executable_sha256": HASH,
                                     "session_id": "session-native", "command": "get_studio_study",
                                     "duration_ms": 12 + index, "request_bytes": 32,
                                     "response_bytes": 250})
            trace.write_text("\n".join(json.dumps(x) for x in observations))
            with self.assertRaisesRegex(ValueError, "not an explicitly native"):
                summarize_trace(trace, SHA, HASH, HASH)
            for event in observations:
                event["evidence_kind"] = "installed-tauri-webview-ipc"
            trace.write_text("\n".join(json.dumps(x) for x in observations))
            result = summarize_trace(trace, SHA, HASH, HASH)
            self.assertEqual(result["certification"], "PENDING_INDEPENDENT_NATIVE_QA")
            self.assertEqual(result["commands"]["get_studio_study"]["duration_ms_p95_nearest_rank"], 14.0)
            with self.assertRaisesRegex(ValueError, "hash mismatch"):
                summarize_trace(trace, SHA, "b" * 64, HASH)


if __name__ == "__main__":
    unittest.main()
