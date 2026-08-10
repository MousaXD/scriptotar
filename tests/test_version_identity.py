from __future__ import annotations

import json
import re
import tomllib
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def read(path: str) -> str:
    return (ROOT / path).read_text(encoding="utf-8")


def assignment(path: str, name: str) -> str:
    match = re.search(
        rf'^\s*{re.escape(name)}\s*=\s*["\']([^"\']+)["\']\s*$',
        read(path),
        re.MULTILINE,
    )
    if match is None:
        raise AssertionError(f"could not resolve {name} from {path}")
    return match.group(1)


class VersionIdentityTests(unittest.TestCase):
    def test_next_versions_are_aligned(self) -> None:
        workspace = tomllib.loads(read("Cargo.toml"))
        workspace_version = workspace["workspace"]["package"]["version"]
        tauri_version = json.loads(read("apps/desktop/src-tauri/tauri.conf.json"))["version"]
        frontend_version = json.loads(read("apps/desktop-ui/package.json"))["version"]
        sidecar_version = assignment(
            "sidecars/transcription/scriptotar_sidecar/version.py", "SIDECAR_VERSION"
        )

        self.assertEqual(
            {workspace_version, tauri_version, frontend_version, sidecar_version},
            {"0.1.0"},
            "Scriptotar Next preview versions drifted; update the version policy and all release-visible sources together",
        )

    def test_classic_application_and_debian_versions_are_aligned(self) -> None:
        classic_version = assignment("scriptotar_common.py", "VERSION")
        build_script = read("build-deb.sh")
        shell_version = re.search(r'^VERSION="([^"]+)"$', build_script, re.MULTILINE)
        deb_version = re.search(r'^Version:\s*([^\s]+)\s*$', build_script, re.MULTILINE)

        self.assertIsNotNone(shell_version)
        self.assertIsNotNone(deb_version)
        self.assertEqual(classic_version, "1.2.0")
        self.assertEqual(shell_version.group(1), classic_version)
        self.assertEqual(deb_version.group(1), classic_version)

    def test_public_docs_name_both_product_lines_and_channels(self) -> None:
        readme = read("README.md")
        versioning = read("docs/VERSIONING.md")
        classic_version = assignment("scriptotar_common.py", "VERSION")
        next_version = json.loads(read("apps/desktop/src-tauri/tauri.conf.json"))["version"]

        for document in (readme, versioning):
            self.assertIn("Scriptotar Next", document)
            self.assertIn(next_version, document)
            self.assertIn("Scriptotar Classic", document)
            self.assertIn(classic_version, document)
            self.assertIn("tauri-next-latest", document)
            self.assertIn("continuous", document)

        self.assertIn("Scriptotar-Next-latest-x64-setup.exe", readme)
        self.assertIn("Scriptotar-Next-latest-amd64.deb", readme)
        self.assertIn("scriptotar-latest_all.deb", readme)
        self.assertIn("Scriptotar-latest-x86_64.AppImage", readme)
        self.assertIn("Scriptotar-latest-x86_64.flatpak", readme)

    def test_intentionally_independent_versions_are_documented(self) -> None:
        versioning = read("docs/VERSIONING.md")
        engine_version = assignment("scriptotar_common.py", "ENGINE_VERSION")
        protocol_version = assignment(
            "sidecars/transcription/scriptotar_sidecar/version.py", "PROTOCOL_VERSION"
        )

        self.assertIn(engine_version, versioning)
        self.assertIn(f"protocol | `{protocol_version}`", versioning)
        self.assertIn("not supposed to match", versioning)


if __name__ == "__main__":
    unittest.main()
