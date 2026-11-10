# Agent 08 — independent QA and release certification

**Repository:** https://github.com/MousaXD/scriptotar  
**QA branch:** agent-08/gui-performance-qa-certification  
**Initial pinned baseline:** f741244634b5143602a21a554094a498a0005b11 (main, inspected 2026-11-11)  
**Verdict: BLOCKED.** No integrated candidate has been certified.

## Independent QA infrastructure

Added Chromium GUI QA workflow, immutable pinned Actions, production frontend build/check/tests, 140 real rendered screenshots across seven workspaces plus MCP, 1440/1100/760 widths, EN/AR, dark/system-light, empty library and synthetic loading/error states. Added browser-level native select/range, keyboard focus, modal focus trapping, RTL, overflow and exception checks. Added measured browser cold/warm shell, useful-workspace and navigation timings and optional JS heap metrics. Added narrow genuine-native input allowlist and manual-only exact-SHA screenshot hash baseline approval with comparison gate.

Instructions: apps/desktop-ui/qa/README.md. None of these files changes application functionality, production code or security policy.

## Evidence integrity and prior certification

The earlier docs/QA_RELEASE_CERTIFICATION.md audit covered a different 2026-11-05 reference commit; its green jobs do NOT certify this GUI overhaul. At authoring there is no local clone with network access to install browser dependencies. The QA runner has been authored but no screenshots or numeric benchmark measurements have been executed in this session. All screenshot and CI outcomes must be taken from the actual GitHub Actions run for the QA PR exact HEAD and the final integration exact SHA. Never manufacture numbers or regard cancelled, skipped or queued jobs as PASS.

Until a human has inspected the new screenshots and approved the image manifest, image-diff regression is NOT certified. Browser timings using built-in mock data are NOT native startup, database, IPC, or large-workspace results.

## Release test matrix

| Gate | Current state |
| --- | --- |
| Frontend typecheck, Vitest, production build | NOT VERIFIED on QA/integrated HEAD |
| Rendered screenshots, themes, locales, responsive widths and states | 140 cases implemented; CI evidence NOT YET AVAILABLE |
| Browser native controls, keyboard, dialogs, focus, RTL and overflow | Assertions authored; actual run NOT YET AVAILABLE |
| Approved pixel/image regression comparison | BLOCKED: no approved manifest |
| Browser performance timings and browser heap | Instrumentation authored; values NOT YET AVAILABLE |
| Native cold/warm startup and time to useful app | NOT MEASURED |
| Bootstrap IPC payload, SQLite initialization and large-DB memory/latency | NOT MEASURED |
| Project/research/watchlist/job/transcript/AI/library/settings preservation | Baseline tests exist; integrated overhaul NOT VERIFIED |
| DB migration, crash recovery, job cancellation and retries | NOT VERIFIED on integrated candidate |
| MCP default-deny, path security, authorization and revocation | NOT VERIFIED on integrated candidate |
| Installed Windows/Linux acceptance | NOT VERIFIED on integrated candidate |
| Real model and social-provider canaries | NOT VERIFIED on integrated candidate |

No browser-only green check replaces the real Rust/SQLite/MCP/installer gates.

## Agent-by-agent review queue

| Owner | Branch and known PR | Decision |
| --- | --- | --- |
| 01 Design/controls | agent-01/design-system-controls-overhaul; PR #89 | PENDING adoption into actual views |
| 02 Startup | agent-02/frontend-startup-performance; PR #90 | PENDING native and progressive-startup validation |
| 03 Rust/bootstrap | agent-03/rust-bootstrap-optimization; PR #92 | PENDING summary opt-in, DB/perf and payload contract |
| 04 Dashboard/Jobs | agent-04/dashboard-jobs-overhaul; PR #95 | PENDING queue and cancel/retry parity |
| 05 Research/Library | agent-05/research-library-overhaul; PR #91 | PENDING pagination, watchlists and Arabic |
| 06 Transcript/AI | agent-06/transcript-ai-workspace-overhaul; PR #93 | PENDING export, search, Copy Prompt and BYOK |
| 07 Settings/MCP/RTL | agent-07/settings-localization-overhaul; PR #96 | PENDING persistence, permissions and direction |

Potential baseline defect for Agent 01 and view owners: AppShell and several views still use native HTML select widgets; global CSS provides colors and borders but no global native-arrow suppression. PR #89 explicitly says new components still need adoption. Reproduce through the browser-native-control test; do not add blanket allowlist exceptions. This is source-based baseline evidence, not a rejection of a later SHA.

## Required final steps

1. Obtain exact-SHA GUI screenshot artifact and all job results. Repair any QA harness errors inside QA-owned files only.
2. Independently review each new Agent 01–07 PR and open owning-agent defects with reproduction steps rather than editing their production code.
3. After Agent 00 integration, repeat exact-commit Svelte/Rust/database/MCP/security/installed Linux/Windows/live-model checks.
4. Review screenshots and approve a baseline only after visual consistency and functional behavior pass.
5. Benchmark native cold/warm start, initial shell, useful workspace, backend payload, large SQLite database, navigation and memory on identical controlled workloads; report measured pre/post values.
6. Record concrete run, artifact, commit and failure links before giving GO.

**Merge readiness: BLOCKED.** No unmeasured speed claim or old green run is release proof.

## Source-reviewed cross-agent integration findings (2026-11-11)

The following are **unresolved integration dependencies**, not successful/failed live browser tests:

- PR #90, reviewed head da78cd386809750311574eef4ab2ce9025295260: AppShell project and locale dropdowns remain native. Owns migration of these controls after Agent 01 component PR #89. See QA comment on #90.
- PR #91, reviewed head e799779503bca12773002a60b34cc3f16d786290: three Research selectors and Library sort selector remain native, with selection checkboxes requiring visual review. See QA comment on #91.
- PR #92, reviewed head f566e69e844df3cb1c786887df3e748050c87020: metadata-only bootstrap is explicitly opt-in; PR #90 has not yet adopted it. Until full transcripts are hydrated on demand in an integrated candidate, dashboard payload improvements are not proven. See QA comment on #92.
- PR #93, reviewed head 6a476808cb3efdb6fbef513bf2cba25c4076e4ab: Transcript/AI selectors remain native. See QA comment on #93.
- PR #95, reviewed head 3554b13cf671314552002fe6cf077ef08c86388f: Dashboard and Jobs view changes present; real queue/cancel/retry parity and layout require exact-head test and screenshot evidence. No acceptance issued.
- PR #96, reviewed head 49fff93b53b28602c032926d280a2f5b132c6b9c: Settings watch interval selector remains native; verify saved values and MCP authorization remain unchanged. See QA comment on #96.

GitHub Actions jobs for Agent 08 were queued when checked; no completed PASS had been observed. The QA PR is #94 and remains draft. Read the latest PR head and its check-run list, because this report is not a substitute for exact-commit CI evidence.
