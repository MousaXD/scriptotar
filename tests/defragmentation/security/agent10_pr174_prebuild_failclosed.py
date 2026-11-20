#!/usr/bin/env python3
"""Agent10 P0 source-pin fail-closed test for review-only PR174.

Proves that ORIGINAL existing four installed/build job source gates accept
reviewed integration 7d025... and reject staged P2 source e48bfc...
before executing any first-party code. Tests original Git pathspec semantics,
not synthetic string check alone. PR174 checkout is treated solely as DATA.
"""
from __future__ import annotations

import argparse
import re
import subprocess
from pathlib import Path

REVIEWED = "0f8e33f0e2abef003440ef27ef76b7cb448d2b32"
INTEGRATION = "7d025abc2577ab614772b7a3aa9e41e9b5c2707c"
STAGE = "e48bfc6848000a4c2b49cf26b8d68da504172df0"

ALLOWED = (
    ".github/workflows/windows-tauri.yml",
    ".github/workflows/installed-webview-acceptance.yml",
    ".github/workflows/linux-tauri.yml",
)
P2 = (
    "docs/defragmentation/handoffs/agent08-installed-performance-audit-20261120.md",
    "tests/defragmentation/benchmarks/native_ui_trace.py",
    "tests/defragmentation/benchmarks/test_performance_suite.py",
)
JOBS = (
    (ALLOWED[0], "build-windows-installer", "Verify reviewed Python/runtime/QA inputs before script execution"),
    (ALLOWED[1], "linux-webkit", "Verify reviewed Python/runtime/QA inputs before script execution"),
    (ALLOWED[1], "windows-webview2", "Verify reviewed Python/runtime/QA inputs before script execution"),
    (ALLOWED[2], "build-linux-deb", "Verify independently reviewed Linux package executable source scope"),
)
PYTHON_PATHS = (
    "sidecars/transcription/", "scripts/",
    "tests/defragmentation/native-e2e/",
    ":(glob)**/*.py", ":(glob)**/requirements*.txt",
    ":(glob)**/pyproject.toml", ":(glob)**/setup.py",
    ":(glob)**/setup.cfg", ":(glob)**/Pipfile*",
    ":(glob)**/tox.ini",
)


def git(root: Path, *args: str, check: bool = True) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        ["git", "-C", str(root), *args],
        text=True, capture_output=True, check=check,
    )


def checkout_commit(root: Path, commit: str) -> None:
    if git(root, "cat-file", "-e", commit + "^{commit}", check=False).returncode:
        git(root, "fetch", "--no-tags", "--depth=1", "origin", commit)
    assert git(root, "cat-file", "-t", commit).stdout.strip() == "commit"


def source_guard(root: Path, head: str) -> int:
    return git(
        root, "diff", "--quiet", REVIEWED, head, "--", ".",
        *(f":(exclude){path}" for path in ALLOWED),
        check=False,
    ).returncode


def python_provenance(root: Path, head: str) -> int:
    return git(root, "diff", "--quiet", REVIEWED, head, "--", *PYTHON_PATHS, check=False).returncode


def extract_job(text: str, name: str) -> str:
    m = re.search(rf"(?m)^  {re.escape(name)}:\s*$", text)
    assert m, f"missing protected job {name}"
    body = text[m.end():]
    next_job = re.search(r"(?m)^  [\w-]+:\s*$", body)
    return body[:next_job.start()] if next_job else body


def extract_step(job: str, name: str) -> str:
    anchors = list(re.finditer(r"(?m)^ {6}- (?:name|uses): .+$", job))
    for i, a in enumerate(anchors):
        chunk = job[a.start():anchors[i+1].start() if i+1<len(anchors) else len(job)]
        if re.search(rf"(?m)^      - name: {re.escape(name)}\s*$", chunk):
            return chunk
    raise AssertionError(f"missing mandatory source check step {name}")


def inspect_job_sources(root: Path) -> None:
    expected_excludes = tuple(f":(exclude){p}" for p in ALLOWED)
    for filename, job, step_name in JOBS:
        original = git(root, "show", f"{INTEGRATION}:{filename}").stdout
        staged = git(root, "show", f"{STAGE}:{filename}").stdout
        assert original == staged, f"Stage silently changed protected workflow {filename}"
        protected = extract_job(original, job)
        audit = extract_step(protected, step_name)
        assert "set -euo pipefail" in audit, f"not fail-closed {job}"
        assert f"reviewed={REVIEWED}" in audit, f"incorrect source pin {job}"
        assert 'git fetch --no-tags --depth=1 origin "$reviewed"' in audit, f"reviewed ref not fetched {job}"
        assert 'git diff --quiet "$reviewed" HEAD -- .' in audit, f"no complete source Git diff {job}"
        all_excludes = re.findall(r":\(exclude\)[^'\s]+", audit)
        assert all_excludes == list(expected_excludes), (job, "source allowlist drift", all_excludes)
        assert 'git diff --quiet "$reviewed" HEAD --' in audit, f"missing Python source check {job}"
        for path in PYTHON_PATHS:
            assert path in audit, (job, "source check omitted", path)
        print(f"PASS: original protected {job} checks exact reviewed base and ONLY three YAML exclusions")


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--target", type=Path, default=Path(".agent10-pr174-data"))
    args = ap.parse_args()
    root = args.target.resolve(strict=True)
    assert git(root, "rev-parse", "HEAD").stdout.strip() == STAGE, "stage HEAD drift"
    for commit in (REVIEWED, INTEGRATION):
        checkout_commit(root, commit)
    changed = git(root, "diff", "--name-only", INTEGRATION, STAGE).stdout.splitlines()
    assert tuple(sorted(changed)) == tuple(sorted(P2)), ("PR174 changed unreviewed paths", changed)
    inspect_job_sources(root)

    base_clean = source_guard(root, INTEGRATION)
    staged_rejected = source_guard(root, STAGE)
    python_clean = python_provenance(root, INTEGRATION)
    python_rejected = python_provenance(root, STAGE)
    assert base_clean == 0, ("FAIL CLOSED regression: approved integration source rejected", base_clean)
    assert python_clean == 0, ("FAIL CLOSED regression: approved integration Python rejected", python_clean)
    assert staged_rejected == 1, (
        "FAIL OPEN: PR174 new unreviewed Python accepted by existing trusted prebuild guard",
        staged_rejected,
    )
    assert python_rejected == 1, (
        "FAIL OPEN: PR174 new unreviewed parser unexpectedly passes Python provenance check",
        python_rejected,
    )
    print("PASS: all FOUR original protected jobs would ACCEPT independently reviewed current integration")
    print("PASS: all FOUR original protected jobs would DENY staged PR174 unreviewed parser source")
    print("PASS: original unmodified git diff restricted-source and global Python checks both fail closed")
    print("PASS: no staged code executed, no publisher/paid/provider/native evidence is certified")


if __name__ == "__main__":
    main()
