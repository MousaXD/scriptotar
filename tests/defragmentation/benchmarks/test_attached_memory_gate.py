"""Synthetic parser negatives; not evidence from an installed WebView PID."""
from __future__ import annotations

import copy
from pathlib import Path
import tempfile
import unittest

from .attached_memory_gate import preflight
from .native_session import sha256

SHA = "3" * 40
URL = "https://github.com/MousaXD/scriptotar/actions/runs/123456"


class AttachedMemoryGateTests(unittest.TestCase):
    def setUp(self):
        folder = tempfile.TemporaryDirectory()
        self.addCleanup(folder.cleanup)
        self.pkg = Path(folder.name) / "sample-not-installed.deb"
        self.exe = Path(folder.name) / "sample-not-installed-exe"
        self.pkg.write_bytes(b"synthetic package bytes")
        self.exe.write_bytes(b"synthetic executable bytes")
        self.pkg_hash = sha256(self.pkg)
        self.exe_hash = sha256(self.exe)
        self.report = {
            "scope": "ATTACHED_INSTALLED_PROCESS_MEMORY_ONLY_NOT_UI_CERTIFICATION",
            "certification": "PENDING_INDEPENDENT_NATIVE_QA",
            "source_sha_operator_supplied": SHA,
            "source_run_url_operator_supplied": URL,
            "package_path": str(self.pkg), "executable_path": str(self.exe),
            "package_sha256": self.pkg_hash, "executable_sha256": self.exe_hash,
            "platform": "Linux-6.8.0", "pid": 1234,
            "kernel_process_creation_id": "linux-4567",
            "end_reason": "duration_elapsed",
            "measurement_started_monotonic_ns": 10_000_000_000,
            "measurement_finished_monotonic_ns": 11_100_000_000,
            "requested_duration_seconds": 1.0,
            "monotonic_clock": "python.time.perf_counter_ns",
            "study_count_operator_supplied": 100,
            "sample_count": 3, "complete_tree_sample_count": 3,
            "raw_samples": [
                {"provider": "linux-proc-smaps_rollup",
                 "sample_started_monotonic_ns": 10_000_000_000 + i * 300_000_000,
                 "sample_finished_monotonic_ns": 10_100_000_000 + i * 300_000_000,
                 "sampled_process_count": 2, "tree_memory_complete": True,
                 "main_pss_bytes": 1000 + i, "tree_pss_bytes": 2000 + i,
                 "main_private_resident_bytes": 500 + i,
                 "tree_private_resident_bytes": 1500 + i}
                for i in range(3)
            ],
        }

    def check(self, report=None):
        return preflight(report if report is not None else self.report,
                         self.pkg, self.exe, SHA, URL, self.pkg_hash, self.exe_hash)

    def test_full_consistent_memory_preflight_still_never_certifies_native(self):
        result = self.check()
        self.assertEqual(result["sample_count"], 3)
        self.assertEqual(result["uncertified_memory_sample_percentiles"]
                         ["main_pss_bytes"]["p95_bytes_nearest_rank"], 1002)
        self.assertIn("NOT_CERTIFIED", result["status"])
        self.assertEqual(result["study_count_operator_supplied_not_verified"], 100)
        self.assertNotIn("gui_p95_ms", result)
        self.assertNotIn("startup_p95_ms", result)

    def test_partial_samples_pid_reuse_and_time_replay_are_not_accepted(self):
        for field, bad_value, error in (
            ("end_reason", "process_gone_or_identity_changed", "prematurely"),
            ("complete_tree_sample_count", 2, "incomplete"),
            ("sample_count", 4, "inconsistent"),
            ("source_sha_operator_supplied", "a" * 40, "identity mismatch"),
            ("measurement_finished_monotonic_ns", 10_050_000_000, "truncated"),
            ("kernel_process_creation_id", "missing", "creation identity"),
            ("monotonic_clock", "time.time", "sampling clock"),
        ):
            with self.subTest(field=field):
                bad = copy.deepcopy(self.report)
                bad[field] = bad_value
                with self.assertRaisesRegex(ValueError, error):
                    self.check(bad)
        edits = (
            ("tree_memory_complete", False, "incomplete"),
            ("main_pss_bytes", None, "missing/invalid"),
            ("sample_started_monotonic_ns", 10_000_000_000, "overlapping"),
            ("provider", "chromium-mock", "provider"),
        )
        for key, value, error in edits:
            with self.subTest(sample_field=key):
                bad = copy.deepcopy(self.report)
                bad["raw_samples"][1][key] = value
                with self.assertRaisesRegex(ValueError, error):
                    self.check(bad)

    def test_original_package_and_executable_digests_are_rechecked(self):
        self.pkg.write_bytes(b"replaced package")
        with self.assertRaisesRegex(ValueError, "package.*checksum"):
            self.check()
        self.pkg.write_bytes(b"synthetic package bytes")
        self.exe.write_bytes(b"replaced executable")
        with self.assertRaisesRegex(ValueError, "executable.*checksum"):
            self.check()

    def test_windows_private_memory_is_not_linux_pss(self):
        bad = copy.deepcopy(self.report)
        bad["platform"] = "Windows-10.0.20348"
        bad["kernel_process_creation_id"] = "windows-654321"
        for row in bad["raw_samples"]:
            row["provider"] = "windows-psapi-process_memory_counters_ex2"
            row["main_private_working_set_bytes"] = 900
            row["tree_private_working_set_bytes"] = 1200
            row["main_private_commit_bytes"] = 1100
            row["tree_private_commit_bytes"] = 1400
        result = self.check(bad)
        self.assertIn("tree_private_commit_bytes", result["uncertified_memory_sample_percentiles"])
        self.assertNotIn("tree_pss_bytes", result["uncertified_memory_sample_percentiles"])
        bad["raw_samples"][0]["tree_private_working_set_bytes"] = None
        with self.assertRaisesRegex(ValueError, "missing/invalid"):
            self.check(bad)


if __name__ == "__main__":
    unittest.main()
