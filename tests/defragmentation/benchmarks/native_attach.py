"""Attach to an ALREADY-RUNNING installed Scriptotar process for memory evidence.

Agent 07 controls real native UI interactions. This read-only sampler never starts,
injects, edits, stops or kills the application. It verifies the observed PID's
executable path, hashes the provided installed package and binary, and retains
raw per-sample data. Package installation and real WebView UI cannot be proven
by process inspection; all observations remain PENDING independent QA.
"""
from __future__ import annotations

import argparse
import ctypes
import json
import os
from pathlib import Path
import platform
import re
import time

try:
    from .native_memory import memory_snapshot
    from .native_session import sha256
    from .native_trials import summary
except ImportError:
    from native_memory import memory_snapshot
    from native_session import sha256
    from native_trials import summary

SOURCE_SHA = re.compile(r"[0-9a-f]{40}\Z")
MEMORY_FIELDS = (
    "main_pss_bytes", "tree_pss_bytes",
    "main_private_resident_bytes", "tree_private_resident_bytes",
    "main_private_working_set_bytes", "tree_private_working_set_bytes",
    "main_private_commit_bytes", "tree_private_commit_bytes",
)


def process_image(pid: int) -> Path | None:
    """Return the kernel-reported executable path, never a user supplied label."""
    if not isinstance(pid, int) or pid <= 0:
        return None
    if platform.system() == "Linux":
        try:
            return Path(os.readlink(f"/proc/{pid}/exe")).resolve(strict=True)
        except (OSError, ValueError):
            return None
    if platform.system() == "Windows":
        from ctypes import wintypes as w
        kernel = ctypes.WinDLL("kernel32", use_last_error=True)
        kernel.OpenProcess.argtypes = [w.DWORD, w.BOOL, w.DWORD]
        kernel.OpenProcess.restype = w.HANDLE
        kernel.QueryFullProcessImageNameW.argtypes = [w.HANDLE, w.DWORD, w.LPWSTR, ctypes.POINTER(w.DWORD)]
        kernel.QueryFullProcessImageNameW.restype = w.BOOL
        kernel.CloseHandle.argtypes = [w.HANDLE]
        handle = kernel.OpenProcess(0x1000, False, pid)  # PROCESS_QUERY_LIMITED_INFORMATION
        if not handle:
            return None
        try:
            buffer = ctypes.create_unicode_buffer(32768)
            n = w.DWORD(len(buffer))
            if not kernel.QueryFullProcessImageNameW(handle, 0, buffer, ctypes.byref(n)):
                return None
            return Path(buffer.value).resolve(strict=True)
        except (OSError, ValueError):
            return None
        finally:
            kernel.CloseHandle(handle)
    return None


def same_image(observed: Path | None, expected: Path) -> bool:
    if observed is None:
        return False
    return os.path.normcase(str(observed)) == os.path.normcase(str(expected.resolve(strict=True)))



def process_instance_id(pid: int) -> str | None:
    """Kernel-derived creation identity; refuse silent PID reuse during sampling."""
    if not isinstance(pid, int) or pid <= 0:
        return None
    if platform.system() == "Linux":
        try:
            stat = Path(f"/proc/{pid}/stat").read_text(encoding="ascii")
            # The process comm between parentheses may contain whitespace.
            after_comm = stat[stat.rfind(")") + 1:].split()
            # Stat field 22 (starttime) is index 19 after field 2 (comm).
            return f"linux-{int(after_comm[19])}"
        except (OSError, UnicodeError, IndexError, ValueError):
            return None
    if platform.system() == "Windows":
        from ctypes import wintypes as w
        kernel = ctypes.WinDLL("kernel32", use_last_error=True)
        kernel.OpenProcess.argtypes = [w.DWORD, w.BOOL, w.DWORD]
        kernel.OpenProcess.restype = w.HANDLE
        kernel.GetProcessTimes.argtypes = [w.HANDLE] + [ctypes.POINTER(w.FILETIME)] * 4
        kernel.GetProcessTimes.restype = w.BOOL
        kernel.CloseHandle.argtypes = [w.HANDLE]
        handle = kernel.OpenProcess(0x1000, False, pid)
        if not handle:
            return None
        try:
            created, exited, kernel_time, user_time = (w.FILETIME() for _ in range(4))
            if not kernel.GetProcessTimes(handle, ctypes.byref(created),
                                          ctypes.byref(exited),
                                          ctypes.byref(kernel_time),
                                          ctypes.byref(user_time)):
                return None
            creation_ticks = (created.dwHighDateTime << 32) | created.dwLowDateTime
            return f"windows-{creation_ticks}"
        finally:
            kernel.CloseHandle(handle)
    return None


def attach(pid: int, executable: Path, package: Path, source_sha: str,
           seconds: float, interval_seconds: float, study_count: int | None,
           run_url: str | None) -> dict:
    if not SOURCE_SHA.fullmatch(source_sha):
        raise ValueError("source SHA must be exact 40 lowercase hex")
    if not 1 <= seconds <= 180 or not 0.1 <= interval_seconds <= 2:
        raise ValueError("seconds must be 1..180; interval 0.1..2.0 seconds")
    if study_count is not None and study_count not in (100, 1000, 10000):
        raise ValueError("study_count must be 100, 1000, 10000 or omitted")
    if not executable.is_file() or not package.is_file():
        raise ValueError("exact installed executable and package files are required")
    expected = executable.resolve(strict=True)
    if not same_image(process_image(pid), expected):
        raise ValueError("kernel process image does not match the supplied installed executable")
    instance = process_instance_id(pid)
    if instance is None:
        raise ValueError("could not independently determine process creation identity")
    begin = time.perf_counter()
    begin_monotonic_ns = time.perf_counter_ns()
    records = []
    end_reason = "duration_elapsed"
    while time.perf_counter() - begin < seconds:
        if (not same_image(process_image(pid), expected)
                or process_instance_id(pid) != instance):
            end_reason = "process_gone_or_identity_changed"
            break
        clock = time.perf_counter()
        sample_started_monotonic_ns = time.perf_counter_ns()
        try:
            memory = memory_snapshot(pid)
        except (OSError, RuntimeError, ValueError) as exc:
            memory = {"provider": platform.system(), "tree_memory_complete": False,
                      "sampling_error": type(exc).__name__}
        sample_finished_monotonic_ns = time.perf_counter_ns()
        # The app can exit and the kernel can reuse its PID *during* the
        # memory snapshot. Never report a sample unless both probes bracket
        # the same executable + birth identity. Discard the ambiguous sample.
        if (not same_image(process_image(pid), expected)
                or process_instance_id(pid) != instance):
            end_reason = "process_gone_or_identity_changed_during_sample"
            break
        records.append({
            "elapsed_ms": round((clock - begin) * 1000, 3),
            "sample_started_monotonic_ns": sample_started_monotonic_ns,
            "sample_finished_monotonic_ns": sample_finished_monotonic_ns,
            **memory,
        })
        remaining = seconds - (time.perf_counter() - begin)
        if remaining > 0:
            time.sleep(min(interval_seconds, remaining))
    finished_monotonic_ns = time.perf_counter_ns()
    samples = {name: summary([row.get(name) for row in records]) for name in MEMORY_FIELDS}
    return {
        "scope": "ATTACHED_INSTALLED_PROCESS_MEMORY_ONLY_NOT_UI_CERTIFICATION",
        "certification": "PENDING_INDEPENDENT_NATIVE_QA",
        "attestation": "Operator-supplied PID and package. Kernel process-image path matched at each sample; native WebView/Study size/installation and driver provenance are not independently proved.",
        "source_sha_operator_supplied": source_sha,
        "source_run_url_operator_supplied": run_url,
        "package_path": str(package.resolve()), "package_sha256": sha256(package),
        "executable_path": str(expected), "executable_sha256": sha256(executable),
        "pid": pid, "kernel_process_creation_id": instance,
        "monotonic_clock": "python.time.perf_counter_ns",
        "measurement_started_monotonic_ns": begin_monotonic_ns,
        "measurement_finished_monotonic_ns": finished_monotonic_ns,
        "platform": platform.platform(),
        "cpu": platform.processor() or "unknown", "logical_cpus": os.cpu_count(),
        "study_count_operator_supplied": study_count,
        "requested_duration_seconds": seconds, "interval_seconds": interval_seconds,
        "end_reason": end_reason, "sample_count": len(records),
        "complete_tree_sample_count": sum(x.get("tree_memory_complete") is True for x in records),
        "metrics": samples, "raw_samples": records,
        "limitations": "No independent UI readiness, action/IPC timing or installed package attestation. Monotonic sample brackets do NOT establish cross-process WebView UI clock synchronization; a separately attested shared-clock calibration is required before joining native-driver events. Process image matching is not package signature verification. Resident Linux PSS apportions shared pages; private resident excludes shared mappings. Windows private working set is resident and differs from private commit. Descendant enumeration can miss reparented WebView helper processes, and missing permissions yield null rather than partial memory totals.",
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--pid", type=int, required=True)
    parser.add_argument("--executable", type=Path, required=True)
    parser.add_argument("--package", type=Path, required=True)
    parser.add_argument("--source-sha", required=True)
    parser.add_argument("--seconds", type=float, default=30)
    parser.add_argument("--interval-seconds", type=float, default=0.25)
    parser.add_argument("--study-count", type=int, choices=(100, 1000, 10000))
    parser.add_argument("--run-url")
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if not args.output.parent.is_dir() or args.output.exists():
        parser.error("output must not exist and its parent must exist")
    try:
        evidence = attach(args.pid, args.executable, args.package,
                          args.source_sha, args.seconds, args.interval_seconds,
                          args.study_count, args.run_url)
        with args.output.open("x", encoding="utf-8") as fp:
            json.dump(evidence, fp, indent=2)
            fp.write("\n")
        print("Process-only memory evidence: " + str(args.output))
    except (OSError, ValueError) as exc:
        parser.exit(2, f"attached memory benchmark refused: {exc}\n")


if __name__ == "__main__":
    main()
