"""Self-tests for the QA fixture generator; synthetic schemas are NOT app migrations."""
from pathlib import Path
import sqlite3
import tempfile
import unittest
from .benchmarks.seed_workspace import seed_workspace

SCHEMA = """
PRAGMA user_version=5;
CREATE TABLE schema_migrations(version INTEGER PRIMARY KEY,name TEXT,applied_at TEXT);
CREATE TABLE projects(id TEXT PRIMARY KEY,name TEXT NOT NULL UNIQUE,created_at TEXT NOT NULL);
CREATE TABLE creators(id TEXT PRIMARY KEY);
CREATE TABLE sources(id TEXT PRIMARY KEY,project_id TEXT REFERENCES projects(id),
 source_type TEXT,locator TEXT,title TEXT,created_at TEXT);
CREATE TABLE media(id TEXT PRIMARY KEY,source_id TEXT REFERENCES sources(id),
 local_path TEXT,created_at TEXT);
CREATE TABLE transcripts(id TEXT PRIMARY KEY,media_id TEXT REFERENCES media(id),
 language TEXT,text TEXT,created_at TEXT,updated_at TEXT);
CREATE TABLE research_items(id TEXT PRIMARY KEY,project_id TEXT REFERENCES projects(id),
 source_url TEXT,platform TEXT,title TEXT,view_count INTEGER,like_count INTEGER,
 comment_count INTEGER,scanned_at TEXT);
CREATE TABLE analyses(id TEXT PRIMARY KEY);
CREATE TABLE ai_runs(id TEXT PRIMARY KEY,project_id TEXT REFERENCES projects(id),
 task TEXT,mode TEXT,prompt TEXT,result TEXT,created_at TEXT);
CREATE TABLE jobs(id TEXT PRIMARY KEY,project_id TEXT REFERENCES projects(id),
 input_kind TEXT,input_value TEXT,state TEXT,created_at TEXT,updated_at TEXT);
"""

def blank_schema(conn):
    conn.executescript(SCHEMA)
    conn.executemany("INSERT INTO schema_migrations VALUES (?,?,?)",
                     [(i, f"migration_{i}", "2026-01-01") for i in range(1, 6)])

class SyntheticFixtureTests(unittest.TestCase):
    def test_creates_disjoint_copy_and_preserves_template(self):
        with tempfile.TemporaryDirectory() as folder:
            original = Path(folder) / "blank.db"
            output = Path(folder) / "synthetic.db"
            with sqlite3.connect(original) as conn:
                blank_schema(conn)
            result = seed_workspace(original, output, 100)
            self.assertEqual(result["record_count_per_collection"], 100)
            self.assertEqual(set(result["tables"].values()), {100})
            with sqlite3.connect(original) as conn:
                self.assertEqual(conn.execute("SELECT COUNT(*) FROM projects").fetchone()[0], 0)
            with sqlite3.connect(output) as conn:
                self.assertEqual(conn.execute("PRAGMA quick_check").fetchone()[0], "ok")
                self.assertEqual(conn.execute("SELECT COUNT(*) FROM transcripts").fetchone()[0], 100)
            with self.assertRaisesRegex(ValueError, "output must not exist"):
                seed_workspace(original, output, 100)

    def test_refuses_private_nonempty_template(self):
        with tempfile.TemporaryDirectory() as folder:
            original = Path(folder) / "private.db"
            output = Path(folder) / "synthetic.db"
            with sqlite3.connect(original) as conn:
                blank_schema(conn)
                conn.execute("INSERT INTO projects VALUES (?,?,?)",
                             ("user", "Private project", "2026-01-01"))
            with self.assertRaisesRegex(ValueError, "template is not empty: projects"):
                seed_workspace(original, output, 100)
            self.assertFalse(output.exists())


    def test_refuses_settings_and_unrelated_data_even_when_projects_empty(self):
        with tempfile.TemporaryDirectory() as folder:
            original = Path(folder) / "not-blank.db"
            output = Path(folder) / "should-not-exist.db"
            with sqlite3.connect(original) as conn:
                blank_schema(conn)
                conn.execute("CREATE TABLE application_settings(singleton INTEGER PRIMARY KEY,settings_json TEXT)")
                conn.execute("INSERT INTO application_settings VALUES(1,?)", ('{"localSecret":"private"}',))
            with self.assertRaisesRegex(ValueError, "template is not empty: application_settings"):
                seed_workspace(original, output, 100)
            self.assertFalse(output.exists())

    def test_refuses_forged_or_unmigrated_schema(self):
        with tempfile.TemporaryDirectory() as folder:
            original = Path(folder) / "unmigrated.db"
            output = Path(folder) / "output.db"
            with sqlite3.connect(original) as conn:
                blank_schema(conn)
                conn.execute("DELETE FROM schema_migrations WHERE version=4")
            with self.assertRaisesRegex(ValueError, "migration ledger"):
                seed_workspace(original, output, 100)
            self.assertFalse(output.exists())

    def test_refuses_nonempty_extra_tables_and_unknown_history(self):
        with tempfile.TemporaryDirectory() as folder:
            original = Path(folder) / "other.db"
            output = Path(folder) / "output.db"
            with sqlite3.connect(original) as conn:
                blank_schema(conn)
                conn.execute("CREATE TABLE new_sensitive_history(id TEXT)")
                conn.execute("INSERT INTO new_sensitive_history VALUES('private')")
            with self.assertRaisesRegex(ValueError, "new_sensitive_history"):
                seed_workspace(original, output, 100)
            self.assertFalse(output.exists())


    def test_native_sampler_counts_main_process_and_children_on_linux(self):
        import os
        import platform
        from .benchmarks.native_session import process_tree_rss
        if platform.system() != "Linux" or not Path("/proc").is_dir():
            self.skipTest("Linux procfs only")
        total, child_count = process_tree_rss(os.getpid())
        self.assertIsInstance(total, int)
        self.assertGreater(total, 0)
        self.assertGreaterEqual(child_count, 0)
        self.assertEqual(process_tree_rss(999999999), (None, 0))

    def test_rejects_arbitrary_scale(self):
        with tempfile.TemporaryDirectory() as folder:
            a, b = Path(folder) / "a.db", Path(folder) / "b.db"
            a.touch()
            with self.assertRaisesRegex(ValueError, "100, 1000 or 10000"):
                seed_workspace(a, b, 99)

if __name__ == "__main__":
    unittest.main()
