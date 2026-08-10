from __future__ import annotations

import argparse
import hashlib
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
    "legal/FFMPEG-LGPL-3.0.txt",
}
REQUIRED_COMPLIANCE_FILES = {
    "RUNTIME-PROVENANCE.json",
    "NATIVE-COMPONENTS.json",
    "RUST-LICENSES.json",
    "FRONTEND-LICENSES.json",
}
PROHIBITED_LICENSE_MARKERS = (
    "AGPL-",
    "SSPL-",
    "BUSL-",
    "Business-Source-License",
    "Commons-Clause",
    "LicenseRef-",
    "UNLICENSED",
)


def _exe_name(stem: str) -> str:
    return f"{stem}.exe" if os.name == "nt" else stem


def _sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _runtime_env(root: Path) -> dict[str, str]:
    env = os.environ.copy()
    env["SCRIPTOTAR_SIDECAR_ENGINE_EXECUTABLE"] = str(
        root / "engine" / _exe_name("scriptotar-engine")
    )
    env["SCRIPTOTAR_YTDLP_EXECUTABLE"] = str(root / _exe_name("scriptotar-ytdlp"))
    env["PATH"] = str(root / "ffmpeg") + os.pathsep + env.get("PATH", "")
    env["PYTHONUNBUFFERED"] = "1"
    env.setdefault("HF_HOME", str(root / "model-cache-smoke"))
    return env


def _require_relative_file(root: Path, relative: str) -> Path:
    candidate = (root / relative).resolve()
    try:
        candidate.relative_to(root.resolve())
    except ValueError as exc:
        raise RuntimeError(f"manifest path escapes runtime root: {relative}") from exc
    if not candidate.is_file() or candidate.stat().st_size == 0:
        raise RuntimeError(f"required packaged file is missing or empty: {relative}")
    return candidate


def _load_manifest(root: Path, relative: str) -> dict:
    path = _require_relative_file(root, relative)
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict) or value.get("schema") != 1:
        raise RuntimeError(f"unsupported or malformed compliance manifest: {relative}")
    return value


def _run_ffmpeg_version(path: Path) -> tuple[str, str]:
    completed = subprocess.run(
        [str(path), "-version"],
        check=True,
        capture_output=True,
        text=True,
        timeout=30,
    )
    lines = completed.stdout.splitlines()
    version_line = lines[0].strip() if lines else ""
    configuration = next(
        (line.split("configuration:", 1)[1].strip() for line in lines if "configuration:" in line),
        "",
    )
    return version_line, configuration


def _validate_standalone_ffmpeg(root: Path, record: object) -> None:
    if not isinstance(record, dict):
        raise RuntimeError("runtime license manifest is missing standalone FFmpeg/ffprobe information")
    if record.get("license") != "GPL-3.0-or-later":
        raise RuntimeError(f"unexpected standalone FFmpeg license: {record.get('license')}")
    configuration = str(record.get("configuration") or "")
    if "--enable-gpl" not in configuration or "--enable-version3" not in configuration:
        raise RuntimeError(f"standalone FFmpeg GPLv3 switches are missing: {configuration}")
    if "--enable-nonfree" in configuration:
        raise RuntimeError("standalone FFmpeg configuration contains --enable-nonfree")
    license_file = record.get("license_file")
    if not isinstance(license_file, str):
        raise RuntimeError("standalone FFmpeg inventory does not identify its GPL license text")
    _require_relative_file(root, license_file)

    binaries = record.get("binary_sha256")
    if not isinstance(binaries, dict):
        raise RuntimeError("standalone FFmpeg inventory contains no binary hashes")
    ffmpeg = root / "ffmpeg" / _exe_name("ffmpeg")
    ffprobe = root / "ffmpeg" / _exe_name("ffprobe")
    for name, path in (("ffmpeg", ffmpeg), ("ffprobe", ffprobe)):
        expected = binaries.get(name)
        if not isinstance(expected, str) or len(expected) != 64:
            raise RuntimeError(f"standalone FFmpeg inventory has no valid {name} hash")
        actual = _sha256(path)
        if actual != expected:
            raise RuntimeError(f"installed {name} hash drift: expected {expected}, got {actual}")

    actual_version, actual_configuration = _run_ffmpeg_version(ffmpeg)
    if actual_version != str(record.get("version_line") or ""):
        raise RuntimeError(
            f"installed FFmpeg version drift: expected {record.get('version_line')!r}, got {actual_version!r}"
        )
    if actual_configuration != configuration:
        raise RuntimeError("installed FFmpeg configure flags differ from the generated manifest")


def _validate_pyav_ffmpeg(root: Path, record: object) -> None:
    if not isinstance(record, dict):
        raise RuntimeError("runtime license manifest is missing the PyAV-bundled FFmpeg inventory")
    if record.get("license") != "LGPL-3.0-or-later":
        raise RuntimeError(f"unexpected PyAV FFmpeg core license: {record.get('license')}")
    license_file = record.get("license_file")
    if not isinstance(license_file, str):
        raise RuntimeError("PyAV FFmpeg inventory does not identify its LGPL license text")
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
        if inferred != "LGPL-3.0-or-later":
            raise RuntimeError(f"unexpected inferred PyAV FFmpeg core license: {inferred}")
        if reported.lower() != "lgpl version 3 or later":
            raise RuntimeError(f"unexpected reported PyAV FFmpeg core license: {reported}")
        if "--enable-version3" not in configuration or "--enable-gpl" in configuration:
            raise RuntimeError(f"PyAV FFmpeg core LGPLv3 switches are unexpected: {configuration}")
        if "--enable-nonfree" in configuration:
            raise RuntimeError("PyAV FFmpeg configuration contains --enable-nonfree")


def _validate_python_licenses(root: Path) -> dict:
    path = root / "RUNTIME-LICENSES.json"
    if not path.is_file():
        raise RuntimeError(f"runtime license manifest is missing: {path}")
    manifest = json.loads(path.read_text(encoding="utf-8"))
    if manifest.get("schema") != 1:
        raise RuntimeError(f"unsupported runtime license manifest schema: {manifest.get('schema')}")
    for relative in REQUIRED_LEGAL_FILES:
        _require_relative_file(root, relative)

    python_runtime = manifest.get("python_runtime") or {}
    python_license = python_runtime.get("license_file")
    if not isinstance(python_license, str):
        raise RuntimeError("runtime license manifest does not identify the embedded Python license")
    _require_relative_file(root, python_license)
    _validate_standalone_ffmpeg(root, manifest.get("ffmpeg"))
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
        if not component.get("license_declared") and not license_files:
            raise RuntimeError(f"no license metadata or license file for bundled Python component: {name}")
        if not isinstance(license_files, list):
            raise RuntimeError(f"license_files is malformed for bundled Python component: {name}")
        for relative in license_files:
            if not isinstance(relative, str):
                raise RuntimeError(f"non-string license path for bundled Python component: {name}")
            _require_relative_file(root, relative)
    return manifest


def _validate_native_hashes(root: Path, native_manifest: dict) -> None:
    internal = root / "engine" / "_internal"
    for section in ("pyav", "curl_cffi"):
        record = native_manifest.get(section)
        if not isinstance(record, dict):
            raise RuntimeError(f"native component inventory is missing {section}")
        wheel = record.get("wheel")
        if (
            not isinstance(wheel, dict)
            or not isinstance(wheel.get("sha256"), str)
            or len(wheel["sha256"]) != 64
        ):
            raise RuntimeError(f"native component inventory has no valid wheel hash for {section}")
        hashes = record.get("native_file_sha256")
        if not isinstance(hashes, dict) or not hashes:
            raise RuntimeError(f"native component inventory has no native hashes for {section}")
        for relative, expected in hashes.items():
            if not isinstance(relative, str) or not isinstance(expected, str) or len(expected) != 64:
                raise RuntimeError(f"malformed native hash in {section}: {relative}={expected}")
            candidate = (internal / relative).resolve()
            try:
                candidate.relative_to(internal.resolve())
            except ValueError as exc:
                raise RuntimeError(f"native component path escapes engine runtime: {relative}") from exc
            if not candidate.is_file():
                raise RuntimeError(f"installed native component is missing: {section}:{relative}")
            actual = _sha256(candidate)
            if actual != expected:
                raise RuntimeError(
                    f"installed native component drift: {section}:{relative} expected {expected}, got {actual}"
                )


def _validate_license_inventory(root: Path, name: str, manifest: dict, shipped_category: str) -> None:
    packages = manifest.get("packages")
    if not isinstance(packages, list) or not packages:
        raise RuntimeError(f"{name} license inventory contains no packages")
    if not any(
        isinstance(package, dict) and package.get("category") == shipped_category
        for package in packages
    ):
        raise RuntimeError(f"{name} license inventory contains no {shipped_category} packages")
    for package in packages:
        if not isinstance(package, dict):
            raise RuntimeError(f"{name} license inventory contains a malformed package")
        license_expression = str(package.get("license") or "")
        if not license_expression:
            raise RuntimeError(f"{name} package lacks license metadata: {package}")
        if package.get("category") == shipped_category and any(
            marker.lower() in license_expression.lower() for marker in PROHIBITED_LICENSE_MARKERS
        ):
            raise RuntimeError(
                f"{name} shipped package uses a license blocked by project policy: {package.get('name')} {license_expression}"
            )
        copied = package.get("copied_license_files") or []
        if copied and not isinstance(copied, list):
            raise RuntimeError(f"{name} copied_license_files is malformed for {package.get('name')}")
        for relative in copied:
            if not isinstance(relative, str):
                raise RuntimeError(f"{name} copied license path is malformed: {relative}")
            _require_relative_file(root, relative)


def _validate_compliance(root: Path, runtime_manifest: dict) -> None:
    for relative in REQUIRED_COMPLIANCE_FILES:
        _require_relative_file(root, relative)
    provenance = _load_manifest(root, "RUNTIME-PROVENANCE.json")
    native = _load_manifest(root, "NATIVE-COMPONENTS.json")
    rust = _load_manifest(root, "RUST-LICENSES.json")
    frontend = _load_manifest(root, "FRONTEND-LICENSES.json")

    standalone = provenance.get("standalone_ffmpeg")
    if not isinstance(standalone, dict):
        raise RuntimeError("runtime provenance is missing standalone_ffmpeg")
    archive = standalone.get("archive")
    if not isinstance(archive, dict) or archive.get("checksum_verified") is not True:
        raise RuntimeError("runtime provenance does not prove the static FFmpeg archive checksum")
    archive_hash = archive.get("archive_sha256")
    if not isinstance(archive_hash, str) or len(archive_hash) != 64:
        raise RuntimeError("runtime provenance has no valid static FFmpeg archive SHA-256")
    if archive.get("archive_sha256") != archive.get("git_lfs_oid_sha256"):
        raise RuntimeError("runtime provenance archive SHA does not match the reviewed provider LFS object")
    if standalone.get("runtime") != runtime_manifest.get("ffmpeg"):
        raise RuntimeError("runtime provenance FFmpeg record differs from RUNTIME-LICENSES.json")
    source = standalone.get("corresponding_source")
    if not isinstance(source, dict) or not source.get("status"):
        raise RuntimeError("runtime provenance does not state Corresponding Source status")

    pyav_native = native.get("pyav") or {}
    if pyav_native.get("ffmpeg_runtime_report") != runtime_manifest.get("pyav_ffmpeg"):
        raise RuntimeError("native PyAV FFmpeg report differs from RUNTIME-LICENSES.json")
    source_components = pyav_native.get("source_components")
    if not isinstance(source_components, list) or not source_components:
        raise RuntimeError("PyAV native inventory has no source component manifest")
    source_names = {
        str(component.get("name"))
        for component in source_components
        if isinstance(component, dict)
    }
    for required in ("ffmpeg", "x264", "x265", "gnutls", "vpx"):
        if required not in source_names:
            raise RuntimeError(f"PyAV native source inventory is missing {required}")

    curl_native = native.get("curl_cffi") or {}
    recipe = curl_native.get("recipe") or {}
    if recipe.get("curl_impersonate_version") != "1.5.2" or recipe.get("curl_version") != "8.15.0":
        raise RuntimeError(f"unexpected curl-cffi native recipe: {recipe}")

    _validate_native_hashes(root, native)
    _validate_license_inventory(root, "Rust", rust, "runtime")
    _validate_license_inventory(root, "frontend", frontend, "production")
    rust_names = {
        str(package.get("name"))
        for package in rust.get("packages", [])
        if isinstance(package, dict)
    }
    if "scriptotar-desktop" not in rust_names:
        raise RuntimeError("Rust inventory is missing scriptotar-desktop")


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
    if not completed.stdout.strip():
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


def validate(root: Path) -> None:
    root = root.resolve()
    if not root.is_dir():
        raise RuntimeError(f"runtime directory does not exist: {root}")
    if not (root / "RUNTIME-VERSIONS.txt").is_file():
        raise RuntimeError(f"runtime version file is missing: {root / 'RUNTIME-VERSIONS.txt'}")
    runtime_manifest = _validate_python_licenses(root)
    _validate_compliance(root, runtime_manifest)
    with tempfile.TemporaryDirectory(prefix="scriptotar-runtime-smoke-") as model_cache:
        env = _runtime_env(root)
        env["HF_HOME"] = model_cache
        _validate_engine(root, env)
        _validate_ytdlp(root, env)
        _validate_supervisor(root, env)


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Validate a packaged Scriptotar transcription runtime."
    )
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
