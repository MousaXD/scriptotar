"""Seed synthetic 100/1000/10000-row workloads into an isolated migrated SQLite copy.

Only use an EMPTY schema-migrated scratch template, NEVER an ordinary user database.
No production code is touched, and the source is opened read-only before backup.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import sqlite3
import uuid

SIZES = {100, 1000, 10000}
EMPTY_TABLES = ("creators", "sources", "media", "transcripts", "research_items",
                "analyses", "ai_runs", "jobs")
NEEDED_TABLES = set(EMPTY_TABLES) | {"projects", "schema_migrations"}
STAMP = "2026-01-01T00:00:00Z"

def identity(kind, n):
    return str(uuid.uuid5(uuid.NAMESPACE_URL, f"scriptotar-defrag-fixture/{kind}/{n}"))

def digest(path):
    h = hashlib.sha256()
    with path.open("rb") as source:
        for block in iter(lambda: source.read(1024 * 1024), b""):
            h.update(block)
    return h.hexdigest()

def assert_blank_migrated_template(conn):
    """Reject ALL preexisting user records; allow only the migration ledger and empty FTS internals."""
    names = {r[0] for r in conn.execute(
        "SELECT name FROM sqlite_master WHERE type='table'")}
    if not NEEDED_TABLES.issubset(names):
        raise ValueError("template must include the app migration ledger and domain tables")
    version = conn.execute("PRAGMA user_version").fetchone()[0]
    if version < 5:
        raise ValueError("template must be initialized by Scriptotar schema migration v5+")
    versions = [row[0] for row in conn.execute(
        "SELECT version FROM schema_migrations ORDER BY version")]
    if versions != list(range(1, version + 1)):
        raise ValueError("template migration ledger and user_version disagree")

    # FTS5 shadow tables may hold internal bookkeeping even when index is empty.
    # The virtual transcript_fts table itself is tested for zero visible rows.
    shadows = {"transcript_fts_data", "transcript_fts_idx",
               "transcript_fts_content", "transcript_fts_docsize",
               "transcript_fts_config"}
    for table in sorted(names):
        if table.startswith("sqlite_") or table == "schema_migrations" or table in shadows:
            continue
        # Only table names provided by sqlite_master, not user parameters.
        escaped = table.replace('"', '""')
        if conn.execute(f'SELECT 1 FROM "{escaped}" LIMIT 1').fetchone():
            raise ValueError(f"template is not empty: {table}; use a blank migrated scratch DB")

def seed_workspace(template: Path, destination: Path, rows: int):
    template, destination = template.resolve(), destination.resolve()
    if rows not in SIZES:
        raise ValueError("expected 100, 1000 or 10000 rows")
    if template == destination or destination.exists() or not template.is_file():
        raise ValueError("template must exist; output must not exist and cannot be the template")
    if not destination.parent.is_dir():
        raise ValueError("destination parent directory must already exist")

    original = sqlite3.connect(template.as_uri() + "?mode=ro", uri=True)
    try:
        assert_blank_migrated_template(original)
        # Revalidate the copied snapshot before inserting; a user DB must never be seeded.
        created = False
        try:
            # Exclusive 0600 creation prevents overwriting files or racing a symlink.
            descriptor = os.open(destination, os.O_CREAT | os.O_EXCL | os.O_WRONLY, 0o600)
            os.close(descriptor)
            created = True
            target = sqlite3.connect(destination)
            try:
                original.backup(target)
                assert_blank_migrated_template(target)
                target.execute("PRAGMA foreign_keys=ON")
                project = identity("project", rows)
                with target:
                    target.execute(
                        "INSERT INTO projects (id,name,created_at) VALUES (?,?,?)",
                        (project, f"SYNTHETIC QA {rows}", STAMP))
                    for index in range(rows):
                        suffix = f"{rows}-{index}"
                        source_id = identity("source", suffix)
                        media_id = identity("media", suffix)
                        target.execute(
                            "INSERT INTO sources (id,project_id,source_type,locator,title,created_at)"
                            " VALUES (?,?,?,?,?,?)",
                            (source_id, project, "url",
                             f"https://example.invalid/qa/{suffix}", f"SYNTHETIC source {index}", STAMP))
                        target.execute(
                            "INSERT INTO media (id,source_id,local_path,created_at) VALUES (?,?,?,?)",
                            (media_id, source_id, f"/nonexistent/qa/{suffix}.mp4", STAMP))
                        target.execute(
                            "INSERT INTO transcripts (id,media_id,language,text,created_at,updated_at)"
                            " VALUES (?,?,?,?,?,?)",
                            (identity("transcript", suffix), media_id, "ar" if index % 2 else "en",
                             f"Synthetic QA transcript {index}. No real audio or creator content.",
                             STAMP, STAMP))
                        target.execute(
                            "INSERT INTO research_items (id,project_id,source_url,platform,title,"
                            "view_count,like_count,comment_count,scanned_at) VALUES (?,?,?,?,?,?,?,?,?)",
                            (identity("research", suffix), project,
                             f"https://example.invalid/qa/{suffix}", "youtube",
                             f"SYNTHETIC research {index}", index * 13, index * 2, index, STAMP))
                        target.execute(
                            "INSERT INTO ai_runs (id,project_id,task,mode,prompt,result,created_at)"
                            " VALUES (?,?,?,?,?,?,?)",
                            (identity("ai", suffix), project, "new_script", "copy_prompt",
                             f"SYNTHETIC prompt {index}", f"SYNTHETIC answer {index}", STAMP))
                        target.execute(
                            "INSERT INTO jobs (id,project_id,input_kind,input_value,state,created_at,updated_at)"
                            " VALUES (?,?,?,?,?,?,?)",
                            (identity("job", suffix), project, "url",
                             f"https://example.invalid/qa/{suffix}", "cancelled", STAMP, STAMP))
                if target.execute("PRAGMA quick_check").fetchone()[0] != "ok":
                    raise ValueError("fixture failed SQLite quick_check")
                counts = {name: target.execute(
                    f"SELECT COUNT(*) FROM {name} WHERE project_id=?", (project,)).fetchone()[0]
                    for name in ("sources", "research_items", "ai_runs", "jobs")}
                for name in ("media", "transcripts"):
                    counts[name] = target.execute(f"SELECT COUNT(*) FROM {name}").fetchone()[0]
                if any(value != rows for value in counts.values()):
                    raise ValueError("synthetic row count mismatch")
            finally:
                target.close()
        except Exception:
            if created:
                for suffix in ("", "-wal", "-shm"):
                    Path(str(destination) + suffix).unlink(missing_ok=True)
            raise
    finally:
        original.close()

    return {
        "kind": "SYNTHETIC scratch database, never copied from user data",
        "record_count_per_collection": rows,
        "tables": counts,
        "db_sha256": digest(destination),
        "db_bytes": destination.stat().st_size,
        "fixture_project_id": project,
        "limitations": "No media files; all jobs cancelled; SQL scale fixture only, not native performance evidence."
    }

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--template", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--rows", type=int, required=True, choices=sorted(SIZES))
    args = parser.parse_args()
    result = seed_workspace(args.template, args.output, args.rows)
    print(json.dumps(result, indent=2))

if __name__ == "__main__":
    main()
