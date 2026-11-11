# Scriptotar Next 1.4.0 — Integration Candidate Readiness

**Snapshot date:** 2026-11-11  
**Integration PR:** https://github.com/MousaXD/scriptotar/pull/99  
**Branch:** `integration/scriptotar-agent00-gui-performance-20261111`  
**Verdict:** **NO-GO for merge and distribution until exact-final-SHA checks and independent QA are complete.**

## Scope and product identity

Next 1.4.0 combines the eight-agent desktop GUI/startup/performance workstreams and patched bundled SQLite supply chain. The source versions in Tauri, the Rust workspace and nine local Cargo lockfile entries, Svelte and npm package-lock root, and the transcription sidecar are synchronized to `1.4.0`. The sidecar protocol stays `1`; archived Scriptotar Classic remains `1.3.0` and is not modified.

Agent 00 combined source PRs #89 (design system), #90 (startup), #92 (Rust bootstrap), #95 (Dashboard/Jobs), #91 (Research/Library), #93 (Transcript/AI), #96 (Settings/localization), #98 (SQLite security), and #94 (independent QA tooling), preserving their source commits as merge parents. Agent 00 then adopted metadata-only startup with full on-demand transcript hydration and migrated real app selectors to shared accessible controls.

## Evidence is exact-commit only

Prior to the version bump, integration commit `f3695908753bb5947bf00f6aea05a2d5e7446571` passed [Desktop UI](https://github.com/MousaXD/scriptotar/actions/runs/43045436283) and [Security hygiene](https://github.com/MousaXD/scriptotar/actions/runs/43045436709). These prove **only those two workflows for that commit**. They do not certify later release-version changes or any other platform.

The 2026-11-11 Linux native-dependency setup in earlier integration runs repeatedly failed or timed out resolving Ubuntu/Azure apt mirrors: see [Rust foundation](https://github.com/MousaXD/scriptotar/actions/runs/43041383836) and [CI](https://github.com/MousaXD/scriptotar/actions/runs/43041384017). This is an infrastructure blocker, **not** evidence that skipped Rust, database, or installer tests passed. Do not bypass native compilation or silence the mandatory checks.

Independent GUI QA implements 140 browser screenshots and interaction cases but requires actual exact-SHA execution and manual image review. A browser-mock timing measurement cannot establish native app startup or SQLite IPC speed. No approved image-diff baseline or native performance certification exists for the 1.4.0 candidate at this snapshot.

The SQLite remediation source in PR #98 contains `rusqlite 0.41.0` and `libsqlite3-sys 0.39.1`, bundling patched SQLite 3.54.x. Preserve schema-5 migration/WAL/recovery/backup/security tests and obtain a **new successful exact-final-SHA** RustSec audit and installed package acceptance. See [security-agent handoff](../agent-handoffs/security-sqlite-rustsec-2026-0459.md).

## Required go/no-go gates

1. Exact final commit: Svelte typecheck, Vitest, production build, command API contract, Arabic/English and RTL, keyboard, project switch, source lineage, prompt isolation and Settings/MCP persistence.
2. Exact final commit: Rustfmt, locked Rust check/Clippy/tests, SQLite migrations, recovery and the bundled patched SQLite compatibility acceptance, mandatory Rust and other supply-chain audits.
3. Exact final commit: MCP stdio/HTTP auth, default-deny security, token revocation, replay/path protections and restart persistence.
4. Exact final commit: clean installed Windows NSIS and Linux Debian packages, host startup, cancellation/shutdown/recovery, package integrity, license/compliance artifacts.
5. Exact final commit: real Chromium screenshots across themes, RTL, viewports and failure states; zero unexplained native controls, errors or focus regressions; independent human visual review and approved baseline.
6. Reproducible, same-workload native cold/warm startup, payload size, large database memory/latency and regression evidence. Do not assert an unmeasured speedup.
7. Hosted YouTube live E2E remains distinct from source-access verification and local model inference; never swap its URL, inject fixture success or relabel bot/sign-in denial. Manual GPU, mixed-language and real-provider acceptance remain outstanding where not measured.

## Release protection

`.github/workflows/tauri-next-release.yml` is the sole rolling release writer. The workflow requires exact-commit push CI on `main`, matching validated native artifacts, and explicit `SCRIPTOTAR_NEXT_DISTRIBUTION_APPROVED=true`. Do **not** set this approval as a CI workaround, publish a stable tag or move `tauri-next-latest` as part of integration. An unpublished 1.4.0 source-version bump is **not** a release.

**Promotion decision:** keep PR #99 draft and `main` untouched until the final source commit passes the mandatory gates and Agent 08 independently certifies the screenshots, keyboard/accessibility, responsiveness and native startup evidence.
