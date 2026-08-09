import sqlite3
import tempfile
import unittest
from pathlib import Path

import scriptotar
import persistence_mixin


class DatabaseMigrationTests(unittest.TestCase):
    def test_v11_jobs_database_gets_project_and_new_tables(self):
        with tempfile.TemporaryDirectory() as tmp:
            db_path = Path(tmp) / "history.sqlite3"
            with sqlite3.connect(db_path) as db:
                db.execute(
                    """CREATE TABLE jobs (
                        id TEXT PRIMARY KEY,
                        created_at TEXT NOT NULL,
                        source TEXT NOT NULL,
                        input_type TEXT NOT NULL,
                        title TEXT,
                        status TEXT NOT NULL,
                        language TEXT,
                        output_dir TEXT,
                        transcript TEXT,
                        error TEXT
                    )"""
                )
                db.execute(
                    "INSERT INTO jobs(id,created_at,source,input_type,status) VALUES(?,?,?,?,?)",
                    ("old", "2026-08-09 00:00:00", "file.mp4", "file", "Done"),
                )
            old = persistence_mixin.DB_FILE
            try:
                persistence_mixin.DB_FILE = db_path
                scriptotar.App._init_db(object())
            finally:
                persistence_mixin.DB_FILE = old
            with sqlite3.connect(db_path) as db:
                columns = {row[1] for row in db.execute("PRAGMA table_info(jobs)")}
                self.assertIn("project", columns)
                self.assertEqual(db.execute("SELECT project FROM jobs WHERE id='old'").fetchone()[0], "Inbox")
                tables = {row[0] for row in db.execute("SELECT name FROM sqlite_master WHERE type='table'")}
                self.assertTrue({"projects", "research_items", "watchlists", "ai_runs"}.issubset(tables))


if __name__ == "__main__":
    unittest.main()
