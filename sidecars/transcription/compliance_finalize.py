from __future__ import annotations

import argparse
import json
import shutil
import subprocess
from pathlib import Path

from compliance_enrichment import (
    REPO_ROOT,
    _read_json,
    _validate_curl_enrichment,
    _validate_ffmpeg_enrichment,
    _validate_frontend_enrichment,
)
from distribution_compliance import _frontend_inventory as _lockfile_frontend_inventory

BUNDLE_FRONTEND_MANIFEST = "FRONTEND-BUNDLE-LICENSES.json"


def _write_json(path: Path, payload: object) -> None:
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def prepare_frontend_bundle(repo_root: Path = REPO_ROOT) -> None:
    """Produce current Vite source maps before the runtime compliance pass."""
    repo_root = repo_root.resolve()
    frontend_root = repo_root / "apps" / "desktop-ui"
    package_lock = frontend_root / "package-lock.json"
    if not package_lock.is_file():
        raise RuntimeError(f"frontend package lock is missing: {package_lock}")

    npm = shutil.which("npm.cmd") or shutil.which("npm")
    if not npm:
        raise RuntimeError(
            "npm is required to prove the production frontend bundle before packaging"
        )
    subprocess.run([npm, "ci"], cwd=frontend_root, check=True, timeout=600)
    subprocess.run([npm, "run", "build"], cwd=frontend_root, check=True, timeout=600)
    maps = sorted((frontend_root / "dist").rglob("*.map"))
    if not maps:
        raise RuntimeError(
            "Vite build produced no source maps; vite.config.ts must keep build.sourcemap enabled for license provenance"
        )


def finalize_compliance_bundle(
    runtime_root: Path,
    repo_root: Path = REPO_ROOT,
) -> None:
    """Preserve lock metadata while making exact Vite output inventory primary."""
    runtime_root = runtime_root.resolve()
    repo_root = repo_root.resolve()
    enriched_path = runtime_root / "FRONTEND-LICENSES.json"
    enriched = _read_json(enriched_path)
    if enriched.get("inventory_method") != "vite-source-map":
        raise RuntimeError(
            f"release compliance requires Vite source-map inventory, got {enriched.get('inventory_method')}"
        )
    _write_json(runtime_root / BUNDLE_FRONTEND_MANIFEST, enriched)

    # validate_runtime.py predates output-graph inventory and intentionally keeps
    # checking the lock/declaration graph as a separate signal. Restore that
    # representation after preserving the exact production bundle above.
    legacy = _lockfile_frontend_inventory(repo_root)
    _write_json(enriched_path, legacy)

    notice_path = runtime_root / "legal" / "THIRD_PARTY_NOTICES.md"
    notice = notice_path.read_text(encoding="utf-8")
    notice = notice.replace(
        "See `FRONTEND-LICENSES.json` and `legal/frontend/`.",
        f"See `{BUNDLE_FRONTEND_MANIFEST}`, `FRONTEND-LICENSES.json` and `legal/frontend/`.",
    )
    if BUNDLE_FRONTEND_MANIFEST not in notice:
        raise RuntimeError(
            "generated notice did not reference the exact frontend bundle inventory"
        )
    notice_path.write_text(notice, encoding="utf-8")


def validate_final_compliance(
    runtime_root: Path,
    repo_root: Path = REPO_ROOT,
) -> None:
    runtime_root = runtime_root.resolve()
    repo_root = repo_root.resolve()
    exact_frontend = _read_json(runtime_root / BUNDLE_FRONTEND_MANIFEST)
    if exact_frontend.get("inventory_method") != "vite-source-map":
        raise RuntimeError(
            f"packaged frontend inventory is not production-output based: {exact_frontend.get('inventory_method')}"
        )
    native = _read_json(runtime_root / "NATIVE-COMPONENTS.json")
    provenance = _read_json(runtime_root / "RUNTIME-PROVENANCE.json")
    _validate_curl_enrichment(runtime_root, native)
    _validate_frontend_enrichment(runtime_root, repo_root, exact_frontend)
    _validate_ffmpeg_enrichment(runtime_root, provenance)

    notice_path = runtime_root / "legal" / "THIRD_PARTY_NOTICES.md"
    if not notice_path.is_file() or BUNDLE_FRONTEND_MANIFEST not in notice_path.read_text(
        encoding="utf-8"
    ):
        raise RuntimeError(
            "packaged third-party notice does not reference exact frontend bundle inventory"
        )


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Prepare/finalize/validate Scriptotar production compliance evidence."
    )
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--prepare-frontend", action="store_true")
    mode.add_argument("--finalize", type=Path)
    mode.add_argument("--validate", type=Path)
    parser.add_argument("--repo-root", type=Path, default=REPO_ROOT)
    args = parser.parse_args()

    if args.prepare_frontend:
        prepare_frontend_bundle(args.repo_root)
        print("prepared Vite production bundle evidence")
    elif args.finalize:
        finalize_compliance_bundle(args.finalize, args.repo_root)
        print(f"finalized compliance bundle: {args.finalize.resolve()}")
    else:
        validate_final_compliance(args.validate, args.repo_root)
        print(f"validated final compliance bundle: {args.validate.resolve()}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
