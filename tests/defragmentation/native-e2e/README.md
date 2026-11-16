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

## Evidence authenticity and review

Keep screenshots and exact run artifacts; the JSON is a **machine-generated claim**, not independent human visual approval. A native PASS also requires a matching source commit, installed binary provenance, reviewed screenshots, verified source fixtures, accessibility audit, and release owner signoff. Issue #100 and #116 remain release blockers; Agent 00 owns merging. No release flags are changed here.
