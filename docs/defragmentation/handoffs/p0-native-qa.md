# Agent 07 — P0 native installed WebView QA handoff

**Date:** 2026-11-16  
**Repo:** MousaXD/scriptotar  
**Integration target:** integration/scriptotar-defragmentation-agent00-20261113  
**Verified starting source:** 3009989a3aa73248b6432ba0cd84821c59110993 (PR #114)  
**Branch:** agent-07/p0-native-webview-qa-20261116  
**Release decision:** **NO-GO — native graphical certification NOT COMPLETE.** Main and production implementation are untouched.

## Executed CI — first immutable native run (2026-11-16)

**Actual tested source SHA:** `45c0e71f684063aed45eeb17a3f8355302bbd02e` (before the subsequent harness-only diagnostic patches).

**Workflow:** https://github.com/MousaXD/scriptotar/actions/runs/43875131901 — **FAIL**, as required by the fail-closed release gate.

- Linux job: https://github.com/MousaXD/scriptotar/actions/runs/43875131901/job/133326081585
- Windows job: https://github.com/MousaXD/scriptotar/actions/runs/43875131901/job/133326081192
- Linux packaged evidence: https://github.com/MousaXD/scriptotar/actions/runs/43875131901/artifacts/14231550069
- Windows packaged evidence: https://github.com/MousaXD/scriptotar/actions/runs/43875131901/artifacts/14231890199

Both jobs checked out the exact immutable source SHA; successfully built frontend/package, installed the resulting package, reached their genuine native graphical probe, returned **NOT TESTED** for the native document requirement and failed the job. Linux successfully started the AT-SPI D-Bus registry and installed `scriptotar` 1.4.1 with `/usr/bin/scriptotar-desktop`; it did **not** prove a PID-owned accessible WebKit document. Windows successfully installed NSIS and recorded package SHA-256 `12b2b4da097d481aca5c6f219b7b8761eb2846aa80e64ae98ac7c70e401472e0` and executable SHA-256 `cf9b2da78d2705e38417833a7796e2f5adcf526fa31696be9e2ef53b889e8f87`. It observed a shell but could not locate both a WebView2 UIA Document and interactive Studio control. Linux's package/executable hashes are saved in the original SHA256SUMS artifact; the text logs alone do not independently expose those values.

The first run's Linux artifact had five files and Windows artifact had three; neither probe produced an accepted installed GUI screenshot or UI-accessibility tree. There is no basis for screenshot or navigation approval. An unavailable accessibility provider is **NOT TESTED**, not proof of a production UI bug and not a PASS.

| Observed, exact SHA 45c0e71 | Linux WebKitGTK | Windows WebView2 |
|---|---|---|
| Immutable-source checkout and package build | PASS | PASS |
| Real package installation + hash recorded | PASS | PASS |
| Accessible installed WebView document + UI interaction | NOT TESTED | NOT TESTED |
| Project create/switch | NOT TESTED | NOT TESTED |
| Study create/open | NOT TESTED | NOT TESTED |
| Candidate edit/save/duplicate/rename | NOT TESTED | NOT TESTED |
| Quit/relaunch persistence | NOT TESTED | NOT TESTED |
| Wrong-project UI or IPC denial | NOT TESTED | NOT TESTED |
| Real multi-source/transcript/revocation | NOT TESTED | NOT TESTED |
| Keyboard/focus, EN/AR, RTL, custom controls, errors | NOT TESTED | NOT TESTED |
| Light/dark actual installed screenshots and human review | NOT TESTED | NOT TESTED |
| Three scripts and authorized paid generation | NOT TESTED | NOT TESTED |

**Harness follow-up on the same agent-owned branch (NOT YET CERTIFIED):** bind Linux AT-SPI applications first by the genuine installed process PID rather than rejecting non-Scriptotar accessible names; preserve bounded AT-SPI desktop tree, X11 windows, unverified screenshot and failure reason; preserve Windows UIA RawView tree and runner/app session details; upload lightweight review artifacts distinct from hundreds-of-MB installer artifacts. The next exact-head CI must be checked independently. Never credit the first run's install PASS to new commits.

**Windows execution requirement:** If hosted CI's session still does not expose WebView2 UIA, run the same installed NSIS test in an unlocked, interactive Windows 11 desktop session under a normal logged-in user with WebView2 Runtime and UIA provider available. Save the installer hash, installed exe hash, UIA raw tree, screenshots, case matrix and full run provenance. A hosted runner's process launch is not Windows graphical certification.

**Agent 00 fixture dependency:** Request canonical disposable Study DB fixtures with genuine authorized transcript/source records, deterministic revocation/failure controls and project boundaries. Do not inject fake AI/provider content. Agent 00 must finish production integration before three-candidate or consented-provider native cases can be executed. Keep this PR draft and the release gate NO-GO.

## Baseline investigation

Read CONTINUATION.md, ROADMAP_P0_P3.md, issues #100/#116, PR #114, tests/defragmentation, Linux/Windows Tauri package workflows and Chromium GUI QA. Existing Linux package CI performs a real package install, startup process check and Rust installed IPC/restart tests, not native graphical WebKit actions. Windows installer CI installs NSIS and performs process/backend/MCP checks, not installed WebView2 user journeys. Chromium screenshots are mocked frontend rendering and are **not** native installed evidence.

Existing Agent 07 QA branch agent-07/defragmentation-independent-qa and PR #106 contain fixture/validation/benchmark work already incorporated into integration; do not replace or relabel that work. The requested new native branch did not exist before this implementation.

## Implemented on this branch

1. tests/defragmentation/native-e2e/linux_webkit.py: actual executable launch using a fresh isolated data directory and native PID-backed AT-SPI WebKit document discovery; user-facing project/Study/candidate interactions, quit/relaunch assertions and scenario matrix.
2. tests/defragmentation/native-e2e/windows_webview2.ps1: NSIS install, actual process/window WebView2 UIA Document check, Studio invocation and screenshot where interactive Windows sessions allow it; fail-closed incomplete case matrix.
3. .github/workflows/installed-webview-acceptance.yml: dedicated immutable-source package builds, Xvfb + D-Bus native Linux automation and installed Windows UIA, hashes, machine evidence, artifact uploads even on failed certification.
4. tests/defragmentation/native-e2e/test_contract.py and README.md: fail-closed metadata/unit contract and reproducible invocation.

No Tauri commands, production Svelte UI, backend DB implementation, main branch, release pipeline flags or old CI gates are changed.

## Independent certification matrix

This matrix reports **verified evidence as of handoff**, not implementation intentions. No installed graphical run has yet been observed and reviewed from this working session. Tests can advance only after exact SHA CI artifacts are available.

| Acceptance requirement | Linux | Windows | Honest note |
|---|---|---|---|
| Actual package install, SHA-256 and host provenance | NOT TESTED | NOT TESTED | CI jobs defined, no completed run evidence yet |
| Installed WebKit/WebView2 accessibility document | NOT TESTED | NOT TESTED | Probe implemented, must be executed and reviewed |
| Create/switch projects | NOT TESTED | NOT TESTED | Linux AT-SPI journey implemented; Windows not yet |
| Create/open Content Study | NOT TESTED | NOT TESTED | Linux journey implemented |
| Draft edit/save/duplicate/rename/select | NOT TESTED | NOT TESTED | Linux journey implemented |
| Quit/relaunch + durable saved candidate | NOT TESTED | NOT TESTED | Linux journey implemented; requires real run |
| Wrong-project visible Study isolation | NOT TESTED | NOT TESTED | UI isolation only; not security boundary proof |
| Forged wrong-project IPC denial | NOT TESTED | NOT TESTED | Not covered by native UI probe |
| Select multiple real sources + transcript preview | NOT TESTED | NOT TESTED | Requires genuine authorized media fixtures |
| Revoked source + error recovery | NOT TESTED | NOT TESTED | Requires safe test fixture/failure control |
| Custom selector Escape/keyboard | NOT TESTED | NOT TESTED | Partial Linux journey implemented |
| Arabic language selector operation | NOT TESTED | NOT TESTED | Linux installed UI interaction added; no observed run yet |
| Dark/system theme selector operation | NOT TESTED | NOT TESTED | Linux installed UI interaction added; visual review separate |
| Arabic locale + actual RTL layout, mixed content | NOT TESTED | NOT TESTED | Independent native visual/a11y review required |
| Light/dark screenshots and approval | NOT TESTED | NOT TESTED | App currently offers dark/system; verify OS light mapping |
| Three original agent-produced candidates, approved egress | NOT TESTED | NOT TESTED | Not a GUI-only feature; depends on Agent 00/01/02/05/06 integration |

**PASS 0 / FAIL 0 / NOT TESTED all.** This is a pre-execution audit matrix, not a claim that the GUI succeeded. Once CI runs, attach result.json, checksums, screenshots, action job URLs and reviewer verdict, then update this table per exact SHA. Do not retroactively count other PRs' green checks.

## Explicit blockers / ownership

- **P0:** Prove AT-SPI exposure on installed Linux WebKitGTK and native WebView2 UIA on an actual interactive Windows session; refine selectors based on captured accessibility trees. Hosted Windows CI desktop may be noninteractive.
- **P0:** Add authorized real transcript and research source fixtures, revoke their availability, and prove cross-project denials. Do not fabricate source claims.
- **P0:** Add native screenshot review across EN/AR RTL, theme/sizing/custom controls and keyboard focus, including 200% zoom.
- **P0:** No source-commit-matched installed GUI run or screenshot review has completed in this environment.
- **P1:** Agent 00 must integrate script/AI/provider workflows before graphical acceptance can certify three original candidates and provider budgets.
- **P2:** Startup/RSS/IPC large-database native performance belongs to issue #100's independent performance gate; launching a native process is not a WebView readiness measurement.

## Evidence controls

A PASS row must derive from the same exact source commit and package SHA on the uploaded run, not a prior green run. Native UI probe must actually interact with the installed accessibility tree; no web page/preview, browser-mock, rust-ipc, MCP or screenshot-only substitution. Machines and time must be recorded, and a human reviewer must inspect raw artifacts before assigning release certification.

**Agent 00 action:** Review draft PR against integration, run installed-WebView workflow, attach exact run artifacts, approve additional source fixtures/contracts, and retain draft/NO-GO until all rows pass. Do not merge or publish based on new harness files alone.
