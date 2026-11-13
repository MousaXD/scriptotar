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
    samples = 0
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
            samples += 1
            time.sleep(0.1)
    finally:
        if process.poll() is None:
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
        "child_process_rss_bytes": None,
        "ipc_payload_bytes": None,
        "exit_code_before_cleanup": process.poll(),
        "certification": "NOT_CERTIFIED",
        "limitations": "Main-process RSS excludes sidecars; no automated WebView ready proof; process liveness is not UI readiness."
    }
    encoded = json.dumps(report, indent=2) + "\n"
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(encoded, encoding="utf-8")
    else:
        print(encoded, end="")

if __name__ == "__main__":
    main()
