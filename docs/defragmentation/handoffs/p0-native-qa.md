# Agent 07 — P0 native installed WebView QA handoff

**Date:** 2026-11-16  
**Repo:** MousaXD/scriptotar  
**Integration target:** integration/scriptotar-defragmentation-agent00-20261113  
**Verified starting source:** 3009989a3aa73248b6432ba0cd84821c59110993 (PR #114)  
**Branch:** agent-07/p0-native-webview-qa-20261116  
**Release decision:** **NO-GO — native graphical certification NOT COMPLETE.** Main and production implementation are untouched.

## Exact-head installed WebView recovery — third native run, 2026-11-16

**Observed run:** [#43897349298](https://github.com/MousaXD/scriptotar/actions/runs/43897349298), source `fc8594d2273c5258a247fa11bc041ac302ab17b2`. Both native acceptance jobs **failed closed**, while [ordinary CI #43897349143](https://github.com/MousaXD/scriptotar/actions/runs/43897349143) and security hygiene **passed** on this same head. Evidence is from **actual installed packages**, not mock or browser preview.

**Linux:** `installed_webview = PASS`. After adding a real Openbox WM, explicit GTK accessibility bridge and deeper AT-SPI traversal, the installed binary produced a **real WebKit document with rendered Scriptotar Studio control**. Xvfb/D-Bus/WebKit instrumented successfully. First genuine interaction failure: `create_switch_projects = FAIL`, because after invoking the New project → Project name → Create project sequence, no accessible heading named `NativeQA-fc8594d2-A` appeared within the selector timeout. The current artifact has only the initial native screenshot, not a post-failure screenshot, so **do not attribute the failure to a product mutation defect yet**. A subsequent commit adds failure-snapshot and AT-SPI tree capture *before* GUI quit. `cleanup_warning` is absent in this run: Openbox graphical quit recovery worked.

- Linux exact-source .deb SHA-256: `98f20dc21d483183ee1e6309731a08b83d0f7e1356050c9ce598382e04ea8f06`
- Linux installed executable SHA-256: `808544f2aaf3b235dc7f3c78b13d5abb2d3029bc0c5cf1cf2510a2546a6156d4`
- [Small Linux evidence ZIP](https://github.com/MousaXD/scriptotar/actions/runs/43897349298/artifacts/14239444701) and [full .deb evidence](https://github.com/MousaXD/scriptotar/actions/runs/43897349298/artifacts/14241750374)

**Windows:** The *same Windows hosted runner* confirms an interactive desktop (sessions 2/2), six WebView2 processes, trusted Tauri-owned child HWNDs, and a **genuine UIA Document** on `Chrome_RenderWidgetHostHWND`. However the document is a **localhost network error page** (`ERR_CONNECTION_REFUSED`) and contains no Scriptotar Studio button. The script's original `installed_webview=NOT TESTED` instrument classification was too coarse: this is a **broken installed frontend payload / dev URL product packaging** observation, not absent UIA. The Windows CI job used `cargo build --release --locked -p scriptotar-desktop` followed by `tauri bundle`, i.e. bundled an executable which appears to use `build.devUrl=http://localhost:5173`. The workflow now instead calls the supported `tauri build --bundles nsis -- --locked` so production-mode embedded assets are compiled. Test logic also explicitly reports `FAIL` if a genuine installed UIA document displays `ERR_CONNECTION_REFUSED`. These changes need a **new exact-head rerun** to prove resolution; the previous ZIP cannot prove the fix.

- Windows installed NSIS SHA-256: `a4d3846013e96ba74c2f4a394fe53fce73cf0eb91c343496202144b43415d287`
- Windows installed executable SHA-256: `14b2eacbacad2121b775135855995e2f3b13bb23b7617846a159b61d43d44144`
- [Small Windows evidence ZIP](https://github.com/MousaXD/scriptotar/actions/runs/43897349298/artifacts/14240018444) and [full NSIS evidence](https://github.com/MousaXD/scriptotar/actions/runs/43897349298/artifacts/14240467880)

| Installed release-blocking case at `fc8594d` | Linux WebKitGTK | Windows WebView2 |
|---|---|---|
| Exact-source package installed with hashes | PASS | PASS |
| Actual native WebView document and Scriptotar controls | PASS | FAIL (network-error document instead of Scriptotar) |
| Create two projects, switch both directions | FAIL (new heading not observed) | NOT TESTED |
| Create Study, edit/save/select three candidates | NOT TESTED | NOT TESTED |
| Graphical quit/restart and SQLite persistence | NOT TESTED | NOT TESTED |
| Keyboard navigation/custom controls; Arabic RTL; visual themes | NOT TESTED | NOT TESTED |
| Revocation, cross-project forged IPC, real multi-source, fault states | NOT TESTED | NOT TESTED |

**Status:** **NO-GO.** Agent 07 has only changed the QA-owned harness, native packaging workflow, diagnostics, tests, handoff and instructions. Agent 00 owns production code and genuine fixture contracts. Neither first-gate accessibility success nor ordinary CI success implies installed Study persistence acceptance.

## Exact-head diagnostic recovery — second native run, 2026-11-16

**Observed run:** [#43879991872](https://github.com/MousaXD/scriptotar/actions/runs/43879991872), source commit `a233911d7a62ed70320f93b9d25ff3f17399da3e`. Both native jobs **FAILED their release gate**. Actual installation and rendering evidence was obtained, but native accessibility-driven user interaction was **NOT TESTED**.

**Linux WebKitGTK:** the installed deb and binary were verifiably installed and the D-Bus session activated `org.a11y.atspi.Registry`. The actual `scriptotar-desktop` process (PID 26382) exposed an AT-SPI application and a native Scriptotar frame. A captured screenshot shows the rendered home UI. But the diagnostic AT-SPI traversal stopped at depth 5, exactly at an unnamed scroll pane. No `document web` or actionable Studio control was observed. First failing case: `installed_webview = NOT TESTED`. Other graphical and persisted cases were not executed. The new harness expands diagnostics beyond depth 5 and no longer prunes whole WebKit subtrees solely on `STATE_VISIBLE`. This is a test-harness correction, **not proof the WebKit document is exposed**.

Linux package SHA-256: `229f43466c8770086fe505955b7849067692daf5a847d8a030141d015c7ad439`.
Linux installed binary SHA-256: `808544f2aaf3b235dc7f3c78b13d5abb2d3029bc0c5cf1cf2510a2546a6156d4`.

**Windows WebView2:** the Windows Server 2025 hosted runner reported `UserInteractive=true`, runner session 2, app session 2, and a valid native Tauri HWND. Its UIA RawView tree exposed a `Tauri Window` with one `WRY_WEBVIEW` Pane but **no Document provider or actionable Studio control**. The captured screenshot showed a blank white webview area; treat this as a readiness, startup or accessibility-provider finding, not a certified product defect. The old runner stopped checking as soon as the native shell HWND appeared. The updated runner now polls for 90 seconds, captures readiness metadata and enumerates only child HWNDs of the installed app to find an independent WebView2 UIA provider. It still requires actual UIA Studio invocation and heading verification for PASS.

Windows installer SHA-256: `e4826f754ab668968bab7dfde119cd60bb61ced59edd73a7e06a08d1d019b318`.
Windows installed binary SHA-256: `9267e1132ba9888574df3e6c39cadcbf209c73486bf71c0a642e92f27be06473`.

**Inspect original artifacts:**
- [Linux small diagnostic ZIP](https://github.com/MousaXD/scriptotar/actions/runs/43879991872/artifacts/14232259592) — result.json, a11y-diagnostics.json, screenshot, SHA256SUMS, host.txt
- [Windows small diagnostic ZIP](https://github.com/MousaXD/scriptotar/actions/runs/43879991872/artifacts/14232636975) — result.json, uia-diagnostics.json, screenshot, host.json
- [Linux full package evidence](https://github.com/MousaXD/scriptotar/actions/runs/43879991872/artifacts/14233067625)
- [Windows full installer evidence](https://github.com/MousaXD/scriptotar/actions/runs/43879991872/artifacts/14233112003)

| Second run at `a233911d` | Linux | Windows |
|---|---|---|
| Exact-source checkout / package build / installation | PASS | PASS |
| Hashes and host evidence preserved | PASS | PASS |
| Actual native process/window observed | PASS | PASS |
| Rendered app screenshot observed | PASS (home rendered) | FAIL (blank web area) |
| Web content accessibility document + actionable interaction | NOT TESTED | NOT TESTED |
| Study create/open, candidate edit/save/selection | NOT TESTED | NOT TESTED |
| Quit/restart persisted reopening | NOT TESTED | NOT TESTED |
| Keyboard custom controls, Arabic RTL and theme review | NOT TESTED | NOT TESTED |
| Multi-source, revocation, error states and three-script integration | NOT TESTED | NOT TESTED |

**Recovery commits** on Agent 07's branch are harness-only. They need **their own exact-head CI** before any row can advance. Until the installed document is exposed and real persisted interactions are observed, retain **NO-GO**. The Windows hosted desktop is interactive at the shell level; a fully functioning WebView2 provider has not been established.

**Agent 00 dependency:** provide a disposable authorized Study/project source fixture and a controlled revocation/failure state, plus the integrated production three-script contract. Those inputs must not be simulated as paid provider output, and foreign-project UI isolation must not be represented as an IPC authorization test.

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
