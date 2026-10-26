#!/usr/bin/env python3
"""Bundle audited MCP tunnel executables into Scriptotar installers.

Downloads exact upstream release artifacts and verifies pinned SHA-256 *before*
extracting or executing anything. Only recognized filenames are extracted.
Does not download binaries at runtime on customer machines.
"""
from __future__ import annotations

import argparse
import hashlib
import io
import json
from pathlib import Path
import os
import stat
from urllib.request import Request, urlopen
from zipfile import ZipFile

TUNNEL_TAG = "v0.0.18"
CF_TAG = "2026.10.2"
ARTIFACTS = {
    "linux": {
        "tunnel_archive": "tunnel-client-v0.0.18-linux-amd64.zip",
        "tunnel_sha256": "3c82307316acd93c88a384b62af1d81d3f4cb0efa975f9c0bfaef922866a04c4",
        "cloudflared": "cloudflared-linux-amd64",
        "cloudflared_sha256": "35fbdd2beb378436c7da377d7358cea8e33f9c94a6705d1d8036bf9b0d6d8005",
        "extension": "",
    },
    "windows": {
        "tunnel_archive": "tunnel-client-v0.0.18-windows-amd64.zip",
        "tunnel_sha256": "8d0da1ce79532d44ccc5ed5d8bd36cdfffba37dfd287c602d69635c8bb5db2b2",
        "cloudflared": "cloudflared-windows-amd64.exe",
        "cloudflared_sha256": "12c7c887f8aaab779aef1dfa9b60673530a5adb88ecc55dd6df8f56e0b70f199",
        "extension": ".exe",
    },
}


def sha(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def retrieve(url: str, expected: str | None = None, limit: int = 110 * 1024 * 1024) -> bytes:
    if not url.startswith("https://"):
        raise RuntimeError("Refusing a non-HTTPS binary source")
    with urlopen(Request(url, headers={"User-Agent": "Scriptotar-MCP-Bundle/1"}), timeout=120) as response:
        data = response.read(limit + 1)
    if len(data) > limit:
        raise RuntimeError("Upstream artifact exceeds the expected size bound")
    if expected is not None and sha(data) != expected:
        raise RuntimeError(f"SHA-256 mismatch for {url.split('/')[-1]}")
    return data


def extract_exact_binary(archive: bytes, expected_name: str) -> bytes:
    with ZipFile(io.BytesIO(archive)) as bundle:
        candidates = [name for name in bundle.namelist() if Path(name).name == expected_name and not name.endswith("/")]
        if len(candidates) != 1:
            raise RuntimeError(f"Expected exactly one {expected_name} in verified tunnel-client archive; got {len(candidates)}")
        return bundle.read(candidates[0])


def build(platform: str, output: Path) -> None:
    spec = ARTIFACTS[platform]
    output.mkdir(parents=True, exist_ok=True)
    extension = spec["extension"]
    archive_url = f"https://github.com/openai/tunnel-client/releases/download/{TUNNEL_TAG}/{spec['tunnel_archive']}"
    archive = retrieve(archive_url, spec["tunnel_sha256"])
    tunnel = extract_exact_binary(archive, "tunnel-client" + extension)
    cloudflare_url = f"https://github.com/cloudflare/cloudflared/releases/download/{CF_TAG}/{spec['cloudflared']}"
    cloudflare = retrieve(cloudflare_url, spec["cloudflared_sha256"])
    destinations = {"tunnel-client" + extension: tunnel, "cloudflared" + extension: cloudflare}
    for filename, data in destinations.items():
        path = output / filename
        path.write_bytes(data)
        if platform != "windows":
            path.chmod(0o755)
    # Record official notices and machine-readable SBOM for the tunnel client.
    notices = output / "legal"
    notices.mkdir(exist_ok=True)
    for extension_name in ("-licenses.txt", ".spdx.json"):
        legal_name = f"tunnel-client-{TUNNEL_TAG}-{platform}-amd64{extension_name}"
        legal = retrieve(f"https://github.com/openai/tunnel-client/releases/download/{TUNNEL_TAG}/{legal_name}", limit=5 * 1024 * 1024)
        (notices / legal_name).write_bytes(legal)
    license_data = retrieve(f"https://raw.githubusercontent.com/cloudflare/cloudflared/{CF_TAG}/LICENSE", limit=256 * 1024)
    (notices / "cloudflared-LICENSE").write_bytes(license_data)
    manifest = {
        "sources": {"tunnel-client": archive_url, "cloudflared": cloudflare_url},
        "archive_sha256": spec["tunnel_sha256"], "upstream_cloudflared_sha256": spec["cloudflared_sha256"],
        "platform": platform, "tunnel_version": TUNNEL_TAG, "cloudflared_version": CF_TAG,
        "files": {name: sha(data) for name, data in destinations.items()},
    }
    (output / "MCP-TOOLS-PROVENANCE.json").write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    validate(output)
    print(f"Bundled pinned OpenAI tunnel-client and Cloudflare Quick Tunnel for {platform}; SHA-256 verified")


def validate(output: Path) -> None:
    data = json.loads((output / "MCP-TOOLS-PROVENANCE.json").read_text(encoding="utf-8"))
    if data["platform"] not in ARTIFACTS or len(data["files"]) != 2:
        raise RuntimeError("Invalid packaged tunnel provenance")
    for name, expected in data["files"].items():
        if name not in {"cloudflared", "cloudflared.exe", "tunnel-client", "tunnel-client.exe"}:
            raise RuntimeError("Unrecognized packaged executable")
        path = output / name
        if not path.is_file() or sha(path.read_bytes()) != expected:
            raise RuntimeError(f"Packaged MCP executable failed SHA-256 validation: {name}")
        if os.name != "nt" and path.stat().st_mode & stat.S_IXUSR == 0:
            raise RuntimeError(f"MCP executable is not executable: {name}")
    if not (output / "legal" / "cloudflared-LICENSE").is_file():
        raise RuntimeError("Missing cloudflared license")
    print("MCP tunnel binary SHA-256 validation passed")


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--platform", choices=ARTIFACTS)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--validate", action="store_true")
    options = parser.parse_args()
    if options.validate:
        validate(options.output)
    else:
        if not options.platform:
            parser.error("--platform is required for building")
        build(options.platform, options.output)
