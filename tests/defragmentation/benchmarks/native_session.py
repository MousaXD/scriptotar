"""Collect process-only data from an operator-supplied INSTALLED desktop executable.

No WebView automation here: file readiness is operator-observed and cannot certify UI
startup. Never pass a debug build off as an installed Tauri package.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import subprocess
import time

def sha256(path):
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()

def rss_bytes(pid):
    try:
        for row in Path(f"/proc/{pid}/status").read_text().splitlines():
            if row.startswith("VmRSS:"):
                return int(row.split()[1]) * 1024
    except (OSError, IndexError, ValueError):
        pass
    return None

def process_tree_rss(root_pid):
    """Return sampled (sum-RSS bytes, descendant count), or (None, 0).

    Sum of RSS includes shared mappings more than once; it is NOT PSS or
    physical memory consumed. Only currently parented Linux /proc tasks count.
    """
    proc = Path("/proc")
    if not proc.is_dir():
        return None, 0
    entries = {}
    for path in proc.iterdir():
        if not path.name.isdigit():
            continue
        ppid = rss = None
        try:
            for line in (path / "status").read_text().splitlines():
                if line.startswith("PPid:"):
                    ppid = int(line.split()[1])
                elif line.startswith("VmRSS:"):
                    rss = int(line.split()[1]) * 1024
        except (OSError, ValueError, IndexError):
            continue
        if ppid is not None:
            entries[int(path.name)] = (ppid, rss)
    if root_pid not in entries:
        return None, 0
    selected = {root_pid}
    while True:
        added = {pid for pid, (ppid, _) in entries.items()
                 if pid not in selected and ppid in selected}
        if not added:
            break
        selected.update(added)
    available = [entries[pid][1] for pid in selected if entries[pid][1] is not None]
    return (sum(available) if available else None), max(0, len(selected) - 1)

def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--executable", required=True, type=Path)
    p.add_argument("--source-sha", required=True)
    p.add_argument("--trial", required=True, choices=["cold", "warm", "concurrent-jobs"])
    p.add_argument("--seconds", type=float, default=15)
    p.add_argument("--ready-file", type=Path, help="Optional external manual UI acknowledgement")
    p.add_argument("--output", type=Path)
    args = p.parse_args()
    if not re.fullmatch(r"[0-9a-f]{40}", args.source_sha):
        p.error("--source-sha must be a 40-hex commit")
    if not args.executable.is_file() or args.seconds < 1 or args.seconds > 180:
        p.error("an existing installed executable and 1..180 seconds are required")
    if args.ready_file and args.ready_file.exists():
        p.error("ready file must not exist before launch")
    start = time.perf_counter()
    process = subprocess.Popen([str(args.executable.resolve())],
                               stdin=subprocess.DEVNULL,
                               stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    alive_at = None
    ready_at = None
    peak_rss = None
    peak_process_tree_rss = None
    max_descendants = 0
    samples = 0
    exit_before_cleanup = None
    try:
        until = start + args.seconds
        while time.perf_counter() < until and process.poll() is None:
            now = time.perf_counter()
            alive_at = alive_at or now
            if args.ready_file and args.ready_file.exists() and ready_at is None:
                ready_at = now
            rss = rss_bytes(process.pid)
            if rss is not None:
                peak_rss = max(peak_rss or 0, rss)
            tree_rss, descendants = process_tree_rss(process.pid)
            if tree_rss is not None:
                peak_process_tree_rss = max(peak_process_tree_rss or 0, tree_rss)
            max_descendants = max(max_descendants, descendants)
            samples += 1
            time.sleep(0.1)
    finally:
        exit_before_cleanup = process.poll()
        if exit_before_cleanup is None:
            process.terminate()
            try:
                process.wait(timeout=3)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait(timeout=3)
    report = {
        "scope": "native-process-sampling-only; NOT installed-app certification",
        "source_sha_operator_supplied": args.source_sha,
        "executable": str(args.executable.resolve()),
        "executable_sha256": sha256(args.executable),
        "os": platform.platform(),
        "cpu": platform.processor() or "unknown",
        "logical_cpus": os.cpu_count(),
        "trial": args.trial, "sample_count": samples,
        "process_alive_detected_ms": round((alive_at - start) * 1000, 2) if alive_at else None,
        "useful_workspace_operator_ack_ms": round((ready_at - start) * 1000, 2) if ready_at else None,
        "peak_main_process_rss_bytes": peak_rss,
        "peak_process_tree_rss_bytes": peak_process_tree_rss,
        "max_descendants_observed": max_descendants,
        "process_tree_rss_is_shared_memory_inclusive": True,
        "ipc_payload_bytes": None,
        "exit_code_before_cleanup": exit_before_cleanup,
        "certification": "NOT_CERTIFIED",
        "limitations": "Linux parent/child RSS summed, shared mappings double-counted, orphans missed; no automated WebView readiness; no IPC transfer measurement; process liveness is not UI readiness."
    }
    encoded = json.dumps(report, indent=2) + "\n"
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(encoded, encoding="utf-8")
    else:
        print(encoded, end="")

if __name__ == "__main__":
    main()
