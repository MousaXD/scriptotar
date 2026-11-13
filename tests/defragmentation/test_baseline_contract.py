"""Static change detectors and fixture validation; NOT feature or native certification."""
import json
import re
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]
FIXTURES = Path(__file__).resolve().parent / "fixtures"
EXPECTED_BOUNDARIES = {
    "project", "research", "jobs", "transcript", "ai", "library",
    "mcp", "security", "draft", "ui", "agent", "writing", "evidence", "performance"
}

def load(name):
    return json.loads((FIXTURES / name).read_text(encoding="utf-8"))

class BaselineContractTests(unittest.TestCase):
    def test_source_feature_parity_footprint(self):
        rows = load("feature_parity.json")["items"]
        self.assertGreaterEqual(len(rows), 15)
        self.assertEqual(len({row["id"] for row in rows}), len(rows))
        for row in rows:
            with self.subTest(feature=row["id"]):
                path = Path(row["path"])
                self.assertFalse(path.is_absolute())
                self.assertNotIn("..", path.parts)
                file = ROOT / path
                self.assertTrue(file.is_file(), f"Removed feature source: {path}")
                source = file.read_text(encoding="utf-8")
                for marker in row["markers"]:
                    self.assertIn(marker, source, f"Review {row['id']} before removing {marker}")
                self.assertEqual(row["verification"], "source-presence-only")

    def test_acceptance_coverage_is_explicitly_unexecuted(self):
        payload = load("acceptance_cases.json")
        cases = payload["cases"]
        self.assertGreaterEqual(len(cases), 25)
        self.assertEqual(len({row["id"] for row in cases}), len(cases))
        self.assertEqual({row["phase"] for row in cases}, {"P0", "P1", "P2", "P3"})
        self.assertTrue(EXPECTED_BOUNDARIES.issubset({row["surface"] for row in cases}))
        for row in cases:
            with self.subTest(case=row["id"]):
                self.assertTrue(all(isinstance(row[key], str) and len(row[key]) > 8
                                    for key in ("steps", "expected")))
                self.assertIn(row["environment"], {
                    "native-installed", "rust-ipc", "browser-mock", "mcp-e2e",
                    "agent-fixture", "research-fixture", "manual-native"
                })
                self.assertEqual(row["automation"], "NOT_WIRED")
                self.assertEqual(row["result"], "NOT_RUN")
                self.assertTrue(re.fullmatch(r"p[0-3]-[a-z0-9-]+", row["id"]))

    def test_native_evidence_gate_refuses_missing_proof(self):
        from .verify_evidence import verify
        record = {"source_sha": "a" * 40, "scope": "native-installed",
                  "outcome": "PASS", "run_url": "https://github.com/MousaXD/scriptotar/actions/runs/1",
                  "observed_at": "2026-11-13T00:00:00Z",
                  "measurements": {"cold_start_ms": 5}}
        with self.assertRaisesRegex(ValueError, "package_sha256"):
            verify(record)
        record.update(package_sha256="b" * 64, executable_sha256="c" * 64,
                      hardware={"os": "Linux", "cpu": "fixture", "ram_bytes": 1},
                      method="fixture", native_ui_probe="browser-mock")
        with self.assertRaisesRegex(ValueError, "native UI"):
            verify(record)
        self.assertEqual(verify({"source_sha": "a" * 40, "scope": "native-installed",
                                 "outcome": "NOT_RUN"}), "NOT_RUN")

if __name__ == "__main__":
    unittest.main()
