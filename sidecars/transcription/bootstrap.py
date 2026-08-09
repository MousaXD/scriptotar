#!/usr/bin/env python3
from __future__ import annotations

import argparse
import subprocess
import sys
import venv
from pathlib import Path


def main() -> int:
    parser = argparse.ArgumentParser(description="Create or repair Scriptotar's transcription sidecar environment.")
    parser.add_argument("--venv", required=True, help="Target virtual environment directory")
    args = parser.parse_args()

    venv_dir = Path(args.venv).expanduser().resolve()
    requirements = Path(__file__).with_name("requirements-engine.txt")
    venv.EnvBuilder(with_pip=True, clear=False, upgrade_deps=False).create(venv_dir)
    python = venv_dir / ("Scripts/python.exe" if sys.platform == "win32" else "bin/python")
    subprocess.run([str(python), "-m", "pip", "install", "--upgrade", "pip", "wheel"], check=True)
    subprocess.run([str(python), "-m", "pip", "install", "--upgrade", "-r", str(requirements)], check=True)
    subprocess.run([str(python), "-m", "pip", "check"], check=True)
    subprocess.run(
        [
            str(python),
            "-c",
            "import faster_whisper, yt_dlp; print(faster_whisper.__version__); print(yt_dlp.version.__version__)",
        ],
        check=True,
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
