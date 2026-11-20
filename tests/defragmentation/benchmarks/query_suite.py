"""Read-only, query-shaped SQLite/Study scale profile (NOT Tauri IPC).

Only accepts the generated SYNTHETIC QA fixture project. Uses actual indexed
query shapes for Study search, selected-draft fetch and source/version lists;
profiled SQL is not an end-to-end Rust AppServices timing or JSON IPC payload.
"""
from __future__ import annotations

import argparse
from contextlib import closing
import json
import math
from pathlib import Path
import re
import sqlite3
import statistics
import time

try:
    from .seed_workspace import digest, identity
except ImportError:
    from seed_workspace import digest, identity

QUERIES = {
    "study_search_200_metadata": (
        "SELECT id,project_id,title,goal,status,revision,created_at,updated_at,archived_at "
        "FROM content_studies WHERE project_id=? AND (?=1 OR archived_at IS NULL) "
        "AND (title LIKE ? ESCAPE '\\' OR goal LIKE ? ESCAPE '\\') "
        "ORDER BY updated_at DESC,id DESC LIMIT ?", "search"),
    "study_exact_open": (
        "SELECT id,project_id,title,goal,status,revision,created_at,updated_at,archived_at "
        "FROM content_studies WHERE project_id=? AND id=?", "exact"),
    "study_source_list": (
        "SELECT id,project_id,study_id,kind,reference_id,locator,created_at "
        "FROM study_sources WHERE project_id=? AND study_id=? ORDER BY created_at,id", "exact"),
    "study_brief_exact": (
        "SELECT study_id,audience,goal,language,platform,tone,revision "
        "FROM study_briefs WHERE project_id=? AND study_id=?", "exact"),
    "draft_selector_200_metadata": (
        "SELECT id,title,revision FROM creative_drafts WHERE project_id=? AND study_id=? "
        "AND archived_at IS NULL ORDER BY updated_at DESC,id DESC LIMIT ?", "draft_list"),
    "draft_selected_body": (
        "SELECT id,title,body,revision FROM creative_drafts WHERE project_id=? AND id=?", "draft"),
    "draft_version_history": (
        "SELECT v.id,v.version,v.title,v.body,v.saved_at FROM draft_versions v "
        "JOIN creative_drafts d ON d.id=v.draft_id "
        "WHERE d.project_id=? AND v.draft_id=? ORDER BY v.version", "draft"),
    "research_page_50": (
        "SELECT id,title,view_count,like_count,scanned_at FROM research_items "
        "WHERE project_id=? ORDER BY scanned_at DESC LIMIT 50", "project"),
    "job_page_50": (
        "SELECT id,state,created_at FROM jobs WHERE project_id=? ORDER BY created_at DESC LIMIT 50", "project"),
    "transcript_metadata_50": (
        "SELECT t.id,t.language,t.created_at FROM transcripts t "
        "JOIN media m ON m.id=t.media_id JOIN sources s ON s.id=m.source_id "
        "WHERE s.project_id=? ORDER BY t.created_at DESC LIMIT 50", "project"),
    "transcript_full_body_1mb": (
        "SELECT t.id,t.language,t.text FROM transcripts t "
        "JOIN media m ON m.id=t.media_id JOIN sources s ON s.id=m.source_id "
        "WHERE s.project_id=? AND t.id=?", "transcript"),
}


def percentile(values: list[float], q: float) -> float | None:
    if not values:
        return None
    ordered = sorted(values)
    return round(ordered[max(0, math.ceil(q * len(ordered)) - 1)], 4)


def params_for(kind: str, rows: int) -> tuple:
    project = identity("project", rows)
    if kind == "search":
        return (project, 0, "%SYNTHETIC%", "%SYNTHETIC%", 200)
    if kind == "exact":
        return (project, identity("study", f"{rows}-0"))
    if kind == "draft_list":
        return (project, identity("study", f"{rows}-0"), 200)
    if kind == "draft":
        return (project, identity("draft", f"{rows}-0"))
    if kind == "transcript":
        return (project, identity("transcript", f"{rows}-0"))
    return (project,)


def row_bytes(row: tuple) -> int:
    # Approximate fetched text/blob bytes, *not* serialized JSON or IPC bytes.
    return sum(len(v.encode("utf-8")) if isinstance(v, str) else
               len(v) if isinstance(v, bytes) else 0 for v in row)


def profile(db_file: Path, source_sha: str, iterations: int = 11) -> dict:
    if not re.fullmatch(r"[0-9a-f]{40}", source_sha) or not 3 <= iterations <= 100:
        raise ValueError("40-hex source SHA and 3..100 measured iterations required")
    if not db_file.is_file():
        raise ValueError("database must already exist")
    with closing(sqlite3.connect(db_file.resolve().as_uri() + "?mode=ro", uri=True)) as db:
        db.execute("PRAGMA query_only=ON")
        db.execute("PRAGMA busy_timeout=5000")
        project_rows = db.execute("SELECT id,name FROM projects WHERE name LIKE 'SYNTHETIC QA %'").fetchall()
        if len(project_rows) != 1 or not re.fullmatch(r"SYNTHETIC QA (100|1000|10000)", project_rows[0][1]):
            raise ValueError("refusing to profile non-synthetic workspace")
        rows = int(project_rows[0][1].rsplit(" ", 1)[1])
        if project_rows[0][0] != identity("project", rows):
            raise ValueError("synthetic fixture project ID mismatch")
        for table in ("content_studies", "study_sources", "creative_drafts", "draft_versions", "transcripts"):
            count = db.execute(f"SELECT COUNT(*) FROM {table}").fetchone()[0]
            if count != rows:
                raise ValueError(f"expected {rows} synthetic {table}, got {count}")
        output = {}
        for label, (query, kind) in QUERIES.items():
            params = params_for(kind, rows)
            plan = [entry[-1] for entry in db.execute("EXPLAIN QUERY PLAN " + query, params)]
            db.execute(query, params).fetchall()
            samples = []
            row_count, fetched_bytes = None, None
            for _ in range(iterations):
                start = time.perf_counter_ns()
                result = db.execute(query, params).fetchall()
                samples.append((time.perf_counter_ns() - start) / 1_000_000)
                row_count = len(result)
                fetched_bytes = sum(row_bytes(row) for row in result)
            if not row_count:
                raise ValueError(f"unexpected empty query: {label}")
            output[label] = {
                "samples_ms": [round(n, 4) for n in samples],
                "p50_ms": round(statistics.median(samples), 4),
                "p95_ms_nearest_rank": percentile(samples, 0.95),
                "rows_returned": row_count, "fetched_field_bytes_estimate": fetched_bytes,
                "explain_query_plan": plan,
            }
        return {
            "scope": "SQL_QUERY_ONLY_NOT_RUST_IPC_OR_NATIVE_GUI",
            "source_sha_operator_supplied": source_sha,
            "database_sha256": digest(db_file), "db_file_bytes": db_file.stat().st_size,
            "records_per_collection": rows, "iterations_per_query": iterations,
            "sqlite_version": sqlite3.sqlite_version, "query_profiles": output,
            "certification": "NOT_NATIVE_CERTIFIED",
            "limitations": "Warm read-only SQLite SELECT+fetchall only; no Rust connection ownership, Tauri serialization, native UI, IPC or OS cold cache.",
        }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--db", required=True, type=Path)
    parser.add_argument("--source-sha", required=True)
    parser.add_argument("--iterations", type=int, default=11)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    report = profile(args.db, args.source_sha, args.iterations)
    encoded = json.dumps(report, indent=2) + "\n"
    if args.output:
        if not args.output.parent.is_dir() or args.output.exists():
            parser.error("output directory must exist and JSON report must not exist")
        args.output.write_text(encoded, encoding="utf-8")
    else:
        print(encoded, end="")


if __name__ == "__main__":
    main()
