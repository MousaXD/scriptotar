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

## Installed integration run #43897947678 — independently inspected (2026-11-16)

Production integration source: `e65e08e732de21dd29725438afc7712be3deb799` (this is **not** the Agent 07 PR head).
Run: https://github.com/MousaXD/scriptotar/actions/runs/43897947678

I inspected the actual **small review ZIPs**, not merely the job conclusion:
- Linux: https://github.com/MousaXD/scriptotar/actions/runs/43897947678/artifacts/14241565980 — `result.json`, boot screenshot, host, checksum file, and empty desktop log. The initial screenshot shows a rendered Scriptotar shell **while workspace bootstrap was still displaying skeleton/loading text**. The run reported `installed_webview=PASS` based on an actual process-owned AT-SPI WebKit document and Studio control; `create_switch_projects=FAIL` because the expected dashboard h1 `NativeQA-e65e08e7-A` never appeared. The older integrated source **did not capture a post-click screenshot/tree**, so no claim that project creation committed to SQLite is justified. All downstream Studio and persistence cases were **NOT TESTED**.
- Windows: https://github.com/MousaXD/scriptotar/actions/runs/43897947678/artifacts/14240258082 — actual installed window screenshot is WebView2's **localhost refused-to-connect error page** (`ERR_CONNECTION_REFUSED`), not Scriptotar. Diagnostic UIA exposes a trusted `Chrome_RenderWidgetHostHWND`, a real `ControlType.Document`, and text `ERR_CONNECTION_REFUSED`; hosted Windows Server 2025 had an interactive session 2 and six `msedgewebview2.exe` processes. The old runner reported `installed_webview=NOT TESTED` because it could not find Studio; **the packaging/application state is a concrete FAIL**, not a desktop/UIA availability limitation. All downstream cases remained **NOT TESTED**.

| Native installed scenario | Linux exact-run result | Windows exact-run result |
|---|---|---|
| Production package installed, executable SHA captured | PASS | PASS |
| Actual installed WebKit/WebView automation against Scriptotar controls | PASS | FAIL — browser network error; old `result.json` says NOT TESTED |
| Two projects created, selected, active dashboard confirmed | FAIL — missing first project heading; post-action state unobserved | NOT TESTED |
| Study open/create, candidate edit/save/rename/selection | NOT TESTED | NOT TESTED |
| Graphical quit and actual data restoration after relaunch | NOT TESTED | NOT TESTED |
| Wrong-project UI isolation and forged IPC denial | NOT TESTED | NOT TESTED |
| Multiple authorized real source attachments and source revocation | NOT TESTED | NOT TESTED |
| Keyboard, custom controls, Arabic RTL, high zoom and visual themes | NOT TESTED | NOT TESTED |
| Integrated production three-script workflow | NOT TESTED | NOT TESTED |

Immutable run hashes recorded by the runners:
- Linux `.deb` SHA-256: `533b6e065c8748510071f0cc1bc49b1cf757964a5b3a4ba7acf6b438c1957d57`.
- Linux installed `/usr/bin/scriptotar-desktop` SHA-256: `d67e0de8f01e3962977428db44ab7f0357ff9b5ce07ca5e3b191de41c1d66918`.
- Windows NSIS SHA-256: `b58bedb2820a85f049a01c99f4b4525d510a498e223ecf9d1c0ae6b5cb2f94da`.
- Windows installed executable SHA-256: `f4922f3269ba0895189b6fb6c6ce4713d402ab2c9050e43a8c722f3ffd702504`.

### Agent 07 QA-only recovery after the integrated run

The Agent 07 branch previously added production `tauri build --bundles nsis -- --locked` on Windows instead of bundling a bare dev-mode `cargo build` binary; the integrated run above predates that fix. Current recovery commits additionally:
1. Wait for a real accessible first-project or ready dashboard state, and handle an empty-profile `Create your first project` form. Maintain the **actual dashboard h1** assertion after creation; do not substitute a dropdown entry for proof of activation.
2. Capture an explicit post-create screenshot plus bounded PID-owned AT-SPI tree/expected name on heading failure so the next run can separate failed UI action, backend error, stale state, and selector defects.
3. Detect a genuine WebView2 `ERR_CONNECTION_REFUSED` document during readiness polling, classifying it as an **installed packaging FAIL** promptly instead of a 90-second unexplained missing-document condition.

These modifications are **not native PASS evidence** until a new exact-head installed workflow has run. If no PR synchronize run appears, manually dispatch **Independent Installed WebView Acceptance** against `agent-07/p0-native-webview-qa-20261116`, then compare the run's reported source SHA with the branch head. Agent 00 must separately provide controlled multi-source fixtures and the integrated three-script workflow. Until both platforms complete real installed GUI/SQLite persistence, release certification stays **NO-GO** and this PR stays draft.

## 2026-11-17: Exact artifacts independently inspected, corrections pending CI

QA branch source `ea6160556fcefc38bfb41e5045ed2c5177463c77`, [run #43955788990](https://github.com/MousaXD/scriptotar/actions/runs/43955788990), Linux review artifact **14272114540**: `project-1-failure.png` shows **NativeQA-ea616055-A** visibly on the real installed dashboard with a live **New project** button. Exact PID-owned AT-SPI tree `a11y-diagnostics.json` exposes its h1 as `document frame: NativeQA-ea616055-A`, not `heading`, and the topbar chooser as `combo box: PROJECT` (uppercase). Thus the failed h1 role check was a **harness selector false negative**, not proof project creation failed. The new runner requires an exact-name semantic heading/document frame **and the dashboard New project action**; it uses the observed topbar name. The screenshot alone does not prove SQLite persistence; second project/switch and downstream actions remain **NOT TESTED** pending new installed execution.

Windows on the same QA head failed in the **build** step before NSIS installation because npm Tauri CLI 2.11.2 rejected `-- --locked`. The repaired workflow checks `cargo metadata --locked` before/after **Tauri's production build** `tauri build --bundles nsis` and ensures no tracked lockfile changed. Earlier integration source `46cb36efe2ca4e6ec3f9dd51b7b99c2d59804583`, [run #43956319863](https://github.com/MousaXD/scriptotar/actions/runs/43956319863), Windows UIA review artifact **14273167426**: NSIS installed, native window and WebView2 Document existed but screenshot shows localhost `ERR_CONNECTION_REFUSED` instead of Scriptotar. The actual root UIA pane is `localhost - Network error - Web content`; the new runner detects it as **packaging FAIL** before the full readiness timeout, instead of NOT TESTED.

Exact QA Linux .deb SHA-256 `65b143ca14eccdfea6872b2235e4dbe735f6454d9ea0e34508eef9cca04adff3`; installed executable `e4ee295cec72dbb0c2fe5e54c06f91bb704cb7d3521bce2ab9951c3410ce77a4`. Integrated Windows NSIS SHA-256 `d3bfca9388fd4add6ac1628a7fb989eaf31cc01b3b509ca07c654d87a7fda8ee`; installed executable `3d93505da09b6a12986a1f76c180bbc6bcb1f4ad8fa122b6bc8c624900c5b717`.

**Current release matrix:** Linux installed document PASS; Linux create/switch FAIL in old run (role mismatch; correction unverified), everything after project NOT TESTED. Windows QA packaging FAIL; integrated Windows app rendered a network-error page FAIL; Studio and every downstream Windows scenario NOT TESTED. No PASS is inferred from this patch. Keep PR #126 DRAFT, release NO-GO; request Agent 00's authorized controlled source fixtures and product defect triage if later UI or persistence genuinely fails.

## Exact installed QA run #44091975817 — Linux project chooser selector recovery

Source `a6b3ae9c50ab5549ea6f72ee5613da4c8cc7410a`; [installed workflow](https://github.com/MousaXD/scriptotar/actions/runs/44091975817), Linux review artifact **14332143767**. The production .deb built, installed and executed under native WebKitGTK. Reported: `installed_webview=PASS`; `create_switch_projects=FAIL`; all downstream cases `NOT TESTED`.

The inspected screenshot `failed-create_switch_projects.png` shows **both new project dashboards successfully created** (individual `project-1-created.png`, `project-2-created.png` also captured), and the topbar project chooser open with both names. The actual PID-owned AT-SPI tree exposes exact choice labels `NativeQA-a6b3ae9c-A` and `NativeQA-a6b3ae9c-B` as role **`list item`**, not `option` (and popup parent `comment Project`). The runner had clicked the real combobox but rejected real options due to a hardcoded `option` role. This is a confirmed automation selector defect, **not evidence of a broken project chooser**; its actual switch action and persistence remain NOT TESTED until re-run.

Fix pending final-head CI: select exact-name native `list item` instead of unavailable `option` role. Add a **read-only** check of `SCRIPTOTAR_DATA_DIR/scriptotar.sqlite3` for two distinct persisted active project IDs after both native GUI creates; report separately as `canonical_project_persistence`. This is not a mock and does not replace native switching assertions. Windows job for this source was still building its NSIS package when Linux artifacts were inspected; no Windows PASS is inferred.

## Exact Windows CI result from run #44091975817 — build PASS, post-build gate FAIL

Run [44091975817](https://github.com/MousaXD/scriptotar/actions/runs/44091975817), Windows job **134069147206**: source `a6b3ae9c50ab5549ea6f72ee5613da4c8cc7410a`. The corrected `tauri build --bundles nsis` **successfully built** a production binary (Cargo release finished in 6m33s) and generated `target/release/bundle/nsis/Scriptotar_1.4.1_x64-setup.exe` via NSIS. The job nevertheless **failed before staging/installing or WebView2 UIA** because an overstrict `git diff --exit-code Cargo.lock` detected Cargo's **ordering-only** change within `scriptotar-desktop` dependencies: `scriptotar-agent` moved after `keyring`/`rusqlite`. There is **no evidence of package/checksum/version drift** in that diff. Reproducibility must still fail on any material lockfile difference.

The new workflow computes a SHA-256 of fully parsed/canonicalized `Cargo.lock` TOML using pinned Python 3.12 `tomllib` and `json.dumps(sort_keys=True)`, before and after Tauri production bundling, plus `cargo metadata --locked` pre/post. Dependency order-only rewrites are accepted while actual resolved versions/IDs/checksums/metadata still fail. This is a **new untested patch** until its own exact-SHA installed Windows CI produces native UIA evidence. Previous Windows native localhost error still stands as a historical packaging FAIL; the newer production package is not yet certified as installed. Release NO-GO.

### Lock fingerprint correction on the next exact source

The first implementation used JSON `sort_keys=True`, which normalizes TOML object keys **but not list order**. The actual Windows diff is specifically the order of `[[package]].dependencies`; therefore a JSON-key-only digest would still falsely fail. The revised implementation uses a separate unit-tested `cargo_lock_fingerprint.py`: it parses `Cargo.lock`, sorts each package's dependency-name list (which is order-insensitive), keeps the rest of the full parsed lock graph including exact versions/checksums/sources intact, then SHA-256 hashes canonical JSON. A regression test enforces equality for dependency reorder and inequality for a material dependency change. Still **NOT INSTALLED-PASS** until new exact CI completion.

## 2026-11-17: run #44095877399 native graphical evidence and targeted correction

**Source:** `cede4c861bb7249f451215608843f5e21cf50fc4`. Linux production .deb SHA-256 `075ca75a9fccfcd0f90b659107b6982f9eed2e33ec0409c38989f3f26f2431bb`, installed executable `e4ee295cec72dbb0c2fe5e54c06f91bb704cb7d3521bce2ab9951c3410ce77a4`. Review artifact 14334093720: `failed-create_switch_projects.png` SHA-256 `aedc8758f8272f79de18a7d0f5b15a0f1b5f417f59c451270a0b3bb055a84bab`. Canonical SQLite two projects **PASS**. Native UI screenshot and PID-owned AT-SPI tree still show B dashboard active after attempting to choose A; popup closed but no A navigation. Thus `create_switch_projects=FAIL`, not a missing-heading selector. The generic AT-SPI list-item activation/focus Return did not perform the switch. **New patch** targets the exact named option's native AT-SPI bounding rectangle with a real X11 mouse click, then requires the actual A/B semantic dashboard headings and New project control; no data-state shortcut, must rerun for proof.

Windows same run, artifact 14335016666: production NSIS SHA-256 `7dbe39e4bdadb402c4bd23ef5580d2daa350313daf26dc590b32d935ea96468b`, installed EXE `63529179e73f1af288985fbf36bf26b12c8be158421585c63655839baf533e65`. Real WebView2 UIA document + Studio button **PASS**, Studio invoked and real screenshot `installed-webview2.png` SHA-256 `b62bfc9a6fd9d2509bdc8bea2320f7ae110a4927bdce2c18093f592a7b2e3f99`; `dev_server_error_seen=false`. Red Windows job is caused by incomplete **NOT TESTED** scenarios, not packaging. Full graphical journey remains unproven. Keep PR DRAFT / NO-GO.

## Windows exact-head follow-up: real project/Study UIA journey (new patch, unverified)

The first Windows UIA gate genuinely PASSED on exact source `cede4c86`, but all subsequent scenarios were NOT TESTED because the harness stopped at opening Studio. New QA-only code **attempts** real installed-WebView2 project A/B creation and switching via trusted render HWND; text fields receive native user keyboard events; custom options are clicked using exact-named UIA geometry; selected dashboards must expose both current project text and the actual New project action. It then attempts Study creation in project A, requiring the candidate controls. Each step has explicit FAIL and screenshot/UIA evidence, never a simulated success, and all other unimplemented cases remain NOT TESTED. This is **not yet verified** until the new exact-SHA Windows job finishes; release stays NO-GO.

## Run #44100162447 — first post-pointer real graphical failures (2026-11-17)

**Source** `71f0205e6d58abb4b334bc1ac3907c0c701fdf8f`. Linux: installed WebKitGTK + Studio control **PASS**; SQLite 2-project persistence **PASS**; **A→B→A native project switching PASS**, with `project-switch-to-A.png` (SHA-256 `883a7bd31714d1cee3d31b48b4216658cc6af80975fc4f5d21e7d78516face07`) and B screenshot. **First new failure** `create_study=FAIL`: `AT-SPI control not found: role=None, name='Study name'`. Actual PID-owned native tree shows `embedded 'Study name e.g. Launch video'`, because WebKit combines label and placeholder. Screenshot `failed-create_study.png` SHA-256 `aa7e16165079097f5958b75b70acb93de43783f1f8c9ec2cf86c918a22c106aa` visibly confirms real Studio/Creative brief. Corrective test now selects only a real editable AT-SPI role with exact label or label-plus-placeholder prefix; it does not match static labels/buttons and must pass actual Study creation before proceeding. Linux installed .deb SHA-256 `b840673f1145fff6f4333d8f8eb50b0c0d97f0e90acb3a9d00570fe7f0c84253`, installed exe `e4ee295cec72dbb0c2fe5e54c06f91bb704cb7d3521bce2ab9951c3410ce77a4`.

Windows exact source: installed NSIS + real WebView2 Document + Studio invocation **PASS**, two projects **created through actual UIA** and their dashboard screenshots captured, but `create_switch_projects=FAIL` because A activation did not replace B. `failed-create_switch_projects.png` SHA-256 `1401b9560d1414e94bd0e5ca8662cf117425cf467ebc84f9b936b1263bc76ef9`; UIA still names B as active. Windows harness tried list-item pointer but failed to change page. Corrective test now derives the exact target option's keyboard position from accessible on-screen UIA list item order, then sends native Home/ArrowDown/Enter to the actual focused Svelte listbox; it **still requires** the visible target dashboard to PASS and fails closed otherwise. Windows NSIS SHA-256 `39861edba6d1acf6411d328f7804f6e952e7e733e248435c8363826b90d2ae72`, installed exe `c714c8a54a54c5425f8c5ca6ede9a0b6b7744e5d0a13067215d0e3f03d1e71ab`.

**Both corrections are source-level hypotheses until a fresh exact-source installed run.** Linux draft/restart/RTL and Windows Study/draft beyond first failed switch remain NOT TESTED. Release NO-GO, PR DRAFT.


## 2026-11-17 — exact-source Arabic/Windows focus recovery (Agent 07)

**Last inspected native run:** [#44118342637](https://github.com/MousaXD/scriptotar/actions/runs/44118342637), immutable installed source `ffe1e8282df4d2e3bde695926b866d9d18ec7b08`. Both platform jobs FAIL overall. Review ZIPs: Linux artifact `14344164393`, Windows artifact `14344259372` (these contain screenshots, native accessibility tree and per-case JSON). Do not confuse previously passing Linux cases with an overall native-acceptance PASS.

### Actual case evidence at ffe1e828

| Scenario | Installed Linux .deb / WebKitGTK | Installed Windows NSIS / WebView2 |
| --- | --- | --- |
| Native app document + Studio | PASS | PASS |
| Real project A/B create and bidirectional switch | PASS | FAIL at unsupported UIA List `SetFocus()` before switch |
| Two canonical SQLite project rows | PASS | NOT TESTED |
| Real Study creation through GUI | PASS | NOT TESTED |
| Draft edit/save, copy, rename and immutable latest versions | PASS | NOT TESTED |
| Graphical quit/relaunch, selected project and two candidates | PASS | NOT TESTED |
| Wrong-project UI isolation | PASS | NOT TESTED |
| Custom candidate keyboard Escape | PASS | NOT TESTED |
| Arabic semantic heading, locale change, return to English | FAIL at mixed-language **English restoration**, not Arabic entry | NOT TESTED |
| System/Dark theme settings and persistence | NOT TESTED | NOT TESTED |
| Full Arabic RTL, independent visual Light/Dark review | NOT CERTIFIED (Arabic screenshot manually inspected; automated end-to-end RTL and Light mode unavailable) | NOT TESTED |
| Three-original-script UI, real source/media/transcript, source revocation, forged IPC refusal | NOT TESTED | NOT TESTED |

Linux package SHA-256: `d44b0c986efdcbf378f4d141ab541ba7dad7da8b21ae3d83d7346862fd7be2dc`; installed executable SHA-256: `e4ee295cec72dbb0c2fe5e54c06f91bb704cb7d3521bce2ab9951c3410ce77a4`. Windows NSIS SHA-256: `c7bdf41926d73d3aa6b9a03e866bcfebd1803e79615f9d03136cae008e587478`; installed executable SHA-256: `22583632d0369a724da5de447bf4dcd70ba0b5892405e6da6f283b41d2dae027`.

**Arabic forensic evidence:** `arabic-locale.png` SHA-256 `f85efe97e917d1729a31230779caceb869482b38d29dfd0d999d8ec254db573e` visibly shows Arabic Studio, Arabic right-hand navigation and right-to-left layout. After selecting English, `failed-arabic_locale_control.png` SHA-256 `7c5404cbc6b3098edd08a25b63bf5df5120c88fc4ace13614d3c280efa36ad99` shows English navigation with `EN` but Arabic Studio content/heading. The native tree simultaneously exposes English `Studio` navigation and Arabic semantic `الاستوديو` heading. Treat as an observed mixed-locale product regression; investigate Svelte locale re-render versus the global `i18n.ts` DOM translation observer, without assuming root cause. Reported directly to Agent 00 in [PR #114 comment](https://github.com/MousaXD/scriptotar/pull/114#issuecomment-6955022253).

**Windows forensic evidence:** `failed-create_switch_projects.png` SHA-256 `f88ee5c9b3fea2386f2d88bd8d3f6a611032b25b0c78693a420e582211b385e0` retains the B dashboard; actual UIA error is `Exception calling "SetFocus" ... "Unrecognized error."`. The control hierarchy is real WebView2 `ComboBox 'PROJECT'` → popup `List 'Project'` → concrete `ListItem`. No native switch PASS can be inferred from having created B.

### Subsequent QA-only commits (new native run required)

- `76a5e2a`: remove unsupported popup List `SetFocus()`, target the verified native List-owned option with actual pointer coordinates and require popup closure and existing exact dashboard/candidate assertions.
- `47cd5d5`: Arabic locale case verifies translated native Studio nav and heading, quits via real Alt+F4, relaunches Arabic, then requires both shell and Studio return to English. Mixed-locale remains FAIL, never normalized.
- `2de4da3`: update fail-closed contract checks for pointer selection and Arabic restart.
- `f3430c8`: require closed popup for both ListItem and Button-typed UIA options.
- `adcdc63`: execute actual System/Dark Settings controls with save and graphical restart before the independent Arabic regression. This version of Settings exposes only `system` and `dark`, **no `light` option**; do not invent a Light control or certify visual contrast based solely on a settings label.

These commits have been pushed to the existing PR #126 branch, targeting integration PR #114. **No new exact-source installed CI result was observed as of this handoff**; green CI/security from `ffe1e828` does not validate these edits. Both installed platform gates remain **NO-GO** pending fresh screenshots/reports and actual Linux and Windows installed jobs. PR #126 must stay DRAFT. Do not merge or publish.

### Next actions

Dispatch `Independent Installed WebView Acceptance` on the QA branch and inspect its *exact* source SHA before assigning any PASS. Check Linux System/Dark screenshot and real theme restore, then Arabic semantic nav/RTL after restart and English restoration; coordinate any reproducible mixed-locale defect fix with Agent 00 (QA must not silently correct it). Check Windows project switch, canonical DB, Study, independent original/copy candidate selection, true quit/relaunch and wrong-project UI. Keep real multi-source transcript, source revocation, forged wrong-project IPC and three-original-script interaction NOT TESTED until their production commands and deterministic non-paid fixtures exist. Save installer and installed-executable hashes, screenshot/tree paths, verified source and exact case statuses at every attempt.
