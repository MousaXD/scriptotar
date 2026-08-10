from __future__ import annotations

import argparse
import base64
import hashlib
import io
import json
import os
import platform
import re
import tarfile
import urllib.request
from pathlib import Path
from typing import Any

SCHEMA_VERSION = 1
REPO_ROOT = Path(__file__).resolve().parents[2]

PROHIBITED_LICENSE_MARKERS = (
    "AGPL-",
    "SSPL-",
    "BUSL-",
    "Business-Source-License",
    "Commons-Clause",
    "LicenseRef-",
    "UNLICENSED",
)
NOTICE_PREFIXES = ("license", "licence", "copying", "notice", "copyright")

CURL_IMPERSONATE = {
    "name": "curl-impersonate",
    "version": "1.5.2",
    "source_revision": "9607b22ccf6c440e560c5f8ad5292b8044bb6dd7",
    "source_url": "https://github.com/lexiforest/curl-impersonate/tree/9607b22ccf6c440e560c5f8ad5292b8044bb6dd7",
    "license": "MIT",
    "notices": [
        {
            "name": "LICENSE",
            "url": "https://raw.githubusercontent.com/lexiforest/curl-impersonate/9607b22ccf6c440e560c5f8ad5292b8044bb6dd7/LICENSE",
        }
    ],
}

CURL_NATIVE_COMPONENTS = [
    {
        "name": "curl",
        "version": "8.15.0",
        "source_revision": "cfbfb65047e85e6b08af65fe9cdbcf68e9ad496a",
        "source_url": "https://github.com/curl/curl/tree/cfbfb65047e85e6b08af65fe9cdbcf68e9ad496a",
        "license": "curl",
        "platforms": ["linux", "windows"],
        "linux_marker": "libcurl/8.15.0-IMPERSONATE",
        "notices": [
            {
                "name": "COPYING",
                "url": "https://raw.githubusercontent.com/curl/curl/cfbfb65047e85e6b08af65fe9cdbcf68e9ad496a/COPYING",
            }
        ],
    },
    {
        "name": "BoringSSL",
        "version": "673e61fc215b178a90c0e67858bbf162c8158993",
        "source_revision": "673e61fc215b178a90c0e67858bbf162c8158993",
        "source_url": "https://github.com/google/boringssl/tree/673e61fc215b178a90c0e67858bbf162c8158993",
        "license": "Apache-2.0 plus bundled third-party notices",
        "platforms": ["linux", "windows"],
        "linux_marker": "boringssl-673e61fc215b178a90c0e67858bbf162c8158993",
        "notices": [
            {
                "name": "LICENSE",
                "url": "https://raw.githubusercontent.com/google/boringssl/673e61fc215b178a90c0e67858bbf162c8158993/LICENSE",
            }
        ],
    },
    {
        "name": "Brotli",
        "version": "1.2.0",
        "source_revision": "028fb5a23661f123017c060daa546b55cf4bde29",
        "source_url": "https://github.com/google/brotli/tree/028fb5a23661f123017c060daa546b55cf4bde29",
        "license": "MIT",
        "platforms": ["linux", "windows"],
        "linux_marker": "brotli-1.2.0",
        "notices": [
            {
                "name": "LICENSE",
                "url": "https://raw.githubusercontent.com/google/brotli/028fb5a23661f123017c060daa546b55cf4bde29/LICENSE",
            }
        ],
    },
    {
        "name": "nghttp2",
        "version": "1.63.0",
        "source_revision": "8f44147c385fb1ed93a6f39911eeb30279bfd2dd",
        "source_url": "https://github.com/nghttp2/nghttp2/tree/8f44147c385fb1ed93a6f39911eeb30279bfd2dd",
        "license": "MIT",
        "platforms": ["linux", "windows"],
        "linux_marker": "nghttp2-1.63.0",
        "notices": [
            {
                "name": "COPYING",
                "url": "https://raw.githubusercontent.com/nghttp2/nghttp2/8f44147c385fb1ed93a6f39911eeb30279bfd2dd/COPYING",
            }
        ],
    },
    {
        "name": "nghttp3",
        "version": "1.15.0",
        "source_revision": "d326f4c1eb3f6a780d77793b30e16756c498f913",
        "source_url": "https://github.com/ngtcp2/nghttp3/tree/d326f4c1eb3f6a780d77793b30e16756c498f913",
        "license": "MIT",
        "platforms": ["linux", "windows"],
        "linux_marker": "nghttp3-1.15.0",
        "notices": [
            {
                "name": "COPYING",
                "url": "https://raw.githubusercontent.com/ngtcp2/nghttp3/d326f4c1eb3f6a780d77793b30e16756c498f913/COPYING",
            }
        ],
    },
    {
        "name": "ngtcp2",
        "version": "1.20.0",
        "source_revision": "ca898d32348c93af9fbbc81538505a1c1c062685",
        "source_url": "https://github.com/ngtcp2/ngtcp2/tree/ca898d32348c93af9fbbc81538505a1c1c062685",
        "license": "MIT",
        "platforms": ["linux", "windows"],
        "linux_marker": "ngtcp2-1.20.0",
        "notices": [
            {
                "name": "COPYING",
                "url": "https://raw.githubusercontent.com/ngtcp2/ngtcp2/ca898d32348c93af9fbbc81538505a1c1c062685/COPYING",
            }
        ],
    },
    {
        "name": "zlib",
        "version": "1.3.1",
        "source_revision": "51b7f2abdade71cd9bb0e7a373ef2610ec6f9daf",
        "source_url": "https://github.com/madler/zlib/tree/51b7f2abdade71cd9bb0e7a373ef2610ec6f9daf",
        "license": "Zlib",
        "platforms": ["linux", "windows"],
        "linux_marker": "zlib-1.3.1",
        "notices": [
            {
                "name": "LICENSE",
                "url": "https://raw.githubusercontent.com/madler/zlib/51b7f2abdade71cd9bb0e7a373ef2610ec6f9daf/LICENSE",
            }
        ],
    },
    {
        "name": "zstd",
        "version": "1.5.6",
        "source_revision": "794ea1b0afca0f020f4e57b6732332231fb23c70",
        "source_url": "https://github.com/facebook/zstd/tree/794ea1b0afca0f020f4e57b6732332231fb23c70",
        "license": "BSD-3-Clause",
        "platforms": ["linux", "windows"],
        "linux_marker": "zstd-1.5.6",
        "notices": [
            {
                "name": "LICENSE",
                "url": "https://raw.githubusercontent.com/facebook/zstd/794ea1b0afca0f020f4e57b6732332231fb23c70/LICENSE",
            }
        ],
    },
    {
        "name": "libidn2",
        "version": "2.3.7",
        "source_revision": "v2.3.7",
        "source_url": "https://ftp.gnu.org/pub/gnu/libidn/libidn2-2.3.7.tar.gz",
        "source_sha256": "4c21a791b610b9519b9d0e12b8097bf2f359b12f8dd92647611a929e6bfd7d64",
        "license": "GPL-2.0-or-later OR LGPL-3.0-or-later; Unicode data has separate terms",
        "platforms": ["linux"],
        "linux_marker": "libidn2-2.3.7",
        "archive_notices": ["COPYINGv2", "COPYING.LESSERv3", "COPYING.unicode"],
    },
]

WINDOWS_SYSTEM_LIBRARIES = ["Crypt32", "Secur32", "wldap32", "Normaliz", "iphlpapi"]
WINDOWS_FFMPEG_CORE = {
    "revision": "894da5ca7d",
    "version": "8.0.1",
    "source_url": "https://github.com/FFmpeg/FFmpeg/commit/894da5ca7d",
    "evidence": "gyan.dev 8.0.1 release metadata identifies the FFmpeg source revision",
}

IMPORT_RE = re.compile(r"""(?:import|export)\s+(?:[^'\"]*?\s+from\s+)?['\"]([^'\"]+)['\"]""")
DYNAMIC_IMPORT_RE = re.compile(r"""import\(\s*['\"]([^'\"]+)['\"]\s*\)""")


def _sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def _sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _write_json(path: Path, payload: object) -> None:
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def _read_json(path: Path) -> dict[str, Any]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise RuntimeError(f"expected JSON object: {path}")
    return value


def _platform_key() -> str:
    if os.name == "nt":
        return "windows"
    if platform.system().lower() == "linux":
        return "linux"
    raise RuntimeError(f"unsupported compliance platform: {platform.system()} {os.name}")


def _download(url: str, *, expected_sha256: str | None = None, timeout: int = 120) -> bytes:
    request = urllib.request.Request(url, headers={"User-Agent": "Scriptotar-compliance-builder/1"})
    with urllib.request.urlopen(request, timeout=timeout) as response:
        data = response.read()
    if not data:
        raise RuntimeError(f"downloaded empty compliance source: {url}")
    if expected_sha256:
        actual = _sha256_bytes(data)
        if actual != expected_sha256:
            raise RuntimeError(
                f"compliance source checksum drift for {url}: expected {expected_sha256}, got {actual}"
            )
    return data


def _safe_component_dir(name: str, version: str) -> str:
    token = f"{name}-{version}".replace("/", "__").replace("\\", "__")
    return re.sub(r"[^A-Za-z0-9_.@+-]+", "_", token)


def _write_notice_bytes(
    runtime_root: Path,
    category: str,
    component_name: str,
    component_version: str,
    notice_name: str,
    data: bytes,
    source_url: str,
) -> dict[str, Any]:
    destination = runtime_root / "legal" / category / _safe_component_dir(component_name, component_version) / notice_name
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.write_bytes(data)
    return {
        "name": notice_name,
        "source_url": source_url,
        "path": str(destination.relative_to(runtime_root)).replace("\\", "/"),
        "sha256": _sha256_bytes(data),
        "size": len(data),
    }


def _curl_component_with_notices(runtime_root: Path, component: dict[str, Any]) -> dict[str, Any]:
    record = {key: value for key, value in component.items() if key not in {"notices", "archive_notices"}}
    notices: list[dict[str, Any]] = []
    if component.get("notices"):
        for notice in component["notices"]:
            data = _download(str(notice["url"]))
            notices.append(
                _write_notice_bytes(
                    runtime_root,
                    "curl-native",
                    str(component["name"]),
                    str(component["version"]),
                    str(notice["name"]),
                    data,
                    str(notice["url"]),
                )
            )
    elif component.get("archive_notices"):
        archive = _download(
            str(component["source_url"]),
            expected_sha256=str(component["source_sha256"]),
            timeout=180,
        )
        with tarfile.open(fileobj=io.BytesIO(archive), mode="r:gz") as payload:
            members = {Path(member.name).name: member for member in payload.getmembers() if member.isfile()}
            for notice_name in component["archive_notices"]:
                member = members.get(str(notice_name))
                if member is None:
                    raise RuntimeError(f"libidn2 source archive is missing reviewed notice {notice_name}")
                handle = payload.extractfile(member)
                if handle is None:
                    raise RuntimeError(f"could not extract libidn2 notice {notice_name}")
                data = handle.read()
                notices.append(
                    _write_notice_bytes(
                        runtime_root,
                        "curl-native",
                        str(component["name"]),
                        str(component["version"]),
                        str(notice_name),
                        data,
                        f"{component['source_url']}#{notice_name}",
                    )
                )
    if not notices:
        raise RuntimeError(f"no native notices packaged for {component['name']}")
    record["notices"] = notices
    return record


def _find_curl_wrapper(runtime_root: Path) -> Path:
    candidates = sorted((runtime_root / "engine" / "_internal").glob("curl_cffi/_wrapper*"))
    files = [candidate for candidate in candidates if candidate.is_file()]
    if len(files) != 1:
        raise RuntimeError(f"expected one packaged curl-cffi wrapper, found {[str(path) for path in files]}")
    return files[0]


def _linux_curl_marker_evidence(runtime_root: Path, components: list[dict[str, Any]]) -> dict[str, Any]:
    wrapper = _find_curl_wrapper(runtime_root)
    data = wrapper.read_bytes()
    markers: list[dict[str, str]] = []
    for component in components:
        marker = component.get("linux_marker")
        if not marker:
            continue
        encoded = str(marker).encode("utf-8")
        if encoded not in data:
            raise RuntimeError(f"curl-cffi Linux native component marker disappeared: {component['name']} {marker}")
        markers.append({"component": str(component["name"]), "marker": str(marker)})
    return {
        "wrapper_path": str(wrapper.relative_to(runtime_root)).replace("\\", "/"),
        "wrapper_sha256": _sha256_file(wrapper),
        "markers": markers,
    }


def _package_root(specifier: str) -> str | None:
    specifier = specifier.strip()
    if not specifier or specifier.startswith((".", "/", "#", "\0")):
        return None
    if specifier.startswith(("http:", "https:", "data:", "node:")):
        return None
    parts = specifier.split("/")
    if specifier.startswith("@"):
        return "/".join(parts[:2]) if len(parts) >= 2 else None
    return parts[0]


def _source_runtime_roots(repo_root: Path) -> list[str]:
    source_root = repo_root / "apps" / "desktop-ui" / "src"
    if not source_root.is_dir():
        raise RuntimeError(f"frontend source directory is missing: {source_root}")
    roots: set[str] = set()
    saw_svelte = False
    for path in sorted(source_root.rglob("*")):
        if not path.is_file() or path.suffix.lower() not in {".ts", ".js", ".mjs", ".cjs", ".svelte"}:
            continue
        if path.suffix.lower() == ".svelte":
            saw_svelte = True
        text = path.read_text(encoding="utf-8")
        for regex in (IMPORT_RE, DYNAMIC_IMPORT_RE):
            for match in regex.finditer(text):
                root = _package_root(match.group(1))
                if root:
                    roots.add(root)
    if saw_svelte:
        roots.add("svelte")
    return sorted(roots)


def _lock_package_name(path_key: str, entry: dict[str, Any]) -> str:
    if entry.get("name"):
        return str(entry["name"])
    return path_key.rsplit("node_modules/", 1)[-1]


def _resolve_lock_dependency(packages: dict[str, Any], from_path: str, dependency: str) -> str | None:
    candidates: list[str] = []
    base = from_path
    while base:
        candidates.append(f"{base}/node_modules/{dependency}")
        if "/node_modules/" not in base:
            break
        base = base.rsplit("/node_modules/", 1)[0]
    candidates.append(f"node_modules/{dependency}")
    for candidate in candidates:
        if candidate in packages:
            return candidate
    return None


def _frontend_bundle_paths(
    repo_root: Path,
) -> tuple[list[str], set[str], dict[str, Any], str, list[str]]:
    lock_path = repo_root / "apps" / "desktop-ui" / "package-lock.json"
    lock = _read_json(lock_path)
    if lock.get("lockfileVersion") != 3 or not isinstance(lock.get("packages"), dict):
        raise RuntimeError("frontend compliance requires package-lock.json lockfileVersion 3")
    packages: dict[str, Any] = lock["packages"]
    roots = _source_runtime_roots(repo_root)

    dist_root = repo_root / "apps" / "desktop-ui" / "dist"
    source_maps = sorted(dist_root.rglob("*.map")) if dist_root.is_dir() else []
    if source_maps:
        lock_paths = sorted(
            (path_key for path_key, entry in packages.items() if path_key and isinstance(entry, dict)),
            key=len,
            reverse=True,
        )
        closure: set[str] = set()
        source_evidence: list[str] = []
        for source_map in source_maps:
            payload = _read_json(source_map)
            sources = payload.get("sources")
            if not isinstance(sources, list):
                continue
            for source in sources:
                if not isinstance(source, str):
                    continue
                normalized = source.replace("\\", "/")
                matched = next(
                    (
                        path_key
                        for path_key in lock_paths
                        if f"/{path_key}/" in f"/{normalized.lstrip('./')}/"
                    ),
                    None,
                )
                if matched:
                    closure.add(matched)
                    source_evidence.append(f"{source_map.relative_to(dist_root).as_posix()}::{source}")
        if not closure:
            raise RuntimeError(
                f"Vite source maps exist but identify no locked node_modules contributors: {[str(path) for path in source_maps]}"
            )
        return roots, closure, packages, "vite-source-map", sorted(set(source_evidence))

    closure: set[str] = set()
    queue: list[str] = []
    for name in roots:
        path_key = f"node_modules/{name}"
        if path_key not in packages:
            raise RuntimeError(f"production frontend import is absent from package-lock.json: {name}")
        queue.append(path_key)
    while queue:
        path_key = queue.pop(0)
        if path_key in closure:
            continue
        entry = packages.get(path_key)
        if not isinstance(entry, dict):
            raise RuntimeError(f"malformed frontend lock package: {path_key}")
        closure.add(path_key)
        dependencies: dict[str, Any] = {}
        for field in ("dependencies", "optionalDependencies"):
            value = entry.get(field)
            if isinstance(value, dict):
                dependencies.update(value)
        for dependency in sorted(dependencies):
            resolved = _resolve_lock_dependency(packages, path_key, dependency)
            if resolved:
                queue.append(resolved)
    return roots, closure, packages, "source-import-lock-fallback", []


def _verify_sri(data: bytes, integrity: str) -> None:
    tokens = [token for token in integrity.split() if "-" in token]
    if not tokens:
        raise RuntimeError(f"unsupported npm integrity value: {integrity!r}")
    for token in tokens:
        algorithm, encoded = token.split("-", 1)
        try:
            digest = hashlib.new(algorithm, data).digest()
        except ValueError:
            continue
        if base64.b64encode(digest).decode("ascii") == encoded:
            return
    raise RuntimeError("npm tarball did not match package-lock integrity")


def _npm_notices(
    runtime_root: Path,
    name: str,
    version: str,
    resolved: str,
    integrity: str,
) -> list[dict[str, Any]]:
    data = _download(resolved, timeout=180)
    _verify_sri(data, integrity)
    notices: list[dict[str, Any]] = []
    with tarfile.open(fileobj=io.BytesIO(data), mode="r:gz") as payload:
        for member in payload.getmembers():
            if not member.isfile():
                continue
            relative = Path(member.name)
            if len(relative.parts) != 2 or relative.parts[0] != "package":
                continue
            filename = relative.parts[1]
            if not filename.lower().startswith(NOTICE_PREFIXES):
                continue
            handle = payload.extractfile(member)
            if handle is None:
                continue
            notice_data = handle.read()
            notices.append(
                _write_notice_bytes(
                    runtime_root,
                    "frontend",
                    name,
                    version,
                    filename,
                    notice_data,
                    f"{resolved}#{filename}",
                )
            )
    if not notices:
        raise RuntimeError(
            f"production frontend package has no top-level license/notice in its locked tarball: {name} {version}"
        )
    return notices


def _frontend_inventory(runtime_root: Path, repo_root: Path) -> dict[str, Any]:
    roots, closure, packages, method, source_evidence = _frontend_bundle_paths(repo_root)
    records: list[dict[str, Any]] = []
    for path_key, entry in sorted(packages.items()):
        if not path_key or not isinstance(entry, dict):
            continue
        name = _lock_package_name(path_key, entry)
        version = str(entry.get("version") or "")
        license_expression = str(entry.get("license") or "")
        if not version:
            raise RuntimeError(f"frontend lock entry has no version: {path_key}")
        if not license_expression:
            raise RuntimeError(f"frontend lock entry has no license metadata: {name} {version}")
        bundled = path_key in closure
        if bundled and any(
            marker.lower() in license_expression.lower() for marker in PROHIBITED_LICENSE_MARKERS
        ):
            raise RuntimeError(
                f"frontend bundled package license requires explicit review: {name} {version} {license_expression}"
            )
        notices: list[dict[str, Any]] = []
        if bundled:
            resolved = str(entry.get("resolved") or "")
            integrity = str(entry.get("integrity") or "")
            if not resolved or not integrity:
                raise RuntimeError(
                    f"frontend bundled package lacks locked tarball provenance: {name} {version}"
                )
            notices = _npm_notices(runtime_root, name, version, resolved, integrity)
        records.append(
            {
                "lock_path": path_key,
                "name": name,
                "version": version,
                "license": license_expression,
                "npm_dev_flag": bool(entry.get("dev")),
                "bundle_category": "production-bundled" if bundled else "build-only",
                "resolved": entry.get("resolved"),
                "integrity": entry.get("integrity"),
                "optional": bool(entry.get("optional")),
                "license_files": notices,
            }
        )
    bundled_records = [record for record in records if record["bundle_category"] == "production-bundled"]
    if not bundled_records:
        raise RuntimeError("frontend production bundle closure is unexpectedly empty")
    return {
        "schema": SCHEMA_VERSION,
        "source_of_truth": "apps/desktop-ui/package-lock.json plus Vite production source maps when available; source-import lock traversal is only a non-package fallback",
        "inventory_method": method,
        "production_import_roots": roots,
        "production_package_count": len(bundled_records),
        "vite_source_map_evidence": source_evidence,
        "policy": {
            "unknown_license_metadata": "fail",
            "production_prohibited_markers": list(PROHIBITED_LICENSE_MARKERS),
            "npm_dev_flag": "recorded but not treated as proof that a package is absent from production output",
            "production_method": "Release packages use node_modules modules proven by Vite source maps; non-package fallback uses bare production imports plus implicit Svelte runtime followed through locked dependencies/optional dependencies",
        },
        "packages": records,
    }


def _add_curl_notices_to_python_manifest(runtime_root: Path, curl_records: list[dict[str, Any]]) -> None:
    path = runtime_root / "RUNTIME-LICENSES.json"
    manifest = _read_json(path)
    components = manifest.get("python_components")
    if not isinstance(components, list):
        raise RuntimeError("RUNTIME-LICENSES.json has no python_components list")
    curl = next(
        (
            component
            for component in components
            if isinstance(component, dict)
            and str(component.get("name") or "").lower().replace("_", "-") == "curl-cffi"
        ),
        None,
    )
    if not isinstance(curl, dict):
        raise RuntimeError("RUNTIME-LICENSES.json has no curl-cffi component")
    license_files = curl.setdefault("license_files", [])
    if not isinstance(license_files, list):
        raise RuntimeError("curl-cffi license_files is malformed")
    for record in curl_records:
        for notice in record.get("notices") or []:
            path_value = notice.get("path")
            if isinstance(path_value, str) and path_value not in license_files:
                license_files.append(path_value)
    license_files.sort()
    _write_json(path, manifest)


def _enrich_ffmpeg_provenance(runtime_root: Path) -> None:
    path = runtime_root / "RUNTIME-PROVENANCE.json"
    provenance = _read_json(path)
    standalone = provenance.get("standalone_ffmpeg")
    if not isinstance(standalone, dict):
        raise RuntimeError("RUNTIME-PROVENANCE.json has no standalone_ffmpeg object")
    platform_key = _platform_key()
    revision = standalone.get("upstream_ffmpeg_revision_from_binary")
    if platform_key == "linux":
        if not isinstance(revision, str) or not re.fullmatch(r"[0-9a-f]{7,40}", revision):
            raise RuntimeError("Linux standalone FFmpeg binary no longer reports an upstream Git revision")
        standalone["core_source"] = {
            "version": str((standalone.get("runtime") or {}).get("version_line") or ""),
            "revision": revision,
            "source_url": f"https://github.com/FFmpeg/FFmpeg/commit/{revision}",
            "evidence": "revision parsed directly from the shipped ffmpeg -version output",
        }
        standalone["provider_build_recipe_evidence"] = {
            "repository": "https://github.com/BtbN/FFmpeg-Builds",
            "candidate_revision": "7036a5cf9e3ba9570d174c263bddebdebe2e4168",
            "status": "historical-repository-state-inference",
            "reason": "This is the last BtbN build-repository commit before the binary's 2026-01-16 build date; it is useful evidence but not treated as proof of the exact daily release build inputs.",
        }
    else:
        standalone["core_source"] = WINDOWS_FFMPEG_CORE
    source = standalone.get("corresponding_source")
    if isinstance(source, dict):
        source["status"] = "partial-core-source-provenance-only"
        source["technically_resolved"] = False
        source["remaining_engineering_gap"] = (
            "Exact FFmpeg core source is identified, but the current prebuilt provider chain does not expose a retained complete source/build-input mapping for every statically linked GPL dependency."
        )
    _write_json(path, provenance)


def _rewrite_generated_notice(
    runtime_root: Path,
    frontend: dict[str, Any],
    curl_records: list[dict[str, Any]],
    curl_evidence: dict[str, Any] | None,
) -> None:
    path = runtime_root / "legal" / "THIRD_PARTY_NOTICES.md"
    text = path.read_text(encoding="utf-8")
    lines = []
    replaced = False
    for line in text.splitlines():
        if line.startswith("- Production frontend npm dependency closure:"):
            lines.append(
                f"- Production frontend bundled dependency closure: {frontend['production_package_count']} locked npm package(s), proven by `{frontend['inventory_method']}` evidence. See `FRONTEND-LICENSES.json` and `legal/frontend/`."
            )
            replaced = True
        else:
            lines.append(line)
    if not replaced:
        raise RuntimeError("generated third-party notice frontend summary format drifted")
    lines.extend(
        [
            "",
            "## curl-cffi native notice bundle",
            "",
            "The native curl-cffi stack is inventoried independently from curl-cffi's top-level MIT metadata. Exact versioned upstream notices are packaged under `legal/curl-native/` and hashed in `NATIVE-COMPONENTS.json`.",
            "",
        ]
    )
    for record in curl_records:
        notice_paths = ", ".join(f"`{notice['path']}`" for notice in record.get("notices") or [])
        lines.append(
            f"- {record['name']} {record['version']}: {record['license']}; source `{record['source_url']}`; notice(s) {notice_paths}."
        )
    if curl_evidence:
        lines.extend(
            [
                "",
                f"Linux binary marker validation found {len(curl_evidence.get('markers') or [])} reviewed native component markers in `{curl_evidence['wrapper_path']}` and hash-pinned that wrapper.",
            ]
        )
    lines.extend(
        [
            "",
            "On Windows, curl-cffi's reviewed build recipe also links system libraries Crypt32, Secur32, wldap32, Normaliz and iphlpapi; those are system-provided rather than copied third-party payloads.",
            "",
        ]
    )
    path.write_text("\n".join(lines), encoding="utf-8")


def write_compliance_enrichment(runtime_root: Path, repo_root: Path = REPO_ROOT) -> None:
    runtime_root = runtime_root.resolve()
    repo_root = repo_root.resolve()
    native_path = runtime_root / "NATIVE-COMPONENTS.json"
    native = _read_json(native_path)
    curl_section = native.get("curl_cffi")
    if not isinstance(curl_section, dict):
        raise RuntimeError("NATIVE-COMPONENTS.json has no curl_cffi object")

    platform_key = _platform_key()
    reviewed_components = [CURL_IMPERSONATE] + [
        component for component in CURL_NATIVE_COMPONENTS if platform_key in component["platforms"]
    ]
    packaged_records = [_curl_component_with_notices(runtime_root, component) for component in reviewed_components]
    evidence = _linux_curl_marker_evidence(runtime_root, packaged_records) if platform_key == "linux" else None
    if platform_key == "windows":
        native_names = {Path(relative).name.lower() for relative in (curl_section.get("native_file_sha256") or {})}
        if not any("libcurl-impersonate" in name for name in native_names):
            raise RuntimeError(f"Windows curl-cffi native inventory lost libcurl-impersonate: {sorted(native_names)}")
        if len(native_names) < 2:
            raise RuntimeError(f"Windows curl-cffi native inventory is unexpectedly small: {sorted(native_names)}")

    curl_section["reviewed_native_components"] = packaged_records
    curl_section["platform"] = platform_key
    curl_section["binary_component_evidence"] = evidence
    curl_section["system_provided_libraries"] = WINDOWS_SYSTEM_LIBRARIES if platform_key == "windows" else []
    curl_section["notice_policy"] = (
        "Versioned upstream notice text is copied into the installer and SHA-256 pinned in this manifest; Linux additionally requires reviewed version markers in the statically folded wrapper."
    )
    _write_json(native_path, native)
    _add_curl_notices_to_python_manifest(runtime_root, packaged_records)

    frontend = _frontend_inventory(runtime_root, repo_root)
    _write_json(runtime_root / "FRONTEND-LICENSES.json", frontend)
    _enrich_ffmpeg_provenance(runtime_root)
    _rewrite_generated_notice(runtime_root, frontend, packaged_records, evidence)


def _require_relative_file(runtime_root: Path, relative: str, expected_sha256: str | None = None) -> Path:
    candidate = (runtime_root / relative).resolve()
    try:
        candidate.relative_to(runtime_root.resolve())
    except ValueError as exc:
        raise RuntimeError(f"compliance path escapes runtime root: {relative}") from exc
    if not candidate.is_file() or candidate.stat().st_size == 0:
        raise RuntimeError(f"packaged compliance file is missing or empty: {relative}")
    if expected_sha256:
        actual = _sha256_file(candidate)
        if actual != expected_sha256:
            raise RuntimeError(
                f"packaged compliance file hash drift: {relative} expected {expected_sha256}, got {actual}"
            )
    return candidate


def _validate_curl_enrichment(runtime_root: Path, native: dict[str, Any]) -> None:
    curl = native.get("curl_cffi")
    if not isinstance(curl, dict):
        raise RuntimeError("native manifest has no curl_cffi object")
    platform_key = _platform_key()
    if curl.get("platform") != platform_key:
        raise RuntimeError(f"curl-cffi native manifest platform mismatch: {curl.get('platform')} != {platform_key}")
    records = curl.get("reviewed_native_components")
    if not isinstance(records, list) or not records:
        raise RuntimeError("curl-cffi native manifest has no reviewed native components")
    expected = {
        CURL_IMPERSONATE["name"]: CURL_IMPERSONATE["version"],
        **{
            str(component["name"]): str(component["version"])
            for component in CURL_NATIVE_COMPONENTS
            if platform_key in component["platforms"]
        },
    }
    actual = {
        str(record.get("name")): str(record.get("version"))
        for record in records
        if isinstance(record, dict)
    }
    if actual != expected:
        raise RuntimeError(f"curl-cffi reviewed native component drift: expected {expected}, got {actual}")
    for record in records:
        if not isinstance(record, dict):
            raise RuntimeError("curl-cffi native component record is malformed")
        notices = record.get("notices")
        if not isinstance(notices, list) or not notices:
            raise RuntimeError(f"curl-cffi native component has no packaged notice: {record.get('name')}")
        for notice in notices:
            if not isinstance(notice, dict):
                raise RuntimeError("curl-cffi native notice record is malformed")
            relative = notice.get("path")
            digest = notice.get("sha256")
            if not isinstance(relative, str) or not isinstance(digest, str):
                raise RuntimeError("curl-cffi native notice lacks path/hash")
            _require_relative_file(runtime_root, relative, digest)
    if platform_key == "linux":
        evidence = curl.get("binary_component_evidence")
        if not isinstance(evidence, dict):
            raise RuntimeError("Linux curl-cffi manifest lacks binary component evidence")
        wrapper = _require_relative_file(
            runtime_root,
            str(evidence.get("wrapper_path") or ""),
            str(evidence.get("wrapper_sha256") or ""),
        )
        data = wrapper.read_bytes()
        markers = evidence.get("markers")
        if not isinstance(markers, list) or len(markers) != len(expected) - 1:
            raise RuntimeError(f"Linux curl-cffi marker evidence is incomplete: {markers}")
        for marker in markers:
            value = str((marker or {}).get("marker") or "")
            if not value or value.encode("utf-8") not in data:
                raise RuntimeError(f"Linux curl-cffi component marker is missing: {value}")
    else:
        system = curl.get("system_provided_libraries")
        if system != WINDOWS_SYSTEM_LIBRARIES:
            raise RuntimeError(f"Windows system-library classification drifted: {system}")


def _expected_frontend_inventory(
    repo_root: Path,
) -> tuple[list[str], set[tuple[str, str, str]], dict[str, Any], str, list[str]]:
    roots, closure, packages, method, source_evidence = _frontend_bundle_paths(repo_root)
    expected: set[tuple[str, str, str]] = set()
    for path_key in closure:
        entry = packages[path_key]
        expected.add((path_key, str(entry.get("version") or ""), str(entry.get("integrity") or "")))
    return roots, expected, packages, method, source_evidence


def _validate_frontend_enrichment(runtime_root: Path, repo_root: Path, frontend: dict[str, Any]) -> None:
    roots, expected, _packages, method, source_evidence = _expected_frontend_inventory(repo_root)
    if frontend.get("production_import_roots") != roots:
        raise RuntimeError(
            f"frontend production import roots drifted: expected {roots}, got {frontend.get('production_import_roots')}"
        )
    if frontend.get("inventory_method") != method:
        raise RuntimeError(f"frontend inventory method drifted: expected {method}, got {frontend.get('inventory_method')}")
    if method == "vite-source-map" and frontend.get("vite_source_map_evidence") != source_evidence:
        raise RuntimeError("frontend Vite source-map evidence drifted")
    records = frontend.get("packages")
    if not isinstance(records, list):
        raise RuntimeError("frontend compliance packages is not a list")
    bundled = [
        record
        for record in records
        if isinstance(record, dict) and record.get("bundle_category") == "production-bundled"
    ]
    actual = {
        (str(record.get("lock_path") or ""), str(record.get("version") or ""), str(record.get("integrity") or ""))
        for record in bundled
    }
    if actual != expected:
        raise RuntimeError(
            f"frontend production bundle closure drifted: expected {sorted(expected)}, got {sorted(actual)}"
        )
    if frontend.get("production_package_count") != len(expected):
        raise RuntimeError("frontend production_package_count does not match locked closure")
    for record in bundled:
        license_expression = str(record.get("license") or "")
        if not license_expression:
            raise RuntimeError(f"frontend bundled package lacks license metadata: {record.get('name')}")
        if any(marker.lower() in license_expression.lower() for marker in PROHIBITED_LICENSE_MARKERS):
            raise RuntimeError(
                f"frontend bundled package uses a blocked/unreviewed license: {record.get('name')} {license_expression}"
            )
        notices = record.get("license_files")
        if not isinstance(notices, list) or not notices:
            raise RuntimeError(f"frontend bundled package has no packaged license notice: {record.get('name')}")
        for notice in notices:
            if not isinstance(notice, dict):
                raise RuntimeError("frontend license notice record is malformed")
            relative = notice.get("path")
            digest = notice.get("sha256")
            if not isinstance(relative, str) or not isinstance(digest, str):
                raise RuntimeError("frontend license notice lacks path/hash")
            _require_relative_file(runtime_root, relative, digest)


def _validate_ffmpeg_enrichment(runtime_root: Path, provenance: dict[str, Any]) -> None:
    standalone = provenance.get("standalone_ffmpeg")
    if not isinstance(standalone, dict):
        raise RuntimeError("runtime provenance lacks standalone_ffmpeg")
    source = standalone.get("core_source")
    if not isinstance(source, dict):
        raise RuntimeError("standalone FFmpeg core source provenance is missing")
    platform_key = _platform_key()
    if platform_key == "linux":
        revision = standalone.get("upstream_ffmpeg_revision_from_binary")
        if source.get("revision") != revision or not revision:
            raise RuntimeError("Linux standalone FFmpeg core source revision does not match shipped binary")
    else:
        if source.get("revision") != WINDOWS_FFMPEG_CORE["revision"]:
            raise RuntimeError(f"Windows standalone FFmpeg core source revision drifted: {source}")
    corresponding = standalone.get("corresponding_source")
    if not isinstance(corresponding, dict):
        raise RuntimeError("standalone FFmpeg Corresponding Source status is missing")
    if corresponding.get("status") != "partial-core-source-provenance-only":
        raise RuntimeError(
            f"standalone FFmpeg Corresponding Source status drifted: {corresponding.get('status')}"
        )
    if corresponding.get("technically_resolved") is not False:
        raise RuntimeError("standalone FFmpeg provenance must not claim full Corresponding Source is resolved")


def validate_compliance_enrichment(runtime_root: Path, repo_root: Path = REPO_ROOT) -> None:
    runtime_root = runtime_root.resolve()
    repo_root = repo_root.resolve()
    native = _read_json(runtime_root / "NATIVE-COMPONENTS.json")
    frontend = _read_json(runtime_root / "FRONTEND-LICENSES.json")
    provenance = _read_json(runtime_root / "RUNTIME-PROVENANCE.json")
    _validate_curl_enrichment(runtime_root, native)
    _validate_frontend_enrichment(runtime_root, repo_root, frontend)
    _validate_ffmpeg_enrichment(runtime_root, provenance)


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Generate or validate Scriptotar distribution compliance enrichment."
    )
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--write", type=Path)
    mode.add_argument("--validate", type=Path)
    parser.add_argument("--repo-root", type=Path, default=REPO_ROOT)
    args = parser.parse_args()
    if args.write:
        write_compliance_enrichment(args.write, args.repo_root)
        print(f"wrote compliance enrichment: {args.write.resolve()}")
    else:
        validate_compliance_enrichment(args.validate, args.repo_root)
        print(f"validated compliance enrichment: {args.validate.resolve()}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
