# Agent 09 — RTL bidi and accessibility follow-up (2026-11-21)

## Provenance and scope

- **Exact parent integration:** `b12843c75764601248a69c0dd89ff753af8df366` on `integration/scriptotar-defragmentation-agent00-20261113`.
- **Branch:** `agent-09/p3-rtl-bidi-accessibility-20261121`.
- **Coordination:** Agent 00 integration PR #114 comment 7050748451 records the ownership boundary. Agent 06 is responsible for Studio and shared functional components; Agent 07 is the independent installed WebView accessibility certifier.
- Previous Agent 09 PR #125 remains an older, unmerged draft; its shell/navigation/localization behavior is already in the current integration source. This patch does not replay #125.
- Only the non-Studio Research, Library and Settings renderers, their existing regression suites, the existing Chromium QA visual matrix, and this handoff change. No shared control, backend, provider or native QA code is changed. Agent 00 was also notified of the Settings scope refinement in #114 comment 7050831344.

## Before / after (source-level visual intent; not screenshot certification)

| View | Before | After |
| --- | --- | --- |
| Library Arabic | Mixed-script title and subtitle inherit RTL, dates inherit the surrounding direction | Local `dir="auto"` for titles and origin metadata, `dir="ltr"` for ISO date and page counter |
| Research Arabic | Source title, creator, platform and watchlist labels inherit RTL even when the actual content is English; ISO dates and `m:ss` can reorder around RTL labels | Auto-direction source text and errors; LTR-isolated URL, source date, duration, percent and diagnostic timestamps |
| Filtering | Arabic and English search phrases both inherited overall document direction | Search input text direction follows the entered phrase, without changing the app locale |
| Settings errors | Save/output picker failures appear below the long Settings layout, remote from the action | Status and unredacted technical errors appear directly beneath the Settings header; failed actions focus the alert for keyboard users |
| Library open failure | Technical error appears after the complete paginated list, far from the selected row | Unredacted error appears beside result summary and receives keyboard focus |
| Failure reporting | Technical watchlist error text inherits RTL regardless of language | `dir="auto"` on original error messages, with no truncation or suppression |

All isolation is presentational. Underlying dates, source IDs, project/Study ownership, sorting, research metrics and selection/queue operations remain unchanged.

- **Research bulk selection:** when only some selectable rows on the current page are checked, the page checkbox now presents a visible dash, the accessible `aria-checked="mixed"` state, and the **native HTML checkbox `indeterminate` property**; all three are reset on selection, filter or pagination changes. Clicking it still selects all remaining page rows; queued rows remain disabled. This aligns keyboard/screen-reader feedback with the existing selection semantics.

## Verification

- Chromium QA fixture now covers 1440/1100/760 widths, English/Arabic, **explicit Dark, explicit Light and System under OS Light**; it additionally captures System under OS Dark at desktop width 1100 for both locales. Expected screenshot count is now 232 (previously 140), with 20 distinct workspace scenarios (previously 12); this is only *planned QA coverage* until the workflow actually finishes and its screenshot artifact is inspected. All existing verifier/approval/browser-metrics guardrails now fail closed on incomplete or legacy 140/12 evidence. The manual baseline remains **human-only** and its SHA-256 comparison is still mandatory; the repository branch has no `qa/baselines` directory at this checkpoint, so successful rendering alone can result only in `NOT_CERTIFIED`.
- Added five source-level Svelte Testing Library regression cases (two bidi cases, two native mixed-checkbox cases and one Settings error-focus case), plus an enhanced existing Library open-failure case to `research-library-overhaul.test.ts`, using Arabic mixed-direction source data. The bidi cases assert the exact DOM `dir` isolation for titles, creator labels, source ISO dates, durations, watchlist errors and times, and Library pagination; the checkbox case asserts false → mixed → true state with an unchanged bulk-selection outcome; Settings asserts the unredacted failure, near-header placement and focus; Library verifies opening-failure focus and proximity to results. Checkbox tests assert real native `indeterminate` state and reset across pagination.
- **Not run locally:** repository cannot be cloned from this execution environment (DNS/outbound GitHub unavailable); no local `npm ci`, Svelte typecheck, `vitest`, or screenshot run is claimed.
- Required exact-head CI (from `apps/desktop-ui`): `npm ci && npm run check && npm test -- --run && npm run build`. Run `qa/visual-regression.mjs` via repository's existing GUI QA workflow; review 1440/1100/760 EN/AR light/dark/system and 1100 System + OS Dark screenshots, including real overflow. Source QA remains Chromium-only.
- **Not independently certified:** installed WebKitGTK / WebView2 focus, screen reader, physical keyboard behavior, visual RTL screenshot inspection, and packaged native restart. Those are Agent 07's installed-app gates; mocked Chromium and source assertions are not substitutes.

## Remaining out-of-scope gates

- Agent 06 owns Studio source selection, candidate comparison, dialogs and unsaved Study/project context UX; avoid patching its live components here.
- Shared `Select`, `Popover`, global shell focus and global styling require an explicit ownership review before editing them from the P3 branch.
- Native accessibility, dark/light/system appearance and real installed Arabic workspace transitions remain unverified here. Agent 07 must issue an independent pass/fail on the exact installed build.
- PR remains **DRAFT / NOT A RELEASE OR MAIN MERGE APPROVAL** until exact-head tests, screenshot inspection and owner review are completed.

## Latest QA evidence-gate additions

- `qa/visual-regression.mjs` defines exact expected image inventory: 18 standard viewport/locale/appearance cases × 11 screenshots, 2 additional System/OS-Dark desktop cases × 11, and 12 boot loading/error captures = 232 PNGs.
- `qa/verify-snapshots.mjs` requires the 232/20 exact-source report and hashes every generated PNG before comparing to any approved baseline; no baseline implies `NOT_CERTIFIED` rather than PASS.
- `qa/approve-baseline.mjs` continues to require an explicit human approval process and now refuses incomplete 232/20 reports; Agent 09 **has not** approved any baseline.
- `qa/compare-browser-metrics.mjs` refuses comparisons across incompatible old/new scenario inventories; do not report performance improvements from mismatched samples.
- `qa/tooling.nodecheck.mjs` updates isolated synthetic verifier fixtures to 232 and includes a rejection test for legacy 140-case reports. Synthetic fixture bytes do **not** constitute captured screenshots.

## Final non-Studio keyboard/RTL pass (2026-11-21)

- Home: keyboard focus moves to the project name when opening the inline form, returns to **New project** on cancel, and focuses unredacted failure alerts after rejected creates. No new actions or navigation.
- Transcripts: EN/AR ownership-denial copy is localized without weakening the existing project/ID guard; focus moves to the denial so it can be read with a keyboard/screen reader; list headings, source platform names, reader timestamps, metadata and Arabic search queries receive local bidi isolation without changing stored transcript text.
- Transcript view controls: **Use in Studio** and **Show more segments** now use typed shared EN/AR message keys. Queue diagnostic **Request ID** is localized while the opaque identifier stays LTR.
- Added frontend regressions for Arabic wrong-project refusal/focus, mixed-direction transcript UI and Home project-form keyboard focus. No edits to Agent 06-owned StudioView, studio/ components, backend, payment, native harness, or security-sensitive workflows.
- Exact-head native installation, appearance and screen-reader verification remains Agent 07's independent responsibility; Chromium screenshots are not native proof. No manual visual baseline was approved and no production release is authorized by this pass.
- Verify the *new* PR #189 GitHub head SHA with Desktop UI, CI, visual QA and source-guard workflows; earlier success on `bcb1f98ceb1beedd63512a107ffed93e3fd06395` does not certify the new commit. Existing package gates were failing at reviewed/trusted-source verification before this patch.

- Final Queue follow-up: URL and advanced manual-path text inputs use LTR display isolation in both locales. Request-ID regression checks now assert the localized EN/AR accessible label and that the authenticated opaque ID remains unchanged. Transcript timestamp export has an Arabic label; file output formats and naming are unchanged.


## 2026-11-21 — Screenshot-inspected Queue and narrow Research completion

- At `a0c5ca92c4716f9389028d07034cc983b6eb93af`, [Chromium GUI QA run 44893399871](https://github.com/MousaXD/scriptotar/actions/runs/44893399871) produced **232 screenshots, 20 workspace combinations, zero capture failures**, and an artifact `scriptotar-gui-qa-a0c5ca92c4716f9389028d07034cc983b6eb93af`. Its verifier states **NO APPROVED VISUAL BASELINE**. The screenshots are built-in mocked browser captures, not installed WebKitGTK/WebView2.
- Inspected representative EN/AR dark/light/system Queue, Studio, Settings, Library and Research screenshots. **At 760px, the EN/AR Research Save watchlist control was squeezed into the fixed 96px track, splitting words into vertical fragments.** The scoped Research CSS now permits a 152–190px second column at intermediate widths and avoids `overflow-wrap:anywhere` on buttons; the existing 480px stacked-button rule is retained. Chromium QA now explicitly records a failure if the 760px save action renders narrower than 140px. The corrected branch requires a **new exact-head** screenshot capture; inspection of the old screenshots is not proof of the fix.
- Queue text is now sourced from typed EN/AR messages for empty filtered results, Show all jobs, Show more jobs, and filename-only drops. Filename-only drops and native-drop subscription errors focus the existing technical alert after render; the native path remains subject to Rust validation. Replacing a rejected media URL by dragging a supported URL clears stale `aria-invalid` and denial identity; editing the URL also clears prior status. Tests cover English and Arabic drop-error focus, Arabic filter/show-more recovery and a stale URL-rejection reset.
- No work was performed in Agent 06-owned Studio files, project switching, provider authorization, backend/native installed app, or persistent data. Older integration-stage screenshots cannot certify Agent 06's newer Studio PR #194; Studio EN/AR user-facing copy must be re-reviewed on the integrated exact source.

### Independent acceptance requested from Agent 07 and Agent 00

1. Install an exact-SHA package and prove its installer hash, executable hash, PID/window ownership and native WebView identity on **Linux WebKitGTK and Windows WebView2**. Do not borrow Chromium screenshots, synthetic focus tests or an older package SHA.
2. With keyboard only, open project → Research → Queue → Transcript → Studio → Library → Settings; exercise tab/shift-tab, Enter/Space/Escape, source selection, consent boundary, error alert focus, focus return, scroll and custom controls. Record AT-SPI/UIA focus roles, names and active-window proof, including Arabic quit/reopen and denial paths. Do not send blind keys when active-window or keyboard focus ownership is uncertain.
3. Capture native EN/AR, RTL and mixed-direction URL/path/source IDs/ISO time/creator metadata at desktop and narrow widths. Inspect 760px Research scan/save labels, 760px Queue inputs, long project/source names, dialogs and candidate comparison; avoid horizontal clipping or broken single words. Test explicit light, explicit dark and system appearance under both OS light and OS dark, and persist/reopen.
4. Supply independently reviewed native screenshots, keyboard/focus traces, exact source/package provenance and unresolved failures. **Do not mark native/visual PASS automatically**. Agent 00 must approve source and human-reviewed Chromium baseline evidence separately.

**This is a GUI-only draft handoff; all current-head GitHub checks and independent visual/native certification remain gates.**

## 2026-11-21 — Follow-up exact screenshot inspection, Settings and Research keyboard failures

- **Completed prior exact-head browser/UI evidence** at `c2f6f0f42b8b67f6a883e12ef168e21f2956fe9b`: [Desktop UI run 44907275554](https://github.com/MousaXD/scriptotar/actions/runs/44907275554) passed all **274 Vitest tests across 31 files**, Svelte check, and the production build. [GUI QA run 44907275612](https://github.com/MousaXD/scriptotar/actions/runs/44907275612) captured **232 / 232 PNGs in 20 combinations**, zero recorded browser failures. Artifact 14708218372 is bound to that commit and contains a report with matching `sha`. Its verifier explicitly states **NO APPROVED VISUAL BASELINE**; this is browser mock/Chromium evidence only.
- Inspecting that exact artifact's 760px EN/AR light/dark/system screenshots confirms **Research scan/save controls have readable labels** and the original Settings last-link clipping is corrected by wrapping existing section links onto a second line. The new Chromium check verifies the 760px Settings navigation anchor geometry without creating a second navigation system. This inspection is Agent09 review, **not an independent approved baseline** and not a real installed app.
- Audit of shared `shellMessages`: **412 EN keys / 412 AR keys**, zero missing, duplicate or dynamic placeholder mismatches; existing `i18n/messages.test.ts` enforces key and interpolation parity. These counts do not prove professional translation of user-facing prose in other modules. The older integrated Studio browser capture still contains English `Prompt setup` in Arabic mode; Agent06 was notified on PR194, and current Studio integration must be re-evaluated rather than patching its owned components here.
- **Latest Research UI source change**: scan, save-watchlist and multi-batch queue failures now focus their original, unredacted backend diagnostic alert after Svelte renders it; errors preserve source/URL bytes and presentational `dir=auto`. Regressions cover English scan and partial queue failure focus plus Arabic watchlist save denial/focus. This is display/accessibility behavior only; no backend queue, URL, project authorization, state mutation or native contract changed.
- Prior `c2f6f0f` Linux and Windows packaging workflows both fail **at independently reviewed-source verification before package creation**, while the migration integration workflow was cancelled. Neither is evidence of a product interaction failure. No guard or source hash pin was changed or bypassed.
- **Latest source after these changes requires new exact-head UI/Chromium tests**, which cannot be counted as PASS from prior SHA. No automatic screenshot baseline approval, source-guard override, installed native certification, Agent00 integration merge, or main release is authorized.

**Agent07 native focus target:** in genuine installed EN/AR Research, cause a real scan/watchlist-save denial and a partial batch queue denial through the permitted QA method; confirm focus moves to a named alert in AT-SPI/UIA, readout retains mixed Arabic/English host diagnostics, and keyboard navigation remains recoverable. Confirm wrapped Settings anchors at 760px on both native OS WebViews. The browser mock checks do not substitute for this.

## 2026-11-21 — Legacy AI Studio static Arabic parity (Agent 06 coordinated)

- Coordination on PR #194 [comment 7063366421](https://github.com/MousaXD/scriptotar/pull/194#issuecomment-7063366421) precedes this text-only edit. Agent 06 remains owner of `StudioView`, `studio/` components and the canonical approval/generation workflow; **none** of those files or backend/provider IPC were modified in this pass.
- `AiStudioView.svelte` legacy Copy Prompt / disabled BYOK view had English headings and controls in Arabic mode (`Prompt setup`, `Source context`, `Creative brief`, `Generated prompt`, model labels, actions, copy/status/error messages and disabled-provider warnings). They now use typed EN/AR `shellMessages` entries, with user transcript names/metadata rendered using display-only direction isolation. Internal task values, provider identities, exact transcript UUIDs, request payload, prompt construction, and authorization checks remain untouched.
- The provider key entry remains `disabled`, `Run with API` remains `disabled`, and the security copy is explicit in both languages that provider sending is **unavailable pending certification and no API key is accepted**; older wording that incorrectly suggested a key exists for the session has been corrected. This is a disclosure correction only, not approval to dispatch.
- Current shared message inventory: **481 EN / 481 AR keys**, zero duplicates, missing keys or interpolation placeholder mismatches by source-level inspection. The typed i18n key parity and placeholder tests remain in place. The new Arabic UI regression chooses localized `Hook ideas` while asserting the exact backend task code remains `Hook ideas`, `apiKey` and `baseUrl` remain undefined for local prompt generation, and the BYOK execution button stays disabled with no `runAi` call.
- **No screenshot baseline approved**; the new Arabic legacy-Studio rendering must be captured and reviewed on this new exact SHA, not inferred from old `c2f6f0f` artifact. Linux WebKitGTK / Windows WebView2 remain Agent 07 independent acceptance. Trusted source gates are untouched and prior packaging is NO-GO before native launch.

**No main merge/release or paid AI egress. All exact-current-head test and independent native/visual gates must pass before Agent00 integration.**
