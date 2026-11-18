#!/usr/bin/env python3
"""Agent 10: adversarial SQLite snapshot-isolation security evidence regression.

This is a deterministic SOURCE-LEVEL contract test for the Agent 07 native QA
evidence collector, not an installed WebKitGTK/WebView2 PASS. Two canonical tables
are atomically updated by a different SQLite connection between the collector's
per-table reads. The collected digest must represent one committed database state,
never a torn combination. The intended fix is a single read-only SQLite transaction
covering schema enumeration and *all* table scans; do not weaken this assertion.
"""
from pathlib import Path
from unittest import TestCase, mock
import runpy
import sqlite3
import tempfile
import sys

ROOT = Path(__file__).resolve().parents[3]
# Use the exact SHA-pinned Agent 07 checkout supplied by the independent gate.
# Do not silently use the integration checkout, which may lack this helper.
NATIVE = Path(sys.argv[1]).resolve() if __name__ == "__main__" and len(sys.argv) == 2 else None


class NativeCanonicalSnapshotAtomicity(TestCase):
    @classmethod
    def setUpClass(cls):
        if NATIVE is None or not NATIVE.is_file():
            raise AssertionError("Missing exact source-pinned Agent 07 native runner")
        cls.native = runpy.run_path(str(NATIVE), run_name="_agent10_snapshot_audit")
        cls.snapshot = cls.native["canonical_records_snapshot"]

    def test_canonical_snapshot_cannot_mix_two_committed_states(self):
        # WAL allows a genuine concurrent writer while a read transaction holds
        # its original snapshot. An implicit, statement-only SELECT does not.
        with tempfile.TemporaryDirectory(prefix="agent10-native-snapshot-") as td:
            data = Path(td)
            database = data / "scriptotar.sqlite3"
            original_connect = sqlite3.connect
            with original_connect(database) as setup:
                setup.execute("PRAGMA journal_mode=WAL")
                setup.executescript("""
                    CREATE TABLE content_studies (revision INTEGER NOT NULL);
                    CREATE TABLE jobs (revision INTEGER NOT NULL);
                    INSERT INTO content_studies VALUES (1);
                    INSERT INTO jobs VALUES (1);
                """)
            before = self.snapshot(data)
            writer = original_connect(database, timeout=5)
            try:
                writer.execute("UPDATE content_studies SET revision=2")
                writer.execute("UPDATE jobs SET revision=2")
                writer.commit()
                after = self.snapshot(data)
                self.assertNotEqual(before["digest"], after["digest"],
                                    "Fixture must model a genuine changed database")
                writer.execute("UPDATE content_studies SET revision=1")
                writer.execute("UPDATE jobs SET revision=1")
                writer.commit()

                switched = {"once": False}

                class InterleavedReader(sqlite3.Connection):
                    def execute(self, sql, parameters=(), /):
                        if sql.startswith("SELECT * FROM "):
                            if getattr(self, "_agent10_first_table", False) and not switched["once"]:
                                # This transaction changes BOTH tables at once.
                                # No valid committed state ever contains one old
                                # table and one new table.
                                writer.execute("UPDATE content_studies SET revision=2")
                                writer.execute("UPDATE jobs SET revision=2")
                                writer.commit()
                                switched["once"] = True
                            self._agent10_first_table = True
                        return super().execute(sql, parameters)

                def interleaved_connect(*args, **kwargs):
                    kwargs = dict(kwargs)
                    kwargs["factory"] = InterleavedReader
                    return original_connect(*args, **kwargs)

                with mock.patch.object(sqlite3, "connect",
                                       side_effect=interleaved_connect):
                    observed = self.snapshot(data)

                self.assertTrue(switched["once"],
                                "The independent writer must actually execute between table reads")
                self.assertEqual(self.snapshot(data)["digest"], after["digest"],
                                 "The transaction must actually have committed both new values")
                self.assertIn(
                    observed["digest"], {before["digest"], after["digest"]},
                    "FAIL: Native QA canonical digest combines old and new table contents "
                    "from two committed states. A read-only mode=ro connection is not "
                    "itself a multi-table snapshot transaction. BEGIN a single reader "
                    "transaction before enumerating/reading canonical SQLite tables."
                )
            finally:
                writer.close()


if __name__ == "__main__":
    import unittest
    if NATIVE is None:
        raise SystemExit("Usage: native_sqlite_snapshot_atomicity_probe.py <exact-Agent07-linux_webkit.py>")
    unittest.main(argv=[sys.argv[0]], verbosity=2)
