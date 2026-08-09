from __future__ import annotations

import argparse
import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

from static_ffmpeg import run as static_ffmpeg_run

ROOT = Path(__file__).resolve().parent


def _run_pyinstaller(args: list[str]) -> None:
    subprocess.run([sys.executable, "-m", "PyInstaller", *args], cwd=ROOT, check=True)


def _exe_name(stem: str) -> str:
    return f"{stem}.exe" if os.name == "nt" else stem


def build(output: Path) -> None:
    output = output.resolve()
    if output.exists():
        shutil.rmtree(output)
    output.mkdir(parents=True)

    ffmpeg_source, ffprobe_source = map(
        Path, static_ffmpeg_run.get_or_fetch_platform_executables_else_raise()
    )

    with tempfile.TemporaryDirectory(prefix="scriptotar-runtime-build-") as temp_raw:
        temp = Path(temp_raw)
        supervisor_dist = temp / "supervisor-dist"
        engine_dist = temp / "engine-dist"
        work = temp / "work"
        specs = temp / "specs"

        _run_pyinstaller(
            [
                "--noconfirm",
                "--clean",
                "--onefile",
                "--name",
                "scriptotar-transcription",
                "--distpath",
                str(supervisor_dist),
                "--workpath",
                str(work / "supervisor"),
                "--specpath",
                str(specs),
                str(ROOT / "sidecar.py"),
            ]
        )

        _run_pyinstaller(
            [
                "--noconfirm",
                "--clean",
                "--onedir",
                "--name",
                "scriptotar-engine",
                "--distpath",
                str(engine_dist),
                "--workpath",
                str(work / "engine"),
                "--specpath",
                str(specs),
                "--collect-all",
                "faster_whisper",
                "--collect-all",
                "yt_dlp",
                "--collect-all",
                "ctranslate2",
                "--collect-all",
                "tokenizers",
                "--collect-all",
                "huggingface_hub",
                "--collect-all",
                "av",
                "--collect-all",
                "curl_cffi",
                str(ROOT / "engine_entry.py"),
            ]
        )

        shutil.copy2(
            supervisor_dist / _exe_name("scriptotar-transcription"),
            output / _exe_name("scriptotar-transcription"),
        )
        shutil.copytree(engine_dist / "scriptotar-engine", output / "engine")

    shutil.copy2(ROOT / "sidecar.py", output / "sidecar.py")
    ffmpeg_dir = output / "ffmpeg"
    ffmpeg_dir.mkdir()
    shutil.copy2(ffmpeg_source, ffmpeg_dir / _exe_name("ffmpeg"))
    shutil.copy2(ffprobe_source, ffmpeg_dir / _exe_name("ffprobe"))

    versions = (
        "Scriptotar packaged transcription runtime\n"
        f"Python={sys.version.split()[0]}\n"
        "PyInstaller=6.21.0\n"
        "static-ffmpeg=3.0 (FFmpeg/ffprobe 8.0)\n"
        "faster-whisper=1.2.1\n"
        "yt-dlp=2026.7.4\n"
    )
    (output / "RUNTIME-VERSIONS.txt").write_text(versions, encoding="utf-8")


def main() -> int:
    parser = argparse.ArgumentParser(description="Build the self-contained Scriptotar transcription runtime.")
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    build(args.output)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
