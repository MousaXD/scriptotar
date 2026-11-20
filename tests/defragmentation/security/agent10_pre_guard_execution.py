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
GUARD_BLOB = "5c746abd3caf414ba2c80be825d66983312236c6"
PYTHON_BASELINE = "514ec3ee571c4309e5b7473b8a65ab4fb73f497a"
TARGETS = (
    (".github/workflows/windows-tauri.yml", "build-windows-installer"),
    (".github/workflows/installed-webview-acceptance.yml", "linux-webkit"),
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
    cli = named("Install pinned Tauri CLI") if job != "linux-webkit" else None
    source_review = named("Verify reviewed Python/runtime/QA inputs before script execution")
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
        required_verification = [
            ('rev-parse HEAD', PIN, "trusted guard commit"),
            ('rev-parse "HEAD:$guard"', GUARD_BLOB, "committed guard blob"),
            ('hash-object --path="$guard" "$guard"', GUARD_BLOB, "actual guard worktree bytes"),
            ('diff --quiet -- "$guard"', None, "clean trusted guard checkout"),
            ('--self-test', None, "guard self-test"),
            ('--workspace apps/desktop-ui', None, "actual Vite workspace scan"),
        ]
        for cmd, expected, label in required_verification:
            if cmd not in verified or (expected and expected not in verified):
                problems.append(f"Missing {label} verification")
        order = [verified.find(cmd) for cmd, _, _ in required_verification]
        if min(order) >= 0 and order != sorted(order):
            problems.append("Guard verification does not precede execution in required order")
        if not re.search(r"(?m)^\s+set -euo pipefail\s*$", verified):
            problems.append("Trusted guard verification is not fail-closed (bash -euo pipefail)")
        if source_review is not None and source_review <= audit:
            problems.append("Python baseline source comparison occurs before the guard")

        # Reject checkout/supply-chain caching before the guard, and reject
        # new unreviewed actions (which execute arbitrary action code).
        for i, s in enumerate(steps[:audit]):
            match = re.search(r"(?m)^\s+(?:-\s*)?uses:\s*(\S+)", s)
            if match and not (
                match.group(1) == "actions/checkout@11d5960a326750d5838078e36cf38b85af677262"
                or match.group(1) == "actions/setup-node@49933ea5288caeca8642d1e84afbd3f7d6820020"
            ):
                problems.append(f"Unreviewed action before guard at step {i + 1}")
            if "actions/setup-node@" in s and re.search(r"(?m)^\s+cache:\s*\S+", s):
                problems.append("setup-node restored untrusted npm cache before guard")
        for i, s in enumerate(steps[:audit]):
            # Fail closed on ALL shell execution, including aliases, PowerShell,
            # bash, custom shell scripts and tool commands not in a regex list.
            # Only exact source-SHA confirmation is allowed before the guard.
            if not re.search(r"(?m)^\s+run:\s*", s):
                continue
            is_source_assertion = (
                "name: Confirm exact unmerged source checkout" in s or
                "name: Check exact immutable source" in s
            )
            if is_source_assertion:
                run = re.search(r"(?ms)^\s+run:\s*(.*)$", s)
                body = run.group(1).strip() if run else ""
                if body.startswith("|"):
                    body = body[1:].strip()
                allowed = (
                    'test "$(git rev-parse HEAD)" = "$EXPECTED_SOURCE_SHA"',
                    'test "$(git rev-parse HEAD)" = "$EXACT_SHA"',
                    "if ((git rev-parse HEAD) -ne $env:EXACT_SHA) { throw 'Source mismatch' }",
                )
                if body not in allowed:
                    problems.append(f"Pre-guard source checkout assertion modified/extended at step {i + 1}")
                continue
            commands = [line.strip() for line in s.splitlines() if EXEC.search(line)]
            problems.append(
                f"pre-guard executable step {i + 1} (not an immutable SHA assertion): " +
                (" | ".join(commands[:3]) if commands else "unrecognized shell command")
            )
        if cli is not None and cli <= audit:
            problems.append("Global npm Tauri installation occurs before trusted guard")

    if source_review is not None:
        reviewed = steps[source_review]
        source_checks = [
            "reviewed=" + PYTHON_BASELINE,
            'git fetch --no-tags --depth=1 origin "$reviewed"',
            'test "$(git cat-file -t "$reviewed")" = commit',
            'git diff --quiet "$reviewed" HEAD --',
            "sidecars/transcription/", "scripts/",
            "tests/defragmentation/native-e2e/",
            ":(glob)**/*.py", ":(glob)**/requirements*.txt",
            ":(glob)**/pyproject.toml", ":(glob)**/setup.py",
            ":(glob)**/setup.cfg", ":(glob)**/Pipfile*",
            ":(glob)**/tox.ini",
        ]
        for expected in source_checks:
            if expected not in reviewed:
                problems.append(f"Missing required reviewed Python provenance element: {expected}")
        if not re.search(r"(?m)^\s+set -euo pipefail\s*$", reviewed):
            problems.append("Python provenance comparison is not fail-closed (bash -euo pipefail)")
        if audit is not None:
            for i in range(audit + 1, source_review):
                if re.search(r"(?m)^\s+run:\s*", steps[i]):
                    problems.append(f"Executable shell before Python provenance check: step {i + 1}")
        for i, step in enumerate(steps[:source_review]):
            if ("actions/setup-python@" in step or
                "dtolnay/rust-toolchain@" in step or
                "Swatinem/rust-cache@" in step):
                problems.append(f"Language/runtime setup action before Python provenance check: step {i + 1}")
        if cli is not None and cli <= source_review:
            problems.append("Global npm Tauri CLI occurs before Python provenance comparison")

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
    """Positive fixture and adversarial controls; no source code executed."""
    fixture = """name: fixture
permissions:
  contents: read
jobs:
  build-windows-installer:
    steps:
      - uses: actions/checkout@11d5960a326750d5838078e36cf38b85af677262
        with:
          persist-credentials: false
      - name: Confirm exact unmerged source checkout
        run: test "$(git rev-parse HEAD)" = "$EXPECTED_SOURCE_SHA"
      - uses: actions/setup-node@49933ea5288caeca8642d1e84afbd3f7d6820020
        with:
          node-version: '22'
      - name: Checkout immutable reviewed Agent11 pre-Vite gate
        uses: actions/checkout@11d5960a326750d5838078e36cf38b85af677262
        with:
          ref: 55ce3ac389b7e1fcab99b3a5acc84b526630af72
          persist-credentials: false
          sparse-checkout: |
            apps/desktop-ui/scripts/guard-vite-build-patterns.mjs
      - name: Validate pinned prebuild guard and audited PR source before npm/Vite
        run: |
          set -euo pipefail
          guard=apps/desktop-ui/scripts/guard-vite-build-patterns.mjs
          test "$(git -C "$trusted" rev-parse HEAD)" = '55ce3ac389b7e1fcab99b3a5acc84b526630af72'
          test "$(git -C "$trusted" rev-parse "HEAD:$guard")" = '5c746abd3caf414ba2c80be825d66983312236c6'
          test "$(git -C "$trusted" hash-object --path="$guard" "$guard")" = '5c746abd3caf414ba2c80be825d66983312236c6'
          git -C "$trusted" diff --quiet -- "$guard"
          node .agent11-trusted-guard/apps/desktop-ui/scripts/guard-vite-build-patterns.mjs --self-test
          node .agent11-trusted-guard/apps/desktop-ui/scripts/guard-vite-build-patterns.mjs --workspace apps/desktop-ui
      - name: Verify reviewed Python/runtime/QA inputs before script execution
        run: |
          set -euo pipefail
          reviewed=514ec3ee571c4309e5b7473b8a65ab4fb73f497a
          git fetch --no-tags --depth=1 origin "$reviewed"
          test "$(git cat-file -t "$reviewed")" = commit
          if ! git diff --quiet "$reviewed" HEAD -- \
            sidecars/transcription/ scripts/ tests/defragmentation/native-e2e/ \
            ':(glob)**/*.py' ':(glob)**/requirements*.txt' \
            ':(glob)**/pyproject.toml' ':(glob)**/setup.py' \
            ':(glob)**/setup.cfg' ':(glob)**/Pipfile*' ':(glob)**/tox.ini'; then
            exit 1
          fi
      - name: Install pinned Tauri CLI
        run: npm install --global @tauri-apps/cli@2.11.2
      - name: Build frontend
        run: |
          npm ci
          tauri icon ../../scriptotar.svg
"""
    def expect_bad(code: str, marker: str) -> None:
        problems = inspect(code, "build-windows-installer")
        assert any(marker in x for x in problems), (marker, problems)

    assert inspect(fixture, "build-windows-installer") == [], inspect(fixture, "build-windows-installer")
    expect_bad(fixture.replace("      - name: Checkout immutable reviewed Agent11 pre-Vite gate",
                               "      - name: Poisoned precheck\n"
                               "        run: python scripts/prepare_mcp_tools.py\n"
                               "      - name: Checkout immutable reviewed Agent11 pre-Vite gate"),
               "pre-guard executable")
    expect_bad(fixture.replace("          persist-credentials: false\n", "", 1), "credentials")
    expect_bad(fixture.replace("          ref: " + PIN, "          ref: main"), "not pinned")
    early_npm = fixture.replace("      - name: Install pinned Tauri CLI\n"
                                "        run: npm install --global @tauri-apps/cli@2.11.2\n", "")
    early_npm = early_npm.replace("      - name: Checkout immutable reviewed Agent11 pre-Vite gate",
                                  "      - name: Install pinned Tauri CLI\n"
                                  "        run: npm install --global @tauri-apps/cli@2.11.2\n"
                                  "      - name: Checkout immutable reviewed Agent11 pre-Vite gate")
    expect_bad(early_npm, "before trusted guard")
    expect_bad(fixture.replace("hash-object --path=", "hash-object-removed --path="),
               "actual guard worktree")
    expect_bad(fixture.replace("diff --quiet --", "diff --no-quiet --"),
               "clean trusted guard checkout")
    expect_bad(fixture.replace("HEAD:$guard", "HEAD:some-other-file"),
               "committed guard blob")
    expect_bad(fixture.replace("reviewed=" + PYTHON_BASELINE, "reviewed=main"),
               "reviewed Python provenance")
    expect_bad(fixture.replace("sidecars/transcription/ scripts/", "unrelated/ scripts/"),
               "reviewed Python provenance")
    expect_bad(fixture.replace("      - name: Verify reviewed Python/runtime/QA inputs before script execution",
                               "      - name: Poisoned after guard before Python baseline\n"
                               "        run: python scripts/prepare_mcp_tools.py\n"
                               "      - name: Verify reviewed Python/runtime/QA inputs before script execution"),
               "before Python provenance")
    expect_bad(fixture.replace("node-version: '22'",
                               "node-version: '22'\n          cache: npm"),
               "untrusted npm cache")
    expect_bad(fixture.replace(
        'run: test "$(git rev-parse HEAD)" = "$EXPECTED_SOURCE_SHA"',
        'run: test "$(git rev-parse HEAD)" = "$EXPECTED_SOURCE_SHA"; echo bad'),
        "assertion modified/extended")
    print("PASS: positive fixture and 12 fail-closed adversarial mutations")


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
