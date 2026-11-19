#!/usr/bin/env python3
"""OPTIONAL live Linux WebKitGTK attempt using original WebM and bundled sidecars.

This exercises the *installed* application's own Queue/Studio controls via
AT-SPI/X11, never a mocked IPC, dev server, database insert or fake transcript.
This addendum cannot itself award a 16-case PASS: independent review of
sidecar identity, UI source revocation and stale-action denial is still needed.
"""
from __future__ import annotations

import argparse
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import platform
import re
import shutil
import sqlite3
import subprocess
import tempfile
import time
from uuid import uuid4

from linux_webkit import NativeFailure, Session, sha256
from real_source_fixtures import prepare
from real_study_lineage import (
    LineageUnavailable, expected_media, read_installed_lineage,
    verify_revocation,
)


def readonly(db_path: Path, sql: str, params: tuple = ()) -> list[tuple]:
    if not db_path.is_file():
        return []
    with sqlite3.connect(db_path.resolve().as_uri() + "?mode=ro",
                         uri=True, timeout=8) as con:
        con.execute("PRAGMA query_only=ON")
        con.execute("BEGIN")
        rows = con.execute(sql, params).fetchall()
        con.execute("ROLLBACK")
        return rows


def installed_ids(db: Path, project_name: str, study_title: str) -> tuple[str, str]:
    projects = readonly(
        db, "SELECT id FROM projects WHERE name=? AND archived_at IS NULL",
        (project_name,),
    )
    if len(projects) != 1:
        raise NativeFailure("Real GUI project was not uniquely persisted")
    project_id = projects[0][0]
    studies = readonly(
        db, "SELECT id FROM content_studies WHERE project_id=? AND title=? "
            "AND archived_at IS NULL", (project_id, study_title),
    )
    if len(studies) != 1:
        raise NativeFailure("Real GUI Study was not uniquely persisted")
    return project_id, studies[0][0]


def job_states(db: Path, project_id: str, files: list[dict]) -> list[dict]:
    rows = readonly(
        db, "SELECT j.id,j.input_value,j.state,j.last_error,l.transcript_id,"
            "s.title FROM jobs j "
            "LEFT JOIN job_transcript_links l ON l.job_id=j.id "
            "LEFT JOIN transcripts t ON t.id=l.transcript_id "
            "LEFT JOIN media m ON m.id=t.media_id "
            "LEFT JOIN sources s ON s.id=m.source_id "
            "WHERE j.project_id=? AND j.input_kind='local_file'",
        (project_id,),
    )
    result = []
    for item in files:
        match = [r for r in rows if Path(r[1]).resolve() == Path(
            item["webm_path"]).resolve()]
        if len(match) > 1:
            raise NativeFailure("More than one job created for a real source")
        if match:
            job_id, input_path, state, error, transcript_id, title = match[0]
            result.append({
                "origin_sha256": item["original_sha256"],
                "webm_sha256": item["webm_sha256"],
                "job_id": job_id, "state": state,
                "last_error_sha256": hashlib.sha256(
                    (error or "").encode()).hexdigest() if error else None,
                "transcript_id": transcript_id,
                "title": title or Path(input_path).name,
            })
    return result


def real_sidecar_processes() -> list[dict]:
    result = subprocess.run(
        ["ps", "-eo", "pid,ppid,args"], text=True, capture_output=True,
        check=True, timeout=6,
    )
    rows = []
    for line in result.stdout.splitlines()[1:]:
        if not any(token in line for token in (
                "scriptotar-transcription", "scriptotar-engine")):
            continue
        parts = line.split(maxsplit=2)
        if len(parts) != 3:
            continue
        # Do not include a potentially sensitive command line in evidence.
        rows.append({"pid": int(parts[0]), "ppid": int(parts[1]),
                     "command_sha256": hashlib.sha256(
                         parts[2].encode()).hexdigest()})
    return rows


def exercise_installed(args, root: Path, result: dict) -> None:
    fixture_dir = args.evidence_dir / "two-real-origins"
    original = prepare(fixture_dir, args.ffmpeg, args.ffprobe)
    sources = expected_media(fixture_dir / "origin-manifest.json", fixture_dir)
    result["prepared"] = {
        "origin_sha256": [row["original_sha256"] for row in sources],
        "webm_sha256": [row["webm_sha256"] for row in sources],
        "real_origin_count": len(original["sources"]),
        "native_acceptance": "NOT TESTED",
    }
    tag = uuid4().hex[:12]
    project_name = "Real Origin QA " + tag
    study_name = "Two Source Study " + tag
    session = Session(args, root)
    db_path = root / "data" / "scriptotar.sqlite3"
    try:
        session.start()
        a = session.ax
        deadline = time.monotonic() + 45
        while time.monotonic() < deadline:
            if a.heading_available("Create your first project"):
                break
            if a.available("New project", "button"):
                a.click("New project", "button")
                break
            time.sleep(0.35)
        a.type_into("Project name", project_name)
        a.click("Create project", "button")
        a.heading(project_name, timeout=25)
        a.click("Studio", "button")
        a.heading("Studio")
        a.type_into("Study name", study_name)
        a.click("Create study", "button")
        a.find("Duplicate current script", "button")
        project_id, study_id = installed_ids(db_path, project_name, study_name)
        result["created_by_real_native_ui"] = True
        session.screenshot("real-two-source-study-created")

        a.click("Queue", "button")
        a.heading("Jobs")
        a.click("Advanced: enter a local path manually")
        a.edit_field("Manual local media path")
        for source in sources:
            a.type_into("Manual local media path", source["webm_path"])
            if a.read_text("Manual local media path") != source["webm_path"]:
                raise NativeFailure("AT-SPI path readback differs from actual fixture")
            a.click("Queue path", "button")
            session.screenshot("real-queue-submitted-" + source["source_id"].split(":")[-1])

        last_jobs = []
        runtime_pids = set()
        deadline = time.monotonic() + args.transcription_timeout_seconds
        while time.monotonic() < deadline:
            if session.proc.poll() is not None:
                raise NativeFailure("Installed desktop exited during real transcription")
            for process in real_sidecar_processes():
                runtime_pids.add((process["pid"], process["ppid"],
                                  process["command_sha256"]))
            last_jobs = job_states(db_path, project_id, sources)
            result["observed_jobs"] = last_jobs
            if any(job["state"] in ("failed", "cancelled", "interrupted")
                   for job in last_jobs):
                raise NativeFailure(
                    "Real installed transcription job entered failure/interrupt state"
                )
            if (len(last_jobs) == 2 and
                    all(job["state"] == "completed" and job["transcript_id"]
                        for job in last_jobs)):
                break
            time.sleep(4)
        else:
            raise NativeFailure(
                "Real bundled transcription did not persist two completed "
                "job/transcript links within bounded native timeout"
            )
        result["observed_sidecar_processes"] = [
            {"pid": pid, "ppid": ppid, "command_sha256": exe}
            for pid, ppid, exe in sorted(runtime_pids)
        ]
        if not runtime_pids:
            raise NativeFailure(
                "No independently observed bundled transcription sidecar process"
            )
        result["queue_and_completed_transcripts"] = "OBSERVED"
        session.screenshot("real-queue-two-completed")

        a.click("Studio", "button")
        a.heading("Studio")
        a.choose("Open study", study_name)
        a.click("Choose source material")
        for job in last_jobs:
            label = job["title"]
            node = a.find(label, "check", timeout=12)
            try:
                previously_checked = node.getState().contains(a.atspi.STATE_CHECKED)
            except Exception as exc:
                raise NativeFailure(
                    "Real source checkbox did not expose AT-SPI checked state"
                ) from exc
            if previously_checked:
                raise NativeFailure(
                    "Source was already linked without a real native selection"
                )
            a.click(label, "check")
            selected = a.find(label, "check").getState().contains(
                a.atspi.STATE_CHECKED)
            if not selected:
                raise NativeFailure("Native source checkbox did not select exact transcript")
        if a.available("Save changes", "button"):
            a.click("Save changes", "button")
        deadline = time.monotonic() + 60
        before = None
        last_error = ""
        while time.monotonic() < deadline:
            try:
                before = read_installed_lineage(
                    db_path, fixture_dir / "origin-manifest.json",
                    fixture_dir, project_id, study_id,
                )
                if all(link["available"] for link in before["bindings"].values()):
                    break
            except LineageUnavailable as exc:
                last_error = str(exc)
            time.sleep(2)
        if before is None or not all(
                link["available"] for link in before["bindings"].values()):
            raise NativeFailure(
                "Real Studio Select Sources did not persist canonical two-source "
                "lineage: " + last_error
            )
        result["two_source_canonical_readback"] = before
        session.screenshot("real-two-sources-attached")

        session.stop()
        session.start()
        a = session.ax
        a.click("Studio", "button")
        a.choose("Open study", study_name)
        reopened = read_installed_lineage(
            db_path, fixture_dir / "origin-manifest.json",
            fixture_dir, project_id, study_id,
        )
        if before != reopened:
            raise NativeFailure(
                "Canonical two-transcript origin or source binding changed after relaunch"
            )
        result["two_source_relaunch_readback"] = "OBSERVED"
        session.screenshot("real-two-sources-reopened")

        # The production UI can expose a revoke action, but the complete
        # stale-access denial also needs separate authenticated host/native
        # correlation. Never promote this addendum to PASS on DB proof alone.
        a.click(last_jobs[0]["title"], "button")
        a.click("Revoke source access", "button")
        a.click("Confirm revoke", "button")
        deadline = time.monotonic() + 30
        after = None
        while time.monotonic() < deadline:
            after = read_installed_lineage(
                db_path, fixture_dir / "origin-manifest.json",
                fixture_dir, project_id, study_id,
            )
            if not after["bindings"][last_jobs[0]["transcript_id"]]["available"]:
                break
            time.sleep(2)
        if after is None:
            raise NativeFailure("No read-only DB snapshot after actual UI revocation")
        result["revocation_before_after"] = verify_revocation(
            before, after, last_jobs[0]["transcript_id"],
        )
        session.screenshot("real-source-revoked")
        session.stop()
        session.start()
        a = session.ax
        a.click("Studio", "button")
        a.choose("Open study", study_name)
        after_reopen = read_installed_lineage(
            db_path, fixture_dir / "origin-manifest.json",
            fixture_dir, project_id, study_id,
        )
        result["revocation_reopen_comparison"] = verify_revocation(
            before, after, last_jobs[0]["transcript_id"], after_reopen,
        )
        result["native_ui_and_canonical_steps"] = "OBSERVED"
        result["remaining_native_proof"] = [
            "Independent host-correlated stale source access denial",
            "Original attached AT-SPI audit and screenshots",
            "Windows WebView2 equivalent installed run",
        ]
    finally:
        try:
            session.stop()
        except Exception as exc:
            result["graphical_teardown_error"] = str(exc)
        for image in root.glob("*.png"):
            shutil.copy2(image, args.evidence_dir / ("two-source-" + image.name))
        log = root / "desktop.log"
        if log.exists():
            (args.evidence_dir / "two-source-desktop.log").write_bytes(
                log.read_bytes()[-100_000:]
            )


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--executable", required=True, type=Path)
    parser.add_argument("--package", required=True, type=Path)
    parser.add_argument("--source-sha", required=True)
    parser.add_argument("--run-url", required=True)
    parser.add_argument("--evidence-dir", required=True, type=Path)
    parser.add_argument("--ffmpeg", default="ffmpeg")
    parser.add_argument("--ffprobe", default="ffprobe")
    parser.add_argument("--transcription-timeout-seconds", type=int, default=1200)
    args = parser.parse_args()
    if not re.fullmatch(r"[0-9a-f]{40}", args.source_sha):
        parser.error("Expected exact full source SHA")
    if not re.fullmatch(
        r"https://github.com/MousaXD/scriptotar/actions/runs/[1-9][0-9]*",
        args.run_url,
    ):
        parser.error("Expected actual GitHub native workflow URL")
    args.evidence_dir.mkdir(parents=True, exist_ok=True)
    report = {
        "scope": "optional-installed-two-source-attempt",
        "utc": datetime.now(timezone.utc).isoformat(),
        "source_sha": args.source_sha,
        "run_url": args.run_url,
        "platform": platform.platform(),
        "package_sha256": sha256(args.package),
        "executable_sha256": sha256(args.executable),
        "native_multi_source_transcript": "NOT TESTED",
        "native_source_revocation": "NOT TESTED",
        "native_16_case_verdict_unchanged": True,
    }
    with tempfile.TemporaryDirectory(
            prefix="scriptotar-two-source-native-", dir=args.evidence_dir) as td:
        try:
            exercise_installed(args, Path(td), report)
            report["attempt_outcome"] = "PARTIAL_NATIVE_PROOF_REQUIRES_ADJUDICATION"
        except Exception as exc:
            report["attempt_outcome"] = "INCOMPLETE"
            report["blocker"] = type(exc).__name__ + ": " + str(exc)
        finally:
            (args.evidence_dir / "two-source-attempt.json").write_text(
                json.dumps(report, indent=2, ensure_ascii=False, sort_keys=True)
                + "\n", encoding="utf-8",
            )


if __name__ == "__main__":
    main()
