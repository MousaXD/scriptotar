from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path


REQUIRED_PYTHON_COMPONENTS = {
    "faster-whisper",
    "ctranslate2",
    "yt-dlp",
    "curl-cffi",
    "huggingface-hub",
    "tokenizers",
    "av",
}
REQUIRED_LEGAL_FILES = {
    "legal/THIRD_PARTY_NOTICES.md",
    "legal/NOTICE",
    "legal/SCRIPTOTAR-APACHE-2.0.txt",
    "legal/FFMPEG-GPL-3.0.txt",
}


def _exe_name(stem: str) -> str:
    return f"{stem}.exe" if os.name == "nt" else stem


def _runtime_env(root: Path) -> dict[str, str]:
    env = os.environ.copy()
    engine = root / "engine" / _exe_name("scriptotar-engine")
    env["SCRIPTOTAR_SIDECAR_ENGINE_EXECUTABLE"] = str(engine)
    env["SCRIPTOTAR_YTDLP_EXECUTABLE"] = str(root / _exe_name("scriptotar-ytdlp"))
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


def _validate_ytdlp(root: Path, env: dict[str, str]) -> None:
    ytdlp = root / _exe_name("scriptotar-ytdlp")
    if not ytdlp.is_file():
        raise RuntimeError(f"packaged yt-dlp executable is missing: {ytdlp}")
    completed = subprocess.run(
        [str(ytdlp), "--version"],
        env=env,
        check=True,
        capture_output=True,
        text=True,
        timeout=30,
    )
    version = completed.stdout.strip()
    if not version:
        raise RuntimeError("packaged yt-dlp executable returned no version")


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


def _require_relative_file(root: Path, relative: str) -> Path:
    candidate = (root / relative).resolve()
    try:
        candidate.relative_to(root.resolve())
    except ValueError as exc:
        raise RuntimeError(f"license manifest path escapes runtime root: {relative}") from exc
    if not candidate.is_file() or candidate.stat().st_size == 0:
        raise RuntimeError(f"required packaged legal file is missing or empty: {relative}")
    return candidate


def _validate_ffmpeg_record(root: Path, ffmpeg: dict[str, object], label: str) -> None:
    if ffmpeg.get("license") != "GPL-3.0-or-later":
        raise RuntimeError(f"unexpected {label} license: {ffmpeg.get('license')}")
    configuration = str(ffmpeg.get("configuration") or "")
    if "--enable-gpl" not in configuration or "--enable-version3" not in configuration:
        raise RuntimeError(f"{label} GPLv3 build switches are not present in manifest: {configuration}")
    if "--enable-nonfree" in configuration:
        raise RuntimeError(f"{label} manifest contains --enable-nonfree")
    ffmpeg_license = ffmpeg.get("license_file")
    if not isinstance(ffmpeg_license, str):
        raise RuntimeError(f"runtime license manifest does not identify the {label} license text")
    _require_relative_file(root, ffmpeg_license)


def _validate_pyav_ffmpeg(root: Path, record: object) -> None:
    if not isinstance(record, dict):
        raise RuntimeError("runtime license manifest is missing the PyAV-bundled FFmpeg inventory")
    if record.get("license") != "GPL-3.0-or-later":
        raise RuntimeError(f"unexpected PyAV FFmpeg license: {record.get('license')}")
    license_file = record.get("license_file")
    if not isinstance(license_file, str):
        raise RuntimeError("PyAV FFmpeg inventory does not identify its GPL license text")
    _require_relative_file(root, license_file)

    groups = record.get("library_groups")
    if not isinstance(groups, list) or not groups:
        raise RuntimeError("PyAV FFmpeg inventory contains no library groups")
    for group in groups:
        if not isinstance(group, dict):
            raise RuntimeError("PyAV FFmpeg inventory contains a malformed library group")
        configuration = str(group.get("configuration") or "")
        reported = str(group.get("reported_license") or "")
        inferred = str(group.get("inferred_license") or "")
        if inferred != "GPL-3.0-or-later":
            raise RuntimeError(f"unexpected inferred PyAV FFmpeg license: {inferred}")
        if reported.lower() != "gpl version 3 or later":
            raise RuntimeError(f"unexpected reported PyAV FFmpeg license: {reported}")
        if "--enable-gpl" not in configuration or "--enable-version3" not in configuration:
            raise RuntimeError(f"PyAV FFmpeg GPLv3 build switches are missing: {configuration}")
        if "--enable-nonfree" in configuration:
            raise RuntimeError("PyAV FFmpeg configuration contains --enable-nonfree")

    hashes = record.get("native_library_sha256")
    if not isinstance(hashes, dict) or not hashes:
        raise RuntimeError("PyAV FFmpeg inventory contains no native library hashes")
    for name, digest in hashes.items():
        if not isinstance(name, str) or not isinstance(digest, str) or len(digest) != 64:
            raise RuntimeError(f"malformed PyAV native library hash entry: {name}={digest}")


def _validate_licenses(root: Path) -> None:
    manifest_path = root / "RUNTIME-LICENSES.json"
    if not manifest_path.is_file():
        raise RuntimeError(f"runtime license manifest is missing: {manifest_path}")
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    if manifest.get("schema") != 1:
        raise RuntimeError(f"unsupported runtime license manifest schema: {manifest.get('schema')}")

    for relative in REQUIRED_LEGAL_FILES:
        _require_relative_file(root, relative)

    python_runtime = manifest.get("python_runtime") or {}
    python_license = python_runtime.get("license_file")
    if not isinstance(python_license, str):
        raise RuntimeError("runtime license manifest does not identify the embedded Python license")
    _require_relative_file(root, python_license)

    ffmpeg = manifest.get("ffmpeg")
    if not isinstance(ffmpeg, dict):
        raise RuntimeError("runtime license manifest is missing bundled FFmpeg/ffprobe information")
    _validate_ffmpeg_record(root, ffmpeg, "bundled FFmpeg/ffprobe")
    _validate_pyav_ffmpeg(root, manifest.get("pyav_ffmpeg"))

    components = manifest.get("python_components")
    if not isinstance(components, list):
        raise RuntimeError("runtime license manifest python_components is not a list")
    names = {
        str(component.get("name") or "").lower().replace("_", "-").replace(".", "-")
        for component in components
        if isinstance(component, dict)
    }
    missing = REQUIRED_PYTHON_COMPONENTS - names
    if missing:
        raise RuntimeError(f"runtime license manifest is missing bundled Python components: {sorted(missing)}")

    for component in components:
        if not isinstance(component, dict):
            raise RuntimeError("runtime license manifest contains a malformed Python component entry")
        name = str(component.get("name") or "unknown")
        license_files = component.get("license_files") or []
        declaration = component.get("license_declared")
        if not declaration and not license_files:
            raise RuntimeError(f"no license metadata or license file was found for bundled Python component: {name}")
        if not isinstance(license_files, list):
            raise RuntimeError(f"license_files is malformed for bundled Python component: {name}")
        for relative in license_files:
            if not isinstance(relative, str):
                raise RuntimeError(f"non-string license path for bundled Python component: {name}")
            _require_relative_file(root, relative)


def validate(root: Path) -> None:
    root = root.resolve()
    if not root.is_dir():
        raise RuntimeError(f"runtime directory does not exist: {root}")
    versions = root / "RUNTIME-VERSIONS.txt"
    if not versions.is_file():
        raise RuntimeError(f"runtime provenance file is missing: {versions}")
    _validate_licenses(root)
    with tempfile.TemporaryDirectory(prefix="scriptotar-runtime-smoke-") as model_cache:
        env = _runtime_env(root)
        env["HF_HOME"] = model_cache
        _validate_engine(root, env)
        _validate_ytdlp(root, env)
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
