#!/usr/bin/env python3
"""Gate Scriptotar Next preview publication on exact-SHA GitHub Actions health."""

from __future__ import annotations

import json
import os
import subprocess
import sys
import time
from dataclasses import dataclass
from typing import Callable, Iterable, Mapping, Sequence


class GateError(RuntimeError):
    """Raised when a mandatory release gate cannot be proven healthy."""


@dataclass(frozen=True)
class Gate:
    key: str
    workflow_file: str
    workflow_name: str
    required_jobs: tuple[str, ...]


GATES: tuple[Gate, ...] = (
    Gate(
        key="integration",
        workflow_file="integration.yml",
        workflow_name="Tauri migration integration",
        required_jobs=(
            "Rust workspace",
            "Svelte frontend",
            "Python sidecar",
            "Rust ↔ sidecar integration",
            "Supply chain",
            "Integrated Tauri build",
        ),
    ),
    Gate(
        key="security",
        workflow_file="security-hygiene.yml",
        workflow_name="Security hygiene",
        required_jobs=("repository-hygiene",),
    ),
    Gate(
        key="windows",
        workflow_file="windows-tauri.yml",
        workflow_name="Windows Tauri Installer",
        required_jobs=("Build and smoke-test NSIS setup.exe",),
    ),
    Gate(
        key="linux",
        workflow_file="linux-tauri.yml",
        workflow_name="Linux Tauri Package",
        required_jobs=("Build and validate Tauri deb",),
    ),
)


class GithubActionsClient:
    def __init__(self, repository: str) -> None:
        self.repository = repository

    def _json(self, args: Sequence[str]) -> object:
        command = ["gh", *args]
        completed = subprocess.run(command, check=False, capture_output=True, text=True)
        if completed.returncode != 0:
            stderr = completed.stderr.strip() or "unknown gh error"
            raise GateError(f"GitHub CLI command failed: {' '.join(command)}\n{stderr}")
        try:
            return json.loads(completed.stdout)
        except json.JSONDecodeError as exc:
            raise GateError(
                f"GitHub CLI returned invalid JSON for {' '.join(command)}: {exc}"
            ) from exc

    def list_runs(self, workflow_file: str, source_sha: str) -> list[dict[str, object]]:
        payload = self._json([
            "run", "list", "--repo", self.repository, "--workflow", workflow_file,
            "--branch", "main", "--event", "push", "--commit", source_sha,
            "--limit", "50", "--json",
            "databaseId,headSha,headBranch,event,status,conclusion,createdAt,workflowName",
        ])
        if not isinstance(payload, list):
            raise GateError(f"Unexpected run-list payload for {workflow_file}: {payload!r}")
        return [item for item in payload if isinstance(item, dict)]

    def view_run(self, run_id: int) -> dict[str, object]:
        payload = self._json([
            "run", "view", str(run_id), "--repo", self.repository, "--json",
            "databaseId,headSha,headBranch,event,status,conclusion,workflowName,jobs,url",
        ])
        if not isinstance(payload, dict):
            raise GateError(f"Unexpected run-view payload for run {run_id}: {payload!r}")
        return payload


def select_latest_exact_run(
    runs: Iterable[Mapping[str, object]], source_sha: str
) -> Mapping[str, object] | None:
    matches = [
        run
        for run in runs
        if run.get("headSha") == source_sha
        and run.get("headBranch") == "main"
        and run.get("event") == "push"
        and isinstance(run.get("databaseId"), int)
    ]
    if not matches:
        return None
    return max(matches, key=lambda run: str(run.get("createdAt") or ""))


def _required_job_failures(gate: Gate, jobs: object) -> list[str]:
    if not isinstance(jobs, list):
        return ["workflow job payload is missing or malformed"]
    failures: list[str] = []
    for required_name in gate.required_jobs:
        matches = [job for job in jobs if isinstance(job, dict) and job.get("name") == required_name]
        if not matches:
            failures.append(f"mandatory job {required_name!r} is missing")
            continue
        bad = [
            str(job.get("conclusion") or job.get("status") or "unknown")
            for job in matches
            if job.get("status") != "completed" or job.get("conclusion") != "success"
        ]
        if bad:
            failures.append(
                f"mandatory job {required_name!r} is not successful ({', '.join(bad)})"
            )
    return failures


def validate_completed_gate(gate: Gate, run: Mapping[str, object], source_sha: str) -> None:
    run_id = run.get("databaseId", "unknown")
    failures: list[str] = []
    if run.get("headSha") != source_sha:
        failures.append(f"head SHA is {run.get('headSha')!r}, expected {source_sha!r}")
    if run.get("headBranch") != "main":
        failures.append(f"head branch is {run.get('headBranch')!r}, expected 'main'")
    if run.get("event") != "push":
        failures.append(f"event is {run.get('event')!r}, expected 'push'")
    if run.get("workflowName") != gate.workflow_name:
        failures.append(
            f"workflow name is {run.get('workflowName')!r}, expected {gate.workflow_name!r}"
        )
    if run.get("status") != "completed":
        failures.append(f"workflow status is {run.get('status')!r}, expected 'completed'")
    if run.get("conclusion") != "success":
        failures.append(
            f"workflow conclusion is {run.get('conclusion')!r}, expected 'success'"
        )
    failures.extend(_required_job_failures(gate, run.get("jobs")))
    if failures:
        raise GateError(
            f"{gate.workflow_name} run {run_id} blocked publication: {'; '.join(failures)}"
        )


def prove_release_gates(
    client: object,
    source_sha: str,
    *,
    discovery_attempts: int,
    discovery_sleep_seconds: float,
    wait_attempts: int,
    wait_sleep_seconds: float,
    sleep: Callable[[float], None] = time.sleep,
) -> dict[str, dict[str, object]]:
    selected: dict[str, Mapping[str, object]] = {}
    for attempt in range(1, discovery_attempts + 1):
        for gate in GATES:
            if gate.key in selected:
                continue
            match = select_latest_exact_run(client.list_runs(gate.workflow_file, source_sha), source_sha)
            if match is not None:
                selected[gate.key] = match
        missing = [gate.workflow_name for gate in GATES if gate.key not in selected]
        if not missing:
            break
        if attempt < discovery_attempts:
            print(
                f"Waiting for exact-SHA workflow runs for {source_sha}: "
                + ", ".join(missing)
                + f" ({attempt}/{discovery_attempts})"
            )
            sleep(discovery_sleep_seconds)
    else:
        missing = [gate.workflow_name for gate in GATES if gate.key not in selected]
        raise GateError(
            f"Missing mandatory push workflow run(s) for exact SHA {source_sha}: "
            + ", ".join(missing)
        )

    latest_views: dict[str, dict[str, object]] = {}
    for attempt in range(1, wait_attempts + 1):
        pending: list[str] = []
        latest_views = {}
        for gate in GATES:
            run_id = int(selected[gate.key]["databaseId"])
            view = client.view_run(run_id)
            latest_views[gate.key] = view
            status = view.get("status")
            conclusion = view.get("conclusion")
            if status == "completed":
                if conclusion != "success":
                    validate_completed_gate(gate, view, source_sha)
            else:
                pending.append(f"{gate.workflow_name}={status or 'unknown'}")
        if not pending:
            break
        if attempt < wait_attempts:
            print(
                "Waiting for mandatory exact-SHA workflows to complete: "
                + ", ".join(pending)
                + f" ({attempt}/{wait_attempts})"
            )
            sleep(wait_sleep_seconds)
    else:
        pending = [
            f"{gate.workflow_name}={latest_views.get(gate.key, {}).get('status', 'unknown')}"
            for gate in GATES
            if latest_views.get(gate.key, {}).get("status") != "completed"
        ]
        raise GateError(
            "Timed out waiting for mandatory exact-SHA workflows: " + ", ".join(pending)
        )

    for gate in GATES:
        validate_completed_gate(gate, latest_views[gate.key], source_sha)
    return latest_views


def _positive_int(name: str, default: int) -> int:
    value = int(os.environ.get(name, str(default)))
    if value < 1:
        raise GateError(f"{name} must be at least 1")
    return value


def _nonnegative_float(name: str, default: float) -> float:
    value = float(os.environ.get(name, str(default)))
    if value < 0:
        raise GateError(f"{name} must be non-negative")
    return value


def _write_outputs(runs: Mapping[str, Mapping[str, object]]) -> None:
    output_path = os.environ.get("GITHUB_OUTPUT")
    if not output_path:
        return
    with open(output_path, "a", encoding="utf-8") as handle:
        for gate in GATES:
            handle.write(f"{gate.key}_run_id={int(runs[gate.key]['databaseId'])}\n")


def _write_summary(source_sha: str, runs: Mapping[str, Mapping[str, object]]) -> None:
    summary_path = os.environ.get("GITHUB_STEP_SUMMARY")
    if not summary_path:
        return
    with open(summary_path, "a", encoding="utf-8") as handle:
        handle.write("## Exact-SHA release gates passed\n\n")
        handle.write(f"Commit: `{source_sha}`\n\n")
        for gate in GATES:
            run = runs[gate.key]
            handle.write(
                f"- {gate.workflow_name}: run `{run['databaseId']}` completed successfully\n"
            )


def main() -> int:
    source_sha = os.environ.get("SOURCE_SHA", "").strip()
    repository = os.environ.get("GITHUB_REPOSITORY", "").strip()
    if not source_sha:
        raise GateError("SOURCE_SHA is required")
    if not repository:
        raise GateError("GITHUB_REPOSITORY is required")
    runs = prove_release_gates(
        GithubActionsClient(repository),
        source_sha,
        discovery_attempts=_positive_int("RELEASE_GATE_DISCOVERY_ATTEMPTS", 20),
        discovery_sleep_seconds=_nonnegative_float("RELEASE_GATE_DISCOVERY_SLEEP_SECONDS", 15.0),
        wait_attempts=_positive_int("RELEASE_GATE_WAIT_ATTEMPTS", 160),
        wait_sleep_seconds=_nonnegative_float("RELEASE_GATE_WAIT_SLEEP_SECONDS", 30.0),
    )
    _write_outputs(runs)
    _write_summary(source_sha, runs)
    print(f"All mandatory release gates passed for exact SHA {source_sha}.")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except GateError as exc:
        print(f"release gate failure: {exc}", file=sys.stderr)
        raise SystemExit(1)
