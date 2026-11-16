"""Repeated installed-executable process sampling; UI milestone is external-only.

Pass the installed package and its actual installed executable. The operator (or
Agent 07's real native WebView driver) may create each printed ready-file only
AFTER the useful Study workspace is rendered. Without it, readiness remains null.
This cannot establish that the package was actually installed or prove WebView.
"""
from __future__ import annotations

import argparse
import json
import math
import os
from pathlib import Path
import platform
import re
import statistics
import subprocess
import sys

try:
    from .native_session import sha256
except ImportError:
    from native_session import sha256


def summary(samples: list[float | int | None]) -> dict:
    values = sorted(float(x) for x in samples if x is not None)
    return {"valid_trials": len(values),
            "p50": round(statistics.median(values), 3) if values else None,
            "p95_nearest_rank": round(values[math.ceil(0.95 * len(values)) - 1], 3) if values else None}


def run_trials(executable: Path, package: Path, source_sha: str, mode: str,
               cache_state: str, count: int, seconds: float, ready_dir: Path,
               data_dir: Path) -> dict:
    if not re.fullmatch(r"[0-9a-f]{40}", source_sha):
        raise ValueError("source SHA must be 40 lowercase hex")
    if mode not in ("cold", "warm") or cache_state not in ("unknown", "warm-cache", "operator-rebooted", "operator-dropped-caches"):
        raise ValueError("invalid mode or declared cache state")
    if not 3 <= count <= 30 or not 1 <= seconds <= 180:
        raise ValueError("3..30 trials, 1..180 seconds required")
    if not executable.is_file() or not package.is_file() or not ready_dir.is_dir() or not data_dir.is_dir():
        raise ValueError("installed executable, package, ready directory and isolated data directory must exist")
    sentinel = data_dir / ".scriptotar-benchmark-only"
    if data_dir.resolve() == Path.home().resolve() or not sentinel.is_file() or sentinel.read_text(encoding="utf-8").strip() != "SYNTHETIC_ONLY":
        raise ValueError("isolated data directory must contain .scriptotar-benchmark-only with SYNTHETIC_ONLY")
    if mode == "cold" and cache_state == "warm-cache":
        raise ValueError("cannot label warm-cache trials cold")
    if mode == "warm" and cache_state in ("operator-rebooted", "operator-dropped-caches"):
        raise ValueError("cannot label explicitly cold-cache trials warm")
    suite = []
    for index in range(count):
        ready = ready_dir / f"scriptotar-{mode}-trial-{index + 1:02d}.ready"
        if ready.exists():
            raise ValueError(f"stale readiness file exists; remove it: {ready}")
        print(f"Trial {index + 1}/{count}: native UI readiness marker (optional): {ready}", flush=True)
        command = [sys.executable, str(Path(__file__).with_name("native_session.py")),
                   "--executable", str(executable), "--source-sha", source_sha,
                   "--trial", mode, "--seconds", str(seconds), "--ready-file", str(ready)]
        env = dict(os.environ)
        env["SCRIPTOTAR_DATA_DIR"] = str(data_dir.resolve())
        complete = subprocess.run(command, capture_output=True, text=True, check=True, env=env)
        trial = json.loads(complete.stdout)
        # Existing sampler reports only process observations and operator marker.
        # Never upgrade that claim to native GUI certification.
        suite.append(trial)
        ready.unlink(missing_ok=True)
    total_mem_kib = None
    if platform.system() == "Linux":
        try:
            for line in Path("/proc/meminfo").read_text().splitlines():
                if line.startswith("MemTotal:"):
                    total_mem_kib = int(line.split()[1])
                    break
        except (OSError, IndexError, ValueError):
            pass
    return {
        "scope": "REPEATED_INSTALLED_EXECUTABLE_PROCESS_SAMPLES_NOT_WEBVIEW_CERTIFICATION",
        "source_sha_operator_supplied": source_sha,
        "package_path": str(package.resolve()), "package_sha256": sha256(package),
        "executable_path": str(executable.resolve()), "executable_sha256": sha256(executable),
        "isolated_data_dir": str(data_dir.resolve()),
        "platform": platform.platform(), "cpu": platform.processor() or "unknown",
        "cpu_logical_count": os.cpu_count(), "system_ram_bytes": total_mem_kib * 1024 if total_mem_kib else None,
        "cache_mode_operator_declared": mode, "cache_state_operator_declared": cache_state,
        "trial_count": count, "trial_duration_seconds": seconds,
        "raw_trials": suite,
        "alive_ms": summary([x["process_alive_detected_ms"] for x in suite]),
        "useful_workspace_operator_ack_ms": summary([x["useful_workspace_operator_ack_ms"] for x in suite]),
        "peak_main_rss_bytes": summary([x["peak_main_process_rss_bytes"] for x in suite]),
        "peak_tree_rss_bytes": summary([x["peak_process_tree_rss_bytes"] for x in suite]),
        "certification": "NOT_NATIVE_CERTIFIED",
        "limitations": "UI-ready markers are external, unauthenticated operator or driver observations. No internal WebView event/IPC measurement; Linux RSS double-counts shared mappings; cache state and package installation are operator-asserted."
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--executable", type=Path, required=True)
    parser.add_argument("--package", type=Path, required=True)
    parser.add_argument("--source-sha", required=True)
    parser.add_argument("--mode", required=True, choices=["cold", "warm"])
    parser.add_argument("--cache-state", choices=["unknown", "warm-cache", "operator-rebooted", "operator-dropped-caches"], default="unknown")
    parser.add_argument("--trials", type=int, default=5)
    parser.add_argument("--seconds", type=float, default=30)
    parser.add_argument("--ready-dir", type=Path, required=True)
    parser.add_argument("--data-dir", type=Path, required=True, help="Isolated benchmark-only SCRIPTOTAR_DATA_DIR with sentinel")
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if not args.output.parent.is_dir() or args.output.exists():
        parser.error("report parent must exist and file must not exist")
    report = run_trials(args.executable, args.package, args.source_sha, args.mode,
                        args.cache_state, args.trials, args.seconds, args.ready_dir, args.data_dir)
    args.output.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(f"Process-only report: {args.output}")


if __name__ == "__main__":
    main()
