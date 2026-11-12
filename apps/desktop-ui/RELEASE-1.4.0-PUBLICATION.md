# Scriptotar Next 1.4.0 — owner-authorized preview publication

**Publication decision:** 2026-11-12
**Source baseline:** e7d8bb3b207092436686a433599e148a24e1c9df
**Owner approval message:** `owner-approve-scriptotar-next-1.4.0-publish-20261112`
**Release type:** GitHub prerelease on the existing `tauri-next-latest` rolling tag (not a stable release).

The repository owner expressly directed publication after the source integration merged. This document records a risk acceptance for an **early preview**, not a claim that missing manual checks passed.

Verified on merged source `e7d8bb3b207092436686a433599e148a24e1c9df`:
- Windows NSIS build and clean installed-app smoke passed; build artifact 13894319683.
- Linux Debian package build, installed-app E2E and smoke passed; build artifact 13895880067.
- Chromium screenshot gate rendered 140 scenarios with zero automated failures; screenshot artifact 13893899035.
- Integration, MCP E2E, Rust, frontend and security workflows passed on the merged commit.

Independent human approval of the complete screenshot set, reproducible native cold/warm and large-database performance benchmarks, dedicated hardware/provider canaries, and live YouTube E2E are **not** certified. Public IPs may hit YouTube bot/sign-in gates. Users should treat 1.4.0 as an early preview.

**Mandatory safeguards still apply:** the existing sole release workflow must confirm all exact-`main` CI gates for the new publication commit, download its Windows/Linux artifact pair, verify package version equality and nonempty assets, stage and validate a draft release, check that the release source is still current `main`, then switch `tauri-next-latest`. If any of these steps fails, publication must stop. No alternate writer is permitted.

This release decision is deliberately tied to exactly one commit message. Subsequent releases continue to require both normal distribution approval variables.
