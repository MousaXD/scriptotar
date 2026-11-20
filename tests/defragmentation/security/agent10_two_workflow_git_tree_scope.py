#!/usr/bin/env python3
"""Independent Agent10 Git-tree allowlist adversarial probe for combined PR168.

Check exact real combined PR168-vs-integration tree diff, then independently exercise
the *actual git diff/pathspec command semantics* against disposable commits.
Never run code from combined PR168, alter tracked product files, or write into main.
"""
from __future__ import annotations

import argparse
import subprocess
import tempfile
from pathlib import Path

BASE = "0f8e33f0e2abef003440ef27ef76b7cb448d2b32"
TARGET = "a521b706feb278f101fe77fd9fae4445fddd9e0b"
ALLOWED = (
    ".github/workflows/installed-webview-acceptance.yml",
    ".github/workflows/windows-tauri.yml",
    ".github/workflows/linux-tauri.yml",
)
PATHSPEC = (".", *[":(exclude)" + path for path in ALLOWED])
INJECTED = (
    ("Rust build.rs code", "apps/desktop/src-tauri/build.rs", "fn main() { println!(\"cargo:warning=changed\"); }\n"),
    ("frontend install hook", "apps/desktop-ui/package.json", '{"scripts":{"preinstall":"echo changed"}}\n'),
    ("dependency lockfile", "apps/desktop-ui/package-lock.json", '{"lockfileVersion":3}\n'),
    ("Cargo lockfile", "Cargo.lock", "version = 3\n"),
    ("sidecar executable", "sidecars/transcription/build_runtime.py", "# changed by negative control\n"),
    ("extra unreviewed workflow", ".github/workflows/new-privileged-workflow.yml", "name: attack-control\n"),
    ("new bundled resource", "apps/desktop/src-tauri/resources/injected.txt", "changed\n"),
    ("native runner test", "tests/defragmentation/native-e2e/windows_webview2.ps1", "# changed\n"),
    ("Python requirements", "sidecars/transcription/requirements-bundle.txt", "untrusted\n"),
    ("other release workflow", ".github/workflows/tauri-next-release.yml", "permissions: write-all\n"),
    ("CODEOWNERS", ".github/CODEOWNERS", "* @attacker\n"),
    ("unreviewed MCP script", "scripts/prepare_mcp_tools.py", "print('attacker')\n"),
    ("new npm config", "apps/desktop-ui/.npmrc", "ignore-scripts=false\n"),
)


def git(repo: Path, *args: str, check: bool = True) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        ["git", "-C", str(repo), *args],
        capture_output=True, text=True, check=check,
    )


def scoped_diff(repo: Path, old: str, new: str) -> int:
    return git(repo, "diff", "--quiet", old, new, "--", *PATHSPEC, check=False).returncode


def verify_real_source(repo: Path) -> None:
    actual = git(repo, "rev-parse", "HEAD").stdout.strip()
    if actual != TARGET:
        raise AssertionError(f"Exact combined PR168 target drift: {actual}")
    # The immutable integration SHA may need an explicit shallow fetch.
    if git(repo, "cat-file", "-e", BASE + "^{commit}", check=False).returncode:
        git(repo, "fetch", "--no-tags", "--depth=1", "origin", BASE)
    assert git(repo, "cat-file", "-t", BASE).stdout.strip() == "commit"
    names = git(repo, "diff", "--name-only", BASE, TARGET).stdout.splitlines()
    assert sorted(names) == sorted(ALLOWED), (
        f"Unexpected real combined PR168 changed source paths: {names}"
    )
    assert scoped_diff(repo, BASE, TARGET) == 0, (
        "Real combined PR168 tree changed a forbidden path"
    )
    print(f"PASS: exact source {TARGET} changes ONLY the three expected workflow paths")


def isolated_negative_controls() -> None:
    with tempfile.TemporaryDirectory(prefix="agent10-three-file-tree-") as tmp:
        repo = Path(tmp)
        git(repo, "init", "-q")
        git(repo, "config", "user.name", "Agent10 CI-only negative control")
        git(repo, "config", "user.email", "agent10-test@example.invalid")
        for name in ALLOWED:
            target = repo / name
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_text("name: baseline\n", encoding="utf-8")
        git(repo, "add", "--all")
        git(repo, "commit", "-qm", "read-only baseline fixture")
        base = git(repo, "rev-parse", "HEAD").stdout.strip()

        # Both allowlisted workflow YAMLs may change without widening the scope.
        for name in ALLOWED:
            (repo / name).write_text("name: reviewed-workflow-only\n", encoding="utf-8")
        git(repo, "add", "--all")
        git(repo, "commit", "-qm", "three reviewed workflows only")
        acceptable = git(repo, "rev-parse", "HEAD").stdout.strip()
        assert scoped_diff(repo, base, acceptable) == 0, "Valid three-YAML diff was rejected"

        # Every injected unreviewed path must make the ORIGINAL Git pathspec RED,
        # even though the three reviewed workflow changes are still present.
        for label, name, payload in INJECTED:
            git(repo, "reset", "--hard", acceptable)
            git(repo, "clean", "-fd")
            file = repo / name
            file.parent.mkdir(parents=True, exist_ok=True)
            file.write_text(payload, encoding="utf-8")
            git(repo, "add", "--all")
            git(repo, "commit", "-qm", "hostile unreviewed " + label)
            changed = git(repo, "rev-parse", "HEAD").stdout.strip()
            if scoped_diff(repo, base, changed) == 0:
                raise AssertionError(f"FAIL OPEN: unreviewed {label} bypassed exact scope")
            print(f"PASS: deny committed unreviewed {label}: {name}")

        # Verify an unreviewed deletion is also denied.
        git(repo, "reset", "--hard", acceptable)
        git(repo, "clean", "-fd")
        path = repo / "apps/desktop-ui/package.json"
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text('{"name":"baseline"}\n', encoding="utf-8")
        git(repo, "add", "--all")
        git(repo, "commit", "-qm", "seed package for deletion negative control")
        with_package = git(repo, "rev-parse", "HEAD").stdout.strip()
        path.unlink()
        git(repo, "add", "--all")
        git(repo, "commit", "-qm", "unreviewed deletion negative control")
        deleted = git(repo, "rev-parse", "HEAD").stdout.strip()
        assert scoped_diff(repo, with_package, deleted) != 0, (
            "FAIL OPEN: deletion of unreviewed package.json was accepted"
        )
        print("PASS: deny committed unreviewed deletion of package.json")

    print(f"PASS: actual Git-tree behavior: three permitted YAML changes + {len(INJECTED)+1} denied mutations")


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--target", type=Path, default=Path(".agent10-pr158-target"))
    args = ap.parse_args()
    verify_real_source(args.target.resolve(strict=True))
    isolated_negative_controls()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
