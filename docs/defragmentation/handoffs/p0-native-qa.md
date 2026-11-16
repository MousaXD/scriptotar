# Agent 07 — P0 native installed WebView QA handoff

**Date:** 2026-11-16  
**Repo:** MousaXD/scriptotar  
**Integration target:** integration/scriptotar-defragmentation-agent00-20261113  
**Verified starting source:** 3009989a3aa73248b6432ba0cd84821c59110993 (PR #114)  
**Branch:** agent-07/p0-native-webview-qa-20261116  
**Release decision:** **NO-GO — native graphical certification NOT COMPLETE.** Main and production implementation are untouched.

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
