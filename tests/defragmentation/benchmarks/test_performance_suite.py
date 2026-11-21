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
from . import native_attach as native_attach_module
from .native_attach import attach, process_image, same_image, process_instance_id, process_cpu_time_ns
from .test_attached_memory_gate import AttachedMemoryGateTests  # run with current P2 CI suite

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
        if platform.system() == "Linux":
            self.assertGreater(record.get("main_pss_bytes") or 0, 0)
            self.assertGreater(record.get("main_private_resident_bytes") or 0, 0)
        elif platform.system() == "Windows":
            self.assertGreater(record.get("main_private_working_set_bytes") or 0, 0)
            self.assertGreater(record.get("main_private_commit_bytes") or 0, 0)

    def test_root_cpu_counter_is_read_only_not_a_native_benchmark(self):
        import os
        import platform
        cpu = process_cpu_time_ns(os.getpid())
        if platform.system() in ("Linux", "Windows"):
            self.assertIs(type(cpu), int)
            self.assertGreaterEqual(cpu, 0)
        else:
            self.assertIsNone(cpu)
        self.assertIsNone(process_cpu_time_ns(987654321))
        self.assertIsNone(process_cpu_time_ns(-1))

    def test_attach_read_only_does_not_terminate_instrumented_process(self):
        import os
        import sys
        import platform
        self.assertTrue(same_image(process_image(os.getpid()), Path(sys.executable)))
        self.assertFalse(same_image(process_image(987654321), Path(sys.executable)))
        with tempfile.TemporaryDirectory() as folder:
            sample_package = Path(folder) / "dummy-package-not-installed.zip"
            sample_package.write_bytes(b"test only")
            report = attach(os.getpid(), Path(sys.executable), sample_package,
                            SHA, 1.05, 0.2, 100, None)
            self.assertEqual(report["sample_count"] >= 3, True)
            self.assertEqual(report["pid"], os.getpid())
            self.assertEqual(report["certification"], "PENDING_INDEPENDENT_NATIVE_QA")
            self.assertEqual(report["scope"], "ATTACHED_INSTALLED_PROCESS_MEMORY_ONLY_NOT_UI_CERTIFICATION")
            self.assertIsNone(report["source_run_url_operator_supplied"])
            self.assertEqual(report["study_count_operator_supplied"], 100)
            self.assertEqual(report["end_reason"], "duration_elapsed")
            # Bracket every genuine OS memory sample in a Python monotonic
            # interval, without claiming that an external UIA/AT-SPI driver's
            # clock has been independently synchronized with this process.
            self.assertEqual(report["monotonic_clock"], "python.time.perf_counter_ns")
            self.assertLess(report["measurement_started_monotonic_ns"],
                            report["measurement_finished_monotonic_ns"])
            raw = report["raw_samples"]
            self.assertEqual(len(raw), report["sample_count"])
            self.assertTrue(all("main_cpu_time_ns" in row for row in raw))
            self.assertTrue(all(
                row["sample_started_monotonic_ns"]
                <= row["main_cpu_time_observed_monotonic_ns"]
                <= row["sample_finished_monotonic_ns"]
                for row in raw
            ))
            if platform.system() in ("Linux", "Windows"):
                self.assertTrue(all(type(row["main_cpu_time_ns"]) is int for row in raw))
                self.assertEqual(
                    sorted(row["main_cpu_time_ns"] for row in raw),
                    [row["main_cpu_time_ns"] for row in raw],
                )
            self.assertTrue(all(
                report["measurement_started_monotonic_ns"] <= item["sample_started_monotonic_ns"]
                <= item["sample_finished_monotonic_ns"] <= report["measurement_finished_monotonic_ns"]
                for item in raw
            ))
            self.assertEqual(
                sorted(item["sample_started_monotonic_ns"] for item in raw),
                [item["sample_started_monotonic_ns"] for item in raw],
            )
            self.assertNotIn("native_ui_p95_ms", report)
            self.assertGreater(report["metrics"]["main_pss_bytes"]["valid_trials"] if
                               platform.system() == "Linux" else
                               report["metrics"]["main_private_working_set_bytes"]["valid_trials"], 0)
            with self.assertRaisesRegex(ValueError, "source SHA"):
                attach(os.getpid(), Path(sys.executable), sample_package, "invalid",
                       1.0, 0.2, None, None)
            with self.assertRaisesRegex(ValueError, "kernel process image"):
                attach(987654321, Path(sys.executable), sample_package, SHA,
                       1.0, 0.2, None, None)

    def test_attached_memory_rejects_pid_reuse_and_missing_birth_identity(self):
        from unittest.mock import patch
        import os
        import sys
        self.assertIsNotNone(process_instance_id(os.getpid()))
        self.assertIsNone(process_instance_id(987654321))
        with tempfile.TemporaryDirectory() as folder:
            package = Path(folder) / "synthetic-not-installed.zip"
            package.write_bytes(b"unit test only")
            with patch.object(native_attach_module, "process_instance_id",
                       return_value=None):
                with self.assertRaisesRegex(ValueError, "creation identity"):
                    attach(os.getpid(), Path(sys.executable), package, SHA,
                           1.0, 0.2, 100, None)
            with patch.object(native_attach_module, "process_instance_id",
                       side_effect=["first-instance", "recycled-instance"]):
                report = attach(os.getpid(), Path(sys.executable), package,
                                SHA, 1.0, 0.2, 100, None)
            self.assertEqual(report["end_reason"], "process_gone_or_identity_changed")
            self.assertEqual(report["sample_count"], 0)
            self.assertEqual(report["kernel_process_creation_id"], "first-instance")
            self.assertEqual(report["raw_samples"], [])
            # Attack the intra-sample TOCTOU window: initial identity and
            # pre-read probe match, but the post-read birth identity changes.
            # The snapshot MUST be discarded instead of entering p50/p95.
            with patch.object(native_attach_module, "process_instance_id",
                       side_effect=["first-instance", "first-instance", "recycled-instance"]):
                raced = attach(os.getpid(), Path(sys.executable), package,
                               SHA, 1.0, 0.2, 100, None)
            self.assertEqual(raced["end_reason"], "process_gone_or_identity_changed_during_sample")
            self.assertEqual(raced["raw_samples"], [])
            self.assertEqual(raced["sample_count"], 0)
            self.assertLessEqual(report["measurement_started_monotonic_ns"],
                                 report["measurement_finished_monotonic_ns"])
            self.assertEqual(report["certification"], "PENDING_INDEPENDENT_NATIVE_QA")

    def test_attached_memory_rechecks_kernel_identity_at_collection_end(self):
        # Synthetic time: exactly one valid in-loop sample, followed by a
        # completed duration and a kernel birth/PID or image change. Do not
        # count this as a complete installed-process sampling session.
        from unittest.mock import patch
        import sys
        executable = Path(sys.executable).resolve(strict=True)
        timer = [0.0, 0.01, 0.02, 1.1, 1.11]
        fake_memory = {
            "provider": "synthetic-not-native",
            "tree_memory_complete": True,
            "sampled_process_count": 1,
        }
        with tempfile.TemporaryDirectory() as folder:
            package = Path(folder) / "dummy-not-installed.zip"
            package.write_bytes(b"synthetic package only")
            with patch.object(native_attach_module.time, "perf_counter",
                              side_effect=timer), patch.object(
                              native_attach_module, "memory_snapshot",
                              return_value=fake_memory), patch.object(
                              native_attach_module, "process_image",
                              return_value=executable), patch.object(
                              native_attach_module, "process_instance_id",
                              side_effect=["birth-1", "birth-1", "birth-1", "birth-2"]):
                reused = attach(1234, executable, package, SHA, 1.0, 0.2, 100, None)
            self.assertEqual(reused["sample_count"], 1)
            self.assertEqual(reused["end_reason"],
                             "process_gone_or_identity_changed_at_collection_end")

            with patch.object(native_attach_module.time, "perf_counter",
                              side_effect=[0.0, 0.01, 0.02, 1.1, 1.11]), patch.object(
                              native_attach_module, "memory_snapshot",
                              return_value=fake_memory), patch.object(
                              native_attach_module, "process_image",
                              side_effect=[executable, executable, executable, None]), patch.object(
                              native_attach_module, "process_instance_id",
                              return_value="birth-1"):
                vanished = attach(1234, executable, package, SHA,
                                  1.0, 0.2, 100, None)
            self.assertEqual(vanished["sample_count"], 1)
            self.assertEqual(vanished["end_reason"],
                             "process_gone_or_identity_changed_at_collection_end")

    def test_attached_memory_rejects_package_or_executable_changed_during_collection(self):
        # Test the real package bytes without modifying the running Python
        # executable; a fake executable digest change models external replace.
        from unittest.mock import patch
        import os
        import sys
        with tempfile.TemporaryDirectory() as folder:
            package = Path(folder) / "synthetic-not-installed.zip"
            package.write_bytes(b"original package bytes")
            mutated = [False]

            def replace_package_during_sample(_pid):
                if not mutated[0]:
                    package.write_bytes(b"replacement package bytes")
                    mutated[0] = True
                return {"provider": "mock-memory-only",
                        "tree_memory_complete": False}

            with patch.object(native_attach_module, "memory_snapshot",
                              side_effect=replace_package_during_sample):
                with self.assertRaisesRegex(ValueError, "package checksum changed"):
                    attach(os.getpid(), Path(sys.executable), package, SHA,
                           1.0, 0.2, 100, None)
            self.assertTrue(mutated[0])

            real_sha256 = native_attach_module.sha256
            executable = Path(sys.executable).resolve(strict=True)
            observations = [0]

            def simulated_executable_digest_drift(path):
                if Path(path).resolve(strict=True) == executable:
                    observations[0] += 1
                    return "a" * 64 if observations[0] == 1 else "b" * 64
                return real_sha256(path)

            with patch.object(native_attach_module, "sha256",
                              side_effect=simulated_executable_digest_drift):
                with self.assertRaisesRegex(ValueError, "executable checksum changed"):
                    attach(os.getpid(), executable, package, SHA,
                           1.0, 0.2, 100, None)
            self.assertGreaterEqual(observations[0], 2)

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



from .native_ui_trace import (summarize_ui_trace, UI_KIND, MAX_TRACE_BYTES,
                              MAX_TRACE_LINE_CHARS, MAX_TRACE_EVENTS)


class NativeUiTraceContractTests(unittest.TestCase):
    """Synthetic examples validate format only; never native app timings."""

    @staticmethod
    def event(index):
        return {
            "evidence_kind": UI_KIND,
            "source_sha": SHA, "package_sha256": HASH,
            "executable_sha256": HASH, "fixture_sha256": "b" * 64,
            "host_evidence_sha256": "c" * 64,
            "native_qa_run_url": "https://github.com/MousaXD/scriptotar/actions/runs/44333456021",
            "session_id": f"session-native-{index:02}",
            "trial_id": f"trial-native-{index:02}",
            "native_provider": "linux-webkitgtk-atspi",
            "study_count": 100, "cache_state": "warm-cache",
            "action": "study_open",
            "start_monotonic_ns": 1_000_000_000 + index * 1_000_000_000,
            "completion_monotonic_ns": 1_001_000_000 + index * 1_000_000_000,
            "start_evidence_id": f"evidence-before-{index:02}",
            "completion_evidence_id": f"evidence-after-{index:02}",
        }

    def test_required_studio_actions_have_distinct_unverified_cohorts(self):
        # A generic navigation bucket is not an honest substitute for
        # separately measured Study/source/research/version journeys.
        required = (
            "study_create", "source_select", "research_navigation",
            "draft_open", "draft_version_restore", "draft_export",
        )
        with tempfile.TemporaryDirectory() as folder:
            trace = Path(folder) / "offline-actions-not-installed.jsonl"
            for action in required:
                with self.subTest(action=action):
                    trace.write_text("\n".join(
                        json.dumps({**self.event(i), "action": action})
                        for i in range(10)
                    ), encoding="utf-8")
                    result = summarize_ui_trace(trace, SHA, HASH, HASH)
                    self.assertEqual(result["event_count"], 10)
                    self.assertEqual(result["certification"],
                                     "PENDING_INDEPENDENT_AGENT07_INSTALLED_QA")
                    self.assertEqual(result["scope"], "EXTERNAL_NATIVE_UI_TRACE_FORMAT_ONLY")
                    self.assertEqual(result["groups"][0]["action"], action)
                    self.assertEqual(result["groups"][0]["status"], "UNVERIFIED_EXTERNAL_TRACE")
            trace.write_text(json.dumps({
                **self.event(0), "action": "unreviewed-paid-provider-call",
            }), encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "action or Study count invalid"):
                summarize_ui_trace(trace, SHA, HASH, HASH)

    def test_same_native_session_cannot_relabel_host_fixture_or_study_scale(self):
        # Each individual claim passes syntax. Contradictory same-session
        # origin/workload labels are nevertheless refused before aggregation.
        base = self.event(0)
        alternate = self.event(1)
        changed_values = {
            "native_provider": "windows-webview2-uia",
            "study_count": 1000,
            "fixture_sha256": "d" * 64,
            "host_evidence_sha256": "d" * 64,
            "native_qa_run_url": "https://github.com/MousaXD/scriptotar/actions/runs/44333456022",
        }
        with tempfile.TemporaryDirectory() as folder:
            trace = Path(folder) / "contradictory-session.jsonl"
            for field, changed in changed_values.items():
                with self.subTest(field=field):
                    second = {**alternate, "session_id": base["session_id"],
                              field: changed}
                    trace.write_text("\n".join(
                        json.dumps(event) for event in (base, second)
                    ), encoding="utf-8")
                    with self.assertRaisesRegex(ValueError, "inconsistent native session"):
                        summarize_ui_trace(trace, SHA, HASH, HASH)
            # A genuinely distinct session may specify another fixture.
            # It remains below ten trials and entirely uncertified.
            second = {**alternate, "fixture_sha256": "d" * 64}
            trace.write_text("\n".join(json.dumps(event)
                                       for event in (base, second)), encoding="utf-8")
            result = summarize_ui_trace(trace, SHA, HASH, HASH)
            self.assertEqual(len(result["groups"]), 2)
            self.assertTrue(all(bucket["status"] == "INSUFFICIENT_TRIALS"
                                for bucket in result["groups"]))
            self.assertEqual(result["certification"], "PENDING_INDEPENDENT_AGENT07_INSTALLED_QA")

    def test_underpowered_trace_cannot_claim_p50_or_p95(self):
        with tempfile.TemporaryDirectory() as folder:
            trace = Path(folder) / "external.jsonl"
            trace.write_text("\n".join(json.dumps(self.event(i)) for i in range(9)))
            report = summarize_ui_trace(trace, SHA, HASH, HASH)
            bucket = report["groups"][0]
            self.assertEqual(bucket["status"], "INSUFFICIENT_TRIALS")
            self.assertIsNone(bucket["p95_ms_nearest_rank"])
            self.assertIsNone(bucket["p50_ms"])
            trace.write_text("\n".join(json.dumps(self.event(i)) for i in range(10)))
            report = summarize_ui_trace(trace, SHA, HASH, HASH)
            self.assertEqual(report["groups"][0]["p95_ms_nearest_rank"], 1.0)
            self.assertEqual(report["groups"][0]["p50_ms"], 1.0)
            self.assertEqual(report["certification"], "PENDING_INDEPENDENT_AGENT07_INSTALLED_QA")
            self.assertEqual(report["scope"], "EXTERNAL_NATIVE_UI_TRACE_FORMAT_ONLY")
            self.assertIsNone(report["ipc_latency_ms"])

    def test_native_latency_threshold_is_always_at_least_ten(self):
        with tempfile.TemporaryDirectory() as folder:
            trace = Path(folder) / "external-three-synthetic.jsonl"
            trace.write_text("\n".join(json.dumps(self.event(i)) for i in range(3)))
            for invalid in (3, 9, 10.5, True, 101, None):
                with self.subTest(min_trials=invalid):
                    with self.assertRaisesRegex(ValueError, "integer in 10..100"):
                        summarize_ui_trace(trace, SHA, HASH, HASH, min_trials=invalid)
            # Three observations remain insufficient even with an explicit
            # valid threshold; no p50/p95 may leak through this parser.
            report = summarize_ui_trace(trace, SHA, HASH, HASH, min_trials=10)
            self.assertEqual(report["groups"][0]["status"], "INSUFFICIENT_TRIALS")
            self.assertIsNone(report["groups"][0]["p50_ms"])
            self.assertIsNone(report["groups"][0]["p95_ms_nearest_rank"])

    def test_untrusted_native_jsonl_is_bounded_before_decoding_or_parsing(self):
        # All input below is synthetic source-only material, never real UI.
        with tempfile.TemporaryDirectory() as folder:
            trace = Path(folder) / "malicious-external.jsonl"
            trace.write_bytes(b" " * (MAX_TRACE_BYTES + 1))
            with self.assertRaisesRegex(ValueError, "bounded byte limit"):
                summarize_ui_trace(trace, SHA, HASH, HASH)

            valid_json = json.dumps(self.event(0))
            trace.write_text(valid_json + " " * MAX_TRACE_LINE_CHARS)
            with self.assertRaisesRegex(ValueError, "oversized native observation"):
                summarize_ui_trace(trace, SHA, HASH, HASH)

            trace.write_bytes(b"\xff")
            with self.assertRaisesRegex(ValueError, "must be UTF-8"):
                summarize_ui_trace(trace, SHA, HASH, HASH)

            # More valid records than the explicit input cap must fail;
            # their IDs/timestamps remain distinct to exercise the cap
            # rather than the existing replay or malformed JSON controls.
            trace.write_text("\n".join(
                json.dumps(self.event(i)) for i in range(MAX_TRACE_EVENTS + 1)
            ))
            with self.assertRaisesRegex(ValueError, "bounded event count"):
                summarize_ui_trace(trace, SHA, HASH, HASH)

            # The new size limits do not affect normal ten-trial batches,
            # which remain unverified until independent native QA.
            trace.write_text("\n".join(
                json.dumps(self.event(i)) for i in range(10)
            ))
            record = summarize_ui_trace(trace, SHA, HASH, HASH)
            self.assertEqual(record["groups"][0]["status"], "UNVERIFIED_EXTERNAL_TRACE")
            self.assertEqual(record["event_count"], 10)
            self.assertEqual(record["certification"], "PENDING_INDEPENDENT_AGENT07_INSTALLED_QA")

    def test_duplicate_native_json_members_cannot_hide_contradictory_provenance(self):
        # Construct RAW JSON, not a Python dict: json.dumps(dict) alone cannot
        # express repeated keys and json.loads normally discards the first.
        conflicts = {
            "source_sha": "f" * 40,
            "fixture_sha256": "f" * 64,
            "native_provider": "windows-webview2-uia",
            "cache_state": "cold-rebooted",
            "action": "candidate_save",
        }
        with tempfile.TemporaryDirectory() as folder:
            trace = Path(folder) / "contradictory-native.jsonl"
            baseline = json.dumps(self.event(0))
            for key, contradictory in conflicts.items():
                extra = json.dumps(key) + ": " + json.dumps(contradictory)
                last_value_wins = baseline[:-1] + ", " + extra + "}"
                # Demonstrate that the standard JSON parser would discard
                # the original field and retain the attacker-controlled one.
                self.assertEqual(json.loads(last_value_wins)[key], contradictory)
                with self.subTest(field=key, ordering="last-conflict"):
                    trace.write_text(last_value_wins, encoding="utf-8")
                    with self.assertRaisesRegex(ValueError, "duplicate JSON member"):
                        summarize_ui_trace(trace, SHA, HASH, HASH)
                first_value_wins = "{" + extra + ", " + baseline[1:]
                with self.subTest(field=key, ordering="first-conflict"):
                    trace.write_text(first_value_wins, encoding="utf-8")
                    with self.assertRaisesRegex(ValueError, "duplicate JSON member"):
                        summarize_ui_trace(trace, SHA, HASH, HASH)

            # Even repeated identical values are ambiguous raw audit input.
            duplicate_same = baseline[:-1] + ', "source_sha": ' + json.dumps(SHA) + "}"
            trace.write_text(duplicate_same, encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "duplicate JSON member"):
                summarize_ui_trace(trace, SHA, HASH, HASH)

            # The fix must not turn ten ordinary source-format observations
            # into an independently certified installed-app performance result.
            trace.write_text("\n".join(json.dumps(self.event(i)) for i in range(10)), encoding="utf-8")
            clean = summarize_ui_trace(trace, SHA, HASH, HASH)
            self.assertEqual(clean["event_count"], 10)
            self.assertEqual(clean["groups"][0]["status"], "UNVERIFIED_EXTERNAL_TRACE")
            self.assertEqual(clean["certification"], "PENDING_INDEPENDENT_AGENT07_INSTALLED_QA")

    def test_untrusted_trace_rejects_duplicate_clock_and_payload_leaks(self):
        with tempfile.TemporaryDirectory() as folder:
            trace = Path(folder) / "external.jsonl"
            baseline = self.event(0)
            bad_cases = (
                ([baseline, baseline], "duplicate trial"),
                ([{**baseline, "completion_monotonic_ns": 0}], "monotonic"),
                ([{**baseline, "completion_monotonic_ns": True}], "monotonic"),
                ([{**baseline, "raw_transcript_text": "private"}], "unexpected"),
                ([{**baseline, "source_sha": "f" * 40}], "mismatch"),
                ([{**baseline, "evidence_kind": "chromium-mock"}], "external native"),
                ([{**baseline, "cache_state": "cold-rebooted",
                   "action": "warm_start_window"}], "inconsistent"),
                ([{**baseline, "study_count": True}], "Study count"),
                ([{**baseline, "action": ["study_open"]}], "Study count"),
                ([{**baseline, "native_provider": ["fake"]}], "provider"),
                ([{**baseline, "fixture_sha256": "fake"}], "fixture"),
                ([{**baseline, "host_evidence_sha256": "fake"}], "host metadata"),
                ([{**baseline, "native_qa_run_url": "https://untrusted.example"}], "native QA"),
            )
            for events, message in bad_cases:
                with self.subTest(expected=message):
                    trace.write_text("\n".join(json.dumps(x) for x in events))
                    with self.assertRaisesRegex(ValueError, message):
                        summarize_ui_trace(trace, SHA, HASH, HASH, min_trials=10)

    def test_replayed_accessibility_evidence_cannot_inflate_native_trial_count(self):
        # The parser does not prove host authenticity, but a single recorded
        # native accessibility event may never be counted repeatedly merely
        # by changing the self-asserted trial label.
        with tempfile.TemporaryDirectory() as folder:
            trace = Path(folder) / "native-events.jsonl"
            baseline = self.event(0)
            copied = {
                **self.event(1),
                "session_id": baseline["session_id"],
                "trial_id": "trial-native-independent",
                "start_evidence_id": baseline["start_evidence_id"],
                "completion_evidence_id": baseline["completion_evidence_id"],
            }
            trace.write_text("\n".join(json.dumps(x) for x in [baseline, copied]))
            with self.assertRaisesRegex(ValueError, "duplicate native evidence ID"):
                summarize_ui_trace(trace, SHA, HASH, HASH, min_trials=10)

            # A supposedly completed action cannot use the same observation
            # for both its start and completion, even for one reported trial.
            same_id = {**baseline, "completion_evidence_id": baseline["start_evidence_id"]}
            trace.write_text(json.dumps(same_id))
            with self.assertRaisesRegex(ValueError, "start/completion reuse"):
                summarize_ui_trace(trace, SHA, HASH, HASH, min_trials=10)

            # IDs from separate independently recorded sessions have a local
            # namespace. Those are still operator-supplied, not QA-certified.
            separate = {**self.event(1),
                        "start_evidence_id": baseline["start_evidence_id"],
                        "completion_evidence_id": baseline["completion_evidence_id"]}
            trace.write_text("\n".join(json.dumps(x) for x in [baseline, separate]))
            result = summarize_ui_trace(trace, SHA, HASH, HASH, min_trials=10)
            self.assertEqual(result["event_count"], 2)
            self.assertEqual(result["groups"][0]["status"], "INSUFFICIENT_TRIALS")

    def test_same_session_replayed_monotonic_pair_cannot_inflate_trial_count(self):
        # Reassigning both evidence IDs and the trial ID must not let the
        # exact same observed start/completion interval count as two trials.
        with tempfile.TemporaryDirectory() as folder:
            trace = Path(folder) / "native-intervals.jsonl"
            first = self.event(0)
            relabeled = {
                **self.event(1),
                "session_id": first["session_id"],
                "start_monotonic_ns": first["start_monotonic_ns"],
                "completion_monotonic_ns": first["completion_monotonic_ns"],
            }
            # Both evidence IDs are distinct and would pass the older
            # label-only replay guard. The time pair exposes the replay.
            self.assertNotEqual(first["trial_id"], relabeled["trial_id"])
            self.assertNotEqual(first["start_evidence_id"], relabeled["start_evidence_id"])
            self.assertNotEqual(first["completion_evidence_id"], relabeled["completion_evidence_id"])
            trace.write_text("\n".join(json.dumps(e) for e in (first, relabeled)))
            with self.assertRaisesRegex(ValueError, "duplicate native monotonic time pair"):
                summarize_ui_trace(trace, SHA, HASH, HASH, min_trials=10)

            # Ordinary subsequent actions from the same session are valid
            # when their true observed time intervals differ.
            distinct = {**self.event(1), "session_id": first["session_id"]}
            trace.write_text("\n".join(json.dumps(e) for e in (first, distinct)))
            result = summarize_ui_trace(trace, SHA, HASH, HASH, min_trials=10)
            self.assertEqual(result["event_count"], 2)
            self.assertEqual(result["groups"][0]["status"], "INSUFFICIENT_TRIALS")

            # Different sessions can legitimately reuse equal numeric clock
            # values; this does not elevate self-asserted event authenticity.
            another_session = {
                **self.event(1),
                "start_monotonic_ns": first["start_monotonic_ns"],
                "completion_monotonic_ns": first["completion_monotonic_ns"],
            }
            trace.write_text("\n".join(json.dumps(e) for e in (first, another_session)))
            result = summarize_ui_trace(trace, SHA, HASH, HASH, min_trials=10)
            self.assertEqual(result["event_count"], 2)
            self.assertEqual(result["certification"], "PENDING_INDEPENDENT_AGENT07_INSTALLED_QA")

    def test_distinct_cache_conditions_cannot_be_pooled_for_p95(self):
        with tempfile.TemporaryDirectory() as folder:
            trace = Path(folder) / "external.jsonl"
            events = [self.event(i) for i in range(10)]
            for event in events[5:]:
                event["cache_state"] = "unknown"
            trace.write_text("\n".join(json.dumps(e) for e in events))
            report = summarize_ui_trace(trace, SHA, HASH, HASH)
            self.assertEqual(len(report["groups"]), 2)
            self.assertTrue(all(x["status"] == "INSUFFICIENT_TRIALS" for x in report["groups"]))



    def test_distinct_host_runtime_evidence_or_qa_run_cannot_be_pooled(self):
        with tempfile.TemporaryDirectory() as folder:
            trace = Path(folder) / "external.jsonl"
            events = [self.event(i) for i in range(10)]
            for event in events[5:]:
                event["host_evidence_sha256"] = "d" * 64
                event["native_qa_run_url"] = "https://github.com/MousaXD/scriptotar/actions/runs/44313593525"
            trace.write_text("\n".join(json.dumps(e) for e in events))
            report = summarize_ui_trace(trace, SHA, HASH, HASH)
            self.assertEqual(len(report["groups"]), 2)
            self.assertTrue(all(x["p95_ms_nearest_rank"] is None for x in report["groups"]))
            self.assertTrue(all(x["status"] == "INSUFFICIENT_TRIALS" for x in report["groups"]))



from .stage_installed_fixture import stage as stage_fixture, digest as stage_digest


class StageInstalledFixtureSafetyTests(unittest.TestCase):
    """Staging tests use a deliberately simulated schema ledger, NOT real migration proof."""

    def test_stage_requires_guarded_synthetic_v12_provenance_and_never_overwrites(self):
        with tempfile.TemporaryDirectory() as folder:
            base = Path(folder) / "template.sqlite3"
            source = Path(folder) / "synthetic-source"
            target = Path(folder) / "isolated-installed-data"
            source.mkdir()
            target.mkdir()
            for directory in (source, target):
                (directory / ".scriptotar-benchmark-only").write_text("SYNTHETIC_ONLY\n")
            blank(base)
            fixture = source / "scriptotar-synthetic-100.sqlite3"
            meta = build(base, fixture, 100, SHA)
            manifest = source / "scriptotar-synthetic-100.json"
            manifest.write_text(json.dumps(meta))
            output = target / "staging-proof.json"
            with self.assertRaisesRegex(ValueError, "v12 schema mismatch"):
                stage_fixture(fixture, manifest, target, 100, SHA, output)
            self.assertFalse((target / "scriptotar.sqlite3").exists())

            # Stub migration ledger below exercises staging file-copy safety,
            # not Scriptotar's real migration SQL or any installed behavior.
            with closing(sqlite3.connect(fixture)) as conn, conn:
                conn.execute("PRAGMA user_version=12")
                conn.executemany(
                    "INSERT INTO schema_migrations(version,name,applied_at) VALUES(?,?,?)",
                    [(i, f"unit-test-only-{i}", "2026-01-01") for i in (10,11,12)])
            meta["schema_version"] = 12
            meta["db_sha256"] = stage_digest(fixture)
            meta["db_file_bytes"] = fixture.stat().st_size
            manifest.write_text(json.dumps(meta))
            (target / ".scriptotar-benchmark-only").unlink()
            with self.assertRaisesRegex(ValueError, "sentinel"):
                stage_fixture(fixture, manifest, target, 100, SHA, output)
            self.assertFalse((target / "scriptotar.sqlite3").exists())
            (target / ".scriptotar-benchmark-only").write_text("SYNTHETIC_ONLY\n")
            report = stage_fixture(fixture, manifest, target, 100, SHA, output)
            self.assertEqual(report["certification"], "PENDING_REAL_INSTALLED_APP_READBACK")
            # Windows runner_temp may resolve RUNNER~1 to runneradmin; compare
            # filesystem identity, not its short/long directory spelling.
            self.assertEqual(Path(report["staged_db"]).resolve(strict=True),
                             (target / "scriptotar.sqlite3").resolve(strict=True))
            self.assertEqual(stage_digest(target / "scriptotar.sqlite3"), stage_digest(fixture))
            with self.assertRaisesRegex(ValueError, "overwrite"):
                stage_fixture(fixture, manifest, target, 100, SHA, target / "second-proof.json")

    def test_synthetic_wal_fixture_is_frozen_before_sql_profile_and_staging(self):
        # A migrated blank DB may be WAL-mode. After seeding, the synthetic
        # fixture must be a complete single-file SQLite snapshot; read-only
        # SQL profiling must not resurrect sidecars.
        with tempfile.TemporaryDirectory() as folder:
            base = Path(folder) / "template.db"
            blank(base)
            with closing(sqlite3.connect(base)) as db:
                self.assertEqual(db.execute("PRAGMA journal_mode=WAL").fetchone()[0].lower(), "wal")
            source = Path(folder) / "scratch-source"
            dest = Path(folder) / "scratch-dest"
            source.mkdir()
            dest.mkdir()
            for directory in (source, dest):
                (directory / ".scriptotar-benchmark-only").write_text("SYNTHETIC_ONLY\n")
            fixture = source / "fixture.sqlite3"
            manifest = build(base, fixture, 100, SHA)
            for suffix in ("-wal", "-shm"):
                self.assertFalse(Path(str(fixture) + suffix).exists(),
                                 f"staging would reject synthetic SQLite sidecar {suffix}")
            with closing(sqlite3.connect(fixture.resolve().as_uri() + "?mode=ro", uri=True)) as db:
                self.assertEqual(db.execute("PRAGMA journal_mode").fetchone()[0].lower(), "delete")
            profile(fixture, SHA, 3)
            self.assertFalse(Path(str(fixture) + "-wal").exists())
            self.assertFalse(Path(str(fixture) + "-shm").exists())
            manifest["schema_version"] = 12
            with closing(sqlite3.connect(fixture)) as db:
                for i in (10, 11, 12):
                    db.execute("INSERT INTO schema_migrations VALUES (?,?,?)",
                               (i, f"migration-test-only-{i}", "2026-01-01"))
                db.execute("PRAGMA user_version=12")
                db.commit()
            manifest["db_sha256"] = stage_digest(fixture)
            manifest["db_file_bytes"] = fixture.stat().st_size
            source_manifest = source / "manifest.json"
            source_manifest.write_text(json.dumps(manifest))
            proof = stage_fixture(fixture, source_manifest, dest, 100, SHA, dest / "stage-proof.json")
            self.assertEqual(proof["staged_sha256"], manifest["db_sha256"])
            self.assertEqual(Path(proof["staged_db"]).resolve(strict=True),
                             (dest / "scriptotar.sqlite3").resolve(strict=True))

    def test_stage_refuses_changed_manifest_and_private_study_content(self):
        with tempfile.TemporaryDirectory() as folder:
            source = Path(folder) / "scratch-source"
            dest = Path(folder) / "scratch-dest"
            source.mkdir()
            dest.mkdir()
            for directory in (source, dest):
                (directory / ".scriptotar-benchmark-only").write_text("SYNTHETIC_ONLY\n")
            base = Path(folder) / "template.db"
            blank(base)
            fixture = source / "scriptotar-synthetic-100.sqlite3"
            meta = build(base, fixture, 100, SHA)
            manifest = source / "manifest.json"
            manifest.write_text(json.dumps(meta))
            with self.assertRaisesRegex(ValueError, "v12 schema mismatch"):
                stage_fixture(fixture, manifest, dest, 100, SHA, dest / "proof.json")
            meta["schema_version"] = 12
            meta["db_sha256"] = "f" * 64
            manifest.write_text(json.dumps(meta))
            with self.assertRaisesRegex(ValueError, "checksum/size"):
                # Tamper manifest hash even if schema v12 is declared.
                stage_fixture(fixture, manifest, dest, 100, SHA, dest / "proof.json")
            self.assertFalse((dest / "scriptotar.sqlite3").exists())


if __name__ == "__main__":
    unittest.main()
