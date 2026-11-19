#!/usr/bin/env python3
"""Read-only canonical SQLite evidence for TWO genuinely ingested media origins.

This is a QA evidence verifier, not a transcription producer or native UI driver.
It NEVER inserts jobs, sources, transcripts, evidence or fixtures; a successful
invocation does NOT certify installed acceptance, native UI actions, sidecar
execution, source-revocation denial, RTL or themes.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import sqlite3


class LineageUnavailable(RuntimeError):
    """Required real installed provenance is absent or inconsistent."""


HEX64 = re.compile(r"[0-9a-f]{64}\Z")
HISTORY_TABLES = ("study_evidence", "draft_source_links", "study_source_jobs")


def digest_file(path: Path) -> str:
    hasher = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            hasher.update(chunk)
    return hasher.hexdigest()


def digest_rows(rows: list[tuple]) -> dict:
    """Store only equality fingerprints and row counts, never user content."""
    digests = []
    for row in rows:
        serialized = json.dumps(
            [({"blob_sha256": hashlib.sha256(value).hexdigest(),
               "length": len(value)} if isinstance(value, bytes) else value)
             for value in row],
            ensure_ascii=False, separators=(",", ":"), default=str,
        )
        digests.append(hashlib.sha256(serialized.encode("utf-8")).hexdigest())
    digests.sort()
    return {
        "rows": len(digests),
        "sha256": hashlib.sha256("\n".join(digests).encode("ascii")).hexdigest(),
    }


def expected_media(manifest_path: Path, fixture_dir: Path) -> list[dict]:
    """Require the actual bytes of two distinct origin-validated WebM fixtures."""
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    if manifest.get("purpose") != "native-two-real-media-preparation-only":
        raise LineageUnavailable("Unexpected or unreviewed fixture manifest")
    origins = manifest.get("sources")
    if not isinstance(origins, list) or len(origins) != 2:
        raise LineageUnavailable("Exactly two independent real media are required")
    normalized = []
    for row in origins:
        if not isinstance(row, dict):
            raise LineageUnavailable("Invalid fixture record")
        name = row.get("webm_filename")
        expected_sha = row.get("webm_sha256")
        origin_sha = row.get("original_sha256")
        original_url = row.get("original_url")
        if (not isinstance(name, str) or Path(name).name != name or
                not name.endswith(".webm") or
                not isinstance(expected_sha, str) or not HEX64.fullmatch(expected_sha) or
                not isinstance(origin_sha, str) or not HEX64.fullmatch(origin_sha) or
                not isinstance(original_url, str) or not original_url.startswith(
                    "https://upload.wikimedia.org/wikipedia/commons/")):
            raise LineageUnavailable("Missing verified WebM/real-origin provenance")
        media = fixture_dir / name
        if not media.is_file() or media.stat().st_size < 32768:
            raise LineageUnavailable("Real .webm fixture is missing or truncated")
        if digest_file(media) != expected_sha:
            raise LineageUnavailable("WebM bytes differ from original preparation manifest")
        normalized.append({
            "source_id": row.get("source_id"),
            "original_url": original_url,
            "original_sha256": origin_sha,
            "webm_sha256": expected_sha,
            "webm_path": str(media.resolve()),
        })
    for key in ("source_id", "original_url", "original_sha256",
                "webm_sha256", "webm_path"):
        if any(not item[key] for item in normalized) or len({
                item[key] for item in normalized}) != 2:
            raise LineageUnavailable("Duplicate or missing independent origin: " + key)
    return normalized


def read_installed_lineage(
    db_path: Path, manifest_path: Path, fixture_dir: Path,
    project_id: str, study_id: str,
) -> dict:
    """One SQLite WAL-consistent READ transaction; data is never modified."""
    origins = expected_media(manifest_path, fixture_dir)
    if not db_path.is_file():
        raise LineageUnavailable("Actual installed canonical SQLite database missing")
    try:
        with sqlite3.connect(db_path.resolve().as_uri() + "?mode=ro",
                             uri=True, timeout=10) as conn:
            conn.execute("PRAGMA query_only=ON")
            conn.execute("BEGIN")
            study = conn.execute(
                "SELECT revision FROM content_studies WHERE id=? AND project_id=? "
                "AND archived_at IS NULL", (study_id, project_id),
            ).fetchone()
            if study is None:
                raise LineageUnavailable("Study does not belong to active expected project")
            jobs = conn.execute(
                "SELECT j.id,j.input_value,j.state,l.transcript_id,m.id,s.id,"
                "s.project_id,s.source_type,t.text,t.segments_json "
                "FROM jobs j "
                "JOIN job_transcript_links l ON l.job_id=j.id "
                "JOIN transcripts t ON t.id=l.transcript_id "
                "JOIN media m ON m.id=t.media_id "
                "JOIN sources s ON s.id=m.source_id "
                "WHERE j.project_id=? AND j.input_kind='local_file'",
                (project_id,),
            ).fetchall()
            matched = []
            for source in origins:
                target = os.path.normcase(source["webm_path"])
                candidates = [r for r in jobs if os.path.normcase(
                    str(Path(r[1]).resolve())) == target and r[2] == "completed"]
                if len(candidates) != 1:
                    raise LineageUnavailable(
                        "Expected exactly one genuinely completed linked job per WebM"
                    )
                job, path, state, transcript, media, source_id, owner, kind, body, segments = candidates[0]
                if owner != project_id or kind != "local_file":
                    raise LineageUnavailable("Transcription origin ownership is inconsistent")
                if not isinstance(body, str) or not body.strip():
                    raise LineageUnavailable("Persisted transcript text is empty")
                try:
                    parsed = json.loads(segments)
                except (TypeError, json.JSONDecodeError) as exc:
                    raise LineageUnavailable("Persisted transcript segments are invalid") from exc
                if (not isinstance(parsed, list) or not parsed or
                        not any(isinstance(seg, dict) and isinstance(
                            seg.get("text"), str) and seg["text"].strip()
                            for seg in parsed)):
                    raise LineageUnavailable("Real segment texts are missing")
                matched.append({
                    "origin_sha256": source["original_sha256"],
                    "webm_sha256": source["webm_sha256"],
                    "job_id": job, "transcript_id": transcript,
                    "media_id": media, "source_id": source_id,
                    "segments": len(parsed),
                    "transcript_text_sha256": hashlib.sha256(
                        body.encode("utf-8")).hexdigest(),
                })
            for key in ("job_id", "transcript_id", "media_id", "source_id"):
                if len({row[key] for row in matched}) != 2:
                    raise LineageUnavailable("Two recordings collapsed into one " + key)
            bindings = {}
            for item in matched:
                row = conn.execute(
                    "SELECT id,reference_id,transcript_id,fingerprint,locator "
                    "FROM study_sources WHERE project_id=? AND study_id=? "
                    "AND kind='transcript' AND reference_id=?",
                    (project_id, study_id, item["transcript_id"]),
                ).fetchone()
                if row is None:
                    raise LineageUnavailable("Study Select Sources did not persist both transcripts")
                source_key, reference, live_fk, fingerprint, locator = row
                if reference != item["transcript_id"] or not fingerprint or not locator:
                    raise LineageUnavailable("Study source provenance is incomplete")
                history = {
                    table: digest_rows(conn.execute(
                        'SELECT * FROM "' + table + '" WHERE study_source_id=?',
                        (source_key,),
                    ).fetchall()) for table in HISTORY_TABLES
                }
                bindings[item["transcript_id"]] = {
                    "study_source_id": source_key,
                    "fingerprint": fingerprint,
                    "locator_sha256": hashlib.sha256(
                        locator.encode("utf-8")).hexdigest(),
                    "available": live_fk == item["transcript_id"],
                    "tombstoned": live_fk is None,
                    "history": history,
                }
                if live_fk not in (None, item["transcript_id"]):
                    raise LineageUnavailable("Study source FK was reassigned")
            if len({item["fingerprint"] for item in bindings.values()}) != 2:
                raise LineageUnavailable("Study contains duplicate source fingerprints")
            all_drafts = conn.execute(
                "SELECT * FROM creative_drafts WHERE project_id=? AND study_id=?",
                (project_id, study_id),
            ).fetchall()
            versions = conn.execute(
                "SELECT v.* FROM draft_versions v "
                "JOIN creative_drafts d ON d.id=v.draft_id "
                "WHERE d.project_id=? AND d.study_id=?",
                (project_id, study_id),
            ).fetchall()
            conn.execute("ROLLBACK")
    except sqlite3.Error as exc:
        raise LineageUnavailable("Read-only installed canonical query failed") from exc
    return {
        "evidence_kind": "installed-canonical-db-readback-only",
        "native_16_case_certification": "NOT TESTED",
        "project_id_sha256": hashlib.sha256(project_id.encode()).hexdigest(),
        "study_id_sha256": hashlib.sha256(study_id.encode()).hexdigest(),
        "revision": int(study[0]),
        "transcripts": matched, "bindings": bindings,
        "drafts": digest_rows(all_drafts), "draft_versions": digest_rows(versions),
        "limits": ("DB readback cannot establish genuine UIA/AT-SPI input, "
                   "sidecar invocation, relaunch, revocation action, stale IPC "
                   "denial or release PASS; independently attest these."),
    }


def verify_revocation(before: dict, after: dict, revoked_transcript_id: str,
                      reopened: dict | None = None) -> dict:
    """Compare independently collected DB snapshots, never execute revocation."""
    if before["evidence_kind"] != after["evidence_kind"]:
        raise LineageUnavailable("Different provenance sources for revocation snapshots")
    if before["study_id_sha256"] != after["study_id_sha256"] or \
            before["project_id_sha256"] != after["project_id_sha256"]:
        raise LineageUnavailable("Revocation comparison changed canonical Study/project")
    if before["transcripts"] != after["transcripts"]:
        raise LineageUnavailable("Revocation changed original transcript/job lineage")
    if before["revision"] + 1 != after["revision"]:
        raise LineageUnavailable("Revocation did not increment Study revision exactly once")
    if set(before["bindings"]) != set(after["bindings"]) or \
            len(before["bindings"]) != 2 or revoked_transcript_id not in before["bindings"]:
        raise LineageUnavailable("Incorrect or missing two-source Study lineage")
    for tid, old in before["bindings"].items():
        new = after["bindings"][tid]
        for key in ("study_source_id", "fingerprint", "locator_sha256", "history"):
            if old[key] != new[key]:
                raise LineageUnavailable("Historical source or evidence was changed: " + key)
        if not old["available"] or old["tombstoned"]:
            raise LineageUnavailable("Before-snapshot source already revoked")
        if tid == revoked_transcript_id:
            if new["available"] or not new["tombstoned"]:
                raise LineageUnavailable("Selected source was not durably tombstoned")
        elif not new["available"] or new["tombstoned"]:
            raise LineageUnavailable("Unrelated source access was lost")
    if before["drafts"] != after["drafts"] or \
            before["draft_versions"] != after["draft_versions"]:
        raise LineageUnavailable("Revocation mutated historical draft/version content")
    if reopened is not None and after != reopened:
        raise LineageUnavailable("Reopened canonical DB resurrected or altered revocation")
    return {
        "canonical_tombstone_and_history": "OBSERVED",
        "reopen_comparison": "OBSERVED" if reopened is not None else "NOT TESTED",
        "stale_native_action_denial": "NOT TESTED",
        "source_revocation_native_certification": "NOT TESTED",
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--database", required=True, type=Path)
    parser.add_argument("--manifest", required=True, type=Path)
    parser.add_argument("--fixture-dir", required=True, type=Path)
    parser.add_argument("--project-id", required=True)
    parser.add_argument("--study-id", required=True)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    try:
        result = read_installed_lineage(
            args.database, args.manifest, args.fixture_dir,
            args.project_id, args.study_id,
        )
    except (OSError, ValueError, KeyError, LineageUnavailable) as exc:
        parser.exit(2, "NOT TESTED: " + str(exc) + "\n")
    serialized = json.dumps(result, indent=2, sort_keys=True) + "\n"
    if args.output:
        args.output.write_text(serialized, encoding="utf-8")
    else:
        print(serialized, end="")


if __name__ == "__main__":
    main()
