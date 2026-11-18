"""Exact-head synthetic SQL-scale smoke using Scriptotar's REAL Rust migrations.

This is a database correctness and warm-query benchmark, NOT a native
WebKitGTK/WebView2, Tauri IPC, editor or useful-Studio readiness measurement.
Outputs remain operator-independent evidence only for the narrow SQL scope.
"""
from __future__ import annotations

import argparse
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import platform
import re
import sqlite3
import subprocess
import sys

try:
    from .study_scale import build
    from .query_suite import profile
    from .seed_workspace import SIZES, digest
except ImportError:
    from study_scale import build
    from query_suite import profile
    from seed_workspace import SIZES, digest

ROOT = Path(__file__).resolve().parents[3]
SENTINEL = ".scriptotar-benchmark-only"
SHA = re.compile(r"[0-9a-f]{40}\Z")


def git(*args: str) -> str:
    return subprocess.run(
        ["git", *args], cwd=ROOT, check=True, text=True,
        stdout=subprocess.PIPE, stderr=subprocess.PIPE,
    ).stdout.strip()


def write_new(path: Path, data: dict) -> None:
    with path.open("x", encoding="utf-8") as stream:
        json.dump(data, stream, indent=2)
        stream.write("\n")


def run(out_dir: Path, source_sha: str, sizes: list[int], iterations: int) -> dict:
    if not SHA.fullmatch(source_sha):
        raise ValueError("source SHA must be exactly 40 lowercase hexadecimal characters")
    if not sizes or any(n not in SIZES for n in sizes):
        raise ValueError("sizes must be selected from 100, 1000 and 10000")
    if not 3 <= iterations <= 100:
        raise ValueError("iterations must be between 3 and 100")

    # No benchmark may claim a source SHA while building a different tree.
    actual_sha = git("rev-parse", "HEAD")
    if actual_sha != source_sha:
        raise ValueError(f"source SHA mismatch: checkout is {actual_sha}")
    if git("status", "--porcelain", "--untracked-files=no"):
        raise ValueError("tracked working tree must be clean before exact-head benchmarking")
    if not out_dir.is_absolute() or not out_dir.is_dir():
        raise ValueError("output directory must exist and be absolute")
    out_dir = out_dir.resolve()
    if (out_dir / SENTINEL).read_text(encoding="utf-8").strip() != "SYNTHETIC_ONLY":
        raise ValueError("scratch output directory must have SYNTHETIC_ONLY sentinel")

    template = out_dir / "blank-migrated.sqlite3"
    report_file = out_dir / "migrated-sql-report.json"
    expected_files = [template, report_file]
    for n in set(sizes):
        expected_files.extend([
            out_dir / f"scriptotar-synthetic-{n}.sqlite3",
            out_dir / f"scriptotar-synthetic-{n}.json",
            out_dir / f"sql-{n}.json",
        ])
    if any(path.exists() for path in expected_files):
        raise ValueError("refusing any existing benchmark output; use a new scratch directory")

    # The actual scriptotar-db entry point, not a pasted migration SQL stub.
    subprocess.run(
        ["cargo", "run", "--locked", "--release", "-p", "scriptotar-db",
         "--example", "make_blank_perf_template", "--", str(template)],
        cwd=ROOT, check=True,
    )

    data = {
        "scope": "APP_MIGRATED_SYNTHETIC_SQL_ONLY_NOT_NATIVE",
        "certification": "NOT_NATIVE_CERTIFIED",
        "source_sha_exact_git_head": source_sha,
        "template_sha256": digest(template),
        "template_bytes": template.stat().st_size,
        "sqlite_version_python": sqlite3.sqlite_version,
        "host_platform": platform.platform(),
        "logical_cpu_count": os.cpu_count(),
        "python_version": sys.version.split()[0],
        "timestamp_utc": datetime.now(timezone.utc).isoformat(),
        "iterations_per_query": iterations,
        "measurements": {},
        "limitations": (
            "Real Rust app migrations but synthetic Study rows and Python warm SQLite "
            "SELECT/fetchall only; no installed package, WebView, Tauri IPC or UI. "
            "No OS cold-cache controls or cross-machine improvement claims."
        ),
    }
    for n in dict.fromkeys(sizes):
        fixture = out_dir / f"scriptotar-synthetic-{n}.sqlite3"
        manifest = build(template, fixture, n, source_sha)
        if manifest["source_sha_operator_supplied"] != source_sha:
            raise ValueError("fixture provenance mismatch")
        write_new(out_dir / f"scriptotar-synthetic-{n}.json", manifest)
        sampled = profile(fixture, source_sha, iterations)
        if sampled["database_sha256"] != manifest["db_sha256"]:
            raise ValueError("fixture checksum changed during profiling")
        write_new(out_dir / f"sql-{n}.json", sampled)
        data["measurements"][str(n)] = {
            "manifest": f"scriptotar-synthetic-{n}.json",
            "query_report": f"sql-{n}.json",
            "schema_version": manifest["schema_version"],
            "database_sha256": manifest["db_sha256"],
        }
        print(f"SQL-only {n} Study rows: schema v{manifest['schema_version']}, "
              f"11 query shapes, {iterations} samples/query", flush=True)

    write_new(report_file, data)
    print(f"SQL-only report written: {report_file}", flush=True)
    return data


def main() -> None:
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--out-dir", type=Path, required=True)
    p.add_argument("--source-sha", help="must equal exact git HEAD; defaults to git HEAD")
    p.add_argument("--sizes", nargs="+", type=int, choices=sorted(SIZES),
                   default=[100, 1000, 10000])
    p.add_argument("--iterations", type=int, default=21)
    args = p.parse_args()
    try:
        run(args.out_dir, args.source_sha or git("rev-parse", "HEAD"),
            args.sizes, args.iterations)
    except (ValueError, OSError, subprocess.CalledProcessError) as exc:
        p.exit(2, f"migrated SQL benchmark refused/failed: {exc}\n")


if __name__ == "__main__":
    main()
