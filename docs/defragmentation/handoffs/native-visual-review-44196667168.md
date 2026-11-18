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


## 2026-11-18 — new exact installed native menu captures

**Completed run:** [#44203276678](https://github.com/MousaXD/scriptotar/actions/runs/44203276678). **Installed source:** `18768375908968b3bbdf92ab0f6ad229335368fb`. [Linux review artifact 14383885160](https://github.com/MousaXD/scriptotar/actions/runs/44203276678/artifacts/14383885160), [Windows review artifact 14383736564](https://github.com/MousaXD/scriptotar/actions/runs/44203276678/artifacts/14383736564). **Both result.json reports remain exactly 10 PASS, 0 FAIL, 6 NOT TESTED**. This section records independent screenshot review without rewriting the machine-generated verdict.

| Installed visual artifact | Pixels | SHA-256 |
| --- | --- | --- |
| Linux `arabic-locale-menu-open.png` | 1440×900 | `bce13acb3cc2351e49e6d80e2194199bb6606de373919b412eb66ff7a8197c8f` |
| Linux `dark-theme-menu-open.png` | 1440×900 | `bf9465c03997344581089ad8755d71d2804a4c4de38a00dfe6c8382a81079b26` |
| Linux `system-theme.png` | 1440×900 | `bac5599f8b3a7328e1460cb46764dcfb83e4e7efcc02742709e269990b2f8c0c` |
| Windows `windows-arabic-menu-open.png` | 1044×788 | `3e23555b82428f8b800aa2cd950d2c06ee683c96f8a0840b53e12f0be8ae89da` |
| Windows `windows-theme-menu-open.png` | 1044×788 | `dfb6e9fc2f6b9574cf85b8f05bf693e202de3a1092dad509e8045663ab08e770` |
| Windows `windows-theme-system.png` | 1044×788 | `37cc127d43fb6ebec7ad494595f1053fc7ce288a26755eafab4f6f64925fa1fb` |

**Visual observations:** Both installed Arabic popup screenshots show the language selector's open menu anchored to the native top-bar trigger without being cut off at the screen boundary. The Arabic Studio content uses a right-side navigation rail, right-aligned headings, form controls and source cards; the left-to-right project ID remains distinguishable. In System and Dark screenshots the upper Settings panels, labels and selectors are readable and their popup options remain inside the application window. Windows uses a noticeably smaller 1044×788 capture and the lower Settings panels are outside the viewport; the screenshots do not prove their scroll behavior or focus state. No evident overlap in the inspected upper Settings controls. This is **partial independent visual evidence only**: unobserved Arabic dialog placement, full scroll/resize/zoom/keyboard focus and full-coverage theme contrast prevent full `arabic_rtl` or `light_dark_visual` PASS.

**CI note:** Exact QA source `187683759` had three unrelated Svelte `svelte-check` errors because it was behind Agent 00's owner fixes. Agent 07 merged integration `0948a45a` into its follow-up with two parents (merge SHA `89bd8c9f`); [general CI #44277978423](https://github.com/MousaXD/scriptotar/actions/runs/44277978423) and [security hygiene #44277978401](https://github.com/MousaXD/scriptotar/actions/runs/44277978401) then **PASSED on that merge HEAD**, but these checks are not native acceptance of subsequent QA commits.

### Gate 4 — real installed error recovery (source implementation; **NOT TESTED**)

Agent 07 QA-only scripts now attempt a rejected `file:` URL using the actual Jobs/Queue GUI and its production network admission policy. The scripts require a visible native error, read-only identical canonical job counts before/after, navigation back to the saved Study, exact script draft text and immutable candidate DB readback. No SQLite seeding or mock IPC is used. A failure of any proof must remain FAIL. **The changed scripts have not completed a newer exact-head installed workflow yet**; do not promote `error_recovery` to PASS until a report and screenshot with the precise QA HEAD exist. Security and other visual/source scenarios remain NOT TESTED.

## 2026-11-18 — exact-head native gate reconciliation (NO-GO until installed evidence)

The QA branch was merged with integration source `7af95c574643c14565a5c10d0be2cc78c5e57254` using a two-parent commit. This reconciles competing edits to the native harness rather than replacing the owner's 3-angle recovery check or Agent 07's Queue URL denial. The `error_recovery` case now requires **both** (1) visible native rejection of malformed one-angle JSON followed by an accepted valid three-angle response with unchanged canonical immutable drafts, and (2) visible native rejection of an unsupported `file:` Queue URL with no new canonical job plus restored Studio data. If either check fails on a platform, the one release-matrix case FAILS. Static contract tests do not certify native behavior.

**Known installed source evidence preceding this merge:** [integration native workflow #44278805547](https://github.com/MousaXD/scriptotar/actions/runs/44278805547), Linux 10 PASS / 1 FAIL / 5 NOT TESTED, Windows 10 PASS / 0 FAIL / 6 NOT TESTED. Linux failed malformed-angle visible rejection; Agent 00 has exact reproduction in [PR #114 discussion](https://github.com/MousaXD/scriptotar/pull/114#issuecomment-6977592172). Native artifacts: [Linux screenshot/log review](https://github.com/MousaXD/scriptotar/actions/runs/44278805547/artifacts/14415874689), [Windows review](https://github.com/MousaXD/scriptotar/actions/runs/44278805547/artifacts/14416936828). Do not upgrade these prior results to PASS after a source-only merge.

**Still unproven in this native matrix:** forged foreign-project IPC denial; actual two-source transcript attachment; source authorization revocation; full Arabic RTL inspection; comprehensive Light/Dark/System screenshots and visual acceptance. The current scripts preserve these as `NOT TESTED`, and the release gate remains fail-closed. Native acceptance must be repeated on a single exact post-integration SHA; maintain PR #131 in draft.
