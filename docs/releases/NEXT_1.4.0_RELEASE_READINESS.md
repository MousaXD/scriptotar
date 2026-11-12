# Scriptotar Next 1.4.0 — Published Preview and Outstanding Certification

**Publication date:** 2026-11-12
**Release type:** Public GitHub **prerelease** using the existing rolling `tauri-next-latest` tag; **not** a stable release.
**Published source SHA:** `82812c071b6b201ac3dccd66d014a187c838b0fa`
**Source merge:** PR #99 / `e7d8bb3b207092436686a433599e148a24e1c9df`
**Release:** https://github.com/MousaXD/scriptotar/releases/tag/tauri-next-latest
**Publication workflow:** https://github.com/MousaXD/scriptotar/actions/runs/43131757810 (successful)

## Scope

Next 1.4.0 integrates the eight-agent GUI/startup/performance overhaul and the SQLite supply-chain patch. Windows NSIS and Debian/Ubuntu x64 distributions include their self-contained transcription runtime. The Next Rust/Tauri, Svelte and sidecar version identities are 1.4.0; the sidecar protocol stays 1, and archived Scriptotar Classic 1.3.0 is unchanged.

## Verified automated checks on the exact published source commit

- [Main CI](https://github.com/MousaXD/scriptotar/actions/runs/43130493137): success.
- [Tauri migration integration](https://github.com/MousaXD/scriptotar/actions/runs/43130493223): success, including Rust workspace, Svelte, Python sidecar, Rust/sidecar integration, supply chain and Tauri build.
- [Scriptotar MCP E2E](https://github.com/MousaXD/scriptotar/actions/runs/43130493252): success.
- [Security hygiene](https://github.com/MousaXD/scriptotar/actions/runs/43130493257): success.
- [Windows installer](https://github.com/MousaXD/scriptotar/actions/runs/43130493261): success, including clean installed-app smoke; release artifact uploaded.
- [Linux Debian](https://github.com/MousaXD/scriptotar/actions/runs/43130493166): success, including installed-app IPC/persistence, MCP and shell checks; release artifact uploaded.
- [Desktop UI](https://github.com/MousaXD/scriptotar/actions/runs/43130493215): success.
- [Chromium GUI visual/performance QA](https://github.com/MousaXD/scriptotar/actions/runs/43130493375): success; all 140 screenshots/cases with zero automated failures.
- [Release publisher](https://github.com/MousaXD/scriptotar/actions/runs/43131757810): success; both artifact packages, checksum manifest, staging validation, exact-current-main recheck, public rolling tag and uploaded assets verified.

## Explicitly outstanding manual certification — not represented as passed

- Independent human approval of the full 140-scenario image set and signed accessibility/RTL baseline.
- Reproducible native Tauri cold/warm startup and large SQLite metadata/bootstrap latency, payload and memory comparisons versus 1.3.0.
- Manual device-specific Windows/Linux user flows, CUDA, GPU fallback, real Whisper multilingual media, migrated customer databases and live-provider canaries.
- Hosted YouTube sign-in/bot-verification limitations. Source and fixture checks are not evidence that public YouTube access always succeeds.

The repository owner explicitly requested immediate **early-preview** publication despite these incomplete manual checks. This records an informed distribution risk acceptance, not a claim of passing certification. [Issue #100](https://github.com/MousaXD/scriptotar/issues/100) remains open to close these gaps.

## Integrity and release safeguards

The existing sole release writer used exact-published-commit required CI and Windows/Linux package artifact pairing, staged a draft with a SHA-256 checksum manifest, checked that source remained current `main`, then moved `tauri-next-latest`. The new approval was restricted to the publication commit message `owner-approve-scriptotar-next-1.4.0-publish-20261112`. The normal `SCRIPTOTAR_NEXT_DISTRIBUTION_APPROVED` and `SCRIPTOTAR_NEXT_1_4_0_DISTRIBUTION_APPROVED` flags remain the default approval mechanism for other commits.

This page was updated after publication; the documentation-only update does not rebuild or replace the versioned release assets.
