# Installed native WebView acceptance (Agent 07)

**Scope:** real installed Tauri Linux WebKitGTK and Windows WebView2. These are independent of the Chromium mocked GUI suite and the existing installed backend-IPC test.

## Current exact-head acceptance — 20 November 2026

**Owner integration:** `50ed95608fc742db4a7cf1bdd2ac04fa8c5457e6`. The [original installed Linux/Windows run #44736655696](https://github.com/MousaXD/scriptotar/actions/runs/44736655696) **completed FAIL-CLOSED**: each platform has **11 PASS / 0 functional FAIL / 5 NOT TESTED**. This is not release-ready. The true installed UI automation (not Chromium/DOM simulation) passes: `installed_webview`, `create_switch_projects`, `canonical_project_persistence`, `create_study`, `edit_save_duplicate_candidate`, `quit_relaunch_persistence`, `wrong_project_ui_isolation`, `error_recovery`, `keyboard_custom_controls`, `arabic_locale_control`, and `theme_mode_control`.

The remaining five **NOT TESTED on both platforms** are:
- `wrong_project_ipc_denial`: an *authenticated mutating* `save_study` call from the ordinary installed WebView session targeting Project A while B is active, with a same-project positive control, native/host request-correlated denial, complete unchanged canonical SQLite records and persistence after restart; the separate read-only foreign Study-reference denial does not suffice; Agent10 must approve the safe route.
- `multi_source_transcript`: two independently authenticated real audio originals, verified bytes/hashes, native Queue submission, actual bundled Whisper sidecars, two distinct completed transcripts and Study source attachment with canonical SQLite lineage after restart.
- `source_revocation`: actual installed UI revocation, retained provenance, rejected stale access and durable revocation after relaunch; a database-only negative is not enough.
- `arabic_rtl`: mixed Arabic/Latin bidirectional text, dialogs/popovers, complete keyboard focus order, screen-reader announcements, narrow/wide layouts, screenshot geometry and accessible semantics on **both native engines**. Arabic locale and right-sided rail observations alone do not certify this case.
- `light_dark_visual`: genuine System (OS light **and** dark), explicit Light/Dark, restart persistence, contrast, dialog/overlay and focus inspections for each OS and installed engine. Mode-selection and screenshots alone do not certify this case.

**Original evidence:** Linux [review ZIP #14629767447](https://github.com/MousaXD/scriptotar/actions/runs/44736655696/artifacts/14629767447), Windows [review ZIP #14628883692](https://github.com/MousaXD/scriptotar/actions/runs/44736655696/artifacts/14628883692); verify ZIP, embedded `result.json` source SHA, package/executable hashes and screenshot identity before quoting. The Linux optional `two-source-attempt.json` remains `INCOMPLETE` at `origin-acquisition` because no independently reviewed offline original-audio bundle was provided after Wikimedia HTTP 429. Never retry the throttled origin to manufacture a pass.

Agent07 [draft PR #161](https://github.com/MousaXD/scriptotar/pull/161) adds a **noncertifying** original-artifact Arabic geometry verifier: three distinct Arabic Studio/Queue/Settings screenshots, native AT-SPI/UIA visible focusable controls, mirrored rail. It does **not** execute new installed journeys, assign RTL/visual PASS, or alter production. **Merge blocker:** owner PR158 integrated a reviewed-baseline source guard pinned to earlier `514ec3ee...`. Any later Agent07 QA source edit must undergo Agent00/Agent10-reviewed exact combined-source baseline roll-forward before inclusion; do not loosen or skip the guard.

## Guarantees and evidence

- The dedicated workflow builds each package **from the exact immutable PR head SHA**, installs that package, and hashes the installed executable and installer.
- Linux runs the installed desktop executable under Xvfb + D-Bus, then checks the actual process-owned WebKitGTK accessibility document via AT-SPI. It interacts with project/Study/draft controls using AT-SPI actions and X11 keystrokes; it does **not** run a web server, inject a frontend mock, or use the backend smoke CLI.
- Windows installs an exact-source NSIS package under a fresh directory, uses UI Automation against the actual WebView2 Document, creates/switches two projects and a Study, edits/duplicates/renames a candidate, verifies real persistence after relaunch, tests native invalid-input recovery and foreign read-only Study reference denial, and captures semantic Arabic locale and all three theme selectors. Absent a genuine interactive provider it fails closed; current hosted Windows has independently demonstrated the eleven PASS cases above.
- Each runner generates a per-case PASS / FAIL / NOT TESTED JSON matrix. Every run records source SHA, exact GitHub Actions run URL, executable/package SHA-256, OS/CPU/memory info, timestamp, and screenshots when the native GUI can capture them.
- A partial PASS **never** causes a green release gate. Both runners exit nonzero until every required scenario passes. The Python unit tests only check reporter invariants and syntax, never application behavior.

## Remaining required fixtures

Both installed journeys already prove a genuine native GUI recovery path (invalid angle JSON and rejected URL), persistence, Arabic locale switching and real System/Light/Dark option selection as enumerated above. A disposable profile intentionally contains no original audio, transcripts or revocable source. **Only the five cases listed in the current exact-head matrix remain NOT TESTED.** Do not substitute fake provider/SQLite rows, render-only screenshots, project UI isolation, or read-only Study-reference denial for the missing end-to-end proofs.

The Windows UIA probe now fully exercises the same eleven native cases as Linux and writes original runner evidence. Future Windows implementation is required for **the same five missing advanced cases** as Linux; historic first-boot-only status is obsolete.

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

**Historical native-boot regression (16 November):** installed run [#43879991872](https://github.com/MousaXD/scriptotar/actions/runs/43879991872) was interactive at the top-level window but UIA showed only `Tauri Window > WRY_WEBVIEW Pane` and a white screenshot. This was not proof of a headless session. The current probe checks actual WebView2 descendants and a real document/control tree, and has since passed eleven installed scenarios. Preserve the original no-phantom-provider checks.

To independently reproduce on a Windows 11 native-machine, sign in to an **unlocked, interactive desktop** with a normal, non-elevated user, a working WebView2 Runtime and PowerShell 7. Use the repository at the **exact source commit** used to produce the NSIS installer. Run from the repository root:

```powershell
$sha = (git rev-parse HEAD).Trim()
$run = 'https://github.com/MousaXD/scriptotar/actions/runs/RUN_ID'  # actual package source run, not a claimed live manual execution
$installer = (Resolve-Path 'C:\path\to\Scriptotar_1.4.1_x64-setup.exe').Path
New-Item -ItemType Directory -Force '.\native-evidence\windows' | Out-Null
& '.\tests\defragmentation\native-e2e\windows_webview2.ps1' `
  -Installer $installer -SourceSha $sha -RunUrl $run `
  -EvidenceDir '.\native-evidence\windows'
```

Replace the example installer path with the **actual exact-SHA NSIS artifact**, and associate the manual test with its real execution date, operator, OS build, installer SHA and binary SHA. The supplied run URL identifies the original CI artifact, **not** a new machine test run. Include the `result.json`, `uia-diagnostics.json`, screenshots, WebView2 Runtime version and a note identifying the manual host. A locally run test is not hosted-CI success. A missing provider is NOT TESTED, not PASS.

If the WebView2 provider remains absent, inspect descendant HWND class names and PIDs in `uia-diagnostics.json`, review whether `msedgewebview2.exe` has started, capture Windows Event Viewer application errors and verify the desktop remains unlocked. A white app region is insufficient to conclude an accessibility-only issue.

**Current limits:** Both installed scripts execute the eleven cases in the exact-head matrix. Neither one certifies the five missing advanced cases; each remains `NOT TESTED` until authentic source-specific native evidence and independent review. Screenshots and backend-only IPC tests cannot replace installed WebKitGTK/WebView2 sessions.

## Evidence authenticity and review

Keep screenshots and exact run artifacts; the JSON is a **machine-generated claim**, not independent human visual approval. A native PASS also requires a matching source commit, installed binary provenance, reviewed screenshots, verified source fixtures, accessibility audit, and release owner signoff. Issue #100 and #116 remain release blockers; Agent 00 owns merging. No release flags are changed here.

## Historical 2026-11-17 evidence-based fixes (superseded test outcomes; preserve diagnostic rationale)

The actual installed Linux dashboard renders its h1 as AT-SPI `document frame` and the topbar project selector as `PROJECT`. Match only exact-name semantic heading/document frame roles and also require the real dashboard New project button; a chooser option alone is insufficient. This was a historical failure; exact 20 November native jobs subsequently demonstrated project persistence PASS.

Npm Tauri 2.11.2 rejects `tauri build --bundles nsis -- --locked`. Validate `cargo metadata --locked` before/after the genuine production `tauri build --bundles nsis`, and assert no tracked lockfile diff. Do not return to bare `cargo build --release` + bundle: integrated run 43956319863 showed a localhost error page. Classify the exact app-owned browser error UIA pane as FAIL.

## Installed Linux option role and actual project database

Exact native run 44091975817 created and displayed both project dashboards, then failed when a displayed project option was found in AT-SPI as role `list item` rather than `option`. Choose by exact accessible name and the observed role; continue requiring the real switch and active dashboard. Independently read the installed app's own `SCRIPTOTAR_DATA_DIR/scriptotar.sqlite3` in SQLite read-only mode to verify both project rows exist before marking `canonical_project_persistence` PASS. Unit tests for the probe or selectors **are not** native acceptance.

## Windows Cargo.lock ordering-only rewrite on the actual NSIS build

Exact run 44091975817 built the true NSIS production installer, but the workflow's Git byte-diff detected only `scriptotar-agent` being reordered within the `scriptotar-desktop` package dependency list. Windows Cargo 1.97 normalizes order. Do not weaken reproducibility by ignoring arbitrary `Cargo.lock` differences: canonicalize parsed TOML to stable JSON and compare SHA-256 before and after the production `tauri build --bundles nsis`, with `cargo metadata --locked` pre/post. A real checksum/version/graph change still fails. A later exact-commit installed WebView2 run provided the eleven PASS cases listed above.

The lockfile fingerprint is a standalone, regression-tested `cargo_lock_fingerprint.py`. TOML dictionary key sorting is insufficient: it must normalize `[[package]].dependencies` order, while retaining all identities, versions and checksums. The workflow hashes the full normalized parsed lockfile, not just package count or dependency names.

The Linux workflow runs unit/contract tests with pinned Python **3.12** because Cargo.lock fingerprinting uses stdlib `tomllib`. The real AT-SPI WebKit acceptance still executes `/usr/bin/python3` with `python3-pyatspi`; test interpreter separation never substitutes mocked UI for installed GUI.

Run #44095877399 showed Linux project B remained active after a generic AT-SPI list-item action for A. The option activation now uses the actual option's PID-owned AT-SPI rectangle and X11 pointer event, with exact-name matching and subsequent strict A/B dashboard checks. This was an intermediate checkpoint before the later eleven-case Windows implementation and certification.

Run 44100162447: Linux verified A/B graphical switching PASS after using real pointer on an exact AT-SPI option, then failed when WebKit exposed Study name input as `embedded: Study name e.g. Launch video` rather than exact label. Use `edit_field` restricting to input-like roles and stable label prefix. Windows built genuine NSIS and created two projects, but UIA pointer option selection left B active. New Windows native keyboard navigation derives option index from exact UIA popup element bounds/names, then requires actual dashboard switch; was NOT PASS at this historical run; the subsequent exact-head run provides project-switch PASS.
