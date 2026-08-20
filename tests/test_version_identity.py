from __future__ import annotations

import json
import re
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def read(path: str) -> str:
    return (ROOT / path).read_text(encoding="utf-8")


def assignment(path: str, name: str) -> str:
    match = re.search(
        rf'^\s*{re.escape(name)}\s*=\s*(?:["\']([^"\']+)["\']|([^\s#]+))\s*(?:#.*)?$',
        read(path),
        re.MULTILINE,
    )
    if match is None:
        raise AssertionError(f"could not resolve {name} from {path}")
    return match.group(1) or match.group(2)


def toml_string_in_section(path: str, section: str, name: str) -> str:
    document = read(path)
    section_match = re.search(
        rf'^\[{re.escape(section)}\]\s*$\n(.*?)(?=^\[|\Z)',
        document,
        re.MULTILINE | re.DOTALL,
    )
    if section_match is None:
        raise AssertionError(f"could not resolve [{section}] from {path}")
    value_match = re.search(
        rf'^\s*{re.escape(name)}\s*=\s*"([^"]+)"\s*(?:#.*)?$',
        section_match.group(1),
        re.MULTILINE,
    )
    if value_match is None:
        raise AssertionError(f"could not resolve {name} from [{section}] in {path}")
    return value_match.group(1)


class VersionIdentityTests(unittest.TestCase):
    def test_application_versions_are_aligned(self) -> None:
        workspace_version = toml_string_in_section(
            "Cargo.toml", "workspace.package", "version"
        )
        tauri_version = json.loads(read("apps/desktop/src-tauri/tauri.conf.json"))["version"]
        frontend_version = json.loads(read("apps/desktop-ui/package.json"))["version"]
        sidecar_version = assignment(
            "sidecars/transcription/scriptotar_sidecar/version.py", "SIDECAR_VERSION"
        )

        self.assertEqual(
            {workspace_version, tauri_version, frontend_version, sidecar_version},
            {"1.0.0"},
            "Scriptotar application versions drifted; keep Cargo, Tauri, Frontend, and Sidecar aligned",
        )

    def test_legacy_archive_is_preserved_and_documented(self) -> None:
        archive_readme = read("archive/legacy-python/README.md")
        self.assertIn("Scriptotar Classic", archive_readme)
        self.assertIn("historical", archive_readme)

    def test_public_docs_name_main_release_assets(self) -> None:
        readme = read("README.md")
        versioning = read("docs/VERSIONING.md")
        app_version = json.loads(read("apps/desktop/src-tauri/tauri.conf.json"))["version"]

        for document in (readme, versioning):
            self.assertIn("Scriptotar", document)
            self.assertIn(app_version, document)

    def test_protocol_version_is_documented(self) -> None:
        versioning = read("docs/VERSIONING.md")
        protocol_version = assignment(
            "sidecars/transcription/scriptotar_sidecar/version.py", "PROTOCOL_VERSION"
        )
        self.assertIn(f"protocol | `{protocol_version}`", versioning)


if __name__ == "__main__":
    unittest.main()
