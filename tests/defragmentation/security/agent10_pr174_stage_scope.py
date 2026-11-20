#!/usr/bin/env python3
"""Independent Agent10 exact Git-tree source-only scope gate for PR174.

Reads candidate checkout as DATA, not executable application code. This test
does not run native benchmarks, first-party scripts, provider effects or tags.
"""
from __future__ import annotations

import argparse
import subprocess
import tempfile
from pathlib import Path

BASE = "7d025abc2577ab614772b7a3aa9e41e9b5c2707c"
TARGET = "e48bfc6848000a4c2b49cf26b8d68da504172df0"
ALLOWED = (
    "tests/defragmentation/benchmarks/native_ui_trace.py",
    "tests/defragmentation/benchmarks/test_performance_suite.py",
    "docs/defragmentation/handoffs/agent08-installed-performance-audit-20261120.md",
)
EXPECTED = {
    ALLOWED[0]: "285fd92051c3202f56ee0e67aeb185610377e2bf",
    ALLOWED[1]: "7af771711cfbdd8ec9e9701294afc1eb7ebc8cbf",
    ALLOWED[2]: "f74bf4031158394b132392795ea2e90e779cbced",
}
ATTACKED = (
    ("Rust build.rs", "apps/desktop/src-tauri/build.rs"),
    ("root Cargo lock", "Cargo.lock"),
    ("npm dependency manifest", "apps/desktop-ui/package.json"),
    ("npm dependency lock", "apps/desktop-ui/package-lock.json"),
    ("new npm configuration", "apps/desktop-ui/.npmrc"),
    ("Vite guard script", "apps/desktop-ui/scripts/guard-vite-build-patterns.mjs"),
    ("sidecar Python", "sidecars/transcription/build_runtime.py"),
    ("Python requirements", "sidecars/transcription/requirements-bundle.txt"),
    ("MCP preparation", "scripts/prepare_mcp_tools.py"),
    ("Windows installer workflow", ".github/workflows/windows-tauri.yml"),
    ("Linux package workflow", ".github/workflows/linux-tauri.yml"),
    ("installed OS acceptance workflow", ".github/workflows/installed-webview-acceptance.yml"),
    ("privileged publisher workflow", ".github/workflows/tauri-next-release.yml"),
    ("CODEOWNERS review policy", ".github/CODEOWNERS"),
    ("native UIA harness", "tests/defragmentation/native-e2e/windows_webview2.ps1"),
    ("unreviewed Rust resource", "apps/desktop/src-tauri/resources/injected.json"),
    ("unreviewed docs", "docs/defragmentation/handoffs/attacker.md"),
    ("unreviewed performance source", "tests/defragmentation/benchmarks/attacker.py"),
)


def git(repo: Path, *arguments: str, check: bool = True) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        ["git", "-C", str(repo), *arguments], text=True,
        capture_output=True, check=check,
    )


def safe_diff(repo: Path, before: str, after: str) -> int:
    return git(
        repo, "diff", "--quiet", before, after, "--", ".",
        *(f":(exclude){path}" for path in ALLOWED),
        check=False,
    ).returncode


def verify_exact_stage(root: Path) -> None:
    actual = git(root, "rev-parse", "HEAD").stdout.strip()
    assert actual == TARGET, ("Unreviewed stage HEAD", actual)
    if git(root, "cat-file", "-e", BASE + "^{commit}", check=False).returncode:
        git(root, "fetch", "--no-tags", "--depth=1", "origin", BASE)
    assert git(root, "cat-file", "-t", BASE).stdout.strip() == "commit"
    changed = git(root, "diff", "--name-only", BASE, TARGET).stdout.splitlines()
    assert sorted(changed) == sorted(ALLOWED), ("Unreviewed source paths", changed)
    assert safe_diff(root, BASE, TARGET) == 0, "Actual Git pathspec scope failed open"
    for path, blob in EXPECTED.items():
        recorded = git(root, "rev-parse", f"HEAD:{path}").stdout.strip()
        working = git(root, "hash-object", path).stdout.strip()
        assert recorded == working == blob, (path, "committed/executing blob drift", recorded, working)
        assert git(root, "diff", "--quiet", "--", path, check=False).returncode == 0
        print("PASS: exact staged source+working bytes: " + path)
    print("PASS: immutable source stage changes EXACTLY the three reviewed files")


def fixture_git_denials() -> None:
    with tempfile.TemporaryDirectory(prefix="agent10-pr174-") as td:
        repo = Path(td)
        git(repo, "init", "-q")
        git(repo, "config", "user.name", "Independent Agent10 fixture")
        git(repo, "config", "user.email", "agent10@example.invalid")
        for path in ALLOWED:
            file = repo / path
            file.parent.mkdir(parents=True, exist_ok=True)
            file.write_text("reviewed initial content\n", encoding="utf-8")
        git(repo, "add", "--all")
        git(repo, "commit", "-qm", "root fixture")
        base = git(repo, "rev-parse", "HEAD").stdout.strip()
        for path in ALLOWED:
            (repo / path).write_text("reviewed final content\n", encoding="utf-8")
        git(repo, "add", "--all")
        git(repo, "commit", "-qm", "three-source allowlisted fixture")
        allowed = git(repo, "rev-parse", "HEAD").stdout.strip()
        assert safe_diff(repo, base, allowed) == 0, "Legitimate three-blob scope rejected"

        for reason, path in ATTACKED:
            git(repo, "reset", "--hard", allowed)
            git(repo, "clean", "-fd")
            file = repo / path
            file.parent.mkdir(parents=True, exist_ok=True)
            file.write_text("unreviewed hostile tracked change\n", encoding="utf-8")
            git(repo, "add", "--all")
            git(repo, "commit", "-qm", "forbidden " + reason)
            bad = git(repo, "rev-parse", "HEAD").stdout.strip()
            assert safe_diff(repo, base, bad) != 0, ("FAIL OPEN", reason, path)
            print("PASS: deny committed unreviewed " + reason)

        # Deletion of previously approved dependency metadata must be denied.
        git(repo, "reset", "--hard", allowed)
        git(repo, "clean", "-fd")
        file = repo / "apps/desktop-ui/package.json"
        file.parent.mkdir(parents=True, exist_ok=True)
        file.write_text('{"name":"before"}\n', encoding="utf-8")
        git(repo, "add", "--all")
        git(repo, "commit", "-qm", "seed deletion control")
        before = git(repo, "rev-parse", "HEAD").stdout.strip()
        file.unlink()
        git(repo, "add", "--all")
        git(repo, "commit", "-qm", "unreviewed source deletion")
        after = git(repo, "rev-parse", "HEAD").stdout.strip()
        assert safe_diff(repo, before, after) != 0, "FAIL OPEN: unreviewed deletion"
        print("PASS: deny committed unreviewed deletion")
        print(f"PASS: legitimate three-file fixture and {len(ATTACKED)+1} real committed Git denials")


def main() -> None:
    p = argparse.ArgumentParser()
    p.add_argument("--target", type=Path, default=Path(".agent10-pr174-data"))
    args = p.parse_args()
    verify_exact_stage(args.target.resolve(strict=True))
    fixture_git_denials()


if __name__ == "__main__":
    main()
