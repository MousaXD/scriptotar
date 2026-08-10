from __future__ import annotations

import hashlib
import importlib.metadata as metadata
import json
import platform
import re
import shutil
import subprocess
import sys
import tempfile
import tomllib
import urllib.request
import zipfile
from pathlib import Path
from typing import Any

from static_ffmpeg import run as static_ffmpeg_run

SCHEMA_VERSION = 1
FFMPEG_BINS_GENERATION_COMMIT = "df95abcb0ce6efff710dda5ef28a2f6f1dc21493"
STATIC_FFMPEG_WORKFLOW_RUN = 21077413996
STATIC_FFMPEG_WORKFLOW_REVISION = "1d859d5285e966fb01bb95cd980407621afd5b79"

# static-ffmpeg 3.0 maps these platforms to zackees/ffmpeg_bins v8.0. The
# expected digests are the Git LFS OIDs committed for the provider archives.
# The same ffmpeg_bins commit records that both archives were rebuilt from the
# static_ffmpeg workflow run below. That workflow's platform matrix identifies
# BtbN as the Linux x64 provider and gyan.dev as the Windows x64 provider.
STATIC_FFMPEG_ARCHIVES = {
    "linux": {
        "url": "https://github.com/zackees/ffmpeg_bins/raw/main/v8.0/linux.zip",
        "sha256": "ca75b05e887c7a97676632f673031875847be83daa9794298fed9cef8cac14ad",
        "size": 142008975,
        "git_lfs_pointer_blob": "f833867af9e7631772f23e1f9b8b4ba9fca05fd0",
        "upstream_provider": "BtbN/FFmpeg-Builds",
        "upstream_provider_url": "https://github.com/BtbN/FFmpeg-Builds",
        "upstream_selection": "static_ffmpeg download-binaries workflow linux/x64 resolver selects the latest versioned GPL BtbN release asset available before the workflow run date",
    },
    "win32": {
        "url": "https://github.com/zackees/ffmpeg_bins/raw/main/v8.0/win32.zip",
        "sha256": "92662c2241e93fe71b3f3a01e94a0b0dc8cfad726019f96b83bc109ce44c5d0b",
        "size": 72065209,
        "git_lfs_pointer_blob": "39673f4510bc9fb54e7d2d3300a462299394c0c7",
        "upstream_provider": "gyan.dev FFmpeg builds",
        "upstream_provider_url": "https://www.gyan.dev/ffmpeg/builds/",
        "upstream_selection": "static_ffmpeg download-binaries workflow windows/x64 resolver downloads https://www.gyan.dev/ffmpeg/builds/packages/ffmpeg-{requested_version}-essentials_build.zip",
    },
}

PYAV_VERSION = "18.0.0"
PYAV_BUILD_RECIPE = {
    "pyav_tag": "v18.0.0",
    "pyav_recipe_file": "scripts/ffmpeg-8.1.json",
    "binary_release": "https://github.com/PyAV-Org/pyav-ffmpeg/releases/tag/8.1.2-1",
    "build_repository": "https://github.com/PyAV-Org/pyav-ffmpeg",
    "build_recipe_revision": "cf545d99347ea17cd00ac72f0a3c3cb137399eca",
}

# Source URLs and hashes are copied from PyAV-Org/pyav-ffmpeg at the reviewed
# recipe revision above. Header/tool inputs stay in the list because they are
# needed to reconstruct the wheel build even when they are not separate runtime
# shared libraries.
PYAV_SOURCE_COMPONENTS = [
    {"name": "ffmpeg", "source_url": "https://ffmpeg.org/releases/ffmpeg-8.1.2.tar.xz", "sha256": "464beb5e7bf0c311e68b45ae2f04e9cc2af88851abb4082231742a74d97b524c", "license": "LGPL-2.1-or-later; audited build reports LGPL-3.0-or-later with --enable-version3"},
    {"name": "lame", "source_url": "http://deb.debian.org/debian/pool/main/l/lame/lame_3.100.orig.tar.gz", "sha256": "ddfe36cab873794038ae2c1210557ad34857a4b6bdc515785d1da9e175b1da1e", "license": "LGPL-2.0-or-later"},
    {"name": "opus", "source_url": "https://ftp.osuosl.org/pub/xiph/releases/opus/opus-1.6.1.tar.gz", "sha256": "6ffcb593207be92584df15b32466ed64bbec99109f007c82205f0194572411a1", "license": "BSD-3-Clause"},
    {"name": "dav1d", "source_url": "https://code.videolan.org/videolan/dav1d/-/archive/1.5.3/dav1d-1.5.3.tar.bz2", "sha256": "e099f53253f6c247580c554d53a13f1040638f2066edc3c740e4c2f15174ce22", "license": "BSD-2-Clause"},
    {"name": "libsvtav1", "source_url": "https://gitlab.com/AOMediaCodec/SVT-AV1/-/archive/v4.1.0/SVT-AV1-v4.1.0.tar.bz2", "sha256": "184162d3db3a4448882b17230413b4938ca252eef6b3c5e2f1236b2fcf497881", "license": "BSD-3-Clause-Clear plus AOM patent license"},
    {"name": "vpx", "source_url": "https://github.com/webmproject/libvpx/archive/refs/tags/v1.16.0.tar.gz", "sha256": "7a479a3c66b9f5d5542a4c6a1b7d3768a983b1e5c14c60a9396edc9b649e015c", "license": "BSD-3-Clause"},
    {"name": "png", "source_url": "https://downloads.sourceforge.net/project/libpng/libpng16/1.6.58/libpng-1.6.58.tar.xz", "sha256": "28eb403f51f0f7405249132cecfe82ea5c0ef97f1b32c5a65828814ae0d34775", "license": "libpng-2.0"},
    {"name": "webp", "source_url": "https://github.com/webmproject/libwebp/archive/refs/tags/v1.6.0.tar.gz", "sha256": "93a852c2b3efafee3723efd4636de855b46f9fe1efddd607e1f42f60fc8f2136", "license": "BSD-3-Clause"},
    {"name": "opencore-amr", "source_url": "https://downloads.sourceforge.net/project/opencore-amr/opencore-amr/opencore-amr-0.1.6.tar.gz", "sha256": "483eb4061088e2b34b358e47540b5d495a96cd468e361050fae615b1809dc4a1", "license": "Apache-2.0"},
    {"name": "x264", "source_url": "https://code.videolan.org/videolan/x264/-/archive/b35605ace3ddf7c1a5d67a2eb553f034aef41d55/x264-b35605ace3ddf7c1a5d67a2eb553f034aef41d55.tar.bz2", "sha256": "6eeb82934e69fd51e043bd8c5b0d152839638d1ce7aa4eea65a3fedcf83ff224", "license": "GPL-2.0-or-later"},
    {"name": "x265", "source_url": "https://bitbucket.org/multicoreware/x265_git/downloads/x265_4.2.tar.gz", "sha256": "40b1ea0453e0309f0eba934e0ddf533f8f6295966679e8894e8f1c1c8d5e1210", "license": "GPL-2.0-or-later or commercial"},
    {"name": "gmp", "source_url": "https://ftp.gnu.org/gnu/gmp/gmp-6.3.0.tar.xz", "sha256": "a3c2b80201b89e68616f4ad30bc66aee4927c3ce50e33929ca819d5c43538898", "license": "LGPL-3.0-or-later OR GPL-2.0-or-later"},
    {"name": "unistring", "source_url": "https://ftp.gnu.org/gnu/libunistring/libunistring-1.4.2.tar.gz", "sha256": "e82664b170064e62331962126b259d452d53b227bb4a93ab20040d846fec01d8", "license": "LGPL-3.0-or-later OR GPL-2.0-or-later"},
    {"name": "nettle", "source_url": "https://ftp.gnu.org/gnu/nettle/nettle-3.10.2.tar.gz", "sha256": "fe9ff51cb1f2abb5e65a6b8c10a92da0ab5ab6eaf26e7fc2b675c45f1fb519b5", "license": "LGPL-3.0-or-later OR GPL-2.0-or-later"},
    {"name": "gnutls", "source_url": "https://www.gnupg.org/ftp/gcrypt/gnutls/v3.8/gnutls-3.8.13.tar.xz", "sha256": "ffed8ec1bf09c2426d4f14aae377de4753b53e537d685e604e99a8b16ca9c97e", "license": "LGPL-2.1-or-later for the library; separately licensed parts exist"},
    {"name": "alsa-lib", "source_url": "https://www.alsa-project.org/files/pub/lib/alsa-lib-1.2.14.tar.bz2", "sha256": "be9c88a0b3604367dd74167a2b754a35e142f670292ae47a2fdef27a2ee97a32", "license": "LGPL-2.1-or-later"},
    {"name": "libvpl", "source_url": "https://github.com/intel/libvpl/archive/refs/tags/v2.16.0.tar.gz", "sha256": "d60931937426130ddad9f1975c010543f0da99e67edb1c6070656b7947f633b6", "license": "MIT"},
    {"name": "nv-codec-headers", "source_url": "https://github.com/FFmpeg/nv-codec-headers/archive/refs/tags/n13.0.19.0.tar.gz", "sha256": "86d15d1a7c0ac73a0eafdfc57bebfeba7da8264595bf531cf4d8db1c22940116", "license": "MIT; build/header input"},
    {"name": "amf-headers", "source_url": "https://github.com/GPUOpen-LibrariesAndSDKs/AMF/releases/download/v1.5.0/AMF-headers-v1.5.0.tar.gz", "sha256": "d569647fa26f289affe81a206259fa92f819d06db1e80cc334559953e82a3f01", "license": "MIT; build/header input"},
    {"name": "nasm", "source_url": "https://www.nasm.us/pub/nasm/releasebuilds/2.16.03/nasm-2.16.03.tar.xz", "sha256": "1412a1c760bbd05db026b6c0d1657affd6631cd0a63cddb6f73cc6d4aa616148", "license": "BSD-2-Clause; build tool"},
]

CURL_CFFI_VERSION = "0.15.0"
CURL_CFFI_NATIVE_RECIPE = {
    "curl_cffi_source": "https://github.com/lexiforest/curl_cffi/tree/v0.15.0",
    "curl_cffi_license": "MIT",
    "curl_impersonate_version": "1.5.2",
    "curl_impersonate_source": "https://github.com/lexiforest/curl-impersonate/tree/v1.5.2",
    "curl_version": "8.15.0",
    "curl_source": "https://github.com/curl/curl/tree/curl-8_15_0",
    "build_note": "curl-cffi 0.15.0 downloads libcurl-impersonate v1.5.2. Linux links its release mega archive statically into _wrapper; Windows links the release DLL and native libraries.",
}

# Deliberately a small denylist, not an arbitrary allowlist. Unknown metadata
# also fails. These markers require explicit project review before being folded
# into production Rust/frontend code.
PROHIBITED_LICENSE_MARKERS = (
    "AGPL-",
    "SSPL-",
    "BUSL-",
    "Business-Source-License",
    "Commons-Clause",
    "LicenseRef-",
    "UNLICENSED",
)
LICENSE_FILE_PREFIXES = ("license", "licence", "copying", "notice", "copyright")


def _sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _json_write(path: Path, payload: object) -> None:
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def fetch_pinned_static_ffmpeg(destination: Path) -> tuple[Path, Path, dict[str, Any]]:
    key = static_ffmpeg_run.get_platform_key()
    spec = STATIC_FFMPEG_ARCHIVES.get(key)
    if not spec:
        raise RuntimeError(f"no reviewed FFmpeg archive baseline for packaging platform: {key}")
    url = static_ffmpeg_run.get_platform_http_zip()
    if url != spec["url"]:
        raise RuntimeError(f"static-ffmpeg provider URL drifted for {key}: {url}")

    destination.mkdir(parents=True, exist_ok=True)
    archive = destination / f"{key}.zip"
    with urllib.request.urlopen(url, timeout=600) as response, archive.open("wb") as output:
        shutil.copyfileobj(response, output, length=1024 * 1024)
    actual_hash = _sha256(archive)
    if actual_hash != spec["sha256"]:
        raise RuntimeError(
            f"static FFmpeg archive checksum drift for {key}: expected {spec['sha256']}, got {actual_hash}"
        )
    if archive.stat().st_size != spec["size"]:
        raise RuntimeError(
            f"static FFmpeg archive size drift for {key}: expected {spec['size']}, got {archive.stat().st_size}"
        )

    extract_root = destination / "extracted"
    with zipfile.ZipFile(archive) as payload:
        payload.extractall(extract_root)
    executable_dir = extract_root / key
    suffix = ".exe" if key == "win32" else ""
    ffmpeg = executable_dir / f"ffmpeg{suffix}"
    ffprobe = executable_dir / f"ffprobe{suffix}"
    if not ffmpeg.is_file() or not ffprobe.is_file():
        raise RuntimeError(f"verified FFmpeg archive lacks expected executables: {executable_dir}")
    if key != "win32":
        ffmpeg.chmod(ffmpeg.stat().st_mode | 0o555)
        ffprobe.chmod(ffprobe.stat().st_mode | 0o555)
    return ffmpeg, ffprobe, {
        "provider": "zackees/ffmpeg_bins via static-ffmpeg 3.0 URL mapping",
        "platform_key": key,
        "download_url": url,
        "archive_sha256": actual_hash,
        "archive_size": archive.stat().st_size,
        "git_lfs_oid_sha256": spec["sha256"],
        "git_lfs_pointer_blob": spec["git_lfs_pointer_blob"],
        "ffmpeg_bins_generation_commit": FFMPEG_BINS_GENERATION_COMMIT,
        "static_ffmpeg_workflow_run": STATIC_FFMPEG_WORKFLOW_RUN,
        "static_ffmpeg_workflow_revision": STATIC_FFMPEG_WORKFLOW_REVISION,
        "upstream_provider": spec["upstream_provider"],
        "upstream_provider_url": spec["upstream_provider_url"],
        "upstream_selection": spec["upstream_selection"],
        "checksum_verified": True,
    }


def _download_installed_wheel(project: str) -> dict[str, Any]:
    dist = metadata.distribution(project)
    with tempfile.TemporaryDirectory(prefix=f"scriptotar-{project}-wheel-") as raw:
        destination = Path(raw)
        subprocess.run(
            [
                sys.executable,
                "-m",
                "pip",
                "download",
                "--disable-pip-version-check",
                "--only-binary=:all:",
                "--no-deps",
                "--dest",
                str(destination),
                f"{project}=={dist.version}",
            ],
            check=True,
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            timeout=300,
        )
        wheels = sorted(destination.glob("*.whl"))
        if len(wheels) != 1:
            raise RuntimeError(f"expected one wheel for {project}=={dist.version}, found {[p.name for p in wheels]}")
        wheel = wheels[0]
        return {
            "project": project,
            "version": dist.version,
            "filename": wheel.name,
            "sha256": _sha256(wheel),
            "size": wheel.stat().st_size,
            "resolver": "python -m pip download --only-binary=:all: --no-deps",
        }


def _distribution_native_files(project: str) -> dict[str, str]:
    dist = metadata.distribution(project)
    result: dict[str, str] = {}
    for relative in dist.files or []:
        relative_text = str(relative).replace("\\", "/")
        lower = relative_text.lower()
        if not lower.endswith((".so", ".dll", ".dylib", ".pyd")) and ".so." not in lower:
            continue
        candidate = Path(dist.locate_file(relative))
        if candidate.is_file():
            result[relative_text] = _sha256(candidate)
    return dict(sorted(result.items()))


def _extract_ffmpeg_revision(version_line: str) -> str | None:
    match = re.search(r"-g([0-9a-f]{7,40})(?:-|\s|$)", version_line, re.IGNORECASE)
    return match.group(1) if match else None


def _copy_license_candidates(package_root: Path, destination: Path, runtime_root: Path) -> list[str]:
    if not package_root.is_dir():
        return []
    candidates = [
        child
        for child in package_root.iterdir()
        if child.is_file() and child.name.lower().startswith(LICENSE_FILE_PREFIXES)
    ]
    copied: list[str] = []
    for index, source in enumerate(sorted(candidates), start=1):
        destination.mkdir(parents=True, exist_ok=True)
        target = destination / f"{index:02d}-{source.name}"
        shutil.copy2(source, target)
        copied.append(str(target.relative_to(runtime_root)).replace("\\", "/"))
    return copied


def _cargo_checksums(repo_root: Path) -> dict[tuple[str, str, str], str]:
    lock = tomllib.loads((repo_root / "Cargo.lock").read_text(encoding="utf-8"))
    result: dict[tuple[str, str, str], str] = {}
    for package in lock.get("package", []):
        checksum = package.get("checksum")
        source = package.get("source")
        if checksum and source:
            result[(str(package["name"]), str(package["version"]), str(source))] = str(checksum)
    return result


def _rust_inventory(repo_root: Path, runtime_root: Path) -> dict[str, Any]:
    rustc_completed = subprocess.run(
        ["rustc", "-vV"],
        check=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        timeout=30,
    )
    rustc_output = rustc_completed.stdout.decode("utf-8", errors="strict")
    host = next((line.split(":", 1)[1].strip() for line in rustc_output.splitlines() if line.startswith("host:")), None)
    if not host:
        raise RuntimeError("could not determine Rust host target")

    metadata_completed = subprocess.run(
        ["cargo", "metadata", "--locked", "--format-version", "1", "--filter-platform", host],
        cwd=repo_root,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        timeout=600,
    )
    if metadata_completed.returncode != 0:
        stderr = metadata_completed.stderr.decode("utf-8", errors="replace")
        raise RuntimeError(f"cargo metadata --locked failed ({metadata_completed.returncode}): {stderr}")
    try:
        data = json.loads(metadata_completed.stdout.decode("utf-8", errors="strict"))
    except (UnicodeDecodeError, json.JSONDecodeError) as exc:
        stderr = metadata_completed.stderr.decode("utf-8", errors="replace")
        raise RuntimeError(f"cargo metadata produced invalid UTF-8 JSON; stderr={stderr}") from exc

    packages = {package["id"]: package for package in data.get("packages", [])}
    nodes = {node["id"]: node for node in (data.get("resolve") or {}).get("nodes", [])}
    desktop_ids = [pid for pid, package in packages.items() if package.get("name") == "scriptotar-desktop"]
    if len(desktop_ids) != 1:
        raise RuntimeError(f"expected one scriptotar-desktop package, found {desktop_ids}")

    categories: dict[str, str] = {desktop_ids[0]: "runtime"}
    queue = [desktop_ids[0]]
    while queue:
        current = queue.pop(0)
        current_category = categories[current]
        for dep in (nodes.get(current) or {}).get("deps", []):
            dep_id = dep.get("pkg")
            if dep_id not in packages:
                continue
            kinds = {kind.get("kind") or "normal" for kind in dep.get("dep_kinds", [])} or {"normal"}
            if kinds == {"dev"}:
                continue
            candidate = current_category
            if current_category == "runtime" and "normal" not in kinds:
                candidate = "build-only"
            targets = packages[dep_id].get("targets") or []
            if targets and all("proc-macro" in target.get("kind", []) for target in targets):
                candidate = "build-only"
            previous = categories.get(dep_id)
            if previous == "runtime" or previous == candidate:
                continue
            categories[dep_id] = candidate
            queue.append(dep_id)

    checksums = _cargo_checksums(repo_root)
    records = []
    for package_id, category in sorted(
        categories.items(), key=lambda item: (packages[item[0]]["name"], packages[item[0]]["version"])
    ):
        package = packages[package_id]
        license_expression = str(package.get("license") or "")
        if not license_expression:
            raise RuntimeError(f"Cargo package has no license metadata: {package['name']} {package['version']}")
        if category == "runtime" and any(
            marker.lower() in license_expression.lower() for marker in PROHIBITED_LICENSE_MARKERS
        ):
            raise RuntimeError(
                f"Cargo runtime package license requires explicit review: {package['name']} {package['version']} {license_expression}"
            )
        manifest_path = Path(package["manifest_path"])
        source = str(package.get("source") or "workspace")
        copied = []
        if category == "runtime" and source != "workspace":
            copied = _copy_license_candidates(
                manifest_path.parent,
                runtime_root / "legal" / "rust" / f"{package['name']}-{package['version']}",
                runtime_root,
            )
        records.append(
            {
                "name": package["name"],
                "version": package["version"],
                "category": category,
                "license": license_expression,
                "license_file_metadata": package.get("license_file"),
                "copied_license_files": copied,
                "source": source,
                "repository": package.get("repository"),
                "cargo_lock_checksum": checksums.get((package["name"], package["version"], source)),
            }
        )
    return {
        "schema": SCHEMA_VERSION,
        "source_of_truth": "cargo metadata --locked over Cargo.lock",
        "target": host,
        "policy": {
            "unknown_license_metadata": "fail",
            "runtime_prohibited_markers": list(PROHIBITED_LICENSE_MARKERS),
            "dev_dependencies": "excluded",
            "build_and_proc_macro_dependencies": "recorded as build-only",
        },
        "packages": records,
    }


def _npm_name(path_key: str, entry: dict[str, Any]) -> str:
    if entry.get("name"):
        return str(entry["name"])
    return path_key.rsplit("node_modules/", 1)[-1]


def _frontend_inventory(repo_root: Path) -> dict[str, Any]:
    package_path = repo_root / "apps" / "desktop-ui" / "package.json"
    lock_path = repo_root / "apps" / "desktop-ui" / "package-lock.json"
    package_json = json.loads(package_path.read_text(encoding="utf-8"))
    declared_runtime_dependencies = sorted((package_json.get("dependencies") or {}).keys())
    declared_dev_dependencies = sorted((package_json.get("devDependencies") or {}).keys())
    lock = json.loads(lock_path.read_text(encoding="utf-8"))
    if lock.get("lockfileVersion") != 3 or not isinstance(lock.get("packages"), dict):
        raise RuntimeError("frontend inventory requires package-lock.json lockfileVersion 3 packages map")
    records = []
    for path_key, entry in sorted(lock["packages"].items()):
        if not path_key or not isinstance(entry, dict):
            continue
        name = _npm_name(path_key, entry)
        version = str(entry.get("version") or "")
        license_expression = str(entry.get("license") or "")
        if not version:
            raise RuntimeError(f"frontend lock entry has no version: {path_key}")
        if not license_expression:
            raise RuntimeError(f"frontend lock entry has no license metadata: {name} {version}")
        category = "build-only" if entry.get("dev") is True else "production"
        if category == "production" and any(
            marker.lower() in license_expression.lower() for marker in PROHIBITED_LICENSE_MARKERS
        ):
            raise RuntimeError(
                f"frontend production package license requires explicit review: {name} {version} {license_expression}"
            )
        records.append(
            {
                "name": name,
                "version": version,
                "category": category,
                "license": license_expression,
                "resolved": entry.get("resolved"),
                "integrity": entry.get("integrity"),
                "optional": bool(entry.get("optional")),
            }
        )

    production_names = {record["name"] for record in records if record["category"] == "production"}
    missing_direct = sorted(set(declared_runtime_dependencies) - production_names)
    if missing_direct:
        raise RuntimeError(f"frontend runtime dependencies are missing from production lock closure: {missing_direct}")
    return {
        "schema": SCHEMA_VERSION,
        "source_of_truth": "apps/desktop-ui/package.json plus package-lock.json lockfileVersion 3",
        "declared_runtime_dependencies": declared_runtime_dependencies,
        "declared_dev_dependencies": declared_dev_dependencies,
        "production_package_count": len(production_names),
        "policy": {
            "unknown_license_metadata": "fail",
            "production_prohibited_markers": list(PROHIBITED_LICENSE_MARKERS),
            "dev_true": "build-only",
            "empty_production_closure": "valid only when package.json declares no dependencies",
        },
        "packages": records,
    }


def _generated_notice(
    runtime_manifest: dict[str, Any],
    provenance: dict[str, Any],
    rust_inventory: dict[str, Any],
    frontend_inventory: dict[str, Any],
    native_inventory: dict[str, Any],
) -> str:
    rust_runtime = [p for p in rust_inventory["packages"] if p["category"] == "runtime"]
    rust_build = [p for p in rust_inventory["packages"] if p["category"] == "build-only"]
    frontend_prod = [p for p in frontend_inventory["packages"] if p["category"] == "production"]
    frontend_build = [p for p in frontend_inventory["packages"] if p["category"] == "build-only"]
    python_components = runtime_manifest.get("python_components") or []
    standalone = provenance["standalone_ffmpeg"]
    pyav = native_inventory["pyav"]
    curl = native_inventory["curl_cffi"]
    return "\n".join(
        [
            "# Scriptotar generated third-party notices",
            "",
            "This file is generated from the exact package build. Machine-readable inventories beside it are authoritative if this summary and an inventory disagree.",
            "",
            "## Bundled in the installer",
            "",
            f"- Rust/Tauri shipped closure: {len(rust_runtime)} Cargo packages. See `RUST-LICENSES.json` and `legal/rust/`.",
            f"- Production frontend npm dependency closure: {len(frontend_prod)} packages. This project currently builds the UI entirely from dev/build dependencies, so zero is valid when `package.json` has no `dependencies`. See `FRONTEND-LICENSES.json`.",
            f"- Packaged Python closure: {len(python_components)} distributions. See `RUNTIME-LICENSES.json` and `legal/python/`.",
            f"- Standalone FFmpeg/ffprobe archive SHA-256: `{standalone['archive']['archive_sha256']}` from `{standalone['archive']['download_url']}`; upstream provider `{standalone['archive']['upstream_provider']}`; effective FFmpeg license `{standalone['runtime']['license']}`.",
            f"- PyAV {pyav['wheel']['version']} wheel `{pyav['wheel']['filename']}` SHA-256 `{pyav['wheel']['sha256']}`. Its native payload is component-inventoried in `NATIVE-COMPONENTS.json`.",
            f"- curl-cffi {curl['wheel']['version']} wheel `{curl['wheel']['filename']}` SHA-256 `{curl['wheel']['sha256']}`. Its native payload and curl-impersonate lineage are in `NATIVE-COMPONENTS.json`.",
            "- CPython/PyInstaller notices are under `legal/python-runtime/` and `legal/build-tools/`.",
            "- GNU GPLv3/LGPLv3 texts are `legal/FFMPEG-GPL-3.0.txt` and `legal/FFMPEG-LGPL-3.0.txt`.",
            "",
            "## Build-only, not copied as tools",
            "",
            f"- Cargo build/proc-macro closure: {len(rust_build)} packages.",
            f"- npm development/build closure: {len(frontend_build)} packages.",
            "- Rust/Cargo, Node/npm, Vite/TypeScript CLIs, PyInstaller CLI and static-ffmpeg are build tooling; generated/runtime material is inventoried where shipped.",
            "",
            "## System-provided",
            "",
            "- Linux WebKitGTK/GTK and Debian runtime dependencies are supplied by the OS package manager.",
            "- Windows WebView/platform components are supplied by Windows rather than the transcription-runtime resources.",
            "",
            "## Downloaded after installation",
            "",
            "- Whisper model weights and user-requested remote media are not installer payloads.",
            "",
            "## Evidence and remaining legal interpretation",
            "",
            "- The standalone FFmpeg archive is byte-pinned and its binaries/configuration are hashed. Its ffmpeg_bins generation commit and static_ffmpeg workflow run identify BtbN as the Linux x64 upstream provider and gyan.dev as the Windows x64 upstream provider. The current provider chain still does not publish a complete build-to-Corresponding-Source mapping for every statically linked GPL component, so the provenance manifest marks that source-delivery status unresolved instead of presenting a homepage link as proof.",
            "- PyAV source/build coordinates are pinned to PyAV 18.0.0 and pyav-ffmpeg's FFmpeg 8.1.2 recipe. The wheel's FFmpeg core reports LGPLv3, while separately bundled x264/x265 libraries are GPL components; LGPL replacement/relinking and GPL source-delivery treatment require distribution-policy review.",
            "- curl-cffi native files are hash-pinned and the exact curl 8.15.0 / curl-impersonate 1.5.2 build lineage is recorded. curl-cffi's top-level MIT metadata is not used as a blanket license for linked native dependencies.",
            "",
            "This engineering inventory is not a legal opinion.",
            "",
        ]
    )


def write_distribution_compliance_bundle(
    runtime_root: Path,
    repo_root: Path,
    ffmpeg_fetch: dict[str, Any],
) -> None:
    runtime_root = runtime_root.resolve()
    repo_root = repo_root.resolve()
    runtime_manifest = json.loads((runtime_root / "RUNTIME-LICENSES.json").read_text(encoding="utf-8"))
    standalone_record = runtime_manifest.get("ffmpeg") or {}
    version_line = str(standalone_record.get("version_line") or "")
    upstream_revision = _extract_ffmpeg_revision(version_line)
    provenance = {
        "schema": SCHEMA_VERSION,
        "platform": {"system": platform.system(), "machine": platform.machine()},
        "standalone_ffmpeg": {
            "fetcher_version": metadata.version("static-ffmpeg"),
            "archive": ffmpeg_fetch,
            "runtime": standalone_record,
            "upstream_ffmpeg_revision_from_binary": upstream_revision,
            "upstream_ffmpeg_revision_url": (
                f"https://github.com/FFmpeg/FFmpeg/commit/{upstream_revision}" if upstream_revision else None
            ),
            "corresponding_source": {
                "status": "provider-lineage-incomplete",
                "reason": "The archive is byte-pinned and the immediate upstream provider is identified, but the current provider chain does not map the build to complete source/build inputs for every statically linked dependency.",
            },
        },
    }

    pyav_dist = metadata.distribution("av")
    if pyav_dist.version != PYAV_VERSION:
        raise RuntimeError(f"PyAV changed from reviewed version {PYAV_VERSION}: {pyav_dist.version}")
    pyav_native = _distribution_native_files("av")
    if not pyav_native:
        raise RuntimeError("PyAV distribution contains no native library payload")

    curl_dist = metadata.distribution("curl-cffi")
    if curl_dist.version != CURL_CFFI_VERSION:
        raise RuntimeError(f"curl-cffi changed from reviewed version {CURL_CFFI_VERSION}: {curl_dist.version}")
    curl_native = _distribution_native_files("curl-cffi")
    if not curl_native:
        raise RuntimeError("curl-cffi distribution contains no native payload")

    native_inventory = {
        "schema": SCHEMA_VERSION,
        "pyav": {
            "wheel": _download_installed_wheel("av"),
            "native_file_sha256": pyav_native,
            "ffmpeg_runtime_report": runtime_manifest.get("pyav_ffmpeg"),
            "build_recipe": PYAV_BUILD_RECIPE,
            "source_components": PYAV_SOURCE_COMPONENTS,
            "license_scope_note": "The FFmpeg core report does not replace licenses of separately bundled native libraries such as x264/x265.",
            "relinking_note": "The wheel conveys shared FFmpeg/native libraries. LGPL installation/replacement/relinking compliance is a legal/distribution interpretation, not inferred by this generator.",
        },
        "curl_cffi": {
            "wheel": _download_installed_wheel("curl-cffi"),
            "native_file_sha256": curl_native,
            "recipe": CURL_CFFI_NATIVE_RECIPE,
            "license_scope_note": "curl-cffi's MIT metadata covers curl-cffi itself; linked curl-impersonate/native dependencies retain their own licenses.",
        },
    }
    rust_inventory = _rust_inventory(repo_root, runtime_root)
    frontend_inventory = _frontend_inventory(repo_root)

    _json_write(runtime_root / "RUNTIME-PROVENANCE.json", provenance)
    _json_write(runtime_root / "NATIVE-COMPONENTS.json", native_inventory)
    _json_write(runtime_root / "RUST-LICENSES.json", rust_inventory)
    _json_write(runtime_root / "FRONTEND-LICENSES.json", frontend_inventory)
    (runtime_root / "legal" / "THIRD_PARTY_NOTICES.md").write_text(
        _generated_notice(runtime_manifest, provenance, rust_inventory, frontend_inventory, native_inventory),
        encoding="utf-8",
    )
