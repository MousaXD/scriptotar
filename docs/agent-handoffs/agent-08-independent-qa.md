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


## Agent 08 continuation — independent QA infrastructure audit (2026-11-11)

**Assigned branch:** agent-08/gui-performance-qa-certification. **Existing draft PR:** https://github.com/MousaXD/scriptotar/pull/94. **Initial audited baseline:** f741244634b5143602a21a554094a498a0005b11. The branch started this continuation at c10cdf046ad7f03f2eef916787a41b5077803807, and the first completed CI run reviewed below was for 54a54ae387800f21670d37054e0a525589bd777e. Final QA source SHA must be read from PR #94 after this handoff commit; only the exact source SHA's checks can authorize certification. This is remote GitHub-connector work: the container has no Scriptotar checkout and git ls-remote fails DNS resolution, so do not claim a local repository test run.

### Source review and QA-owned fixes

Inspected every QA-owned file, the GitHub workflow, the existing App and GUI integration tests, baseline application navigation and boot logic, the control allowlist, the standalone verification tests, the browser runner and screenshot approval mechanism.

- Fixed a genuine test discovery defect verified by exact-SHA CI logs: the standalone Node verifier test was named qa/tooling.test.mjs, causing frontend Vitest to attempt to bundle node:test under its jsdom/Vite environment. It produced "Cannot bundle built-in module node:test" in every frontend-dependent workflow. Renamed it to qa/tooling.nodecheck.mjs without changing its assertions, updated the workflow to run node --test explicitly, and documented the separation. It is still a required CI test; no Vitest or security checks were disabled.
- The original pull_request checkout used GitHub's implicit merge ref while artifacts were labeled ambiguously; the QA workflow now checks out the actual PR head and asserts git HEAD equals QA_TESTED_SHA, which is also recorded in screenshot JSON and artifact names.
- Expanded real-rendered screenshot inventory from 116 to 140 cases, adding Research empty-search and Transcript no-matches states. The screenshot harness checks English/Arabic direction, dark/system-light, 1440/1100/760 widths, keyboard modal focus, styled listboxes, switches/popovers, unexpected native selectors/ranges, overflow and page exceptions. It saves screenshots before test interactions change focus/scroll.
- Records time to shell, time to useful workspace, browser warm reload and view navigation separately, plus optional JS heap, and provides an exact-SHA paired browser-mock timing comparison script. These do NOT measure native Tauri cold startup, SQLite load or real media.
- Evidence comparison requires a passing report with 12 scenarios and 140 real PNG files. Unapproved baseline is explicitly NOT_CERTIFIED; generated approval manifests begin PENDING_MANUAL_REVIEW, and an approved reviewer/timestamp plus exact image hashes are required to pass a comparison. There is no automatic baseline re-record or assertion suppression.
- Added isolated Node verifier tests for missing reports, failed evidence, unapproved manifests, successful matching hashes and tampered images. These synthetic byte fixtures test only the hash verifier, never the UI. The workflow performs node --check on QA scripts and runs those tests before the frontend test/build/browser tasks.
- Updated only shared QA App.test.ts and gui-overhaul.test.ts to target the real accessible "Queue" navigation label rather than obsolete "Jobs", preserving the same underlying job/retry/transcript assertions. Did not alter production application code or the other agents' dedicated tests.

### Real CI evidence reviewed: commit 54a54ae387800f21670d37054e0a525589bd777e

- Chromium QA workflow run 42928121310: FAILED before screenshot execution because npm run test collected the Node-only tooling.test.mjs. Its standalone node --test earlier in the same job PASSED all four verifier tests, and svelte-check reported 0 errors / 0 warnings. No screenshot artifact or timing result was produced.
- Desktop UI frontend run 42928121341, Svelte CI 42928121387, main CI 42928121347 and MCP 42928121394: frontend stages FAILED for the same test discovery problem. In the reported runs the existing source tests passed 12 suites; the extra Node-only suite failed to bundle.
- Rust workspace, Rust-sidecar integration, Python sidecar, repository hygiene, packaged Linux Tauri deb and Windows NSIS jobs on this same commit: SUCCESS. The integrated Tauri job was SKIPPED, not a pass.
- Supply chain on this same commit: FAILURE, with RUSTSEC-2026-0459 for libsqlite3-sys 0.30.1 (the audit report states patch >=0.39.0) and other unmaintained advisories. Do not silence the advisory. This is outside Agent 08-owned files and must be resolved by the backend/dependency owner with compatibility tests.

The renamed Node test and documentation were pushed after the above CI run, so the historical failures are **not** a result for the current head. The latest exact-head workflow is required before judging the fix.

### Findings for other agents (unmerged, non-certified)

- Agent 03 PR #92: at reviewed head b2e168d1e0d4cdcf02e75fb4217d1d80c9c90e92, Rust job 130404184235 fails Clippy manual_is_multiple_of in apps/desktop/src-tauri/src/services.rs:1945, ordinal % 2 == 0. Reported to Agent 03 on PR #92 with the exact job link. There were additional transient apt mirror errors in the log; this Clippy error is the confirmed Rust gate.
- Agent 06 PR #93: prior exact-head 0ef99cc440867112c51edc7167df23c886968cad failed Svelte checks because TranscriptView.svelte:35 captured potentially undefined selected in a callback. Reported to Agent 06; do not transfer old CI conclusions to a newer PR head.
- Agent 04 PR #95: earlier head ee72f9c44421347e5dfe67425dd6a756d529a7bb failed five shared frontend tests, including obsolete Queue selectors (Agent 08-owned tests corrected) and a removed accessible "Create project" heading (Agent 04 owns production fix). Reported to Agent 04. Recheck its latest head for a restored heading and fresh CI.
- Agent 01 and consuming views: shared styled controls still require coordinated integration into AppShell, Research/Library, Transcript/AI and Settings. Keep native-control assertions strict; no broad allowlisting.
- Agent 03 summary bootstrap remains opt-in and requires integrated client hydration; no speed improvement is claimed until measured on the final candidate.

### Certification verdict

**BLOCKED, not ready to merge as a release gate.** Agent 08's independent QA harness is implemented and source-reviewed, and its known Vitest collection bug has been corrected. However the new exact-head Chromium workflow, 140 actual screenshots, human-approved pixel baseline, full integrated runtime feature preservation, installed Linux/Windows acceptance on the **combined** candidate, native cold/warm startup and large-database/payload measurements, and security/supply-chain gates have not all passed. Older green jobs do not certify the current head. Agent 00 must integrate the seven implementation PRs and retest the exact final SHA without weakening checks.
