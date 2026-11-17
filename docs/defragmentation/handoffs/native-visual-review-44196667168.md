# Agent 07 — independent installed visual review and remaining native gates

**Review date:** 2026-11-18 (UAE). **Product source actually installed and reviewed:** `6291a6261c2ef9be8390a15a0a0b8ea352b3a7c4`. **Completed workflow:** [#44196667168](https://github.com/MousaXD/scriptotar/actions/runs/44196667168). **QA merge into integration:** [`f2c912246f350353a43237f065058d899ce51276`](https://github.com/MousaXD/scriptotar/commit/f2c912246f350353a43237f065058d899ce51276), a normal two-parent merge preserving all individual commits.

This is an **independent review of real installed screenshots**, not a new installed-app run or proof that previously untested native scenarios were executed. The review does **not** modify the archived `result.json` values. Release remains **NO-GO**.

## Artifact authenticity and scope

- [Linux WebKitGTK / AT-SPI screenshots and result.json, artifact 14380357240](https://github.com/MousaXD/scriptotar/actions/runs/44196667168/artifacts/14380357240).
- [Windows WebView2 / UIA screenshots and result.json, artifact 14379224815](https://github.com/MousaXD/scriptotar/actions/runs/44196667168/artifacts/14379224815).
- Platform screenshot size: Linux 1440×900; Windows 1044×788 (the Windows taskbar and installed application frame are visible).
- Both artifact `result.json` files confirm 10 PASS, 0 FAIL, 6 NOT TESTED with verdict `NOT TESTED` on the exact installed source.
- Linux screenshot SHA-256: `arabic-locale.png` `2a276d9ca326ac4bb71d430297bf83ef06783a2cb8157d5d516dad6a3037ab33`; `arabic-reopened.png` `5f2c5b5163ec28ddd5289f9b2b864cb9580ae326569e1980310533f8b0389d1a`; `system-theme.png` `80ab41d3c12f1b20fe8a52d17ab1459a2ef219b0e49fe3a81d770bac132884e4`; `dark-theme.png` `a7d54f7954808e48f27b3911a9917e2c88f02d49e983b9bfb70e2f846b288f6a`; `dark-theme-reopened.png` `d21a36d7f583f8b8b4b49340858e5e3d34c3e9ee001125c10c7a6472ce417e21`.
- Windows screenshot SHA-256: `windows-arabic-locale.png` `1e657c90d90efe60327e83ad8232de8681ed4b6f663666db25e2cb71226d9fed`; `windows-arabic-reopened.png` `0a0524674cf8cc5c104ac1e39ff40f0c6f400a60c1ddd0526d10a49e83c35991`; `windows-theme-system.png` `0a9f69c03141dc34d75eec7ef8001bbcc4f73696b197accece53ffb4a3704c0e`; `windows-theme-dark.png` `23ae28da5d568656cce27e9680a6aedcea663613ed7cf03608288162bfd34efb`; `windows-theme-dark-reopened.png` `58f8036df23224dd293828678446a6d9decc8bdea8c74e4a64503df45ed9266e`.

## Gate 5 — Arabic RTL visual review (partial inspection, **NOT TESTED** overall)

**Linux:** `arabic-locale.png` and `arabic-reopened.png` show the installed Studio rail on the right, the Arabic heading right aligned, field labels and source cards aligned in RTL reading order, and the LTR test project/study identifiers visibly intact within the RTL form. Text for the visible primary controls is readable; the screenshot shows no obvious horizontal clipping of the visible primary fields. The small dark native hover tooltip overlaps one navigation label in `arabic-locale.png`; inspect without hover and consider whether its placement should change. The screenshot's lower form is below the scroll fold, not verified absent.

**Windows:** `windows-arabic-locale.png` and `windows-arabic-reopened.png` show the right-hand collapsed Arabic navigation, right-aligned Studio heading, labels, text inputs, source cards and correctly distinguishable Latin project name and TikTok. Visible primary controls have no obvious horizontal truncation at 1044×788. Bottom content continues beyond the visible viewport; this capture does not prove full-page overflow correctness.

**Not certified by these screenshots:** open Arabic dropdown/menu placement, modal/dialog placement, focus order, mixed bidirectional punctuation in longer transcript text, resize/zoom and offscreen fields, accessibility text direction, or contrast of Arabic error messages. Capture real installed native UI with expanded Arabic menus and an actual dialog, then inspect both platform screenshots and accessibility output. Do **not** change `arabic_rtl` to PASS based solely on locale switching or the two static screens.

## Gate 6 — System/Dark visual review (partial inspection, **NOT TESTED** overall)

**Linux:** `system-theme.png` visibly renders light settings on this runner; `dark-theme.png` and `dark-theme-reopened.png` render dark settings. Visible headings, labels, input outlines, selectors, active sidebar item and Save changes control remain discernible; no obvious overlap in the inspected upper settings area. Lower settings panels are below the viewport. The product exposes **System** and **Dark**, not a separate Light option.

**Windows:** `windows-theme-system.png` visibly renders the light Settings view; `windows-theme-dark.png` and `windows-theme-dark-reopened.png` show the dark view before/after restart. Visible panels, dropdown arrows, labels, text and Save changes remain legible. Smaller viewport and wrapping in Speech engine / Media acquisition are visible but not overlapped. The lower settings page extends past the viewport.

**Not certified:** dialog/menu contrast and stacking in both themes, keyboard-focus indication throughout, all offscreen form controls, high-DPI / large-font settings, dark and System in modal/error contexts, quantitative contrast ratios. The existing `theme_mode_control=PASS` checks GUI selection, save and restart; it is **not** a full visual verdict. Do **not** change `light_dark_visual` to PASS before those views are inspected.

## Exact installed acceptance matrix — workflow #44196667168

| Case | Linux | Windows |
|---|---|---|
| Installed WebView document and Studio | PASS | PASS |
| GUI create/switch projects | PASS | PASS |
| Canonical project SQLite persistence | PASS | PASS |
| GUI create Study | PASS | PASS |
| Edit/save/duplicate/rename candidate | PASS | PASS |
| Graphical quit/relaunch persistence | PASS | PASS |
| Wrong-project GUI isolation | PASS | PASS |
| Custom keyboard popup / Escape | PASS | PASS |
| Arabic switch/restart/English restoration | PASS | PASS |
| System/Dark selection and restart persistence | PASS | PASS |
| Foreign-project forged IPC denial | NOT TESTED | NOT TESTED |
| Actual multi-source transcript | NOT TESTED | NOT TESTED |
| Source authorization revocation | NOT TESTED | NOT TESTED |
| Negative/error recovery | NOT TESTED | NOT TESTED |
| Independent full Arabic RTL inspection | NOT TESTED | NOT TESTED |
| Independent full System/Dark visual inspection | NOT TESTED | NOT TESTED |

## Required narrow follow-up — ownership boundaries

**Agent 07 QA-owned:**
1. Reuse the existing Linux AT-SPI and Windows UIA journeys, only adding real fixture setup and observable assertions for the four nonvisual gaps. Never insert SQLite transcript rows or a test-only authorization bypass.
2. Capture Arabic open custom selector / popup, a real dialog or visible error, expanded source list, and full settings page in System and Dark on both installed platforms. Inspect pixels and accessibility order; record distinct case evidence and exact screenshot hashes.
3. Run those cases on an **exact new QA HEAD** and record both completed installed jobs. A source-only test, manual review on earlier SHA, or an old run is not an exact-head installed PASS.

**Agent 00 production contracts to confirm/implement before QA can certify:**
1. **Foreign IPC denial:** provide a safe controlled way to exercise the **same production Tauri authorization path** with real project A/B Study and draft IDs from an installed application. Denial must occur before foreign data read/mutation, and read-only database inspection must prove unchanged bodies/versions. No test-only bypass.
2. **Multi-source:** expose/document the supported GUI pipeline for importing at least two independent local media/research sources, producing a completed transcript through the application's normal job pipeline, attaching canonical source IDs to a Study, and reopening its persistent source list. Provide valid **source fingerprints/provenance**, not synthetic preinserted transcript rows; a fixed-output fixture engine alone is not evidence that real audio was transcribed.
3. **Source revocation:** distinguish removing a Study reference (currently visible in `StudioView.svelte`) from revoking the underlying source authorization. Specify the production revocation action, durable state, which dependent results become invalid, and what operation fails closed after revocation. No corresponding source-revocation Tauri command was identified in `lib.rs` at inspected SHA.
4. **Error recovery:** identify a safe deterministic *real* recoverable GUI failure route (for example an existing stale-revision conflict or denied operation) that keeps the old SQLite data and visibly supports retry, without modifying production code from the QA branch.

**Final verdict:** `NO-GO`. The normal merge of #126 preserves already proven native code/history but does **not** certify these six gaps or authorize release.
