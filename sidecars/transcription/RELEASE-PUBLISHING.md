# Scriptotar Next release publishing

Scriptotar Next has one authoritative GitHub Release writer: `.github/workflows/tauri-next-release.yml`.

The packaging workflows are artifact producers only:

- `.github/workflows/windows-tauri.yml` builds the Windows runtime and NSIS installer, performs the clean-install/runtime smoke test, stages checksums and runtime metadata, and uploads the validated `scriptotar-windows-tauri` workflow artifact.
- `.github/workflows/linux-tauri.yml` builds the Linux runtime and Debian package, validates the installed package/runtime, stages checksums and package/runtime metadata, and uploads the validated `scriptotar-linux-tauri` workflow artifact.

Neither packaging workflow may create or edit a GitHub Release, upload assets directly to a GitHub Release, grant `contents: write` for publishing, or move the rolling `tauri-next-latest` tag.

`.github/workflows/tauri-next-release.yml` publishes the rolling `tauri-next-latest` prerelease only after proving the complete mandatory release suite for the exact `SOURCE_SHA` being published. The required push-workflow runs on `main` are:

- `Tauri migration integration` (`integration.yml`), including Rust workspace, Svelte frontend, Python sidecar, Rust ↔ sidecar integration, supply-chain, and integrated Tauri build jobs;
- `Security hygiene` (`security-hygiene.yml`), including the repository-hygiene job;
- `Windows Tauri Installer` (`windows-tauri.yml`), including `Build and smoke-test NSIS setup.exe`;
- `Linux Tauri Package` (`linux-tauri.yml`), including `Build and validate Tauri deb`.

The exact-SHA gate queries GitHub Actions by workflow file, `main`, `push` event, and exact commit SHA, then independently rechecks each selected run's `headSha`. If more than one push run exists for the same workflow and SHA, the newest run is authoritative. Runs for another SHA are ignored, and an older green run cannot replace a newer failing run for the same SHA.

The publisher waits for required runs that are queued or in progress. Publication is refused when a required run is missing, fails, is cancelled, times out, is skipped, has the wrong SHA/event/branch, or when any named mandatory job is missing or not successful. The gate emits the exact Windows and Linux run IDs used to download artifacts, so publication cannot silently substitute packages from another commit.

The unified preview release contains:

- `Scriptotar-Next-latest-x64-setup.exe`
- `Scriptotar-Next-latest-amd64.deb`
- platform-specific runtime manifests
- Linux package contents metadata
- one combined `SHA256SUMS.txt`

Publishing is also refused if the two package versions differ or required release assets are missing. The rolling tag is moved only by the release workflow after validated release assets are staged successfully.

`.github/scripts/validate_release_ownership.py`, enforced by `.github/workflows/release-ownership-contract.yml`, protects the single-writer ownership model. The contract fails if a packaging workflow regains GitHub Release mutation commands, force-moves `tauri-next-latest`, or grants release-capable `contents: write`, and it rejects any second workflow that can mutate the rolling release/tag.
