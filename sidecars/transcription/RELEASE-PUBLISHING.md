# Scriptotar Next release publishing

The Windows and Linux Tauri package workflows are the source of truth for validated distributable artifacts.

After either main-branch package workflow completes, `.github/workflows/tauri-next-release.yml` resolves the Windows and Linux runs for the same commit, verifies that both package build jobs succeeded, downloads their uploaded artifacts, and publishes them together under the rolling `tauri-next-latest` GitHub prerelease.

The unified preview release contains:

- `Scriptotar-Next-latest-x64-setup.exe`
- `Scriptotar-Next-latest-amd64.deb`
- platform-specific runtime manifests
- Linux package contents metadata
- one combined `SHA256SUMS.txt`

Publishing is refused if the two package versions differ or either validated package build failed. The rolling tag is moved only after release assets are uploaded successfully.
