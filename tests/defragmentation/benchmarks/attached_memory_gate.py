"""Fail-closed preflight of native_attach memory evidence, NOT native UI certification.

Agent07 alone owns installed AT-SPI/UIA interactions. This checker can reject
truncated, replayed or incomplete memory reports; operator-supplied metadata,
webview child coverage, true Study count and UI provenance still need external
verification. Serial memory samples are not independent latency trials.
"""
from __future__ import annotations

import argparse
import json
import math
from pathlib import Path
import re
import statistics

try:
    from .native_session import sha256
except ImportError:
    from native_session import sha256

SOURCE_SHA = re.compile(r"[0-9a-f]{40}\Z")
HASH = re.compile(r"[0-9a-f]{64}\Z")
RUN = re.compile(r"https://github[.]com/MousaXD/scriptotar/actions/runs/[1-9][0-9]*\Z")
REQUIRED = {
    "Linux": ("main_pss_bytes", "tree_pss_bytes",
              "main_private_resident_bytes", "tree_private_resident_bytes"),
    "Windows": ("main_private_working_set_bytes", "tree_private_working_set_bytes",
                "main_private_commit_bytes", "tree_private_commit_bytes"),
}
PROVIDERS = {"Linux": "linux-proc-smaps_rollup",
             "Windows": "windows-psapi-process_memory_counters_ex2"}


def _int(value: object) -> bool:
    return type(value) is int


def _percentiles(values: list[int]) -> dict:
    values = sorted(values)
    return {"observations": len(values),
            "p50_bytes": statistics.median(values),
            "p95_bytes_nearest_rank": values[math.ceil(0.95 * len(values)) - 1],
            "maximum_bytes": values[-1]}


def preflight(report: dict, package: Path, executable: Path,
              source_sha: str, run_url: str, package_hash: str,
              executable_hash: str) -> dict:
    """Reject internally invalid sampler inputs; never certify installed UI."""
    if not isinstance(report, dict):
        raise ValueError("attached report must be a JSON object")
    if (not SOURCE_SHA.fullmatch(source_sha) or not RUN.fullmatch(run_url)
            or not HASH.fullmatch(package_hash) or not HASH.fullmatch(executable_hash)):
        raise ValueError("exact source, workflow URL and independent hashes required")
    if (report.get("scope") != "ATTACHED_INSTALLED_PROCESS_MEMORY_ONLY_NOT_UI_CERTIFICATION"
            or report.get("certification") != "PENDING_INDEPENDENT_NATIVE_QA"):
        raise ValueError("input is not an unverified native_attach report")
    if (report.get("source_sha_operator_supplied") != source_sha
            or report.get("source_run_url_operator_supplied") != run_url
            or report.get("package_sha256") != package_hash
            or report.get("executable_sha256") != executable_hash):
        raise ValueError("attached source/run/package/executable identity mismatch")
    for path, expected, label in ((package, package_hash, "package"),
                                  (executable, executable_hash, "executable")):
        if not path.is_file() or path.is_symlink() or sha256(path) != expected:
            raise ValueError(f"original {label} bytes absent or checksum mismatch")
    if (Path(report.get("package_path", "")).resolve(strict=False) != package.resolve(strict=True)
            or Path(report.get("executable_path", "")).resolve(strict=False) != executable.resolve(strict=True)):
        raise ValueError("sampler package or executable path does not match original artifact")
    platform_name = report.get("platform")
    os_name = next((name for name in REQUIRED
                    if isinstance(platform_name, str) and platform_name.startswith(name + "-")), None)
    if os_name is None:
        raise ValueError("unsupported or missing host OS metadata")
    if (not _int(report.get("pid")) or report["pid"] <= 0
            or not isinstance(report.get("kernel_process_creation_id"), str)
            or not re.fullmatch(os_name.lower() + r"-[0-9]+",
                                report["kernel_process_creation_id"])):
        raise ValueError("missing kernel PID or creation identity")
    if report.get("end_reason") != "duration_elapsed":
        raise ValueError("attached collection ended prematurely or process identity changed")
    begin = report.get("measurement_started_monotonic_ns")
    end = report.get("measurement_finished_monotonic_ns")
    seconds = report.get("requested_duration_seconds")
    if (not _int(begin) or not _int(end) or begin <= 0 or end <= begin
            or type(seconds) not in (float, int) or isinstance(seconds, bool)
            or not 1 <= seconds <= 180 or end - begin < seconds * 900_000_000):
        raise ValueError("invalid or truncated monotonic measurement interval")
    interval = report.get("interval_seconds")
    if (type(interval) not in (float, int) or not math.isfinite(interval)
            or not 0.1 <= interval <= 2):
        raise ValueError("invalid native sampler interval")
    # Catch a sample train that stops early while its nominal measurement
    # deadline still advances. A truthful 180-second window containing three
    # early observations is not complete memory coverage.
    max_idle_ns = max(3_000_000_000, math.ceil(4 * interval * 1_000_000_000))
    rows = report.get("raw_samples")
    if (not isinstance(rows, list) or len(rows) < 3
            or not _int(report.get("sample_count")) or report["sample_count"] != len(rows)
            or not _int(report.get("complete_tree_sample_count"))
            or report["complete_tree_sample_count"] != len(rows)):
        raise ValueError("missing samples, inconsistent counts or incomplete process-tree coverage")
    if report.get("monotonic_clock") != "python.time.perf_counter_ns":
        raise ValueError("unsupported sampling clock")
    previous_end = begin
    required = REQUIRED[os_name]
    series: dict[str, list[int]] = {name: [] for name in required}
    for i, row in enumerate(rows, 1):
        if not isinstance(row, dict) or row.get("provider") != PROVIDERS[os_name]:
            raise ValueError(f"sample {i}: unsupported or inconsistent memory provider")
        started = row.get("sample_started_monotonic_ns")
        finished = row.get("sample_finished_monotonic_ns")
        if (not _int(started) or not _int(finished)
                or started < previous_end or started >= finished or finished > end):
            raise ValueError(f"sample {i}: replayed, overlapping or unbounded clock interval")
        if started - previous_end > max_idle_ns:
            raise ValueError(f"sample {i}: sparse memory sampling cadence")
        elapsed_ms = row.get("elapsed_ms")
        if (type(elapsed_ms) not in (float, int) or not math.isfinite(elapsed_ms)
                or elapsed_ms < 0
                or abs(elapsed_ms - (started - begin) / 1_000_000) > 1_000):
            raise ValueError(f"sample {i}: elapsed/monotonic timestamp mismatch")
        previous_end = finished
        if (row.get("tree_memory_complete") is not True
                or not _int(row.get("sampled_process_count")) or row["sampled_process_count"] < 1):
            raise ValueError(f"sample {i}: incomplete or missing descendant coverage")
        for name in required:
            value = row.get(name)
            if not _int(value) or value <= 0:
                raise ValueError(f"sample {i}: missing/invalid {name}")
            series[name].append(value)
        if (row[required[1]] < row[required[0]] or row[required[3]] < row[required[2]]):
            raise ValueError(f"sample {i}: process-tree memory below main process")
    if end - previous_end > max_idle_ns:
        raise ValueError("sparse memory sampling cadence at window tail")
    return {
        "scope": "ATTACHED_MEMORY_PRECHECK_ONLY_NOT_INSTALLED_CERTIFICATION",
        "status": "ELIGIBLE_FOR_INDEPENDENT_AGENT07_REVIEW_NOT_CERTIFIED",
        "source_sha_operator_supplied": source_sha,
        "native_qa_run_url_operator_supplied": run_url,
        "package_sha256": package_hash,
        "executable_sha256": executable_hash,
        "host_os_operator_metadata": os_name,
        "pid_operator_supplied": report["pid"],
        "process_creation_id_reported": report["kernel_process_creation_id"],
        "study_count_operator_supplied_not_verified": report.get("study_count_operator_supplied"),
        "sample_count": len(rows),
        "window_elapsed_ms": round((end - begin) / 1_000_000, 3),
        "uncertified_memory_sample_percentiles": {k: _percentiles(v) for k, v in series.items()},
        "certification": "PENDING_INDEPENDENT_NATIVE_QA_AND_CANONICAL_STUDY_READBACK",
        "limitations": (
            "Input JSON/labels can be fabricated. Independent installed WebView PID, package/source "
            "provenance, parent/child/reparented process coverage, and UI activity remain unproved. "
            "SHA stability at the sampler endpoints cannot detect transient change-and-restore. "
            "These serial memory observations are not independent latency trials, neither cold/warm "
            "startup nor GUI responsiveness. Study count is operator-supplied."
        ),
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--attached-json", required=True, type=Path)
    parser.add_argument("--package", required=True, type=Path)
    parser.add_argument("--executable", required=True, type=Path)
    parser.add_argument("--source-sha", required=True)
    parser.add_argument("--run-url", required=True)
    parser.add_argument("--package-sha256", required=True)
    parser.add_argument("--executable-sha256", required=True)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    if args.output.exists() or not args.output.parent.is_dir():
        parser.error("output must not exist and its parent must be present")
    try:
        record = json.loads(args.attached_json.read_text(encoding="utf-8"))
        result = preflight(record, args.package, args.executable, args.source_sha,
                           args.run_url, args.package_sha256, args.executable_sha256)
        with args.output.open("x", encoding="utf-8") as fp:
            json.dump(result, fp, indent=2)
            fp.write("\n")
        print("Uncertified attached-memory preflight: " + str(args.output))
    except (OSError, ValueError, TypeError, json.JSONDecodeError) as exc:
        parser.exit(2, f"attached-memory preflight refused: {exc}\n")


if __name__ == "__main__":
    main()
