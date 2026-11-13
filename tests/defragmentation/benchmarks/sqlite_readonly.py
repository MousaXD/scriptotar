"""Read-only SQLite diagnostic; NOT the application's bootstrap/IPC benchmark.

Use an isolated copied or synthetic migrated DB. Outputs aggregate counts/timings
only, never user content or transcript text. Run at 100, 1000 and 10000 rows.
"""
import argparse
import json
import re
from pathlib import Path
import sqlite3
import statistics
import time

TABLES = ("projects", "creators", "research_items", "sources",
          "transcripts", "jobs", "ai_runs")

def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--db", type=Path, required=True)
    p.add_argument("--source-sha", required=True)
    p.add_argument("--iterations", type=int, default=7)
    p.add_argument("--output", type=Path)
    args = p.parse_args()
    if not re.fullmatch(r"[a-f0-9]{40}", args.source_sha):
        p.error("--source-sha requires exact 40-hex commit")
    if not args.db.is_file() or not 3 <= args.iterations <= 100:
        p.error("existing database and 3..100 iterations required")
    uri = args.db.resolve().as_uri() + "?mode=ro"
    conn = sqlite3.connect(uri, uri=True)
    conn.execute("PRAGMA query_only=ON")
    try:
        names = {row[0] for row in conn.execute(
            "SELECT name FROM sqlite_master WHERE type='table'")}
        readings = {}
        for table in TABLES:
            if table not in names:
                readings[table] = {"status": "MISSING"}
                continue
            count = conn.execute(f"SELECT COUNT(*) FROM {table}").fetchone()[0]
            times = []
            for _ in range(args.iterations):
                start = time.perf_counter()
                conn.execute(f"SELECT id FROM {table} ORDER BY rowid DESC LIMIT 50").fetchall()
                times.append((time.perf_counter() - start) * 1000)
            readings[table] = {"rows": count, "median_recent_id_query_ms": round(
                statistics.median(times), 3)}
        report = {"scope": "read-only SQL diagnostic; NOT native IPC/bootstrap",
                  "source_sha_operator_supplied": args.source_sha,
                  "db_file_bytes": args.db.stat().st_size,
                  "sqlite_version": sqlite3.sqlite_version,
                  "page_count": conn.execute("PRAGMA page_count").fetchone()[0],
                  "page_size": conn.execute("PRAGMA page_size").fetchone()[0],
                  "tables": readings, "certification": "NOT_CERTIFIED",
                  "limits": "Counts and SQLite ID probes are not bootstrap timings, IPC transfer sizes or GUI frame timings."}
    finally:
        conn.close()
    output = json.dumps(report, indent=2) + "\n"
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(output, encoding="utf-8")
    else:
        print(output, end="")

if __name__ == "__main__":
    main()
