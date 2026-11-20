"""Read-only, uncertified process-tree CPU companion to native_attach.

An independent native driver must first prove the installed binary and actions.
This sampler cannot prove descendant completeness, WebView identity, installed
origin, Study counts, or UI readiness. Never label it a native benchmark PASS.
"""
from __future__ import annotations

import argparse
import json
import math
import os
from pathlib import Path
import platform
import re
import time

try:
    from .native_attach import (process_cpu_time_ns, process_image,
                                process_instance_id, same_image)
    from .native_memory import (_linux_processes, _windows_processes,
                                descendants)
    from .native_session import sha256
except ImportError:
    from native_attach import (process_cpu_time_ns, process_image,
                               process_instance_id, same_image)
    from native_memory import (_linux_processes, _windows_processes,
                               descendants)
    from native_session import sha256

SHA40 = re.compile(r"[0-9a-f]{40}\Z")


def _processes() -> dict[int, int]:
    system = platform.system()
    if system == "Linux":
        return _linux_processes()
    if system == "Windows":
        return _windows_processes()
    raise ValueError("unsupported native CPU platform")


def snapshot(root_pid: int, expected_executable: Path) -> dict:
    """Best-effort bounded-time snapshot; reject PID reuse and tree churn."""
    started = time.perf_counter_ns()
    if not same_image(process_image(root_pid), expected_executable):
        raise ValueError("root process image is not the expected executable")
    root_birth = process_instance_id(root_pid)
    if root_birth is None:
        raise ValueError("missing root process birth identity")
    members = descendants(root_pid, _processes())
    if root_pid not in members:
        raise ValueError("root process absent from process enumeration")
    records = {}
    for pid in sorted(members):
        birth_before = process_instance_id(pid)
        cpu_ns = process_cpu_time_ns(pid)
        birth_after = process_instance_id(pid)
        if (birth_before is None or birth_after != birth_before
                or type(cpu_ns) is not int or cpu_ns < 0):
            raise ValueError("incomplete or PID-reused process-tree CPU sample")
        records[str(pid)] = {"birth_id": birth_before, "cpu_ns": cpu_ns}
    # Child creation/removal during enumeration destroys a whole-tree claim.
    if (descendants(root_pid, _processes()) != members
            or process_instance_id(root_pid) != root_birth
            or not same_image(process_image(root_pid), expected_executable)):
        raise ValueError("process tree changed during CPU snapshot")
    finished = time.perf_counter_ns()
    return {"started_ns": started, "finished_ns": finished,
            "root_pid": root_pid, "members": records}


def compare(first: dict, second: dict) -> dict:
    """Compute only stable identity-matched deltas; never interpolate churn."""
    if (not isinstance(first, dict) or not isinstance(second, dict)
            or type(first.get("root_pid")) is not int
            or first["root_pid"] != second.get("root_pid")):
        raise ValueError("root PID mismatch")
    for item in (first, second):
        if (type(item.get("started_ns")) is not int
                or type(item.get("finished_ns")) is not int
                or not 0 < item["started_ns"] <= item["finished_ns"]
                or not isinstance(item.get("members"), dict)):
            raise ValueError("invalid CPU timestamp or member structure")
    if first["finished_ns"] >= second["started_ns"]:
        raise ValueError("CPU snapshots have overlapping time windows")
    left, right = first["members"], second["members"]
    if not left or left.keys() != right.keys() or str(first["root_pid"]) not in left:
        raise ValueError("process-tree membership changed between snapshots")
    changes = {}
    for pid in left:
        old, new = left[pid], right[pid]
        if (not isinstance(old, dict) or not isinstance(new, dict)
                or not isinstance(old.get("birth_id"), str)
                or new.get("birth_id") != old["birth_id"]
                or type(old.get("cpu_ns")) is not int
                or type(new.get("cpu_ns")) is not int
                or not 0 <= old["cpu_ns"] <= new["cpu_ns"]):
            raise ValueError("missing/reused process or decreasing CPU counter")
        changes[pid] = new["cpu_ns"] - old["cpu_ns"]
    # Counters are read sequentially, not atomically. Give bounded time
    # uncertainty, never a fictitious precise whole-tree CPU percentage.
    min_elapsed = second["started_ns"] - first["finished_ns"]
    max_elapsed = second["finished_ns"] - first["started_ns"]
    total = sum(changes.values())
    return {
        "status": "ROOT_AND_VISIBLE_DESCENDANTS_ONLY_UNCERTIFIED",
        "root_cpu_delta_ns": changes[str(first["root_pid"])],
        "visible_tree_cpu_delta_ns": total,
        "process_count": len(changes),
        "min_possible_interval_ns": min_elapsed,
        "max_possible_interval_ns": max_elapsed,
        "visible_tree_cpu_percent_one_core_lower_bound": round(100 * total / max_elapsed, 3),
        "visible_tree_cpu_percent_one_core_upper_bound": round(100 * total / min_elapsed, 3),
    }


def sample(pid: int, executable: Path, package: Path, source_sha: str,
           duration_seconds: float) -> dict:
    if type(pid) is not int or pid <= 0 or not SHA40.fullmatch(source_sha):
        raise ValueError("positive PID and exact source SHA required")
    if (type(duration_seconds) not in (int, float)
            or not math.isfinite(duration_seconds)
            or not 1 <= duration_seconds <= 180):
        raise ValueError("sampling duration must be finite and 1..180 seconds")
    if (not executable.is_file() or executable.is_symlink()
            or not package.is_file() or package.is_symlink()):
        raise ValueError("existing nonsymlink package and executable required")
    exe = executable.resolve(strict=True)
    pkg = package.resolve(strict=True)
    before_hashes = sha256(exe), sha256(pkg)
    before = snapshot(pid, exe)
    time.sleep(duration_seconds)
    after = snapshot(pid, exe)
    if (sha256(exe), sha256(pkg)) != before_hashes:
        raise ValueError("installed executable/package changed during sampling")
    result = compare(before, after)
    return {
        "scope": "READ_ONLY_VISIBLE_PROCESS_TREE_CPU_NOT_INSTALLED_CERTIFICATION",
        "certification": "PENDING_INDEPENDENT_AGENT07_NATIVE_QA",
        "source_sha_operator_supplied": source_sha,
        "executable_path": str(exe), "executable_sha256": before_hashes[0],
        "package_path": str(pkg), "package_sha256": before_hashes[1],
        "platform": platform.platform(), "pid": pid,
        "duration_seconds_requested": duration_seconds,
        "before": before, "after": after, "delta": result,
        "limitations": (
            "PID ancestry alone cannot attest WebKitGTK/WebView2 helpers; reparented, "
            "unreadable, or transient descendants may be omitted. Tree churn is "
            "rejected at observed boundaries but can occur between snapshots. "
            "CPU timestamp brackets are non-atomic, and user-provided source "
            "SHA/package metadata are not independent provenance. No GUI action, "
            "IPC, database size, cold-start or useful UI timings are measured."
        ),
    }


def main() -> None:
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--pid", required=True, type=int)
    p.add_argument("--executable", required=True, type=Path)
    p.add_argument("--package", required=True, type=Path)
    p.add_argument("--source-sha", required=True)
    p.add_argument("--seconds", type=float, default=10.0)
    p.add_argument("--output", required=True, type=Path)
    args = p.parse_args()
    if args.output.exists() or args.output.is_symlink() or not args.output.parent.is_dir():
        p.error("output must not exist and its parent must exist")
    try:
        report = sample(args.pid, args.executable, args.package,
                        args.source_sha, args.seconds)
        with args.output.open("x", encoding="utf-8") as out:
            json.dump(report, out, indent=2)
            out.write("\n")
        print("Read-only visible process-tree CPU sample: " + str(args.output))
    except (OSError, ValueError) as exc:
        p.exit(2, f"native CPU sampling refused: {exc}\n")


if __name__ == "__main__":
    main()
