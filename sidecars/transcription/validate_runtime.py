from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path


def _exe_name(stem: str) -> str:
    return f"{stem}.exe" if os.name == "nt" else stem


def _runtime_env(root: Path) -> dict[str, str]:
    env = os.environ.copy()
    engine = root / "engine" / _exe_name("scriptotar-engine")
    env["SCRIPTOTAR_SIDECAR_ENGINE_EXECUTABLE"] = str(engine)
    env["PATH"] = str(root / "ffmpeg") + os.pathsep + env.get("PATH", "")
    env["PYTHONUNBUFFERED"] = "1"
    env.setdefault("HF_HOME", str(root / "model-cache-smoke"))
    return env


def _validate_engine(root: Path, env: dict[str, str]) -> None:
    engine = root / "engine" / _exe_name("scriptotar-engine")
    if not engine.is_file():
        raise RuntimeError(f"packaged engine executable is missing: {engine}")
    completed = subprocess.run(
        [str(engine), "--self-test"],
        env=env,
        check=True,
        capture_output=True,
        text=True,
        timeout=45,
    )
    report = json.loads(completed.stdout.strip())
    if report.get("ok") is not True:
        raise RuntimeError(f"engine self-test did not report success: {report}")
    for tool in ("ffmpeg", "ffprobe"):
        if not report.get(tool, {}).get("path"):
            raise RuntimeError(f"engine self-test did not resolve {tool}")


def _validate_supervisor(root: Path, env: dict[str, str]) -> None:
    supervisor = root / _exe_name("scriptotar-transcription")
    marker = root / "sidecar.py"
    if not supervisor.is_file():
        raise RuntimeError(f"packaged supervisor executable is missing: {supervisor}")
    if not marker.is_file():
        raise RuntimeError(f"packaged sidecar marker is missing: {marker}")

    payload = (
        '{"protocol":1,"type":"ping","request_id":"runtime-smoke"}\n'
        '{"protocol":1,"type":"shutdown","request_id":"runtime-shutdown"}\n'
    )
    completed = subprocess.run(
        [str(supervisor), str(marker)],
        input=payload,
        env=env,
        check=True,
        capture_output=True,
        text=True,
        timeout=30,
    )
    events = [json.loads(line) for line in completed.stdout.splitlines() if line.strip()]
    types = [event.get("type") for event in events]
    if not types or types[0] != "ready":
        raise RuntimeError(f"packaged supervisor did not start with ready: {events}")
    if "pong" not in types or "shutdown" not in types:
        raise RuntimeError(f"packaged supervisor protocol smoke failed: {events}")
    if events[0].get("protocol") != 1:
        raise RuntimeError(f"packaged supervisor protocol version mismatch: {events[0]}")


def validate(root: Path) -> None:
    root = root.resolve()
    if not root.is_dir():
        raise RuntimeError(f"runtime directory does not exist: {root}")
    versions = root / "RUNTIME-VERSIONS.txt"
    if not versions.is_file():
        raise RuntimeError(f"runtime provenance file is missing: {versions}")
    with tempfile.TemporaryDirectory(prefix="scriptotar-runtime-smoke-") as model_cache:
        env = _runtime_env(root)
        env["HF_HOME"] = model_cache
        _validate_engine(root, env)
        _validate_supervisor(root, env)


def main() -> int:
    parser = argparse.ArgumentParser(description="Validate a packaged Scriptotar transcription runtime.")
    parser.add_argument("runtime", type=Path)
    args = parser.parse_args()
    validate(args.runtime)
    print(f"validated packaged runtime: {args.runtime.resolve()}")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as exc:
        print(f"runtime validation failed: {exc}", file=sys.stderr)
        raise