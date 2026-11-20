"""Synthetic CPU tree security/consistency tests, never installed benchmarks."""
from __future__ import annotations

from copy import deepcopy
import unittest
from unittest.mock import patch
from pathlib import Path

from . import native_tree_cpu as cpu


class ProcessTreeCPUFormatTests(unittest.TestCase):
    def setUp(self):
        self.a = {"root_pid": 11, "started_ns": 1_000_000,
                  "finished_ns": 2_000_000,
                  "members": {"11": {"birth_id": "linux-100", "cpu_ns": 50},
                              "12": {"birth_id": "linux-200", "cpu_ns": 60}}}
        self.b = {"root_pid": 11, "started_ns": 7_000_000,
                  "finished_ns": 8_000_000,
                  "members": {"11": {"birth_id": "linux-100", "cpu_ns": 250},
                              "12": {"birth_id": "linux-200", "cpu_ns": 460}}}

    def test_stable_tree_delta_has_explicit_window_uncertainty(self):
        result = cpu.compare(self.a, self.b)
        self.assertEqual(result["root_cpu_delta_ns"], 200)
        self.assertEqual(result["visible_tree_cpu_delta_ns"], 600)
        self.assertEqual(result["min_possible_interval_ns"], 5_000_000)
        self.assertEqual(result["max_possible_interval_ns"], 7_000_000)
        self.assertIn("UNCERTIFIED", result["status"])

    def test_reject_missing_helper_reused_pid_or_counter_reversal(self):
        for kind in ("missing", "extra", "reuse", "cpu", "nonint", "root"):
            with self.subTest(kind=kind):
                after = deepcopy(self.b)
                if kind == "missing":
                    del after["members"]["12"]
                if kind == "extra":
                    after["members"]["13"] = {"birth_id": "linux-300", "cpu_ns": 1}
                if kind == "reuse":
                    after["members"]["12"]["birth_id"] = "linux-300"
                if kind == "cpu":
                    after["members"]["11"]["cpu_ns"] = 49
                if kind == "nonint":
                    after["members"]["11"]["cpu_ns"] = True
                if kind == "root":
                    after["root_pid"] = 13
                with self.assertRaises(ValueError):
                    cpu.compare(self.a, after)

    def test_reject_overlapping_or_invalid_monotonic_brackets(self):
        for field, value in (("started_ns", 2_000_000),
                             ("finished_ns", 1_000_000),
                             ("root_pid", False)):
            bad = deepcopy(self.b)
            bad[field] = value
            with self.subTest(field=field), self.assertRaises(ValueError):
                cpu.compare(self.a, bad)

    def test_snapshot_matches_real_root_and_rejects_unreadable_child(self):
        fake_executable = Path("/this-is-only-a-synthetic-binary")
        identities = {11: "linux-100", 12: "linux-200"}
        with (patch.object(cpu, "same_image", return_value=True),
              patch.object(cpu, "process_image", return_value=fake_executable),
              patch.object(cpu, "process_instance_id", side_effect=lambda pid: identities.get(pid)),
              patch.object(cpu, "process_cpu_time_ns", side_effect=lambda pid: {11: 10, 12: 20}.get(pid)),
              patch.object(cpu, "_processes", return_value={11: 1, 12: 11})):
            result = cpu.snapshot(11, fake_executable)
            self.assertEqual(set(result["members"]), {"11", "12"})
        with (patch.object(cpu, "same_image", return_value=True),
              patch.object(cpu, "process_image", return_value=fake_executable),
              patch.object(cpu, "process_instance_id", side_effect=lambda pid: identities.get(pid)),
              patch.object(cpu, "process_cpu_time_ns", return_value=None),
              patch.object(cpu, "_processes", return_value={11: 1, 12: 11})):
            with self.assertRaisesRegex(ValueError, "incomplete"):
                cpu.snapshot(11, fake_executable)

    def test_source_duration_rejection_before_any_process_probe(self):
        with patch.object(cpu, "snapshot", side_effect=AssertionError("must not probe")):
            for duration in (float("nan"), float("inf"), 0, 181, True):
                with self.subTest(duration=duration), self.assertRaises(ValueError):
                    cpu.sample(11, Path("fake"), Path("fake"), "a" * 40, duration)


if __name__ == "__main__":
    unittest.main()
