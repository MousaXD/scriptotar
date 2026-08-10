from __future__ import annotations

import argparse
import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

from compliance_enrichment import write_compliance_enrichment
from compliance_finalize import (
    finalize_compliance_bundle,
    prepare_frontend_bundle,
    validate_final_compliance,
)
from distribution_compliance import (
    fetch_pinned_static_ffmpeg,
    write_distribution_compliance_bundle,
)
from runtime_licenses import write_runtime_legal_bundle

ROOT = Path(__file__).resolve().parent
REPO_ROOT = ROOT.parents[1]


def _run_pyinstaller(args: list[str]) -> None:
    subprocess.run([sys.executable, "-m", "PyInstaller", *args], cwd=ROOT, check=True)


def _exe_name(stem: str) -> str:
    return f"{stem}.exe" if os.name == "nt" else stem


def build(output: Path) -> None:
    output = output.resolve()
    if output.exists():
        shutil.rmtree(output)
    output.mkdir(parents=True)

    # Produce the exact production frontend output graph before compliance
    # generation. Vite source maps are the evidence used to distinguish npm
    # packages actually bundled into production JS from build-only tooling.
    prepare_frontend_bundle(REPO_ROOT)

    # Use the same static-ffmpeg 3.0 provider mapping as before, but perform the
    # fetch through Scriptotar's byte-pinned verifier so a floating provider URL
    # cannot silently change the redistributed archive.
    with tempfile.TemporaryDirectory(prefix="scriptotar-ffmpeg-fetch-") as ffmpeg_raw:
        ffmpeg_source, ffprobe_source, ffmpeg_fetch = fetch_pinned_static_ffmpeg(
            Path(ffmpeg_raw)
        )

        with tempfile.TemporaryDirectory(prefix="scriptotar-runtime-build-") as temp_raw:
            temp = Path(temp_raw)
            supervisor_dist = temp / "supervisor-dist"
            engine_dist = temp / "engine-dist"
            ytdlp_dist = temp / "ytdlp-dist"
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
                    "--copy-metadata",
                    "yt-dlp",
                    "--copy-metadata",
                    "faster-whisper",
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

            _run_pyinstaller(
                [
                    "--noconfirm",
                    "--clean",
                    "--onefile",
                    "--name",
                    "scriptotar-ytdlp",
                    "--distpath",
                    str(ytdlp_dist),
                    "--workpath",
                    str(work / "ytdlp"),
                    "--specpath",
                    str(specs),
                    "--collect-all",
                    "yt_dlp",
                    "--collect-all",
                    "curl_cffi",
                    "--copy-metadata",
                    "yt-dlp",
                    str(ROOT / "ytdlp_entry.py"),
                ]
            )

            shutil.copy2(
                supervisor_dist / _exe_name("scriptotar-transcription"),
                output / _exe_name("scriptotar-transcription"),
            )
            shutil.copytree(engine_dist / "scriptotar-engine", output / "engine")
            shutil.copy2(
                ytdlp_dist / _exe_name("scriptotar-ytdlp"),
                output / _exe_name("scriptotar-ytdlp"),
            )

        shutil.copy2(ROOT / "sidecar.py", output / "sidecar.py")
        ffmpeg_dir = output / "ffmpeg"
        ffmpeg_dir.mkdir()
        shutil.copy2(ffmpeg_source, ffmpeg_dir / _exe_name("ffmpeg"))
        shutil.copy2(ffprobe_source, ffmpeg_dir / _exe_name("ffprobe"))

        versions = (
            "Scriptotar packaged transcription runtime\n"
            f"Python={sys.version.split()[0]}\n"
            "PyInstaller=6.21.0\n"
            "static-ffmpeg=3.0 provider mapping (archive byte-pinned by Scriptotar)\n"
            "faster-whisper=1.2.1\n"
            "yt-dlp=2026.7.4 (dedicated scriptotar-ytdlp executable)\n"
        )
        (output / "RUNTIME-VERSIONS.txt").write_text(versions, encoding="utf-8")

        # Generate the exact Python/runtime legal inventory first, then the
        # transitive/native inventories, enrich them with versioned native
        # notices and Vite output evidence, preserve both frontend views, and
        # validate the final package evidence before returning success.
        write_runtime_legal_bundle(output, ffmpeg_source, REPO_ROOT)
        write_distribution_compliance_bundle(output, REPO_ROOT, ffmpeg_fetch)
        write_compliance_enrichment(output, REPO_ROOT)
        finalize_compliance_bundle(output, REPO_ROOT)
        validate_final_compliance(output, REPO_ROOT)


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Build the self-contained Scriptotar transcription runtime."
    )
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    build(args.output)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
