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
