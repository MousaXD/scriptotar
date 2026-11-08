# Scriptotar Next 1.3.0 — Release Readiness

**As of 2026-11-08:** version bumped from the integrated 1.2.0 candidate at the owner's request before merging into `main`. Candidate checks are pending on the final 1.3.0 commit. This document does not certify or authorize a public release.

## Scope and identity

The owner-overhaul integration branch combines the seven specialist workstreams, installed-MCP acceptance fixes, and the 1.2.0 candidate. Version 1.3.0 changes the Rust workspace and nine local lockfile packages, Tauri, Svelte package and npm lockfile, transcription sidecar, MCP version assertions, and current documentation together. Sidecar protocol remains version 1. Archived Classic 1.3.0 is a separate product line.

## Required candidate checks

- Rust formatting, check, Clippy, workspace and migration/recovery tests.
- Svelte check, UI/API contract tests, and production build.
- Python sidecar unit/protocol tests and real engine dependency imports.
- Compiled desktop MCP stdio/HTTP, 14-tool permissions, revocation, and persistence acceptance.
- Installed Windows NSIS and Linux Debian runtime and application acceptance, including native Windows process-tree cleanup.
- Strict dependency audits, repository hygiene, release ownership and exact-commit publisher contracts.

CI evidence must identify the final candidate commit. Passing older runs does not certify version 1.3.0.

Initial 1.3.0 checks on `9d2c665` passed Rust, frontend, sidecar, MCP E2E, full integration, security audits and publisher contracts. Windows/Linux packaging encountered upstream HTTP 504s downloading pinned FFmpeg/tunnel assets. The follow-up adds bounded transport retries and an official GitHub asset API fallback for the same release/name, retaining the existing binary digests and size limits. Permanent HTTP failures and integrity failures remain fatal; retry-exhaustion and tampering regressions are covered by offline tests. Both installed-package checks must pass on the final follow-up commit.

## Known external and manual limitations

The previous integrated commit `42ac2eb256943e19a4c543109df800d0168e192c` passed its deterministic, security, MCP, Linux and Windows checks. Its [live-provider run](https://github.com/MousaXD/scriptotar/actions/runs/42407836286) failed. Earlier diagnostics identify a bot/sign-in challenge on the pinned YouTube source before download or inference. Instagram real inference produced three segments; TikTok inference produced zero segments, which does not prove recognition accuracy. Preserve the exact sources and visible failure; do not substitute fixtures or bypass source access controls.

CUDA fallback, Arabic/multilingual accuracy, a user-authorized live ChatGPT tunnel, interactive Windows journeys, historical customer database upgrades and performance remain unverified. FFmpeg's upstream URL is mutable despite enforced archive digest and size. Three previously disclosed low-severity npm advisory paths remain.

See [the 1.3.0 E2E inventory](../E2E-COVERAGE-1.3.0.md) and [the historical 1.2.0 readiness record](NEXT_1.2.0_RELEASE_READINESS.md).

## Publication

The owner's current request authorizes the version bump and code merge into `main`. Public distribution remains governed by the sole rolling `tauri-next-latest` prerelease publisher, successful exact-`main`-commit mandatory gates and `SCRIPTOTAR_NEXT_DISTRIBUTION_APPROVED=true`. This version bump does not introduce a stable release tag or change the distribution approval variable.
