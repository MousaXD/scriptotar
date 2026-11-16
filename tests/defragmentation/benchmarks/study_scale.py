"""Build disposable 100/1k/10k canonical Study fixtures on an APP-MIGRATED blank DB.

This extends seed_workspace's already guarded synthetic legacy fixtures. The
provided template must have actually been migrated by Scriptotar (v6+), and
must be empty. Never run this against an existing application/user database.
"""
from __future__ import annotations

import argparse
import json
from pathlib import Path
import re
import sqlite3

try:
    from .seed_workspace import SIZES, STAMP, digest, identity, seed_workspace
except ImportError:
    from seed_workspace import SIZES, STAMP, digest, identity, seed_workspace

STUDY_TABLES = ("content_studies", "study_sources", "study_briefs", "creative_drafts", "draft_versions")
SHA = re.compile(r"[0-9a-f]{40}\Z")


def require_study_migrations(db: sqlite3.Connection) -> int:
    version = db.execute("PRAGMA user_version").fetchone()[0]
    names = {row[0] for row in db.execute("SELECT name FROM sqlite_master WHERE type='table'")}
    if version < 6 or not set(STUDY_TABLES).issubset(names):
        raise ValueError("requires actual Scriptotar v6+ Content Study migrations")
    return version


def build(template: Path, output: Path, rows: int, source_sha: str) -> dict:
    if rows not in SIZES or not SHA.fullmatch(source_sha):
        raise ValueError("rows must be 100/1000/10000 and source SHA must be 40 lowercase hex")
    template = template.resolve()
    output = output.resolve()
    # Reject before making a destination; seed_workspace rejects nonempty templates.
    with sqlite3.connect(template.as_uri() + "?mode=ro", uri=True) as original:
        require_study_migrations(original)
    seed_workspace(template, output, rows)
    project = identity("project", rows)
    try:
        with sqlite3.connect(output) as db:
            db.execute("PRAGMA foreign_keys=ON")
            version = require_study_migrations(db)
            db.execute("BEGIN IMMEDIATE")
            for i in range(rows):
                suffix = f"{rows}-{i}"
                study = identity("study", suffix)
                draft = identity("draft", suffix)
                transcript = identity("transcript", suffix)
                source = identity("study-source", suffix)
                title = f"SYNTHETIC Study {i:05d}"
                body = "SYNTHETIC DRAFT — unverified content only. " * 48
                db.execute(
                    "INSERT INTO content_studies(id,project_id,title,goal,status,revision,created_at,updated_at) "
                    "VALUES(?,?,?,?,?,?,?,?)",
                    (study, project, title, "SYNTHETIC creator research", "active", 1, STAMP, STAMP),
                )
                db.execute(
                    "INSERT INTO study_briefs(study_id,project_id,audience,goal,language,platform,tone,"
                    "duration_seconds,objective,constraints_json,revision,updated_at) "
                    "VALUES(?,?,?,?,?,?,?,?,?,?,?,?)",
                    (study, project, "Synthetic audience", "Synthetic goal", "en", "youtube",
                     "neutral", 40, "Benchmark only", "[]", 1, STAMP),
                )
                db.execute(
                    "INSERT INTO study_sources(id,project_id,study_id,kind,reference_id,transcript_id,"
                    "locator,fingerprint,collected_at,created_at) VALUES(?,?,?,?,?,?,?,?,?,?)",
                    (source, project, study, "transcript", transcript, transcript,
                     f"https://example.invalid/qa/{suffix}", f"synthetic-fingerprint-{suffix}", STAMP, STAMP),
                )
                db.execute(
                    "INSERT INTO creative_drafts(id,project_id,study_id,title,body,revision,created_at,updated_at) "
                    "VALUES(?,?,?,?,?,?,?,?)",
                    (draft, project, study, f"SYNTHETIC Draft {i}", body, 1, STAMP, STAMP),
                )
                db.execute(
                    "INSERT INTO draft_versions(id,draft_id,version,title,body,saved_at) "
                    "VALUES(?,?,?,?,?,?)",
                    (identity("draft-version", suffix), draft, 1, f"SYNTHETIC Draft {i}", body, STAMP),
                )
            # Deliberately long transcript, not 10k large files: bounds disk and FTS rebuild.
            # Seeded transcript IDs are deterministic, allowing exact and repeatable probes.
            db.execute("UPDATE transcripts SET text=? WHERE id=?",
                       ("SYNTHETIC LONG TRANSCRIPT. " * 40_000, identity("transcript", f"{rows}-0")))
            for table in STUDY_TABLES:
                actual = db.execute(f"SELECT COUNT(*) FROM {table}").fetchone()[0]
                if actual != rows:
                    raise ValueError(f"fixture count for {table}: expected {rows}, got {actual}")
            if db.execute("PRAGMA foreign_key_check").fetchall():
                raise ValueError("fixture failed foreign_key_check")
            if db.execute("PRAGMA quick_check").fetchone()[0] != "ok":
                raise ValueError("fixture failed quick_check")
            db.commit()
        return {
            "dataset": "SYNTHETIC_ONLY", "source_sha_operator_supplied": source_sha,
            "schema_version": version, "per_collection": rows, "study_tables": list(STUDY_TABLES),
            "project_id": project, "long_transcript_id": identity("transcript", f"{rows}-0"),
            "db_sha256": digest(output), "db_file_bytes": output.stat().st_size,
            "limitations": "Synthetic SQL data; no media files, installed Tauri, native IPC or UI timings.",
        }
    except Exception:
        for suffix in ("", "-wal", "-shm"):
            Path(str(output) + suffix).unlink(missing_ok=True)
        raise


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--template", type=Path, required=True)
    parser.add_argument("--out-dir", type=Path, required=True)
    parser.add_argument("--source-sha", required=True)
    parser.add_argument("--sizes", type=int, nargs="+", choices=sorted(SIZES), default=[100, 1000, 10000])
    args = parser.parse_args()
    if not args.out_dir.is_dir() or not SHA.fullmatch(args.source_sha):
        parser.error("existing output directory and exact 40-hex source SHA required")
    for size in dict.fromkeys(args.sizes):
        destination = args.out_dir / f"scriptotar-synthetic-{size}.sqlite3"
        manifest = build(args.template, destination, size, args.source_sha)
        (args.out_dir / f"scriptotar-synthetic-{size}.json").write_text(
            json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
        print(json.dumps(manifest))


if __name__ == "__main__":
    main()
