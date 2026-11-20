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

- Chromium QA fixture now covers 1440/1100/760 widths, English/Arabic, **explicit Dark, explicit Light and System under OS Light**; it additionally captures System under OS Dark at desktop width 1100 for both locales. Expected screenshot count is now 232 (previously 140); this is only *planned QA coverage* until the workflow actually finishes and its screenshot artifact is inspected.
- Added five source-level Svelte Testing Library regression cases (two bidi cases, two native mixed-checkbox cases and one Settings error-focus case), plus an enhanced existing Library open-failure case to `research-library-overhaul.test.ts`, using Arabic mixed-direction source data. The bidi cases assert the exact DOM `dir` isolation for titles, creator labels, source ISO dates, durations, watchlist errors and times, and Library pagination; the checkbox case asserts false → mixed → true state with an unchanged bulk-selection outcome; Settings asserts the unredacted failure, near-header placement and focus; Library verifies opening-failure focus and proximity to results. Checkbox tests assert real native `indeterminate` state and reset across pagination.
- **Not run locally:** repository cannot be cloned from this execution environment (DNS/outbound GitHub unavailable); no local `npm ci`, Svelte typecheck, `vitest`, or screenshot run is claimed.
- Required exact-head CI (from `apps/desktop-ui`): `npm ci && npm run check && npm test -- --run && npm run build`. Run `qa/visual-regression.mjs` via repository's existing GUI QA workflow; review 1440/1100/760 EN/AR light/dark/system and 1100 System + OS Dark screenshots, including real overflow. Source QA remains Chromium-only.
- **Not independently certified:** installed WebKitGTK / WebView2 focus, screen reader, physical keyboard behavior, visual RTL screenshot inspection, and packaged native restart. Those are Agent 07's installed-app gates; mocked Chromium and source assertions are not substitutes.

## Remaining out-of-scope gates

- Agent 06 owns Studio source selection, candidate comparison, dialogs and unsaved Study/project context UX; avoid patching its live components here.
- Shared `Select`, `Popover`, global shell focus and global styling require an explicit ownership review before editing them from the P3 branch.
- Native accessibility, dark/light/system appearance and real installed Arabic workspace transitions remain unverified here. Agent 07 must issue an independent pass/fail on the exact installed build.
- PR remains **DRAFT / NOT A RELEASE OR MAIN MERGE APPROVAL** until exact-head tests, screenshot inspection and owner review are completed.
