#!/usr/bin/env python3
"""Validate Scriptotar Next release ownership and packaging workflow contracts."""

from __future__ import annotations

import re
import sys
from pathlib import Path
from typing import Any, Iterable

import yaml

ROOT = Path(__file__).resolve().parents[2]
WORKFLOWS = ROOT / ".github" / "workflows"
PUBLISHER = WORKFLOWS / "tauri-next-release.yml"
PACKAGING = {
    WORKFLOWS / "windows-tauri.yml": {
        "job_name": "Build and smoke-test NSIS setup.exe",
        "artifact": "scriptotar-windows-tauri",
    },
    WORKFLOWS / "linux-tauri.yml": {
        "job_name": "Build and validate Tauri deb",
        "artifact": "scriptotar-linux-tauri",
    },
}
ROLLING_TAG = "tauri-next-latest"

GH_RELEASE = re.compile(r"\bgh\s+release\s+(?:create|edit|upload)\b", re.IGNORECASE)
FORCE_TAG = re.compile(
    r"\bgit\s+tag\b(?=[^\n;]*?(?:\s-f\b|\s--force\b))",
    re.IGNORECASE,
)
FORCE_PUSH = re.compile(
    r"\bgit\s+push\b(?=[^\n;]*?(?:\s-f\b|\s--force\b))",
    re.IGNORECASE,
)


def load_workflow(path: Path) -> dict[str, Any]:
    try:
        data = yaml.safe_load(path.read_text(encoding="utf-8"))
    except yaml.YAMLError as exc:
        raise AssertionError(f"{path.relative_to(ROOT)} is invalid YAML: {exc}") from exc
    if not isinstance(data, dict):
        raise AssertionError(f"{path.relative_to(ROOT)} must contain a YAML mapping")
    return data


def normalize_shell(text: str) -> str:
    # Remove shell line continuations, then collapse formatting-only whitespace so
    # split commands are still detected.
    text = re.sub(r"\\\s*\n\s*", " ", text)
    return re.sub(r"[\t ]+", " ", text)


def iter_run_blocks(data: dict[str, Any]) -> Iterable[str]:
    jobs = data.get("jobs", {})
    if not isinstance(jobs, dict):
        return
    for job in jobs.values():
        if not isinstance(job, dict):
            continue
        steps = job.get("steps", [])
        if not isinstance(steps, list):
            continue
        for step in steps:
            if isinstance(step, dict) and isinstance(step.get("run"), str):
                yield normalize_shell(step["run"])


def iter_permissions(data: dict[str, Any]) -> Iterable[Any]:
    yield data.get("permissions")
    jobs = data.get("jobs", {})
    if isinstance(jobs, dict):
        for job in jobs.values():
            if isinstance(job, dict):
                yield job.get("permissions")


def grants_contents_write(permission: Any) -> bool:
    if isinstance(permission, str):
        return permission.strip().lower() == "write-all"
    if isinstance(permission, dict):
        value = permission.get("contents")
        return isinstance(value, str) and value.strip().lower() == "write"
    return False


def assert_packaging_contract(path: Path, expected: dict[str, str]) -> None:
    data = load_workflow(path)
    rel = path.relative_to(ROOT)

    if any(grants_contents_write(p) for p in iter_permissions(data)):
        raise AssertionError(f"{rel} must not grant contents: write or write-all")

    runs = list(iter_run_blocks(data))
    for run in runs:
        if GH_RELEASE.search(run):
            raise AssertionError(f"{rel} must not create, edit, or upload GitHub Releases")
        if FORCE_TAG.search(run) and ROLLING_TAG in run:
            raise AssertionError(f"{rel} must not force-move {ROLLING_TAG}")
        if FORCE_PUSH.search(run) and ROLLING_TAG in run:
            raise AssertionError(f"{rel} must not force-push {ROLLING_TAG}")

    jobs = data.get("jobs", {})
    if not isinstance(jobs, dict):
        raise AssertionError(f"{rel} has no jobs mapping")

    job_names = {
        job.get("name")
        for job in jobs.values()
        if isinstance(job, dict) and isinstance(job.get("name"), str)
    }
    if expected["job_name"] not in job_names:
        raise AssertionError(f"{rel} lost required job: {expected['job_name']}")

    artifact_names: set[str] = set()
    for job in jobs.values():
        if not isinstance(job, dict):
            continue
        for step in job.get("steps", []) if isinstance(job.get("steps"), list) else []:
            if not isinstance(step, dict):
                continue
            uses = step.get("uses")
            with_args = step.get("with")
            if isinstance(uses, str) and "actions/upload-artifact@" in uses and isinstance(with_args, dict):
                name = with_args.get("name")
                if isinstance(name, str):
                    artifact_names.add(name)
    if expected["artifact"] not in artifact_names:
        raise AssertionError(f"{rel} lost required artifact upload: {expected['artifact']}")


def assert_single_rolling_release_owner() -> None:
    publisher_data = load_workflow(PUBLISHER)
    publisher_text = PUBLISHER.read_text(encoding="utf-8")
    publisher_runs = "\n".join(iter_run_blocks(publisher_data))

    if ROLLING_TAG not in publisher_text:
        raise AssertionError(f"{PUBLISHER.relative_to(ROOT)} must own {ROLLING_TAG}")
    if not any(grants_contents_write(p) for p in iter_permissions(publisher_data)):
        raise AssertionError(f"{PUBLISHER.relative_to(ROOT)} needs contents: write to publish")
    if not GH_RELEASE.search(publisher_runs):
        raise AssertionError(f"{PUBLISHER.relative_to(ROOT)} must contain release mutation logic")
    if not (FORCE_TAG.search(publisher_runs) or FORCE_PUSH.search(publisher_runs)):
        raise AssertionError(f"{PUBLISHER.relative_to(ROOT)} must contain rolling-tag mutation logic")

    for path in sorted([*WORKFLOWS.glob("*.yml"), *WORKFLOWS.glob("*.yaml")]):
        if path == PUBLISHER:
            continue
        data = load_workflow(path)
        text = path.read_text(encoding="utf-8")
        runs = "\n".join(iter_run_blocks(data))

        # A workflow that references the rolling tag and also contains release/tag
        # mutation commands is another potential owner, even when the command uses
        # an environment variable rather than spelling the tag inline.
        if ROLLING_TAG in text and (
            GH_RELEASE.search(runs) or FORCE_TAG.search(runs) or FORCE_PUSH.search(runs)
        ):
            raise AssertionError(
                f"{path.relative_to(ROOT)} can mutate {ROLLING_TAG}; only "
                f"{PUBLISHER.relative_to(ROOT)} may do that"
            )


def main() -> int:
    for path, expected in PACKAGING.items():
        assert_packaging_contract(path, expected)
    assert_single_rolling_release_owner()
    print("Release ownership contract OK: packaging is artifact-only and tauri-next-release.yml is sole rolling-release writer.")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except AssertionError as exc:
        print(f"release ownership contract failed: {exc}", file=sys.stderr)
        raise SystemExit(1)
