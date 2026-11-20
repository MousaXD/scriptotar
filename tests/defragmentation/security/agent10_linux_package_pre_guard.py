#!/usr/bin/env python3
"""Agent10 strict independent Linux package pre-guard negative gate.

Audits the exact *merged integration* Linux packaging workflow as READ-ONLY
source. Does not execute or alter first-party code from target checkout.
"""
from __future__ import annotations
import argparse
import re
from pathlib import Path

STEPS = re.compile(r"(?m)^ {6}- (?:name|uses): .*$")
TRUSTED = "Checkout immutable reviewed Agent11 pre-Vite gate"
GUARD = "Validate pinned prebuild guard and audited PR source before npm/Vite"


def inspect(source: str) -> list[str]:
    findings: list[str] = []
    start = re.search(r"(?m)^  build-linux-deb:\s*$", source)
    if not start:
        return ["Linux package build-linux-deb job absent"]
    tail = source[start.end():]
    nextjob = re.search(r"(?m)^  [A-Za-z_][\w-]*:\s*$", tail)
    body = tail[:nextjob.start()] if nextjob else tail
    chunks = list(STEPS.finditer(body))
    if not chunks:
        return ["No parseable Linux package steps"]
    steps = [body[m.start():chunks[i + 1].start() if i + 1 < len(chunks) else len(body)]
             for i, m in enumerate(chunks)]

    def location(name: str) -> int | None:
        positions = [i for i, s in enumerate(steps)
                     if re.search(rf"(?m)^      - name: {re.escape(name)}\s*$", s)]
        if len(positions) != 1:
            findings.append(f"Missing/duplicate trusted step {name} count={len(positions)}")
            return None
        return positions[0]

    trusted = location(TRUSTED)
    guard = location(GUARD)
    if trusted is not None and guard is not None and trusted >= guard:
        findings.append("Pinned trusted checkout not before guard execution")
    if guard is None:
        return findings

    for i, s in enumerate(steps[:guard]):
        if not re.search(r"(?m)^\s+run:\s*", s):
            continue
        is_sha_guard = "name: Confirm exact unmerged source checkout" in s
        if is_sha_guard:
            body = s.split("run:", 1)[1].strip()
            if body == 'test "$(git rev-parse HEAD)" = "$EXPECTED_SOURCE_SHA"':
                continue
        commands = [x.strip() for x in s.splitlines()
                    if re.search(r"^\s*(?:run:)?\s*(?:python|pip|cargo|npm|tauri|sudo|bash|node)\b", x)]
        findings.append(f"Untrusted Linux package command before guard at step {i+1}: "
                        + (" | ".join(commands[:3]) if commands else "unrecognized shell commands"))

    primary = [s for s in steps if re.search(r"(?m)^\s+(?:- )?uses:\s*actions/checkout@", s)]
    if len(primary) < 2:
        findings.append("Required source/guard checkouts missing")
    for i, chunk in enumerate(primary):
        if not re.search(r"(?m)^\s+persist-credentials:\s*false\s*$", chunk):
            findings.append(f"Checkout {i+1} persists credentials before audit")
    if "permissions:\n  contents: read" not in source:
        findings.append("Missing read-only top-level permissions")
    return findings


def fixture_tests() -> None:
    good = """permissions:
  contents: read
jobs:
  build-linux-deb:
    steps:
      - uses: actions/checkout@11d5960a326750d5838078e36cf38b85af677262
        with:
          persist-credentials: false
      - name: Confirm exact unmerged source checkout
        run: test "$(git rev-parse HEAD)" = "$EXPECTED_SOURCE_SHA"
      - name: Checkout immutable reviewed Agent11 pre-Vite gate
        uses: actions/checkout@11d5960a326750d5838078e36cf38b85af677262
        with:
          persist-credentials: false
      - name: Validate pinned prebuild guard and audited PR source before npm/Vite
        run: node trusted-scan.mjs --workspace apps/desktop-ui
      - name: Install and package
        run: |
          python scripts/prepare_mcp_tools.py
          cargo install tauri-cli
"""
    assert inspect(good) == [], inspect(good)
    for line in ("python scripts/prepare_mcp_tools.py", "cargo install tauri-cli",
                 "npm ci", "sudo apt-get install foo", "bash scripts/new.sh"):
        bad = good.replace("      - name: Checkout immutable reviewed Agent11 pre-Vite gate",
                           "      - name: Inject unreviewed early command\n"
                           f"        run: {line}\n"
                           "      - name: Checkout immutable reviewed Agent11 pre-Vite gate")
        assert any("before guard" in x for x in inspect(bad)), line
    broken = good.replace("          persist-credentials: false\n", "", 1)
    assert any("persists credentials" in x for x in inspect(broken))
    print("PASS: clean Linux workflow fixture + six fail-closed adversarial controls")


def main() -> int:
    p = argparse.ArgumentParser()
    p.add_argument("--target", type=Path, default=Path(".agent10-merged-integration"))
    args = p.parse_args()
    fixture_tests()
    file = args.target / ".github/workflows/linux-tauri.yml"
    if not file.is_file():
        print("FAIL: exact Linux package source missing")
        return 2
    problems = inspect(file.read_text(encoding="utf-8"))
    if problems:
        print(f"FAIL: original Linux package pre-guard trust boundary: {len(problems)} findings")
        for item in problems:
            print(" - " + item)
        return 1
    print("PASS: reviewed Linux package trusted execution ordering")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
