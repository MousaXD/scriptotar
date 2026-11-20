#!/usr/bin/env python3
"""Agent10 independent read-only command-order and credential audit for PR158.

Reads source workflow YAML as text from an exact-SHA checkout; never executes
untrusted checkout code or claims the Vite guard covers Python/third parties.
"""
from __future__ import annotations

import argparse
import re
from pathlib import Path

PIN = "55ce3ac389b7e1fcab99b3a5acc84b526630af72"
TARGETS = (
    (".github/workflows/windows-tauri.yml", "build-windows-installer"),
    (".github/workflows/installed-webview-acceptance.yml", "windows-webview2"),
)
STEP = re.compile(r"(?m)^ {6}- (?:name|uses): .*?$")
EXEC = re.compile(
    r"(?m)^\s*(?:run:\s*)?(?:&\s+)?(?:python(?:3)?(?:\.exe)?|"
    r"pip(?:3)?(?:\.exe)?|npm|npx|pnpm|yarn|node|cargo|tauri)(?:[ \t]|$)"
)


def inspect(text: str, job: str) -> list[str]:
    problems: list[str] = []
    found = re.search(rf"(?m)^  {re.escape(job)}:\s*$", text)
    if not found:
        return [f"Missing job {job}"]
    tail = text[found.end():]
    next_job = re.search(r"(?m)^  [A-Za-z_][A-Za-z0-9_-]*:\s*$", tail)
    body = tail[:next_job.start()] if next_job else tail
    spans = list(STEP.finditer(body))
    if not spans:
        return [f"Job {job} has no parsed steps"]
    steps = [body[m.start():spans[i + 1].start() if i + 1 < len(spans) else len(body)]
             for i, m in enumerate(spans)]

    def named(name: str) -> int | None:
        positions = [i for i, s in enumerate(steps)
                     if re.search(rf"(?m)^      - name: {re.escape(name)}\s*$", s)]
        if len(positions) != 1:
            problems.append(f"Missing/duplicate required step: {name} (count {len(positions)})")
            return None
        return positions[0]

    checkout = named("Checkout immutable reviewed Agent11 pre-Vite gate")
    audit = named("Validate pinned prebuild guard and audited PR source before npm/Vite")
    cli = named("Install pinned Tauri CLI")
    git_checkouts = [(i, s) for i, s in enumerate(steps)
                     if re.search(r"(?m)^\s+(?:-\s*)?uses: actions/checkout@", s)]
    if len(git_checkouts) < 2:
        problems.append("First-party and trusted checkouts not both identifiable")
    for i, block in git_checkouts:
        if not re.search(r"(?m)^\s+persist-credentials:\s*false\s*$", block):
            problems.append(f"Checkout step {i + 1} unnecessarily persists credentials")

    if checkout is not None:
        trusted = steps[checkout]
        if not re.search(rf"(?m)^\s+ref:\s*{PIN}\s*$", trusted):
            problems.append("Trusted checkout is not pinned to reviewed guard SHA")
        if "apps/desktop-ui/scripts/guard-vite-build-patterns.mjs" not in trusted:
            problems.append("Expected trusted scanner not sparse-checked out")
        if audit is not None and checkout >= audit:
            problems.append("Guard invoked before immutable checkout")

    if audit is not None:
        verified = steps[audit]
        if PIN not in verified or "rev-parse" not in verified:
            problems.append("No immutable guard SHA assertion")
        if "--self-test" not in verified or "--workspace apps/desktop-ui" not in verified:
            problems.append("Missing guard self-test or actual workspace audit")
        for i, s in enumerate(steps[:audit]):
            commands = [line.strip() for line in s.splitlines() if EXEC.search(line)]
            if commands:
                problems.append(f"PR-controlled/other pre-guard executable step {i + 1}: "
                                + " | ".join(commands[:3]))
        if cli is not None and cli <= audit:
            problems.append("Global npm Tauri installation occurs before trusted guard")

    if cli is not None:
        if not re.search(r"(?m)^\s*run:\s*npm install --global @tauri-apps/cli@2\.11\.2\s*$",
                         steps[cli]):
            problems.append("Global Tauri CLI command differs from pinned expected command")
        for i, block in enumerate(steps):
            if re.search(r"(?m)^\s*(?:run:\s*)?tauri icon\b", block) and cli >= i:
                problems.append("Tauri icon executes before global CLI installation")
            if re.search(r"(?m)^\s*(?:run:\s*)?npm ci\b", block) and audit is not None and audit >= i:
                problems.append("Desktop npm ci executes before trusted scanner")

    if not re.search(r"(?m)^permissions:\s*\n\s+contents:\s+read\s*$", text):
        problems.append("Read-only job token permission not established")
    return problems


def self_test() -> None:
    fixture = """name: fixture
permissions:
  contents: read
jobs:
  build-windows-installer:
    steps:
      - uses: actions/checkout@11d5960a326750d5838078e36cf38b85af677262
        with:
          persist-credentials: false
      - name: Checkout immutable reviewed Agent11 pre-Vite gate
        uses: actions/checkout@11d5960a326750d5838078e36cf38b85af677262
        with:
          ref: 55ce3ac389b7e1fcab99b3a5acc84b526630af72
          persist-credentials: false
          sparse-checkout: |
            apps/desktop-ui/scripts/guard-vite-build-patterns.mjs
      - name: Validate pinned prebuild guard and audited PR source before npm/Vite
        run: |
          node -e "rev-parse 55ce3ac389b7e1fcab99b3a5acc84b526630af72"
          node .agent11-trusted-guard/apps/desktop-ui/scripts/guard-vite-build-patterns.mjs --self-test
          node .agent11-trusted-guard/apps/desktop-ui/scripts/guard-vite-build-patterns.mjs --workspace apps/desktop-ui
      - name: Install pinned Tauri CLI
        run: npm install --global @tauri-apps/cli@2.11.2
      - name: Build frontend
        run: |
          npm ci
          tauri icon ../../scriptotar.svg
"""
    assert inspect(fixture, "build-windows-installer") == []
    poisoned = fixture.replace("      - name: Checkout immutable reviewed Agent11 pre-Vite gate",
                               "      - name: Poisoned precheck\n"
                               "        run: python scripts/prepare_mcp_tools.py\n"
                               "      - name: Checkout immutable reviewed Agent11 pre-Vite gate")
    assert any("pre-guard executable" in x for x in inspect(poisoned, "build-windows-installer"))
    bad_token = fixture.replace("          persist-credentials: false\n", "", 1)
    assert any("credentials" in x for x in inspect(bad_token, "build-windows-installer"))
    bad_pin = fixture.replace("          ref: " + PIN, "          ref: main")
    assert any("not pinned" in x for x in inspect(bad_pin, "build-windows-installer"))
    early_npm = fixture.replace(
        "      - name: Install pinned Tauri CLI\n        run: npm install --global @tauri-apps/cli@2.11.2\n", "")
    early_npm = early_npm.replace(
        "      - name: Checkout immutable reviewed Agent11 pre-Vite gate",
        "      - name: Install pinned Tauri CLI\n"
        "        run: npm install --global @tauri-apps/cli@2.11.2\n"
        "      - name: Checkout immutable reviewed Agent11 pre-Vite gate")
    assert any("before trusted guard" in x for x in inspect(early_npm, "build-windows-installer"))
    print("PASS: positive fixture and four independent negative mutations")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--target", type=Path, default=Path(".agent10-pr158-target"))
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    if args.self_test:
        self_test()
        return 0
    fail = False
    for path, job in TARGETS:
        source = args.target / path
        if not source.is_file():
            print(f"FAIL: missing pinned target {source}")
            fail = True
            continue
        issues = inspect(source.read_text(encoding="utf8"), job)
        if issues:
            fail = True
            print(f"FAIL: {path} ({len(issues)} findings)")
            for item in issues:
                print("  - " + item)
        else:
            print(f"PASS: {path}: scoped ordering/credentials only; third-party + Python sources NOT certified")
    return int(fail)


if __name__ == "__main__":
    raise SystemExit(main())
