"""Self-tests for the QA fixture generator; synthetic schemas are NOT app migrations."""
from pathlib import Path
import sqlite3
import tempfile
import unittest
from .benchmarks.seed_workspace import seed_workspace

SCHEMA = """
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

class SyntheticFixtureTests(unittest.TestCase):
    def test_creates_disjoint_copy_and_preserves_template(self):
        with tempfile.TemporaryDirectory() as folder:
            original = Path(folder) / "blank.db"
            output = Path(folder) / "synthetic.db"
            with sqlite3.connect(original) as conn:
                conn.executescript(SCHEMA)
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
                conn.executescript(SCHEMA)
                conn.execute("INSERT INTO projects VALUES (?,?,?)",
                             ("user", "Private project", "2026-01-01"))
            with self.assertRaisesRegex(ValueError, "personal projects"):
                seed_workspace(original, output, 100)
            self.assertFalse(output.exists())

    def test_rejects_arbitrary_scale(self):
        with tempfile.TemporaryDirectory() as folder:
            a, b = Path(folder) / "a.db", Path(folder) / "b.db"
            a.touch()
            with self.assertRaisesRegex(ValueError, "100, 1000 or 10000"):
                seed_workspace(a, b, 99)

if __name__ == "__main__":
    unittest.main()
