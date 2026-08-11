#!/usr/bin/env python3
from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

FULL_SHA = re.compile(r"^[0-9a-fA-F]{40}$")
REPOSITORY_ACTION = re.compile(
    r"^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+(?:/[^@\s]+)?@(?P<ref>[^@\s]+)$"
)
BLOCK_SCALAR = re.compile(
    r"^(?P<indent>\s*)[^#\n][^:\n]*:\s*[>|][+-]?\d?\s*(?:#.*)?$"
)
USES_KEY = re.compile(r"(?<![A-Za-z0-9_-])uses\s*:\s*")


def strip_yaml_comment(line: str) -> str:
    single = False
    double = False
    escaped = False
    for index, char in enumerate(line):
        if escaped:
            escaped = False
            continue
        if double and char == "\\":
            escaped = True
            continue
        if char == "'" and not double:
            single = not single
            continue
        if char == '"' and not single:
            double = not double
            continue
        if char == "#" and not single and not double:
            return line[:index]
    return line


def scalar_after_uses(line: str, start: int) -> str:
    remainder = line[start:].lstrip()
    if not remainder:
        return ""
    if remainder[0] in {"'", '"'}:
        quote = remainder[0]
        escaped = False
        chars: list[str] = []
        for char in remainder[1:]:
            if escaped:
                chars.append(char)
                escaped = False
                continue
            if quote == '"' and char == "\\":
                escaped = True
                chars.append(char)
                continue
            if char == quote:
                return "".join(chars)
            chars.append(char)
        return "".join(chars)
    return re.split(r"[\s,}]", remainder, maxsplit=1)[0]


def iter_uses(path: Path):
    block_indent: int | None = None
    for line_number, raw in enumerate(
        path.read_text(encoding="utf-8").splitlines(), start=1
    ):
        stripped = raw.strip()
        indent = len(raw) - len(raw.lstrip())

        if block_indent is not None:
            if not stripped or indent > block_indent:
                continue
            block_indent = None

        uncommented = strip_yaml_comment(raw)
        scalar = BLOCK_SCALAR.match(uncommented)
        if scalar:
            block_indent = len(scalar.group("indent"))
            continue

        for match in USES_KEY.finditer(uncommented):
            value = scalar_after_uses(uncommented, match.end())
            if value:
                yield line_number, value


def validate_reference(value: str) -> str | None:
    if value.startswith("./"):
        return None

    if value.startswith("docker://"):
        return None

    match = REPOSITORY_ACTION.fullmatch(value)
    if not match:
        return "unsupported or malformed external action reference"

    ref = match.group("ref")
    if not FULL_SHA.fullmatch(ref):
        return "external action is not pinned to a full 40-character commit SHA"
    return None


def check_workflows(root: Path) -> list[str]:
    failures: list[str] = []
    files = sorted({*root.glob("*.yml"), *root.glob("*.yaml")})
    if not files:
        return [f"no workflow files found under {root}"]

    for path in files:
        for line_number, value in iter_uses(path):
            problem = validate_reference(value)
            if problem:
                failures.append(f"{path}:{line_number}: {problem}: {value}")
    return failures


def run_self_test() -> int:
    cases = {
        "pinned": (
            "- uses: actions/checkout@11d5960a326750d5838078e36cf38b85af677262 # v4\n",
            0,
        ),
        "quoted": (
            '- uses : "actions/setup-node@49933ea5288caeca8642d1e84afbd3f7d6820020"\n',
            0,
        ),
        "flow": (
            "- { name: cache, uses: 'Swatinem/rust-cache@6323deb102c322ba6fcbdcafc7e3dddab59af2b6' }\n",
            0,
        ),
        "local": ("- uses: ./github/actions/local\n", 0),
        "tag": ("- uses: actions/checkout@v4\n", 1),
        "branch": ("- uses: owner/action@main\n", 1),
        "expression": ("- uses: owner/action@${{ github.sha }}\n", 1),
        "run-block": (
            "steps:\n"
            "  - name: harmless text\n"
            "    run: |\n"
            "      echo 'uses: owner/action@main'\n"
            "  - uses: actions/checkout@11d5960a326750d5838078e36cf38b85af677262\n",
            0,
        ),
    }

    import tempfile

    for name, (content, expected_failures) in cases.items():
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "case.yml").write_text(content, encoding="utf-8")
            actual = len(check_workflows(root))
            if actual != expected_failures:
                print(
                    f"self-test {name!r} failed: expected {expected_failures} failures, got {actual}",
                    file=sys.stderr,
                )
                return 1
    print("GitHub Action pinning checker self-test passed.")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Require immutable full-SHA pins for external GitHub Actions."
    )
    parser.add_argument(
        "--root",
        type=Path,
        default=Path(".github/workflows"),
        help="workflow directory to inspect",
    )
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()

    if args.self_test:
        return run_self_test()

    failures = check_workflows(args.root)
    if failures:
        print("GitHub Action pinning check failed:", file=sys.stderr)
        for failure in failures:
            print(f"- {failure}", file=sys.stderr)
        return 1

    print(f"GitHub Action pinning check passed for {args.root}.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
