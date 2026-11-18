#!/usr/bin/env python3
"""Semantic SHA-256 of the complete resolved Cargo.lock.

Cargo may rewrite dependency-name array ordering on Windows without changing
resolved crates. Ignore ONLY package.dependencies order; preserve exact lock
format version, package versions, sources, checksums and metadata. This hash
is a reproducibility gate for a genuine installed-package build, NOT proof
that the package's GUI ran.
"""
from __future__ import annotations

import hashlib
import json
from pathlib import Path
import sys
import tomllib


def semantic_sha256(path: Path) -> str:
    cargo_lock = tomllib.loads(path.read_text(encoding="utf-8"))
    for package in cargo_lock.get("package", []):
        if "dependencies" in package:
            package["dependencies"].sort()
    payload = json.dumps(
        cargo_lock, sort_keys=True, separators=(",", ":"), ensure_ascii=True
    ).encode("utf-8")
    return hashlib.sha256(payload).hexdigest()


if __name__ == "__main__":
    if len(sys.argv) != 2:
        raise SystemExit("usage: cargo_lock_fingerprint.py /path/to/Cargo.lock")
    print(semantic_sha256(Path(sys.argv[1])))
