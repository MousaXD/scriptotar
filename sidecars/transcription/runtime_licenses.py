from __future__ import annotations

import hashlib
import importlib.metadata as metadata
import json
import os
import re
import shutil
import subprocess
import sys
import urllib.request
from collections import deque
from pathlib import Path
from typing import Iterable


SCHEMA_VERSION = 1
ENGINE_REQUIREMENTS = ("faster-whisper", "yt-dlp[default,curl-cffi]")
SUPPORTED_FFMPEG_LICENSE = "GPL-3.0-or-later"
SUPPORTED_PYAV_FFMPEG_LICENSE = "GPL-3.0-or-later"
FFMPEG_GPL3_URL = "https://raw.githubusercontent.com/FFmpeg/FFmpeg/master/COPYING.GPLv3"
FFMPEG_GPL3_SHA256 = "8ceb4b9ee5adedde47b31e975c1d90c73ad27b6b165a1dcd80c7c545eb65b903"
CPYTHON_LICENSE_VERSION = "3.12.13"
CPYTHON_LICENSE_URL = "https://raw.githubusercontent.com/python/cpython/v3.12.13/LICENSE"
CPYTHON_LICENSE_SHA256 = "3b2f81fe21d181c499c59a256c8e1968455d6689d269aa85373bfb6af41da3bf"

# Some binary wheels omit usable License-Expression/License-File metadata even
# though the upstream project has an explicit license. Keep fallbacks narrow,
# version-specific and hash-pinned so an upstream/version change cannot silently
# inherit an old legal assumption.
LICENSE_METADATA_FALLBACKS: dict[str, dict[str, str]] = {
    "tokenizers": {
        "version": "0.23.1",
        "license": "Apache-2.0",
        "url": "https://raw.githubusercontent.com/huggingface/tokenizers/v0.23.1/LICENSE",
        "sha256": "c71d239df91726fc519c6eb72d318ec65820627232b2f796219e87dcf35d0ab4",
    },
}


def _canonical_name(name: str) -> str:
    return re.sub(r"[-_.]+", "-", name).lower()


def _sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


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

    # Evaluate once with no extra as well as every selected extra. This handles
    # compound markers such as `extra == 'x' or sys_platform == 'win32'`.
    extras_to_try = selected_extras or {""}
    if "" not in extras_to_try:
        extras_to_try = {*extras_to_try, ""}
    return any(marker.evaluate({**environment, "extra": extra}) for extra in extras_to_try)


def collect_runtime_distributions(
    root_requirements: Iterable[str] = ENGINE_REQUIREMENTS,
) -> list[tuple[metadata.Distribution, set[str]]]:
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

    version = sys.version.split()[0]
    if sys.implementation.name != "cpython" or version != CPYTHON_LICENSE_VERSION:
        raise RuntimeError(
            "could not locate the embedded Python runtime license and the reviewed CPython "
            f"fallback applies only to {CPYTHON_LICENSE_VERSION}; resolved {sys.implementation.name} {version}"
        )
    return _write_verified_remote_file(
        CPYTHON_LICENSE_URL,
        CPYTHON_LICENSE_SHA256,
        legal_root / "python-runtime" / "LICENSE.txt",
        output_root,
        f"CPython {CPYTHON_LICENSE_VERSION} license text",
    )


def _write_verified_remote_file(
    url: str,
    expected_sha256: str,
    destination: Path,
    output_root: Path,
    label: str,
) -> str:
    request = urllib.request.Request(url, headers={"User-Agent": "Scriptotar-runtime-builder"})
    with urllib.request.urlopen(request, timeout=30) as response:
        data = response.read()
    actual = hashlib.sha256(data).hexdigest()
    if actual != expected_sha256:
        raise RuntimeError(
            f"{label} hash changed; refusing to package an unreviewed legal artifact "
            f"(expected {expected_sha256}, got {actual})"
        )
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.write_bytes(data)
    return destination.relative_to(output_root).as_posix()


def _write_verified_ffmpeg_license(legal_root: Path, output_root: Path) -> str:
    return _write_verified_remote_file(
        FFMPEG_GPL3_URL,
        FFMPEG_GPL3_SHA256,
        legal_root / "FFMPEG-GPL-3.0.txt",
        output_root,
        "FFmpeg GPLv3 license text",
    )


def _apply_license_metadata_fallback(
    dist: metadata.Distribution,
    canonical: str,
    legal_root: Path,
    output_root: Path,
) -> tuple[str | None, list[str], str | None]:
    fallback = LICENSE_METADATA_FALLBACKS.get(canonical)
    if fallback is None:
        return None, [], None
    expected_version = fallback["version"]
    if dist.version != expected_version:
        raise RuntimeError(
            f"license metadata fallback for {canonical} is reviewed only for {expected_version}, "
            f"but packaging resolved {dist.version}"
        )
    license_file = _write_verified_remote_file(
        fallback["url"],
        fallback["sha256"],
        legal_root / "python" / canonical / "UPSTREAM-LICENSE.txt",
        output_root,
        f"{canonical} {dist.version} upstream license text",
    )
    return fallback["license"], [license_file], fallback["url"]


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
        (
            line.partition(":")[2].strip()
            for line in output.splitlines()
            if line.lower().startswith("configuration:")
        ),
        "",
    )
    version_line = next((line.strip() for line in output.splitlines() if line.strip()), "")
    crumb = ffmpeg_executable.parent / "installed.crumb"
    source_record = crumb.read_text(encoding="utf-8", errors="replace").strip() if crumb.is_file() else None
    ffprobe_name = "ffprobe.exe" if ffmpeg_executable.suffix.lower() == ".exe" else "ffprobe"
    ffprobe_executable = ffmpeg_executable.with_name(ffprobe_name)
    binary_sha256 = {"ffmpeg": _sha256_file(ffmpeg_executable)}
    if ffprobe_executable.is_file():
        binary_sha256["ffprobe"] = _sha256_file(ffprobe_executable)

    return {
        "component": "FFmpeg and ffprobe executables fetched by static-ffmpeg",
        "license": inferred_license,
        "version_line": version_line,
        "configuration": configuration,
        "source_record": source_record,
        "binary_sha256": binary_sha256,
        "redistribution_guard": "--enable-nonfree is rejected at package build time",
    }


def parse_pyav_ffmpeg_report(output: str) -> dict[str, object]:
    """Parse `python -m av --version` and enforce the FFmpeg wheel baseline."""

    pyav_version = ""
    groups: list[dict[str, object]] = []
    current: dict[str, object] | None = None

    for raw_line in output.splitlines():
        line = raw_line.strip()
        if not line:
            continue
        if line.startswith("PyAV v"):
            pyav_version = line.removeprefix("PyAV v").strip()
            continue
        if line.startswith("library configuration:"):
            configuration = line.partition(":")[2].strip()
            current = {"configuration": configuration, "libraries": []}
            groups.append(current)
            continue
        if line.startswith("library license:"):
            if current is None:
                raise RuntimeError("PyAV reported a library license before a configuration")
            current["reported_license"] = line.partition(":")[2].strip()
            continue
        if current is not None and re.match(r"^lib[A-Za-z0-9_]+\s+\-?\d+", line):
            libraries = current["libraries"]
            assert isinstance(libraries, list)
            libraries.append(line)

    if not pyav_version or not groups:
        raise RuntimeError(f"could not parse PyAV FFmpeg build information: {output}")

    for group in groups:
        configuration = str(group.get("configuration") or "")
        reported_license = str(group.get("reported_license") or "")
        if not configuration or not reported_license:
            raise RuntimeError(f"incomplete PyAV FFmpeg build group: {group}")
        inferred = classify_ffmpeg_license(configuration)
        if inferred != SUPPORTED_PYAV_FFMPEG_LICENSE:
            raise RuntimeError(
                "PyAV's bundled FFmpeg library license changed from the audited baseline: "
                f"expected {SUPPORTED_PYAV_FFMPEG_LICENSE}, got {inferred}"
            )
        if reported_license.lower() != "gpl version 3 or later":
            raise RuntimeError(
                "PyAV's FFmpeg runtime reported an unexpected license string: "
                f"{reported_license}"
            )
        group["inferred_license"] = inferred

    return {"pyav_version": pyav_version, "library_groups": groups}


def _pyav_native_library_hashes(dist: metadata.Distribution) -> dict[str, str]:
    hashes: dict[str, str] = {}
    for entry in dist.files or []:
        text = str(entry).replace("\\", "/")
        lowered = text.lower()
        if not (
            lowered.startswith("av.libs/")
            or "/av.libs/" in lowered
            or lowered.startswith("av/.dylibs/")
            or "/av/.dylibs/" in lowered
        ):
            continue
        source = Path(dist.locate_file(entry))
        if source.is_file():
            hashes[text] = _sha256_file(source)
    return dict(sorted(hashes.items()))


def _pyav_ffmpeg_report(ffmpeg_license_file: str) -> dict[str, object]:
    completed = subprocess.run(
        [sys.executable, "-m", "av", "--version"],
        check=True,
        capture_output=True,
        text=True,
        timeout=30,
    )
    output = completed.stdout + completed.stderr
    parsed = parse_pyav_ffmpeg_report(output)
    av_dist = metadata.distribution("av")
    native_hashes = _pyav_native_library_hashes(av_dist)
    if not native_hashes:
        raise RuntimeError(
            "PyAV binary wheel did not expose the expected bundled native-library directory; "
            "review the wheel packaging before distributing it"
        )
    parsed.update(
        {
            "component": "FFmpeg shared libraries bundled by the PyAV binary wheel",
            "license": SUPPORTED_PYAV_FFMPEG_LICENSE,
            "license_file": ffmpeg_license_file,
            "native_library_sha256": native_hashes,
            "wheel_project_url": _project_url(av_dist),
            "redistribution_guard": "every PyAV FFmpeg library group must be GPLv3 and must not use --enable-nonfree",
        }
    )
    return parsed


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
    }
    for source, destination in root_files.items():
        if not source.is_file():
            raise RuntimeError(f"required distribution notice/license is missing: {source}")
        shutil.copy2(source, destination)

    ffmpeg_license_file = _write_verified_ffmpeg_license(legal_root, output_root)
    python_license = _copy_python_runtime_license(legal_root, output_root)

    python_components: list[dict[str, object]] = []
    for dist, extras in collect_runtime_distributions():
        canonical = _canonical_name(dist.metadata.get("Name") or "unknown")
        component_dir = legal_root / "python" / canonical
        license_files = _copy_distribution_licenses(dist, component_dir, output_root)
        license_declared = _license_declaration(dist)
        license_reference: str | None = None
        license_source = "installed-wheel-metadata"

        if not license_declared and not license_files:
            license_declared, license_files, license_reference = _apply_license_metadata_fallback(
                dist,
                canonical,
                legal_root,
                output_root,
            )
            if license_declared or license_files:
                license_source = "version-pinned-upstream-fallback"

        python_components.append(
            {
                "name": dist.metadata.get("Name") or canonical,
                "version": dist.version,
                "selected_extras": sorted(extras),
                "license_declared": license_declared,
                "license_files": license_files,
                "license_source": license_source,
                "license_reference": license_reference,
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

    ffmpeg = _ffmpeg_report(ffmpeg_executable)
    ffmpeg["license_file"] = ffmpeg_license_file
    pyav_ffmpeg = _pyav_ffmpeg_report(ffmpeg_license_file)

    manifest = {
        "schema": SCHEMA_VERSION,
        "scope": "Scriptotar Next packaged transcription runtime",
        "python_runtime": {
            "implementation": sys.implementation.name,
            "version": sys.version.split()[0],
            "license_file": python_license,
            "distribution": "embedded by PyInstaller; separate system Python is not required",
        },
        "ffmpeg": ffmpeg,
        "pyav_ffmpeg": pyav_ffmpeg,
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