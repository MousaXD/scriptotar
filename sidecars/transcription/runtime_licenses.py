from __future__ import annotations

import importlib.metadata as metadata
import json
import os
import re
import shutil
import subprocess
import sys
from collections import deque
from pathlib import Path
from typing import Iterable


SCHEMA_VERSION = 1
ENGINE_REQUIREMENTS = ("faster-whisper", "yt-dlp[default,curl-cffi]")
SUPPORTED_FFMPEG_LICENSE = "GPL-3.0-or-later"


def _canonical_name(name: str) -> str:
    return re.sub(r"[-_.]+", "-", name).lower()


def classify_ffmpeg_license(version_output: str) -> str:
    """Infer FFmpeg's effective upstream license from its build switches.

    FFmpeg is LGPL-2.1-or-later by default. Enabling GPL components changes the
    whole FFmpeg build to GPL-2.0-or-later; --enable-version3 upgrades the
    applicable LGPL/GPL family to version 3 or later. A --enable-nonfree build
    is deliberately rejected because FFmpeg documents such builds as not
    redistributable.
    """

    switches = set(re.findall(r"--[A-Za-z0-9_-]+", version_output))
    if "--enable-nonfree" in switches:
        raise RuntimeError("FFmpeg was built with --enable-nonfree and must not be redistributed")
    if "--enable-gpl" in switches:
        return "GPL-3.0-or-later" if "--enable-version3" in switches else "GPL-2.0-or-later"
    return "LGPL-3.0-or-later" if "--enable-version3" in switches else "LGPL-2.1-or-later"


def _requirement_is_active(requirement: object, selected_extras: set[str]) -> bool:
    # packaging is intentionally imported lazily. It is a PyInstaller build
    # dependency, while unit tests for the FFmpeg classifier do not need it.
    from packaging.markers import default_environment

    marker = requirement.marker
    if marker is None:
        return True

    environment = default_environment()
    marker_text = str(marker)
    if "extra" not in marker_text:
        return marker.evaluate(environment)

    # Requirements guarded by an `extra` marker are active only for extras
    # selected on the parent distribution.
    return any(marker.evaluate({**environment, "extra": extra}) for extra in selected_extras)


def collect_runtime_distributions(root_requirements: Iterable[str] = ENGINE_REQUIREMENTS) -> list[tuple[metadata.Distribution, set[str]]]:
    """Resolve the installed distribution closure actually needed by the engine.

    This works from installed wheel metadata rather than a hand-maintained list,
    so transitive dependencies such as CTranslate2, tokenizers, huggingface-hub,
    PyAV and curl-cffi are represented at the exact versions packaged by CI.
    """

    from packaging.requirements import Requirement

    queue: deque[tuple[str, set[str]]] = deque()
    for raw in root_requirements:
        requirement = Requirement(raw)
        queue.append((requirement.name, set(requirement.extras)))

    resolved: dict[str, metadata.Distribution] = {}
    selected_extras: dict[str, set[str]] = {}
    processed_extras: dict[str, set[str]] = {}

    while queue:
        name, extras = queue.popleft()
        canonical = _canonical_name(name)
        selected = selected_extras.setdefault(canonical, set())
        previous = set(selected)
        selected.update(extras)

        if canonical not in resolved:
            try:
                resolved[canonical] = metadata.distribution(name)
            except metadata.PackageNotFoundError as exc:
                raise RuntimeError(f"required packaged Python distribution is missing: {name}") from exc

        already_processed = processed_extras.get(canonical)
        if already_processed is not None and selected == already_processed and selected == previous:
            continue

        dist = resolved[canonical]
        processed_extras[canonical] = set(selected)
        for raw_dependency in dist.requires or []:
            dependency = Requirement(raw_dependency)
            if not _requirement_is_active(dependency, selected):
                continue
            queue.append((dependency.name, set(dependency.extras)))

    return [(resolved[name], selected_extras[name]) for name in sorted(resolved)]


def _project_url(dist: metadata.Distribution) -> str | None:
    for value in dist.metadata.get_all("Project-URL") or []:
        label, separator, url = value.partition(",")
        if separator and label.strip().lower() in {"homepage", "source", "repository"}:
            return url.strip()
    return dist.metadata.get("Home-page") or None


def _license_declaration(dist: metadata.Distribution) -> str | None:
    value = dist.metadata.get("License-Expression") or dist.metadata.get("License")
    if value:
        compact = " ".join(value.split())
        if compact and compact.upper() != "UNKNOWN":
            return compact[:500]
    return None


def _license_candidates(dist: metadata.Distribution) -> list[metadata.PackagePath]:
    declared = [entry.replace("\\", "/") for entry in (dist.metadata.get_all("License-File") or [])]
    candidates: list[metadata.PackagePath] = []
    seen: set[str] = set()

    for entry in dist.files or []:
        text = str(entry).replace("\\", "/")
        basename = Path(text).name.lower()
        declared_match = any(text.endswith(item) for item in declared)
        conventional = basename.startswith(("license", "copying", "notice", "copyright"))
        if not declared_match and not conventional:
            continue
        if text in seen:
            continue
        source = Path(dist.locate_file(entry))
        if source.is_file():
            seen.add(text)
            candidates.append(entry)
    return candidates


def _copy_distribution_licenses(
    dist: metadata.Distribution,
    destination: Path,
    output_root: Path,
) -> list[str]:
    destination.mkdir(parents=True, exist_ok=True)
    copied: list[str] = []
    for index, entry in enumerate(_license_candidates(dist), start=1):
        source = Path(dist.locate_file(entry))
        basename = Path(str(entry)).name
        target = destination / f"{index:02d}-{basename}"
        shutil.copy2(source, target)
        copied.append(target.relative_to(output_root).as_posix())
    return copied


def _copy_python_runtime_license(legal_root: Path, output_root: Path) -> str:
    roots = {
        Path(sys.base_prefix),
        Path(sys.prefix),
        Path(sys.executable).resolve().parent,
        Path(sys.executable).resolve().parent.parent,
    }
    for root in roots:
        for name in ("LICENSE.txt", "LICENSE", "license.txt", "license"):
            source = root / name
            if source.is_file():
                destination = legal_root / "python-runtime" / "LICENSE.txt"
                destination.parent.mkdir(parents=True, exist_ok=True)
                shutil.copy2(source, destination)
                return destination.relative_to(output_root).as_posix()
    raise RuntimeError(
        "could not locate the license shipped with the Python interpreter used by PyInstaller"
    )


def _ffmpeg_report(ffmpeg_executable: Path) -> dict[str, object]:
    completed = subprocess.run(
        [str(ffmpeg_executable), "-version"],
        check=True,
        capture_output=True,
        text=True,
        timeout=30,
    )
    output = completed.stdout + completed.stderr
    if not output.strip():
        raise RuntimeError("bundled FFmpeg returned no version/build information")

    inferred_license = classify_ffmpeg_license(output)
    if inferred_license != SUPPORTED_FFMPEG_LICENSE:
        raise RuntimeError(
            "FFmpeg license changed from the packaged legal baseline: "
            f"expected {SUPPORTED_FFMPEG_LICENSE}, got {inferred_license}. "
            "Review the binary source and ship the matching license/source-compliance artifacts before packaging."
        )

    configuration = next(
        (line.partition(":")[2].strip() for line in output.splitlines() if line.lower().startswith("configuration:")),
        "",
    )
    version_line = next((line.strip() for line in output.splitlines() if line.strip()), "")
    crumb = ffmpeg_executable.parent / "installed.crumb"
    source_record = crumb.read_text(encoding="utf-8", errors="replace").strip() if crumb.is_file() else None
    return {
        "component": "FFmpeg and ffprobe",
        "license": inferred_license,
        "version_line": version_line,
        "configuration": configuration,
        "source_record": source_record,
        "redistribution_guard": "--enable-nonfree is rejected at package build time",
    }


def write_runtime_legal_bundle(
    output_root: Path,
    ffmpeg_executable: Path,
    repo_root: Path,
) -> Path:
    """Write legal notices and an exact installed-dependency manifest into a runtime."""

    output_root = output_root.resolve()
    legal_root = output_root / "legal"
    legal_root.mkdir(parents=True, exist_ok=True)

    root_files = {
        repo_root / "THIRD_PARTY_NOTICES.md": legal_root / "THIRD_PARTY_NOTICES.md",
        repo_root / "NOTICE": legal_root / "NOTICE",
        repo_root / "LICENSE": legal_root / "SCRIPTOTAR-APACHE-2.0.txt",
        repo_root / "third_party" / "licenses" / "ffmpeg" / "COPYING.GPLv3": legal_root / "FFMPEG-GPL-3.0.txt",
    }
    for source, destination in root_files.items():
        if not source.is_file():
            raise RuntimeError(f"required distribution notice/license is missing: {source}")
        shutil.copy2(source, destination)

    python_license = _copy_python_runtime_license(legal_root, output_root)

    python_components: list[dict[str, object]] = []
    for dist, extras in collect_runtime_distributions():
        canonical = _canonical_name(dist.metadata.get("Name") or "unknown")
        license_files = _copy_distribution_licenses(
            dist,
            legal_root / "python" / canonical,
            output_root,
        )
        python_components.append(
            {
                "name": dist.metadata.get("Name") or canonical,
                "version": dist.version,
                "selected_extras": sorted(extras),
                "license_declared": _license_declaration(dist),
                "license_files": license_files,
                "project_url": _project_url(dist),
                "distribution": "bundled into the PyInstaller transcription engine/runtime",
            }
        )

    pyinstaller = metadata.distribution("PyInstaller")
    pyinstaller_licenses = _copy_distribution_licenses(
        pyinstaller,
        legal_root / "build-tools" / "pyinstaller",
        output_root,
    )

    manifest = {
        "schema": SCHEMA_VERSION,
        "scope": "Scriptotar Next packaged transcription runtime",
        "python_runtime": {
            "implementation": sys.implementation.name,
            "version": sys.version.split()[0],
            "license_file": python_license,
            "distribution": "embedded by PyInstaller; separate system Python is not required",
        },
        "ffmpeg": _ffmpeg_report(ffmpeg_executable),
        "python_components": python_components,
        "build_tools": [
            {
                "name": "PyInstaller",
                "version": pyinstaller.version,
                "role": "build tool; bootloader is part of generated executables",
                "license_declared": _license_declaration(pyinstaller),
                "license_files": pyinstaller_licenses,
            },
            {
                "name": "static-ffmpeg",
                "version": metadata.version("static-ffmpeg"),
                "role": "build-time fetcher; package code itself is not copied into the runtime",
            },
        ],
        "not_bundled": [
            "Whisper model weights (downloaded on first uncached use)",
            "Linux WebKitGTK/GTK runtime libraries (system package dependencies)",
        ],
    }

    manifest_path = output_root / "RUNTIME-LICENSES.json"
    manifest_path.write_text(json.dumps(manifest, indent=2, sort_keys=True) + os.linesep, encoding="utf-8")
    return manifest_path
