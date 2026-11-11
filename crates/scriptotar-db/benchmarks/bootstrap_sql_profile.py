#!/usr/bin/env python3
"""Dependency-free, disposable SQL-only baseline/summary microbenchmark.

This is NOT a Rust/Tauri/IPC benchmark. It recreates just the relevant SQLite
tables and pre-existing indexes, inserts two projects of synthetic transcripts
and jobs, and times warm SELECT+fetchall calls on the same connection.
No production data are read or modified.

Run: python crates/scriptotar-db/benchmarks/bootstrap_sql_profile.py
"""
from __future__ import annotations

import json
import sqlite3
import statistics
import tempfile
import time
from pathlib import Path

FULL_TRANSCRIPTS = """SELECT s.project_id, s.id, s.creator_id, s.source_type, s.locator,
 s.title, s.created_at, m.id, m.local_path, m.duration_seconds,
 m.mime_type, m.created_at, t.id, t.language, t.text, t.segments_json,
 t.words_json, t.created_at, t.updated_at
 FROM transcripts t
 JOIN media m ON m.id=t.media_id
 JOIN sources s ON s.id=m.source_id
 WHERE s.project_id=? ORDER BY t.created_at DESC"""
METADATA_TRANSCRIPTS = """SELECT t.id, s.project_id, s.source_type,
 s.locator, s.title, m.duration_seconds, t.language, t.created_at
 FROM transcripts t
 JOIN media m ON m.id=t.media_id
 JOIN sources s ON s.id=m.source_id
 WHERE s.project_id=? ORDER BY t.created_at DESC"""
ALL_JOBS = """SELECT id, project_id, input_kind, input_value, state,
 progress, last_error, created_at, updated_at
 FROM jobs ORDER BY created_at DESC"""
GROUPED_JOBS = """SELECT project_id, COUNT(*), MAX(updated_at)
 FROM jobs GROUP BY project_id"""
SCHEMA = """
CREATE TABLE sources (
 id TEXT PRIMARY KEY, project_id TEXT, creator_id TEXT, source_type TEXT,
 locator TEXT, title TEXT, created_at TEXT);
CREATE TABLE media (
 id TEXT PRIMARY KEY, source_id TEXT, local_path TEXT,
 duration_seconds REAL, mime_type TEXT, created_at TEXT);
CREATE TABLE transcripts (
 id TEXT PRIMARY KEY, media_id TEXT, language TEXT, text TEXT,
 segments_json TEXT, words_json TEXT, created_at TEXT, updated_at TEXT);
CREATE TABLE jobs (
 id TEXT PRIMARY KEY, project_id TEXT, input_kind TEXT, input_value TEXT,
 state TEXT, progress REAL, last_error TEXT, created_at TEXT, updated_at TEXT);
CREATE INDEX idx_sources_project_created ON sources(project_id, created_at DESC);
CREATE INDEX idx_media_source ON media(source_id);
CREATE INDEX idx_transcripts_media ON transcripts(media_id);
CREATE INDEX idx_jobs_project_created ON jobs(project_id, created_at DESC);
"""


def sample_timestamp(index: int) -> str:
    return f"2026-11-10T{index // 3600:02}:{(index // 60) % 60:02}:{index % 60:02}Z"


def populate(db: sqlite3.Connection, per_project: int) -> None:
    body = "longword " * 6000
    segments = json.dumps([{
        "index": 0, "start": 0, "end": 24.5,
        "text": "a long segment " * 500,
    }])
    for p in range(2):
        project = f"project-{p}"
        for i in range(per_project):
            created = sample_timestamp(i)
            source, media, transcript = f"s-{p}-{i}", f"m-{p}-{i}", f"t-{p}-{i}"
            db.execute(
                "INSERT INTO sources VALUES (?,?,?,?,?,?,?)",
                (source, project, None, "local_file",
                 f"/tmp/video{i}.mp4", f"Video {i}", created),
            )
            db.execute(
                "INSERT INTO media VALUES (?,?,?,?,?,?)",
                (media, source, f"/tmp/video{i}.mp4", 24.5, None, created),
            )
            db.execute(
                "INSERT INTO transcripts VALUES (?,?,?,?,?,?,?,?)",
                (transcript, media, "en", body, segments, None, created, created),
            )
        for i in range(per_project * 4):
            created = sample_timestamp(i)
            db.execute(
                "INSERT INTO jobs VALUES (?,?,?,?,?,?,?,?,?)",
                (f"j-{p}-{i}", project, "url", f"url-{i}", "completed",
                 1, None, created, created),
            )
    db.commit()


def measure(db: sqlite3.Connection, sql: str, params: tuple = ()) -> dict:
    times = []
    for iteration in range(9):
        start = time.perf_counter_ns()
        rows = db.execute(sql, params).fetchall()
        elapsed = (time.perf_counter_ns() - start) / 1_000_000
        if iteration:
            times.append(elapsed)
    string_bytes = sum(
        len(value) for row in rows for value in row if isinstance(value, str)
    )
    plan = [row[-1] for row in db.execute("EXPLAIN QUERY PLAN " + sql, params)]
    return {
        "median_ms": round(statistics.median(times), 3),
        "rows": len(rows),
        "string_chars": string_bytes,
        "explain_query_plan": plan,
    }


def main() -> None:
    for count in (2, 128, 1024):
        with tempfile.TemporaryDirectory(prefix="scriptotar-sql-profile-") as root:
            db = sqlite3.connect(Path(root) / "disposable.sqlite3")
            try:
                db.execute("PRAGMA journal_mode=WAL")
                db.execute("PRAGMA synchronous=FULL")
                db.executescript(SCHEMA)
                populate(db, count)
                data = {
                    "transcripts_per_project": count,
                    "jobs_per_project": count * 4,
                    "full_transcripts": measure(db, FULL_TRANSCRIPTS, ("project-0",)),
                    "metadata_transcripts": measure(db, METADATA_TRANSCRIPTS, ("project-0",)),
                    "all_jobs": measure(db, ALL_JOBS),
                    "grouped_jobs": measure(db, GROUPED_JOBS),
                }
                assert data["full_transcripts"]["rows"] == count
                assert data["metadata_transcripts"]["rows"] == count
                assert data["grouped_jobs"]["rows"] == 2
                print(json.dumps(data))
            finally:
                db.close()


if __name__ == "__main__":
    main()
