# Scriptotar Next release publishing

Scriptotar Next has one authoritative GitHub Release writer: `.github/workflows/tauri-next-release.yml`.

The packaging workflows are artifact producers only:

- `.github/workflows/windows-tauri.yml` builds the Windows runtime and NSIS installer, performs the clean-install/runtime smoke test, stages checksums and runtime metadata, and uploads the validated `scriptotar-windows-tauri` workflow artifact.
- `.github/workflows/linux-tauri.yml` builds the Linux runtime and Debian package, validates the installed package/runtime, stages checksums and package/runtime metadata, and uploads the validated `scriptotar-linux-tauri` workflow artifact.

Neither packaging workflow may create or edit a GitHub Release, upload assets directly to a GitHub Release, grant `contents: write` for publishing, or move the rolling `tauri-next-latest` tag.

After the main-branch package runs complete, `.github/workflows/tauri-next-release.yml` resolves the matching Windows and Linux artifacts, verifies the package jobs, and publishes them together under the rolling `tauri-next-latest` GitHub prerelease. It is the sole owner of both the release and rolling tag.

The unified preview release contains:

- `Scriptotar-Next-latest-x64-setup.exe`
- `Scriptotar-Next-latest-amd64.deb`
- platform-specific runtime manifests
- Linux package contents metadata
- one combined `SHA256SUMS.txt`

Publishing is refused if the two package versions differ or either validated package build failed. The rolling tag is moved only by the release workflow after release assets are staged successfully.

`.github/scripts/validate_release_ownership.py`, enforced by `.github/workflows/release-ownership-contract.yml`, protects this ownership model. The contract fails if a packaging workflow regains GitHub Release mutation commands, force-moves `tauri-next-latest`, or grants release-capable `contents: write`, and it rejects any second workflow that can mutate the rolling release/tag.
