# Scriptotar Agent 00 — Integration Owner Handoff

**Branch:** `integration/scriptotar-agent00-gui-performance-20261111`  
**Pull request:** https://github.com/MousaXD/scriptotar/pull/99 (draft)  
**Main baseline:** `f741244634b5143602a21a554094a498a0005b11`  
**Date:** 2026-11-11  
**Readiness:** **BLOCKED — not approved to merge `main` or publish**.

## Combined source and lineage

Integrated nine nonoverlapping source PR heads with original agent commit lineage preserved:

| Source PR | Workstream |
| --- | --- |
| #89 | Agent 01 design foundation, common controls and source contracts |
| #90 | Agent 02 shell-first nonblocking startup, project-state safety |
| #92 | Agent 03 lightweight Rust/SQLite transcript summary bootstrap |
| #95 | Agent 04 Dashboard and Jobs views |
| #91 | Agent 05 Research and Library workflows |
| #93 | Agent 06 Transcript and AI Studio |
| #96 | Agent 07 Settings, MCP wizard, localization/RTL |
| #98 | Dedicated SQLite RustSec remediation |
| #94 | Agent 08 independent QA scripts, visual and timing instrumentation |

Initial source integration included 75 path entries, validated by exact blob SHAs. Additional Agent 00 follow-up edits are intentional in the integration branch, not new agent branches.

## Agent 00 reconciliations

- Called `api.bootstrapSummary()` instead of full-detail `bootstrap()` at initial shell hydration, retains full transcripts on-demand via `getTranscript(id)`, and preserves existing project mutation/recovery semantics.
- Replaced remaining app-owned native selectors with common keyboard-accessible Select in AppShell, Research, Library, Transcript, AI Studio and Settings. The shared Switch is used for Settings toggles; native OS file-picker functionality is retained.
- Preserved user-visible source name `Transcript ID` rather than leaking a TypeScript summary type name into AI provenance.
- Updated frontend interaction tests for role/option semantics and maintained safety assertions for stale source response rejection, project ownership, provider credential boundaries, and save/discard behavior.
- Advanced Next version identity to `1.4.0` across Tauri, Rust, Cargo/npm lockfiles, UI, sidecar and current docs; Classic remains at `1.3.0`. This is an **unpublished** integration candidate.

## Evidence and blockers

The pre-version-bump integration commit `f3695908753bb5947bf00f6aea05a2d5e7446571` passed [Desktop UI](https://github.com/MousaXD/scriptotar/actions/runs/43045436283), including frontend validation and all 184 Vitest cases, and passed [Security hygiene](https://github.com/MousaXD/scriptotar/actions/runs/43045436709).

Other required checks on that and later SHAs are **not certified**. Multiple GitHub Linux runners failed or timed out during `apt` installation because Ubuntu/Azure mirrors were unresolvable; examples: [Rust foundation](https://github.com/MousaXD/scriptotar/actions/runs/43041383836), [CI](https://github.com/MousaXD/scriptotar/actions/runs/43041384017). Do not count infrastructure failure or cancellation as Rust success.

Independent QA must review actual browser screenshots, approve a visual baseline and record native reproducible timings. The 140 screenshot test matrix and browser-mock timings are instrumentation, not GO. Prior live YouTube source-access challenges also remain explicitly documented.

## Final owner action list

Complete CI for the **final** `1.4.0` HEAD and obtain explicit independent QA GO, including Windows/Linux installed packages, SQLite recovery/security, MCP authorization/revocation, full frontend interactions and native performance. Preserve product protections and release single-writer rules. Only then consider a safe squash/merge or merge commit via the PR with an immutable expected head. Confirm `main` has not moved and validate push checks after the merge. Source version bumps must never be treated as publication approval.

See [candidate release readiness](../releases/NEXT_1.4.0_RELEASE_READINESS.md).
