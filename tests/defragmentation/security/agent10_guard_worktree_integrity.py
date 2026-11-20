#!/usr/bin/env python3
"""Agent10 benign proof that a Git HEAD check cannot attest checked-out bytes.

Operates only on a separate disposable/isolated trusted-guard checkout. Never
runs the altered JS file. Restores original bytes before returning.
"""
from __future__ import annotations

import argparse
import subprocess
from pathlib import Path

EXPECTED_COMMIT = "55ce3ac389b7e1fcab99b3a5acc84b526630af72"
EXPECTED_BLOB = "5c746abd3caf414ba2c80be825d66983312236c6"
REL = "apps/desktop-ui/scripts/guard-vite-build-patterns.mjs"


def git(root: Path, *args: str) -> str:
    return subprocess.check_output(
        ["git", "-C", str(root), *args], text=True, stderr=subprocess.PIPE
    ).strip()


def main() -> int:
    p = argparse.ArgumentParser()
    p.add_argument("--checkout", required=True, type=Path)
    args = p.parse_args()
    root = args.checkout.resolve(strict=True)
    path = root / REL
    original = path.read_bytes()
    assert git(root, "rev-parse", "HEAD") == EXPECTED_COMMIT, "Wrong trusted checkout commit"
    committed_blob = git(root, "rev-parse", f"HEAD:{REL}")
    original_blob = git(root, "hash-object", REL)
    assert committed_blob == original_blob == EXPECTED_BLOB, "Original bytes not trusted"
    try:
        # Harmless source comment, never executed or saved back to repository.
        path.write_bytes(original + b"\n// AGENT10 BENIGN WORKTREE TAMPER CONTROL\n")
        assert git(root, "rev-parse", "HEAD") == EXPECTED_COMMIT, (
            "A dirty worktree unexpectedly changed commit identity"
        )
        tampered_blob = git(root, "hash-object", REL)
        assert tampered_blob != committed_blob, "Worktree tamper was not real"
        status = git(root, "status", "--porcelain", "--", REL)
        assert status, "Git status did not identify altered guard bytes"
        assert subprocess.run(
            ["git", "-C", str(root), "diff", "--quiet", "--", REL],
            check=False,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        ).returncode != 0, "Git diff incorrectly treated altered bytes as immutable"
        print("PASS (negative proof): git rev-parse HEAD still matches immutable commit")
        print("PASS: git hash-object / status / diff identify a modified guard working tree")
        print("SECURITY FINDING: HEAD-only assertion does not certify script bytes")
        print("No tampered JavaScript was executed; original immutable source restored")
    finally:
        path.write_bytes(original)
    assert git(root, "hash-object", REL) == EXPECTED_BLOB, "Cleanup failed"
    assert not git(root, "status", "--porcelain", "--", REL), "Worktree left dirty"
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
