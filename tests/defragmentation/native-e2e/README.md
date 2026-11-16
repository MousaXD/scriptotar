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

For an independently packaged Linux binary, invoke on a graphical test host:

    xvfb-run -a dbus-run-session -- env NO_AT_BRIDGE=0 GTK_A11Y=1 \
      /usr/bin/python3 tests/defragmentation/native-e2e/linux_webkit.py \
      --executable /usr/bin/scriptotar-desktop --package /path/to/installed.deb \
      --source-sha FULL_40_CHAR_SHA \
      --run-url https://github.com/MousaXD/scriptotar/actions/runs/RUN_ID \
      --output native-evidence/linux

Use the *actual* dpkg-installed executable path; /usr/bin/scriptotar-desktop above is only an example, not an asserted path. Do not pass a Vite preview binary, browser runner, backend smoke executable, or user profile. The workflow discovers the correct path with dpkg -L.

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
