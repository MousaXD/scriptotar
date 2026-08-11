from __future__ import annotations

import hashlib
import importlib.metadata as metadata
import os
from pathlib import Path
from typing import Any

from compliance_enrichment import (
    CURL_IMPERSONATE,
    CURL_NATIVE_COMPONENTS,
    WINDOWS_SYSTEM_LIBRARIES,
    _add_curl_notices_to_python_manifest,
    _curl_component_with_notices,
    _enrich_ffmpeg_provenance,
    _frontend_inventory,
    _read_json,
    _rewrite_generated_notice,
    _write_json,
    write_compliance_enrichment as _write_compliance_enrichment,
)


def _sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _windows_native_roots() -> tuple[Path, list[Path]]:
    dist = metadata.distribution("curl-cffi")
    site_root = Path(dist.locate_file("")).resolve()
    package_root = Path(dist.locate_file("curl_cffi")).resolve()
    roots = [
        package_root,
        site_root / "curl_cffi.libs",
        package_root / ".libs",
    ]
    return site_root, [root for root in roots if root.is_dir()]


def _windows_curl_binary_evidence() -> dict[str, Any]:
    # curl-cffi's reviewed Windows build is repaired with delvewheel. The
    # distribution metadata does not reliably enumerate delvewheel's adjacent
    # DLL directory on every Python/pip combination, so inspect the installed
    # repaired payload itself and then validate the PE import table.
    site_root, roots = _windows_native_roots()
    native_files: dict[str, str] = {}
    wrapper: Path | None = None
    by_name: dict[str, Path] = {}

    for root in roots:
        for candidate in root.rglob("*"):
            if not candidate.is_file():
                continue
            lower = candidate.name.lower()
            if not lower.endswith((".dll", ".pyd")):
                continue
            try:
                relative = candidate.relative_to(site_root).as_posix()
            except ValueError:
                relative = candidate.as_posix()
            native_files[relative] = _sha256_file(candidate)
            by_name[lower] = candidate
            if lower.startswith("_wrapper") and lower.endswith(".pyd"):
                wrapper = candidate

    if wrapper is None:
        raise RuntimeError(
            f"Windows curl-cffi repaired payload has no _wrapper.pyd: {sorted(native_files)}"
        )

    try:
        import pefile  # PyInstaller's Windows dependency; available in package CI.
    except ImportError as exc:
        raise RuntimeError("Windows curl-cffi PE validation requires pefile") from exc

    pe = pefile.PE(str(wrapper), fast_load=True)
    try:
        pe.parse_data_directories(
            directories=[pefile.DIRECTORY_ENTRY["IMAGE_DIRECTORY_ENTRY_IMPORT"]]
        )
        imports = sorted(
            {
                entry.dll.decode("ascii", errors="strict").lower()
                for entry in getattr(pe, "DIRECTORY_ENTRY_IMPORT", [])
            }
        )
    finally:
        pe.close()

    libcurl_imports = [name for name in imports if "libcurl" in name]
    if not libcurl_imports:
        raise RuntimeError(
            f"Windows curl-cffi _wrapper.pyd no longer imports libcurl-impersonate: {imports}"
        )

    resolved_imports: dict[str, str] = {}
    for import_name in imports:
        candidate = by_name.get(import_name)
        if candidate is None:
            continue
        try:
            relative = candidate.relative_to(site_root).as_posix()
        except ValueError:
            relative = candidate.as_posix()
        resolved_imports[import_name] = relative

    unresolved_libcurl = [name for name in libcurl_imports if name not in resolved_imports]
    if unresolved_libcurl:
        raise RuntimeError(
            "Windows curl-cffi repaired wheel imports libcurl-impersonate but its DLL "
            f"was not found beside the installed package: {unresolved_libcurl}; "
            f"payload={sorted(native_files)}"
        )

    if len(native_files) < 2:
        raise RuntimeError(
            f"Windows curl-cffi repaired native payload is unexpectedly small: {sorted(native_files)}"
        )

    return {
        "inventory_method": "installed repaired wheel scan plus PE import table",
        "wrapper_path": wrapper.relative_to(site_root).as_posix(),
        "wrapper_sha256": _sha256_file(wrapper),
        "native_file_sha256": dict(sorted(native_files.items())),
        "pe_imports": imports,
        "resolved_repaired_imports": dict(sorted(resolved_imports.items())),
        "libcurl_imports": libcurl_imports,
    }


def write_compliance_enrichment(runtime_root: Path, repo_root: Path) -> None:
    if os.name != "nt":
        _write_compliance_enrichment(runtime_root, repo_root)
        return

    runtime_root = runtime_root.resolve()
    repo_root = repo_root.resolve()
    native_path = runtime_root / "NATIVE-COMPONENTS.json"
    native = _read_json(native_path)
    curl_section = native.get("curl_cffi")
    if not isinstance(curl_section, dict):
        raise RuntimeError("NATIVE-COMPONENTS.json has no curl_cffi object")

    reviewed_components = [CURL_IMPERSONATE] + [
        component for component in CURL_NATIVE_COMPONENTS if "windows" in component["platforms"]
    ]
    packaged_records = [
        _curl_component_with_notices(runtime_root, component)
        for component in reviewed_components
    ]
    evidence = _windows_curl_binary_evidence()

    # Keep the original distribution-metadata inventory for provenance, but add
    # the complete repaired-wheel scan as the authoritative Windows payload
    # evidence. This preserves fail-closed behavior without fabricating file
    # names that are absent from importlib.metadata on some runners.
    curl_section["reviewed_native_components"] = packaged_records
    curl_section["platform"] = "windows"
    curl_section["binary_component_evidence"] = evidence
    curl_section["windows_repaired_native_file_sha256"] = evidence["native_file_sha256"]
    curl_section["system_provided_libraries"] = WINDOWS_SYSTEM_LIBRARIES
    curl_section["notice_policy"] = (
        "Versioned upstream notice text is copied into the installer and SHA-256 pinned in this manifest; "
        "Windows additionally scans the repaired wheel payload and validates _wrapper.pyd PE imports, "
        "including a resolvable libcurl-impersonate DLL."
    )
    _write_json(native_path, native)
    _add_curl_notices_to_python_manifest(runtime_root, packaged_records)

    frontend = _frontend_inventory(runtime_root, repo_root)
    _write_json(runtime_root / "FRONTEND-LICENSES.json", frontend)
    _enrich_ffmpeg_provenance(runtime_root)
    # The notice helper's optional evidence paragraph is Linux-specific, so the
    # Windows PE evidence remains machine-readable in NATIVE-COMPONENTS.json.
    _rewrite_generated_notice(runtime_root, frontend, packaged_records, None)
