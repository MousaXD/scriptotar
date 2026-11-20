#!/usr/bin/env python3
"""Agent10 independent native JSONL evidence duplicate-key negative.

Exact source candidate PR170 is authenticated in CI before this script imports
it. This is a synthetic data integrity test ONLY; it does not certify genuine
AT-SPI/UIA interaction, private source transcripts or performance.
"""
from __future__ import annotations

import argparse
import importlib.util
import json
import tempfile
from pathlib import Path

SOURCE_SHA = "a" * 40
PACKAGE_SHA = "b" * 64
EXECUTABLE_SHA = "c" * 64
FIXTURE_SHA = "d" * 64
HOST_SHA = "e" * 64
RUN = "https://github.com/MousaXD/scriptotar/actions/runs/123456"


def event(i: int) -> dict:
    return {
        "evidence_kind": "externally-observed-installed-a11y-action",
        "source_sha": SOURCE_SHA,
        "package_sha256": PACKAGE_SHA,
        "executable_sha256": EXECUTABLE_SHA,
        "fixture_sha256": FIXTURE_SHA,
        "host_evidence_sha256": HOST_SHA,
        "native_qa_run_url": RUN,
        "session_id": f"session-{i:04d}",
        "trial_id": f"trial-{i:04d}",
        "native_provider": "linux-webkitgtk-atspi",
        "study_count": 100,
        "cache_state": "warm-cache",
        "action": "studio_actionable",
        "start_monotonic_ns": 1_000_000_000 + i * 1_000_000_000,
        "completion_monotonic_ns": 1_000_500_000 + i * 1_000_000_000,
        "start_evidence_id": f"evidence-start-{i:04d}",
        "completion_evidence_id": f"evidence-end-{i:04d}",
    }


def test_module(candidate_file: Path) -> None:
    spec = importlib.util.spec_from_file_location(
        "agent10_pinned_candidate_native_trace", candidate_file
    )
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)

    with tempfile.TemporaryDirectory(prefix="agent10-native-trace-") as root:
        data = Path(root) / "external-a11y.jsonl"
        normal = [event(i) for i in range(10)]
        normal_lines = [json.dumps(e, separators=(",", ":")) for e in normal]
        data.write_text("\n".join(normal_lines) + "\n", encoding="utf-8")
        output = module.summarize_ui_trace(
            data, SOURCE_SHA, PACKAGE_SHA, EXECUTABLE_SHA, min_trials=10
        )
        assert output["event_count"] == 10
        assert output["groups"][0]["status"] == "UNVERIFIED_EXTERNAL_TRACE"
        assert output["certification"] == "PENDING_INDEPENDENT_AGENT07_INSTALLED_QA"
        print("PASS: benign synthetic 10-trial trace remains unverified (never native certified)", flush=True)

        # Python's standard json.loads() takes the LAST key silently.
        # That creates an ambiguity between original raw evidence and the
        # interpreted source, fixture, provider, cache, or action identity.
        contradictions = (
            ("source_sha", "f" * 40),
            ("fixture_sha256", "f" * 64),
            ("native_provider", "windows-webview2-uia"),
            ("cache_state", "cold-rebooted"),
            ("action", "candidate_save"),
        )
        failed = []
        for field, contradictory in contradictions:
            e = dict(normal[0])
            e[field] = contradictory  # first raw occurrence is hostile
            first = json.dumps(e, separators=(",", ":"))
            append_last_reviewed = (
                first[:-1]
                + ","
                + json.dumps(field)
                + ":"
                + json.dumps(normal[0][field])
                + "}"
            )
            assert append_last_reviewed.count('"' + field + '"') == 2
            assert json.loads(append_last_reviewed) == normal[0], (
                "Negative fixture must be valid ambiguous JSON"
            )
            data.write_text(
                "\n".join([append_last_reviewed, *normal_lines[1:]]) + "\n",
                encoding="utf-8",
            )
            try:
                module.summarize_ui_trace(
                    data, SOURCE_SHA, PACKAGE_SHA, EXECUTABLE_SHA,
                    min_trials=10,
                )
            except ValueError:
                print(f"PASS: duplicate {field} was rejected", flush=True)
            else:
                failed.append(field)
                print(f"FAIL OPEN: contradictory duplicate {field} accepted", flush=True)

        if failed:
            raise AssertionError(
                "RELEASE VETO: native JSONL verifier accepted contradictory "
                f"duplicate evidence fields: {', '.join(failed)}"
            )
        print("PASS: all duplicate provenance field attacks denied", flush=True)


def main() -> int:
    p = argparse.ArgumentParser()
    p.add_argument("--candidate", type=Path, default=Path(".agent10-pr170-data"))
    args = p.parse_args()
    target = (args.candidate / "tests/defragmentation/benchmarks/native_ui_trace.py").resolve(strict=True)
    test_module(target)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
