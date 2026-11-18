"""Validate external native accessibility journey timestamps without certifying provenance.

Only Agent 07's real installed AT-SPI/UIA driver can provide these events.
This tool does not launch an app, infer IPC, or treat a trace label as proof.
"""
from __future__ import annotations

import argparse
import json
import math
from pathlib import Path
import re
import statistics

SHA40 = re.compile(r"[0-9a-f]{40}\Z")
SHA256 = re.compile(r"[0-9a-f]{64}\Z")
TOKEN = re.compile(r"[A-Za-z0-9][A-Za-z0-9_.:-]{7,127}\Z")
UI_KIND = "externally-observed-installed-a11y-action"
ACTIONS = frozenset({
    "cold_start_window", "warm_start_window", "studio_actionable",
    "project_switch", "study_open", "candidate_typing", "candidate_save",
    "candidate_reopen", "source_navigation", "transcript_navigation",
    "gui_interaction",
})
ALLOWED_KEYS = frozenset({
    "evidence_kind", "source_sha", "package_sha256", "executable_sha256",
    "fixture_sha256", "session_id", "trial_id", "native_provider",
    "study_count", "cache_state", "action", "start_monotonic_ns",
    "completion_monotonic_ns", "start_evidence_id", "completion_evidence_id",
})
PROVIDERS = frozenset({"linux-webkitgtk-atspi", "windows-webview2-uia"})
CACHE = frozenset({"cold-rebooted", "cold-cache-dropped", "warm-cache", "unknown"})


def _percentiles(values: list[float]) -> dict:
    ordered = sorted(values)
    return {
        "p50_ms": round(statistics.median(ordered), 3),
        "p95_ms_nearest_rank": round(ordered[math.ceil(0.95 * len(ordered)) - 1], 3),
    }


def summarize_ui_trace(path: Path, source_sha: str, package_sha256: str,
                       executable_sha256: str, *, min_trials: int = 10) -> dict:
    """Check shape and hashes, NOT whether a physical UI event really happened."""
    if not SHA40.fullmatch(source_sha) or any(
        not SHA256.fullmatch(v) for v in (package_sha256, executable_sha256)
    ):
        raise ValueError("exact source and package/executable hashes required")
    if not 3 <= min_trials <= 100:
        raise ValueError("min_trials must be 3..100")
    if not path.is_file():
        raise ValueError("missing external native trace")
    groups: dict[tuple, list[dict]] = {}
    unique: set[tuple[str, str, str]] = set()
    count = 0
    for number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        if not line.strip():
            continue
        try:
            event = json.loads(line)
        except json.JSONDecodeError as exc:
            raise ValueError(f"line {number}: malformed JSON") from exc
        if not isinstance(event, dict) or set(event) != ALLOWED_KEYS:
            # Disallow arbitrary fields, particularly raw transcript/IPC data.
            raise ValueError(f"line {number}: unexpected or missing evidence fields")
        if event["evidence_kind"] != UI_KIND:
            raise ValueError(f"line {number}: not an external native accessibility event")
        for key, value in (
            ("source_sha", source_sha), ("package_sha256", package_sha256),
            ("executable_sha256", executable_sha256),
        ):
            if event[key] != value:
                raise ValueError(f"line {number}: {key} mismatch")
        if not isinstance(event["fixture_sha256"], str) or not SHA256.fullmatch(event["fixture_sha256"]):
            raise ValueError(f"line {number}: missing fixture SHA-256")
        if (not isinstance(event["native_provider"], str) or event["native_provider"] not in PROVIDERS
                or not isinstance(event["cache_state"], str) or event["cache_state"] not in CACHE):
            raise ValueError(f"line {number}: invalid provider or cache state")
        if (not isinstance(event["action"], str) or event["action"] not in ACTIONS
                or type(event["study_count"]) is not int
                or event["study_count"] not in (100, 1000, 10000)):
            raise ValueError(f"line {number}: action or Study count invalid")
        if (event["action"] == "cold_start_window" and event["cache_state"] == "warm-cache") or (
            event["action"] == "warm_start_window" and event["cache_state"].startswith("cold-")
        ):
            raise ValueError(f"line {number}: inconsistent startup cache state")
        for name in ("session_id", "trial_id", "start_evidence_id", "completion_evidence_id"):
            value = event[name]
            if not isinstance(value, str) or not TOKEN.fullmatch(value):
                raise ValueError(f"line {number}: invalid {name}")
        start, end = event["start_monotonic_ns"], event["completion_monotonic_ns"]
        if (type(start) is not int or type(end) is not int or start <= 0 or end <= start
                or end - start > 600_000_000_000):
            raise ValueError(f"line {number}: invalid monotonic event pair")
        unique_key = (event["session_id"], event["trial_id"], event["action"])
        if unique_key in unique:
            raise ValueError(f"line {number}: duplicate trial/action")
        unique.add(unique_key)
        key = (event["native_provider"], event["action"], event["study_count"],
               event["cache_state"], event["fixture_sha256"])
        groups.setdefault(key, []).append({
            "session_id": event["session_id"], "trial_id": event["trial_id"],
            "start_monotonic_ns": start, "completion_monotonic_ns": end,
            "duration_ms": round((end - start) / 1_000_000, 6),
            "start_evidence_id": event["start_evidence_id"],
            "completion_evidence_id": event["completion_evidence_id"],
        })
        count += 1
    if not count:
        raise ValueError("no external native observations")
    buckets = []
    for (provider, action, studies, cache, fixture), records in sorted(groups.items()):
        enough = len(records) >= min_trials
        metrics = _percentiles([r["duration_ms"] for r in records]) if enough else {
            "p50_ms": None, "p95_ms_nearest_rank": None,
        }
        buckets.append({
            "provider": provider, "action": action, "study_count_operator_supplied": studies,
            "cache_state_operator_supplied": cache, "fixture_sha256_operator_supplied": fixture,
            "observations": len(records), "threshold_met": enough,
            "status": "UNVERIFIED_EXTERNAL_TRACE" if enough else "INSUFFICIENT_TRIALS",
            **metrics, "raw_samples": records,
        })
    return {
        "scope": "EXTERNAL_NATIVE_UI_TRACE_FORMAT_ONLY",
        "certification": "PENDING_INDEPENDENT_AGENT07_INSTALLED_QA",
        "attestation": "An operator-supplied provider label, hash or event ID cannot prove installed UI identity, canonical Study count, event correlation or genuine native interaction; Agent 07 must independently review driver and artifacts.",
        "source_sha_operator_supplied": source_sha,
        "package_sha256_operator_supplied": package_sha256,
        "executable_sha256_operator_supplied": executable_sha256,
        "required_trials_per_bucket": min_trials, "event_count": count,
        "groups": buckets,
        "ipc_latency_ms": None,
        "limitations": "Monotonic clocks must be from the same process per event pair. Cold/warm state, installed UI control completion, exact fixture, package and source must be externally attested. Insufficient samples are NOT TESTED; all values require native QA before publication.",
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--trace", type=Path, required=True)
    parser.add_argument("--source-sha", required=True)
    parser.add_argument("--package-sha256", required=True)
    parser.add_argument("--executable-sha256", required=True)
    parser.add_argument("--min-trials", type=int, default=10)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if args.output.exists() or not args.output.parent.is_dir():
        parser.error("output must not exist and parent must exist")
    result = summarize_ui_trace(args.trace, args.source_sha, args.package_sha256,
                                args.executable_sha256, min_trials=args.min_trials)
    args.output.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    print("Uncertified native UI format report: " + str(args.output))


if __name__ == "__main__":
    main()
