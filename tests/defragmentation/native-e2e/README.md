# Installed native WebView acceptance (Agent 07)

**Scope:** real installed Tauri Linux WebKitGTK and Windows WebView2. These are independent of the Chromium mocked GUI suite and the existing installed backend-IPC test.

## Guarantees and evidence

- The dedicated workflow builds each package **from the exact immutable PR head SHA**, installs that package, and hashes the installed executable and installer.
- Linux runs the installed desktop executable under Xvfb + D-Bus, then checks the actual process-owned WebKitGTK accessibility document via AT-SPI. It interacts with project/Study/draft controls using AT-SPI actions and X11 keystrokes; it does **not** run a web server, inject a frontend mock, or use the backend smoke CLI.
- Windows installs NSIS under a fresh directory and attempts UI Automation against the real app window and WebView2 Document provider, invokes Studio, and captures a screen image when possible. A headless hosted Windows runner may not expose an interactive desktop, in which case the result remains **NOT TESTED**.
- Each runner generates a per-case PASS / FAIL / NOT TESTED JSON matrix. Every run records source SHA, exact GitHub Actions run URL, executable/package SHA-256, OS/CPU/memory info, timestamp, and screenshots when the native GUI can capture them.
- A partial PASS **never** causes a green release gate. Both runners exit nonzero until every required scenario passes. The Python unit tests only check reporter invariants and syntax, never application behavior.

## Remaining required fixtures

The Linux journey creates fresh projects, one Study, a hand-edited script candidate, duplicates it, renames it, switches projects, tests the Arabic language and dark/system selectors, and tests quit/relaunch. A disposable new profile intentionally has no real downloaded research or transcript sources. Multi-source transcript preview, source revocation, forged cross-project API requests, deterministic GUI fault recovery, independently inspected light/dark screenshots, full Arabic RTL, and genuine AI candidate orchestration are therefore **NOT TESTED**. Do not substitute fixtures pretending to be provider output or describe UI isolation as backend authorization proof.

The Windows probe is currently **initial native discovery and Studio invocation only**. Port the Linux workflows to Windows UIA once an actual interactive desktop and stable provider tree are confirmed; each unimplemented scenario stays NOT TESTED.

## CI

Workflow: .github/workflows/installed-webview-acceptance.yml. Trigger by opening/updating a PR with paths under this directory, or start workflow_dispatch for a specific commit/ref after merging this workflow into an accessible branch.

For an independently packaged Linux binary, run under a **real X11 window manager**.
Bare Xvfb has no window manager and cannot certify graphical Alt+F4 quit. Install
`openbox`, `xvfb`, `dbus-x11`, `xdotool`, `scrot`, `python3-pyatspi` and
AT-SPI support, then run from the exact source checkout:

```bash
export APP_BIN=/usr/bin/scriptotar-desktop
export PACKAGE=/path/to/installed.deb
export SOURCE_SHA=FULL_40_CHAR_SHA
export RUN_URL=https://github.com/MousaXD/scriptotar/actions/runs/RUN_ID
export EVIDENCE=native-evidence/linux
mkdir -p "$EVIDENCE"
xvfb-run -a -s '-screen 0 1440x900x24' dbus-run-session -- bash -euo pipefail -c '
  openbox --sm-disable > "$EVIDENCE/openbox.log" 2>&1 &
  wm_pid=$!
  trap "kill $wm_pid 2>/dev/null || true" EXIT
  sleep 1
  NO_AT_BRIDGE=0 GTK_A11Y=1 GTK_MODULES=atk-bridge \
    /usr/bin/python3 tests/defragmentation/native-e2e/linux_webkit.py \
    --executable "$APP_BIN" --package "$PACKAGE" \
    --source-sha "$SOURCE_SHA" --run-url "$RUN_URL" \
    --output "$EVIDENCE"
'
```

Replace each placeholder with the actual installed package, exact commit SHA,
and real run metadata. The run URL is an evidence reference, not proof a manual
machine execution ran in GitHub Actions. The installed binary must be discovered
from the real package (the workflow uses `dpkg -L`) and hashes preserved.

**Windows packaging gotcha exposed by native UIA:** [exact run #43897349298](https://github.com/MousaXD/scriptotar/actions/runs/43897349298) showed a *live* WebView2 Document with `ERR_CONNECTION_REFUSED` from localhost instead of the Scriptotar UI. The previous job used plain `cargo build --release` plus `tauri bundle`, which can package a binary configured for `build.devUrl`. The harness workflow now invokes `tauri build --bundles nsis -- --locked` to build production-mode frontend assets, following the same procedure as Linux. Do not bypass this by starting a dev server; a packaged release must render offline.


## Run against a real interactive Windows desktop

The November 16 hosted Windows Server 2025 run [#43879991872](https://github.com/MousaXD/scriptotar/actions/runs/43879991872) **was interactive at the top-level window** (runner and app session 2), but UIA showed only `Tauri Window > WRY_WEBVIEW Pane`, while the screenshot was white. Do not describe this as a proven headless session. The upgraded probe waits up to 90 seconds for content and checks descendant WebView2 render-widget HWNDs, whose UIA provider can be disconnected from the parent Pane. It still refuses PASS unless a WebView2 Document, actionable Studio control and post-invocation heading are observed.

To independently reproduce on a Windows 11 native-machine, sign in to an **unlocked, interactive desktop** with a normal, non-elevated user, a working WebView2 Runtime and PowerShell 7. Use the repository at the **exact source commit** used to produce the NSIS installer. Run from the repository root:

```powershell
$sha = (git rev-parse HEAD).Trim()
$run = 'https://github.com/MousaXD/scriptotar/actions/runs/43879991872'
$installer = (Resolve-Path 'C:\path\to\Scriptotar_1.4.1_x64-setup.exe').Path
New-Item -ItemType Directory -Force '.\native-evidence\windows' | Out-Null
& '.\tests\defragmentation\native-e2e\windows_webview2.ps1' `
  -Installer $installer -SourceSha $sha -RunUrl $run `
  -EvidenceDir '.\native-evidence\windows'
```

Replace the example installer path with the **actual exact-SHA NSIS artifact**, and associate the manual test with its real execution date, operator, OS build, installer SHA and binary SHA. The supplied run URL identifies the original CI artifact, **not** a new machine test run. Include the `result.json`, `uia-diagnostics.json`, screenshots, WebView2 Runtime version and a note identifying the manual host. A locally run test is not hosted-CI success. A missing provider is NOT TESTED, not PASS.

If the WebView2 provider remains absent, inspect descendant HWND class names and PIDs in `uia-diagnostics.json`, review whether `msedgewebview2.exe` has started, capture Windows Event Viewer application errors and verify the desktop remains unlocked. A white app region is insufficient to conclude an accessibility-only issue.

**Current limits:** the Windows script still implements only installed WebView2 discovery and a Studio invocation. Full project/Study/candidate/persistence and Arabic scenarios must be ported and executed on real UIA controls; until then the Windows release gate remains red. Neither screenshots nor backend IPC tests can replace these journeys.

## Evidence authenticity and review

Keep screenshots and exact run artifacts; the JSON is a **machine-generated claim**, not independent human visual approval. A native PASS also requires a matching source commit, installed binary provenance, reviewed screenshots, verified source fixtures, accessibility audit, and release owner signoff. Issue #100 and #116 remain release blockers; Agent 00 owns merging. No release flags are changed here.

## 2026-11-17 evidence-based fixes

The actual installed Linux dashboard renders its h1 as AT-SPI `document frame` and the topbar project selector as `PROJECT`. Match only exact-name semantic heading/document frame roles and also require the real dashboard New project button; a chooser option alone is insufficient. Pending new native-run proof, project persistence and downstream cases are NOT TESTED.

Npm Tauri 2.11.2 rejects `tauri build --bundles nsis -- --locked`. Validate `cargo metadata --locked` before/after the genuine production `tauri build --bundles nsis`, and assert no tracked lockfile diff. Do not return to bare `cargo build --release` + bundle: integrated run 43956319863 showed a localhost error page. Classify the exact app-owned browser error UIA pane as FAIL.

## Installed Linux option role and actual project database

Exact native run 44091975817 created and displayed both project dashboards, then failed when a displayed project option was found in AT-SPI as role `list item` rather than `option`. Choose by exact accessible name and the observed role; continue requiring the real switch and active dashboard. Independently read the installed app's own `SCRIPTOTAR_DATA_DIR/scriptotar.sqlite3` in SQLite read-only mode to verify both project rows exist before marking `canonical_project_persistence` PASS. Unit tests for the probe or selectors **are not** native acceptance.

## Windows Cargo.lock ordering-only rewrite on the actual NSIS build

Exact run 44091975817 built the true NSIS production installer, but the workflow's Git byte-diff detected only `scriptotar-agent` being reordered within the `scriptotar-desktop` package dependency list. Windows Cargo 1.97 normalizes order. Do not weaken reproducibility by ignoring arbitrary `Cargo.lock` differences: canonicalize parsed TOML to stable JSON and compare SHA-256 before and after the production `tauri build --bundles nsis`, with `cargo metadata --locked` pre/post. A real checksum/version/graph change still fails. Installed WebView2 requires its own later exact-commit run and UIA proof.

The lockfile fingerprint is a standalone, regression-tested `cargo_lock_fingerprint.py`. TOML dictionary key sorting is insufficient: it must normalize `[[package]].dependencies` order, while retaining all identities, versions and checksums. The workflow hashes the full normalized parsed lockfile, not just package count or dependency names.

The Linux workflow runs unit/contract tests with pinned Python **3.12** because Cargo.lock fingerprinting uses stdlib `tomllib`. The real AT-SPI WebKit acceptance still executes `/usr/bin/python3` with `python3-pyatspi`; test interpreter separation never substitutes mocked UI for installed GUI.
