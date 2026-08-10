#!/usr/bin/env python3
from __future__ import annotations

import re
import subprocess
from pathlib import PurePosixPath

FORBIDDEN_EXACT = {
    ".env",
}
FORBIDDEN_SUFFIXES = {
    ".sqlite3",
    ".pem",
    ".p12",
    ".pfx",
    ".AppImage",
    ".flatpak",
    ".deb",
}
FORBIDDEN_PARTS = {
    "node_modules",
    "target",
    ".venv",
    "venv",
    "dist",
    "__pycache__",
}
ALLOWLISTED_ENV_FILES = {".env.example"}

SECRET_PATTERNS = {
    "private key block": re.compile(rb"-----BEGIN (?:RSA |EC |OPENSSH )?PRIVATE KEY-----"),
    "AWS access key": re.compile(rb"\bAKIA[0-9A-Z]{16}\b"),
    "GitHub token": re.compile(rb"\bgh[pousr]_[A-Za-z0-9]{30,}\b"),
    "OpenAI-style secret": re.compile(rb"\bsk-[A-Za-z0-9_-]{20,}\b"),
    "Google API key": re.compile(rb"\bAIza[0-9A-Za-z_-]{35}\b"),
}


def tracked_files() -> list[str]:
    output = subprocess.check_output(["git", "ls-files", "-z"])
    return [entry.decode("utf-8") for entry in output.split(b"\0") if entry]


def forbidden_path(path: str) -> str | None:
    pure = PurePosixPath(path)
    name = pure.name
    if name in ALLOWLISTED_ENV_FILES:
        return None
    if name in FORBIDDEN_EXACT or name.startswith(".env."):
        return "environment/secrets file"
    if any(part in FORBIDDEN_PARTS for part in pure.parts):
        return "generated dependency/build directory"
    if any(name.endswith(suffix) for suffix in FORBIDDEN_SUFFIXES):
        return "generated/private artifact"
    return None


def main() -> int:
    failures: list[str] = []
    for path in tracked_files():
        reason = forbidden_path(path)
        if reason:
            failures.append(f"tracked {reason}: {path}")
            continue
        try:
            data = open(path, "rb").read()
        except OSError as exc:
            failures.append(f"could not inspect tracked file {path}: {exc}")
            continue
        if b"\0" in data:
            continue
        for label, pattern in SECRET_PATTERNS.items():
            if pattern.search(data):
                failures.append(f"possible {label} in tracked file: {path}")

    if failures:
        print("Repository hygiene check failed:")
        for failure in failures:
            print(f"- {failure}")
        return 1

    print("Repository hygiene check passed: no forbidden tracked artifacts or high-confidence secret patterns found.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
