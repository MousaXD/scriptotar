"""Synthetic parser negatives; not evidence from an installed WebView PID."""
from __future__ import annotations

import copy
import json
from pathlib import Path
import tempfile
import unittest

from .attached_memory_gate import (preflight, load_attached_report,
                                   MAX_ATTACHED_REPORT_BYTES, MAX_ATTACHED_SAMPLES)
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
            "interval_seconds": 0.25,
            "monotonic_clock": "python.time.perf_counter_ns",
            "study_count_operator_supplied": 100,
            "sample_count": 3, "complete_tree_sample_count": 3,
            "raw_samples": [
                {"provider": "linux-proc-smaps_rollup",
                 "elapsed_ms": i * 300.0,
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

    def test_external_attached_json_is_bounded_and_duplicate_keys_fail_closed(self):
        # These are hostile synthetic FILES. No installed PID, package or
        # native accessibility measurement is attested by this parser.
        trace = self.pkg.parent / "hostile-memory-report.json"
        baseline = json.dumps(self.report)
        trace.write_text(baseline, encoding="utf-8")
        parsed = load_attached_report(trace)
        self.assertEqual(parsed["sample_count"], 3)
        self.assertIn("NOT_CERTIFIED", self.check(parsed)["status"])

        # Without object_pairs_hook, both contradictory orderings silently
        # normalize into just the last claimed source SHA.
        original = json.dumps(SHA)
        poison = json.dumps("f" * 40)
        for prefix, suffix in ((poison, original), (original, poison)):
            with self.subTest(order=(prefix, suffix)):
                conflicting = (
                    baseline[:-1] + ', "source_sha_operator_supplied": ' + prefix
                    + ', "source_sha_operator_supplied": ' + suffix + "}"
                )
                self.assertEqual(json.loads(conflicting)["source_sha_operator_supplied"],
                                 json.loads(suffix))
                trace.write_text(conflicting, encoding="utf-8")
                with self.assertRaisesRegex(ValueError, "duplicate JSON member"):
                    load_attached_report(trace)

        # Nested sample metadata is audited too, not only top-level fields.
        nested = copy.deepcopy(self.report)
        nested_line = json.dumps(nested["raw_samples"][0])
        duplicated_sample = nested_line[:-1] + ', "provider": "chromium-mock"}'
        nested_raw = ('"raw_samples": [' + duplicated_sample + ', '
                      + ', '.join(json.dumps(x) for x in nested["raw_samples"][1:])
                      + ']')
        raw_other_fields = {k: v for k, v in nested.items() if k != "raw_samples"}
        trace.write_text(json.dumps(raw_other_fields)[:-1] + ", " + nested_raw + "}",
                         encoding="utf-8")
        with self.assertRaisesRegex(ValueError, "duplicate JSON member"):
            load_attached_report(trace)

        # Unicode escape sequences resolve to the same decoded key; deny
        # them too, even when the repeated value is byte-for-byte identical.
        decoded_equivalent = (
            baseline[:-1] + ', "source_\\u0073ha_operator_supplied": ' + original + "}"
        )
        trace.write_text(decoded_equivalent, encoding="utf-8")
        with self.assertRaisesRegex(ValueError, "duplicate JSON member"):
            load_attached_report(trace)

        trace.write_bytes(b" " * (MAX_ATTACHED_REPORT_BYTES + 1))
        with self.assertRaisesRegex(ValueError, "bounded byte limit"):
            load_attached_report(trace)
        trace.write_bytes(b"\xff")
        with self.assertRaisesRegex(ValueError, "must be UTF-8"):
            load_attached_report(trace)
        for forbidden in ("NaN", "Infinity", "-Infinity"):
            with self.subTest(nonfinite=forbidden):
                poisoned = baseline[:-1] + ', "untrusted_extra": ' + forbidden + "}"
                trace.write_text(poisoned, encoding="utf-8")
                with self.assertRaisesRegex(ValueError, "nonfinite JSON constant"):
                    load_attached_report(trace)

        too_many = copy.deepcopy(self.report)
        too_many["raw_samples"] = too_many["raw_samples"][:1] * (MAX_ATTACHED_SAMPLES + 1)
        too_many["sample_count"] = MAX_ATTACHED_SAMPLES + 1
        too_many["complete_tree_sample_count"] = MAX_ATTACHED_SAMPLES + 1
        trace.write_text(json.dumps(too_many), encoding="utf-8")
        with self.assertRaisesRegex(ValueError, "bounded sample count"):
            load_attached_report(trace)
        with self.assertRaisesRegex(ValueError, "inconsistent counts"):
            self.check(too_many)

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

    def test_sparse_cadence_unbounded_tail_and_elapsed_replay_are_rejected(self):
        # Three genuine-looking early samples cannot certify 180 seconds.
        bad = copy.deepcopy(self.report)
        bad["requested_duration_seconds"] = 180.0
        bad["measurement_finished_monotonic_ns"] = 190_000_000_000
        with self.assertRaisesRegex(ValueError, "sparse memory sampling cadence"):
            self.check(bad)

        bad = copy.deepcopy(self.report)
        bad["raw_samples"][1]["sample_started_monotonic_ns"] = 14_000_000_000
        bad["raw_samples"][1]["sample_finished_monotonic_ns"] = 14_100_000_000
        bad["raw_samples"][1]["elapsed_ms"] = 4000.0
        bad["requested_duration_seconds"] = 10.0
        bad["measurement_finished_monotonic_ns"] = 20_000_000_000
        with self.assertRaisesRegex(ValueError, "sparse memory sampling cadence"):
            self.check(bad)

        for interval in (None, 0, 2.01, float("nan")):
            with self.subTest(interval=interval):
                bad = copy.deepcopy(self.report)
                bad["interval_seconds"] = interval
                with self.assertRaisesRegex(ValueError, "native sampler interval"):
                    self.check(bad)

        for elapsed_ms in (None, -1, float("nan"), 9000.0):
            with self.subTest(elapsed_ms=elapsed_ms):
                bad = copy.deepcopy(self.report)
                bad["raw_samples"][1]["elapsed_ms"] = elapsed_ms
                with self.assertRaisesRegex(ValueError, "elapsed/monotonic timestamp"):
                    self.check(bad)

    def test_root_cpu_counters_are_optional_complete_monotonic_and_noncertifying(self):
        # Existing pure-memory archive remains usable without optional CPU.
        self.assertEqual(self.check()["root_process_cpu_evidence"]["status"], "NOT_MEASURED")
        valid = copy.deepcopy(self.report)
        for i, row in enumerate(valid["raw_samples"]):
            row["main_cpu_time_ns"] = 1_000_000_000 + i * 100_000_000
            row["main_cpu_time_observed_monotonic_ns"] = (
                row["sample_started_monotonic_ns"] + 50_000_000
            )
        cpu = self.check(valid)["root_process_cpu_evidence"]
        self.assertEqual(cpu["status"], "ROOT_ONLY_UNCERTIFIED")
        self.assertEqual(cpu["cpu_time_delta_ns"], 200_000_000)
        self.assertAlmostEqual(cpu["root_cpu_core_equivalents_uncertified"], 0.3333, places=4)
        self.assertIn("helper CPU", cpu["limitations"])

        partial = copy.deepcopy(valid)
        del partial["raw_samples"][1]["main_cpu_time_ns"]
        with self.assertRaisesRegex(ValueError, "partial or invalid root process CPU"):
            self.check(partial)

        for invalid in (True, -1, 2.5):
            bad = copy.deepcopy(valid)
            bad["raw_samples"][1]["main_cpu_time_ns"] = invalid
            with self.subTest(invalid=invalid), self.assertRaisesRegex(
                ValueError, "partial or invalid root process CPU"
            ):
                self.check(bad)

        decrease = copy.deepcopy(valid)
        decrease["raw_samples"][2]["main_cpu_time_ns"] = 1_099_999_999
        with self.assertRaisesRegex(ValueError, "CPU time decreased"):
            self.check(decrease)

        for offset in (-1, 100_000_001, None, True):
            invalid_clock = copy.deepcopy(valid)
            invalid_clock["raw_samples"][1]["main_cpu_time_observed_monotonic_ns"] = (
                invalid_clock["raw_samples"][1]["sample_started_monotonic_ns"] + offset
                if type(offset) is int else offset
            )
            with self.subTest(cpu_clock_offset=offset), self.assertRaisesRegex(
                ValueError, "missing or unbounded root CPU observation clock"
            ):
                self.check(invalid_clock)

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
