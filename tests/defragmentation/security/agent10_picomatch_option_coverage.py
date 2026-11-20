#!/usr/bin/env python3
"""Independent Agent10 *bounded* picomatch option-boundary adversarial test.

Run only after exact SHA + blob checks of experimental upstream PR264 + Agent11
normalizer, in disposable npm-ci node_modules. No production manifest changes.
All pathological patterns run in killable Node child processes; no user input.
"""
from __future__ import annotations
import argparse
import json
import subprocess
import sys
from pathlib import Path

# All trailing slashes are outside a wildcard segment: expected no match.
CASES = (
    ("repeated-class alternative", "+([ab]|b)", "b" * 36 + "/"),
    ("wrapped repeated class", "+(@(+([ab])))", "a" * 36 + "/"),
    ("repeated literal alternatives", "+(a|b|ab)", "ab" * 22 + "/"),
)
JS = r"""
const pm = require('picomatch');
const [pattern, input, rawOptions, rawExpected] = process.argv.slice(1);
const actual = pm(pattern, JSON.parse(rawOptions))(input);
const expected = rawExpected === 'true';
console.log(JSON.stringify({actual, expected}));
if (actual !== expected) process.exitCode = 3;
"""


def probe(root: Path, pattern: str, input_value: str,
          options: dict[str, bool], expected: bool) -> tuple[str, str]:
    try:
        run = subprocess.run(
            ["node", "-e", JS, pattern, input_value, json.dumps(options),
             str(expected).lower()],
            cwd=root, capture_output=True, text=True, timeout=1.2, check=False,
        )
    except subprocess.TimeoutExpired:
        return "TIMEOUT", "bounded 1.2-second child deadline"
    try:
        result = json.loads(run.stdout.strip())
    except (ValueError, TypeError):
        return "ERROR", (run.stderr or run.stdout or "missing result").strip()[:150]
    if run.returncode != 0 or result.get("actual") != expected:
        return "MISMATCH", f"got={result.get('actual')} expected={expected}"
    return "PASS", "matched expected value"


def main() -> int:
    p = argparse.ArgumentParser()
    p.add_argument("--frontend", type=Path, default=Path("apps/desktop-ui"))
    args = p.parse_args()
    root = args.frontend.resolve(strict=True)
    if not (root / "node_modules/picomatch/lib/agent11_class_normalizer.cjs").is_file():
        print("FAIL CLOSED: exact experimental candidate not installed")
        return 2

    failures = 0
    # Control: legal extglob matches and the normalizer is safe in its default mode.
    for options, label in (({}, "default"), ({"capture": True}, "capture=true")):
        status, info = probe(root, "+(a|b)", "abab", options, True)
        print(f"CONTROL {label}: {status} {info}", flush=True)
        if status != "PASS":
            failures += 1

    for label, pattern, source in CASES:
        for options, option_label in (({}, "default"), ({"capture": True}, "capture=true")):
            status, info = probe(root, pattern, source, options, False)
            print(f"{label} {option_label}: {status} {info}", flush=True)
            if status != "PASS":
                failures += 1

    print(f"Agent10 option-coverage negative control failures={failures}", flush=True)
    if failures:
        print("REJECT: at least one legitimate matcher option escapes reviewed "
              "bounded security/compatibility contract", file=sys.stderr, flush=True)
    return 1 if failures else 0


if __name__ == "__main__":
    raise SystemExit(main())
