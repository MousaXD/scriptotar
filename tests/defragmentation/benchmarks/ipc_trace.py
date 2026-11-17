"""Validate and summarize OUTSIDE-COLLECTED installed Tauri IPC trace JSONL.

Never infer IPC latency from SQLite SELECTs, Chromium mocks or process liveness.
This accepts an explicit trusted-data-format export from a genuine installed
WebView driver, ideally collected/attested by independent native QA Agent 07.
There is deliberately no fake/mock trace generator or secret payload logging.
"""
from __future__ import annotations

import argparse
import json
import math
from pathlib import Path
import re
import statistics

EXPECTED_KIND = "installed-tauri-webview-ipc"
HASH = re.compile(r"[0-9a-f]{64}\Z")
SOURCE = re.compile(r"[0-9a-f]{40}\Z")


def percentile(items: list[float], p: float) -> float:
    values = sorted(items)
    return round(values[max(0, math.ceil(p * len(values)) - 1)], 4)


def summarize_trace(path: Path, source_sha: str, package_sha256: str,
                    executable_sha256: str) -> dict:
    if not SOURCE.fullmatch(source_sha) or not all(HASH.fullmatch(s) for s in (package_sha256, executable_sha256)):
        raise ValueError("source SHA and both SHA-256 hashes required")
    if not path.is_file():
        raise ValueError("trace JSONL file does not exist")
    grouped: dict[str, list[dict]] = {}
    count = 0
    for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        if not line.strip():
            continue
        row = json.loads(line)
        if not isinstance(row, dict) or row.get("evidence_kind") != EXPECTED_KIND:
            raise ValueError(f"line {line_no}: not an explicitly native IPC event")
        if any(row.get(key) != expected for key, expected in (
            ("source_sha", source_sha), ("package_sha256", package_sha256),
            ("executable_sha256", executable_sha256))):
            raise ValueError(f"line {line_no}: source/package/executable hash mismatch")
        name = row.get("command")
        if not isinstance(name, str) or not re.fullmatch(r"[a-z][a-z_0-9]{1,80}", name):
            raise ValueError(f"line {line_no}: invalid Tauri command name")
        if not isinstance(row.get("session_id"), str) or not 8 <= len(row["session_id"]) <= 128:
            raise ValueError(f"line {line_no}: session ID required")
        for field in ("duration_ms", "request_bytes", "response_bytes"):
            v = row.get(field)
            if isinstance(v, bool) or not isinstance(v, (int, float)) or not math.isfinite(v) or v < 0:
                raise ValueError(f"line {line_no}: invalid {field}")
            if field != "duration_ms" and (not isinstance(v, int) or v > 100_000_000):
                raise ValueError(f"line {line_no}: invalid payload byte length")
        if row["duration_ms"] > 600_000:
            raise ValueError(f"line {line_no}: implausible IPC duration")
        # Avoid returning raw transcripts/headers/requests or arbitrary fields.
        grouped.setdefault(name, []).append({
            "duration_ms": float(row["duration_ms"]), "request_bytes": row["request_bytes"],
            "response_bytes": row["response_bytes"], "session_id": row["session_id"]})
        count += 1
    if count < 3:
        raise ValueError("at least three real IPC observations required")
    commands = {}
    for name, records in sorted(grouped.items()):
        durations = [r["duration_ms"] for r in records]
        commands[name] = {
            "count": len(records), "distinct_sessions": len({r["session_id"] for r in records}),
            "duration_ms_p50": round(statistics.median(durations), 4),
            "duration_ms_p95_nearest_rank": percentile(durations, 0.95),
            "request_bytes_p95": percentile([r["request_bytes"] for r in records], 0.95),
            "response_bytes_p95": percentile([r["response_bytes"] for r in records], 0.95),
            "raw_measurements": records,
        }
    return {
        "scope": "OPERATOR_SUPPLIED_NATIVE_WEBVIEW_IPC_TRACE",
        "attestation": "Format and consistency checked; genuine installed UI provenance must be independently audited.",
        "source_sha": source_sha, "package_sha256": package_sha256,
        "executable_sha256": executable_sha256, "event_count": count,
        "commands": commands, "certification": "PENDING_INDEPENDENT_NATIVE_QA",
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--trace", type=Path, required=True)
    parser.add_argument("--source-sha", required=True)
    parser.add_argument("--package-sha256", required=True)
    parser.add_argument("--executable-sha256", required=True)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    report = summarize_trace(args.trace, args.source_sha, args.package_sha256, args.executable_sha256)
    result = json.dumps(report, indent=2) + "\n"
    if args.output:
        if args.output.exists() or not args.output.parent.is_dir():
            parser.error("output JSON must not exist and parent directory must exist")
        args.output.write_text(result, encoding="utf-8")
    else:
        print(result, end="")


if __name__ == "__main__":
    main()
