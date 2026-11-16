# Agent 09 — P3 GUI simplification and polish

**Base:** `integration/scriptotar-defragmentation-agent00-20261113` @ `3009989a3aa73248b6432ba0cd84821c59110993`  
**Working branch:** `agent-09/p3-gui-polish-20261116`  
**Status:** **DRAFT / SOURCE CHANGES; latest-head CI and screenshot verification pending.** Earlier head `f0e19c013d5ce44435bc1931d7a245b0860e67b2` produced a passing Chromium QA run with 140 real PNGs, zero QA failures and an inspected representative EN/AR and light/dark screenshot sample. Final focused-loading change has its own new CI run.  
**Scope:** desktop shell, non-Studio workspace views, non-Studio tests, Chromium QA navigation contract. Main and Studio-specific files are untouched.

## Implementation (not just an audit)

- **AppShell:** Removed the duplicated legacy AI Studio primary rail entry. The canonical Studio destination remains visible/active even when the preserved legacy AI page is opened. Legacy AI is still accessible from the command palette (`Open AI Studio`) and existing `A` keyboard route. Removed the unnecessary brand subtitle. Made the existing MCP Setup utility button land at the MCP settings section rather than merely navigating to Settings. Kept custom project and locale selects, activity drawer, shortcuts, skip link, responsive rail, and RTL logical layout.
- **Dashboard:** Removed header shortcuts duplicated by already-present Current jobs and Recent creators section actions. Retained primary enqueue, new project, in-flight and failed job details, direct transcript opening, legacy AI history and research links.
- **Research:** Localized visible scanning/watchlist actions, search, results/table headers, compact metric labels, status, error prefixes, selection, empty state, watchlist health, retry diagnostics, and operational state labels into Arabic with English parity. Kept real error strings, source metadata, metric uncertainty, paging, selection and batch queue behavior. Added one-click Clear filters when an existing dataset has no matching results.
- **Queue (Jobs):** Removed duplicated active count from the header (the Active filter already exposes it); retained critical attention warnings, progress and recovery actions. Added an action to return to All jobs when an empty status filter hides existing jobs.
- **Transcript:** Localized Show more and its accessible name for Arabic.
- **Library:** Removed redundant repeated search-query text while preserving search, kind filters, counts, paging and exact-item navigation. Added a Clear filters recovery action when an existing library is filtered to no results.
- **Settings:** No section contents or provider-related controls changed. The shell's MCP shortcut now targets the actual Settings MCP section.
- **Loading:** Made bootstrap's existing shared skeleton appear as a focused title and work rows through *AppShell-owned styles only*, hiding decorative metric placeholders and leaving the shell, busy semantics and shared component source intact. Chromium QA now asserts those fake metrics are not visible.
- **Design-system controls:** Existing custom Select/Switch and styled appearance-none Research checkbox controls remain. No system-native select or slider was introduced; shared components were not changed without Agent 00 approval.

## Before → after copy inventory (EN / AR)

| Area | Before | After |
| --- | --- | --- |
| Sidebar | Studio and AI Studio appear as separate primary destinations | Studio is the single primary writing destination; legacy AI via command palette and `A` |
| Brand | Scriptotar plus generic subtitle | Scriptotar |
| Loading | Four invented metric tiles plus two panels while data is pending | One neutral work-row placeholder; shell remains interactive |
| Home | Add job / New project / Jobs / New research in header, and section links | Add job / New project in header; section links remain |
| Research scan | Scan profile / Save watchlist; English-only during Arabic UI | Scan profile / Save watchlist; فحص الملف / حفظ قائمة المتابعة |
| Research zero results | No matching research + advice, no direct reset | Same concise diagnosis + Clear filters / مسح عوامل التصفية when items exist |
| Watchlist status | Retry scheduled / Next retry even in Arabic | Retry scheduled / Next retry; إعادة المحاولة مجدولة / المحاولة التالية |
| Jobs empty filtered list | Generic queue empty text | No jobs in this filter + Show all jobs / لا توجد مهام بهذه الحالة + عرض كل المهام |
| Transcript pagination | Show more, English accessible name | Show more / عرض المزيد, localized accessible name |
| Library filtered zero results | Empty state without reset; duplicated query text | Empty state + Clear filters / مسح عوامل التصفية; query remains visible in search field |

## Regression coverage

- `src/gui-overhaul.test.ts`: a single primary Studio navigation destination; legacy AI still opens via command palette and marks Studio active.
- `src/views/DashboardView.test.ts`: navigates through section-level Queue and Research shortcuts rather than deleted duplicates.
- `src/views/research-library-overhaul.test.ts`: Arabic Research controls/status/watchlist retry; clear-filter recovery in Research and Library.
- `src/views/JobsView.test.ts`: filtered-empty Queue recovery.
- `qa/visual-regression.mjs`: preserves legacy `A` route screenshot capture after the Studio/AI rail consolidation; expects the Studio rail item active. In loading capture, checks that metric placeholders are visually hidden.
- Browser QA already covers 1440, 1100 and 760 widths, English/Arabic, dark/system-light, scroll/overflow, keyboard focus, styled controls and error/loading fixtures. **These screenshots must actually be generated and inspected at this branch head before certification.** Chromium mock render is not installed Tauri WebView certification.

## Validation and remaining gates

- GitHub connector can write source and create draft PR, but current execution container has no outbound GitHub clone access. Local `npm ci`, `npm run check`, `npm test`, `npm run build` and Chromium screenshots **have not been executed by this agent session**. Do not claim they passed.
- Require green exact-HEAD CI and the screenshot artifact from `qa/visual-regression.mjs`. Visually inspect EN/AR light/dark layouts at 1440/1100/760, especially Research long Arabic copy, filter recovery, legacy AI keyboard navigation and MCP section jump.
- Production native WebView E2E and startup/native performance certification remain Agents 07/08, not evidence from Chromium mock screenshots.
- Existing shell-first `WorkspaceSkeleton.svelte`, shared controls and global styling have intentionally **not** been edited without Agent 00 ownership approval; instead the AppShell applies local descendant styling to its startup view. Remaining P3 items include deeper per-view skeleton tuning, exhaustive text inventory, and Settings density review.
- No Studio-specific code changed; Agent 06 owns it. No backend, schema, external effect or provider behavior was altered.
- **Do not merge into main or publish a release from this PR.** Agent 00 must review overlaps, CI and visual evidence first.
