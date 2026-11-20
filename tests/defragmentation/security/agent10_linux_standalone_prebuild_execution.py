#!/usr/bin/env python3
"""Agent10 read-only, fail-closed Linux standalone .deb prebuild trust audit.

This audit is a negative test of current integration SHA 50ed956..., NOT a fix
or independent native-installed performance acceptance. Its target source is
checked out by immutable SHA as data only; never execute target PR scripts.
"""
from __future__ import annotations
import argparse
import re
from pathlib import Path

REVIEWED_GUARD = "55ce3ac389b7e1fcab99b3a5acc84b526630af72"
REVIEWED_BLOB = "5c746abd3caf414ba2c80be825d66983312236c6"
SOURCE = ".github/workflows/linux-tauri.yml"
JOB = "build-linux-deb"
STEPS = re.compile(r"(?m)^ {6}- (?:name|uses): .+$")


def inspect(source: str) -> list[str]:
    errors: list[str] = []
    match = re.search(r"(?m)^  build-linux-deb:\s*$", source)
    if not match:
        return ["Missing canonical Linux standalone package job"]
    rest = source[match.end():]
    other = re.search(r"(?m)^  [A-Za-z][A-Za-z_0-9-]*:\s*$", rest)
    body = rest[:other.start()] if other else rest
    spans = list(STEPS.finditer(body))
    if not spans:
        return ["No declared package workflow steps"]
    chunks = [body[m.start():spans[i+1].start() if i+1 < len(spans) else len(body)]
              for i, m in enumerate(spans)]

    def unique(name: str) -> int | None:
        matches = [i for i, step in enumerate(chunks)
                   if re.search(rf"(?m)^      - name: {re.escape(name)}\s*$", step)]
        if len(matches) != 1:
            errors.append(f"missing or duplicate {name}: {len(matches)}")
            return None
        return matches[0]

    checkout = unique("Checkout immutable reviewed Agent11 pre-Vite gate")
    guard = unique("Validate pinned prebuild guard and audited PR source before npm/Vite")
    # Require exact checkout input and worktree authenticity at the executing path.
    checkouts = [(i, x) for i,x in enumerate(chunks)
                 if re.search(r"(?m)^\s+(?:-\s*)?uses: actions/checkout@", x)]
    if len(checkouts) < 2:
        errors.append("Missing source and trusted immutable checkout")
    for i, step in checkouts:
        if not re.search(r"(?m)^\s+persist-credentials:\s*false\s*$", step):
            errors.append(f"Checkout {i+1} persists a repository credential")

    if checkout is not None:
        item = chunks[checkout]
        if not re.search(rf"(?m)^\s+ref:\s*{REVIEWED_GUARD}\s*$", item):
            errors.append("Trusted checkout is not source-SHA pinned")
        if "apps/desktop-ui/scripts/guard-vite-build-patterns.mjs" not in item:
            errors.append("Trusted source guard executable missing from sparse checkout")

    if guard is not None:
        script = chunks[guard]
        for phrase, label in (
            (REVIEWED_GUARD, "reviewed guard commit"),
            (REVIEWED_BLOB, "reviewed guard blob identity"),
            ('hash-object --path="$guard" "$guard"', "actual working guard blob validation"),
            ('diff --quiet -- "$guard"', "clean trusted guard working tree"),
            ("--self-test", "real guard self-test"),
            ("--workspace apps/desktop-ui", "source workspace scanner"),
            ("set -euo pipefail", "fail-closed error handling"),
        ):
            if phrase not in script:
                errors.append("Missing "+label)
        if (checkout is not None and checkout >= guard):
            errors.append("Trusted source guard runs before immutable checkout")
        if "hash-object" in script and "--self-test" in script:
            if script.index("hash-object") >= script.index("--self-test"):
                errors.append("Guard working file authentication occurs after running it")
        # No first-party script, external installer, arbitrary shell or build step
        # prior to source authentication; only source-SHA comparison is exempt.
        for i, step in enumerate(chunks[:guard]):
            if "actions/setup-node@" in step and re.search(r"(?m)^\s+cache:\s*\S+", step):
                errors.append(f"Unreviewed npm cache enabled before guard at {i+1}")
            if re.search(r"(?m)^\s+run:\s*", step):
                if "name: Confirm exact unmerged source checkout" in step:
                    if not re.search(r"(?s)run:\s*test \"\$\(git rev-parse HEAD\)\" = \"\$EXPECTED_SOURCE_SHA\"\s*$", step.strip()):
                        errors.append(f"Pre-guard SHA-only source assertion broadened at {i+1}")
                else:
                    errors.append(f"Executable/installer before trusted source guard: step {i+1}")

    if not re.search(r"(?m)^permissions:\s*\n\s+contents:\s+read\s*$", source):
        errors.append("Read-only GITHUB_TOKEN policy missing")
    return errors


def self_test() -> None:
    good = """name: Linux standalone source fixture
permissions:
  contents: read
jobs:
  build-linux-deb:
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
          set -euo pipefail
          guard=apps/desktop-ui/scripts/guard-vite-build-patterns.mjs
          test "$(git -C "$trusted" rev-parse HEAD)" = '55ce3ac389b7e1fcab99b3a5acc84b526630af72'
          test "$(git -C "$trusted" hash-object --path="$guard" "$guard")" = '5c746abd3caf414ba2c80be825d66983312236c6'
          git -C "$trusted" diff --quiet -- "$guard"
          node .agent11-trusted-guard/apps/desktop-ui/scripts/guard-vite-build-patterns.mjs --self-test
          node .agent11-trusted-guard/apps/desktop-ui/scripts/guard-vite-build-patterns.mjs --workspace apps/desktop-ui
      - name: Install checked source runtime
        run: python scripts/prepare_mcp_tools.py
"""
    assert inspect(good) == [], inspect(good)
    cases = [
        (good.replace("      - name: Checkout immutable reviewed",
                      "      - name: Early code\n        run: python scripts/prepare_mcp_tools.py\n"
                      "      - name: Checkout immutable reviewed"),
         "Executable/installer before trusted source guard"),
        (good.replace('hash-object --path=', 'hash-object removed --path='),
         "actual working guard blob validation"),
        (good.replace("          persist-credentials: false\n", "", 1),
         "persists a repository credential"),
        (good.replace("          ref: " + REVIEWED_GUARD, "          ref: main"),
         "not source-SHA pinned"),
        (good.replace("diff --quiet --", "diff --not-quiet --"),
         "clean trusted guard working tree"),
        (good.replace("set -euo pipefail", "set +e"), "fail-closed error handling"),
        (good.replace("          node .agent11-trusted-guard/apps/desktop-ui/scripts/guard-vite-build-patterns.mjs --self-test\n", ""),
         "real guard self-test"),
    ]
    for mutated, expected in cases:
        violations = inspect(mutated)
        assert any(expected in item for item in violations), (expected, violations)
    print(f"PASS: clean source fixture + {len(cases)} independent adversarial changes")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--source", type=Path,
                        default=Path(".agent10-reviewed-integration") / SOURCE)
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    if args.self_test:
        self_test()
        return 0
    if not args.source.is_file():
        print(f"FAIL CLOSED: no verified target workflow at {args.source}")
        return 2
    findings = inspect(args.source.read_text(encoding="utf-8"))
    if findings:
        print(f"REJECT: merged integration standalone Linux package has {len(findings)} P0 source-trust findings")
        for f in findings:
            print("  - " + f)
        return 1
    print("SCOPED PASS: reviewed Linux standalone source ordering and guard-file integrity only")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
