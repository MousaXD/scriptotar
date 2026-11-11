# Agent 05 — Research & Library UX overhaul handoff

**Repository:** https://github.com/MousaXD/scriptotar  
**Branch:** agent-05/research-library-overhaul  
**PR:** https://github.com/MousaXD/scriptotar/pull/91 (draft pending validation)  
**Baseline:** f741244634b5143602a21a554094a498a0005b11  
**Latest implementation commit before this report:** 3bfb405a4f4b8123a925b494f6b362251102c4f0

## Ownership respected

- apps/desktop-ui/src/views/ResearchView.svelte
- apps/desktop-ui/src/views/LibraryView.svelte
- apps/desktop-ui/src/views/research-library-overhaul.test.ts (new)
- This handoff report

No backend code, global CSS, API contracts, other views, shared components or translation dictionaries were modified. Never touched main.

## Research: completed

- Replaced decorative eyebrow/description dashboard chrome with one clear Research header and primary creator/profile URL capture.
- Preserved profile scan, save-to-watchlist, limit selection, and backend error reporting. Prevents concurrent scan/save requests and disables actions while pending.
- Added compact full-width search, available-platform filtering and metric/date sorting. Sorting is local and unchanged in semantics.
- Research rows now emphasize title, creator, platform and observed metrics; keep the derived interaction-rate disclosure.
- Bounded DOM rendering to 50 items per page, while keeping search/sort across the full in-memory list.
- Added select/deselect current page; selections persist across pages, and selected IDs are resolved against current items when queued.
- Disabled double queue submissions, excluded already-queued items from new queue requests, cleared only successfully queued selections, and preserved errors.
- Added fallback to a local platform placeholder if a remote thumbnail cannot be loaded.
- Moved watchlist health **below** the main workflow: compact rows remain visible for every watchlist, including persisted errors and next retry timestamps. Routine last-attempt/last-success timestamps are available via expandable details.

## Library: completed

- Replaced explanatory and ornamental header chrome with a compact indexed artifact list.
- Preserved transcript/research/AI run/project/creator kinds, existing navigation callback, error visibility, Arabic translations and RTL.
- Refined title, kind, source, signal and date hierarchy without duplicating all metadata in every label.
- Retained newest-first backend order; supports title/type sorting, case-insensitive text search and kind filters.
- Switched counts to a single-pass tally, removed quadratic lookups, and limited rendered rows to 75 per page.
- Added accessible pagination and responsive narrow-window column hiding, while preserving full accessible artifact names and keyboard-activated buttons.
- Logical CSS spacing used to align column headers in RTL.

## Regression tests added

The dedicated research-library-overhaul.test.ts covers:
1. Large research sets, pagination, filtering and source/platform selection.
2. Select page and cross-page queueing exactly once.
3. Watchlist retry/failure visibility and expandable diagnostics.
4. Scan failures preserving backend error details.
5. Broken thumbnails falling back to local placeholders.
6. More than 1,000 library artifacts with bounded DOM rows and page navigation.
7. Library kind/text filters and sorting.
8. Arabic Library controls, untouched user-authored title text and open failures.

Existing App.test.ts and gui-overhaul.test.ts also exercise Research sorting, watchlist operational states, library artifact navigation and Arabic localization. Those files were not changed.

## Verification evidence and remaining gates

**Remote:** GitHub PR #91 was created successfully and changes committed to the assigned branch. The initial PR SHA 438a9f457da148aff60b6b7cb1e2b12f28c2f405 triggered Desktop UI, CI, Tauri migration integration, Linux/Windows packages, security and MCP workflows, initially queued.

**Not yet validated:** This environment has no checkout/network route to GitHub via command-line Git, and no cached Svelte test dependencies; local npm ci, npm run check, npm test and npm run build could not be run. Do not report any tests as passing or merge PR until exact latest SHA passes relevant checks. CI queued state may reflect shared runner capacity.

**Manual acceptance gate (not claimed complete):** Check wide/narrow desktop, Arabic RTL, image fallback, keyboard focus, long names and empty/error/busy states in the packaged UI.

## Cross-agent dependencies

- **Agent 01:** native select/input appearance is a shared design-system responsibility; this PR does not override native-control tokens globally. Recheck selectors against that agent's eventual component/styles integration.
- **Agent 03:** local pagination bounds DOM work but still receives the full Bootstrap Research/Library payload. If workspaces grow substantially, coordinate server-side paginated contracts rather than editing backend/API files here.
- **Agent 00 / independent QA:** validate PR #91 with Agents 01–03 and the full application before merge. Keep PR draft until exact-head CI and cross-agent regressions are resolved.

## Known follow-up, not silently claimed solved

- No physical screenshot or packaged-app accessibility audit was possible in the current execution environment.
- Research currently relies on existing legacy UI translation for unchanged labels; new page/selection controls have explicit English/Arabic labels. Consolidating all Research strings into the central translation dictionary belongs to the localization owner.
- This PR does not change data fetching strategy or provide virtual scrolling beyond bounded client-side pages.


## Independent continuation audit — 2026-11-11

**Owned code SHA audited:** 4e0fd6a924d3a6e51fb7deaee139ab9d8444720b (handoff commit follows).

Following review of all owned files and Agent 08's PR #91 feedback, these additional defects were resolved without changing file ownership:

- **Queue backend cap:** Rust validates at most 200 research IDs per queue request. Cross-page selection previously exceeded the cap. Submissions now run sequentially in batches of 200, remove only successful batches from selection, and preserve unsubmitted IDs with a visible partial-error message. Added tests for 201 items and second-batch failure.
- **Already queued media:** Previously queued rows could be selected and counted but were silently excluded from submission. Queued rows remain visibly marked but are disabled for new selection; page selection skips them and selection count is actionable. Regression added.
- **Profile URL entry:** Native input type=url previously prevented some URL-like user text from reaching backend validation, inconsistently with Save watchlist. Input type=text with URL input-mode now delegates policy and feedback to backend. Regression added.
- **Thumbnail retry:** Fallbacks are keyed by item ID plus image URL, so changing a broken image URL can display its replacement instead of retaining a stale failure.
- **Watchlist attention:** A functional text link near the Research heading shows failures/retries, navigating directly to the detailed watchlist status; errors remain visible. Singular/plural copy and narrow header wrap were corrected.
- **Checkbox accessibility:** Research checkboxes have theme-aware native checkbox semantics, consistent checked state, visible keyboard focus and a proper table cell wrapper.
- **Library localization:** Local source labels use the existing Arabic key even if the backend explicitly supplies Local, and title sorting respects the active locale.

### Verified previous-SHA CI (not a current-HEAD claim)

At e799779503bca12773002a60b34cc3f16d786290:
- **PASS** Desktop UI Svelte check/tests/build, full CI, security hygiene, MCP E2E, Linux package, Windows installer.
- **FAIL** Tauri migration integration: only Rust advisories / Supply chain failed. All five other jobs passed (Rust workspace, Svelte frontend, sidecar, Rust-sidecar integration and Tauri build).
- **Root cause** cargo audit: RUSTSEC-2026-0459 / CVE-2026-119375 against libsqlite3-sys 0.30.1, patched advisory range >=0.39.0. Requires Rust/database/release owner dependency resolution, migration regression and full security gate; no suppression or weaker assertions.
- Exact latest-HEAD workflows must finish before this work can be certified. No local Git checkout or node dependencies are available in this agent runtime; GitHub Actions is authoritative.

### External blockers and handoff

1. **Agent 01 PR #89:** Agent 08 confirmed three native Research selectors (scan limit/platform/sort) and one native Library sort selector remain OS-opened. The accessible Select control exists only on Agent 01's draft branch, not this branch's base. Importing it before integration would break Svelte builds, while copying/reimplementing shared controls would violate exclusive ownership. After PR #89 is integrated, migrate the four selectors using Agent 01's CONTROL_MIGRATION.md contract, preserving numeric limits, options, filters, sort callbacks, keyboard behavior and Arabic/RTL. Current selectors remain functionally native but not the desired final visual design.
2. **Agent 03:** Client-side bounded rendering is not backend/server-side pagination; truly huge project datasets require Agent 03's independent backend contract.
3. **Agent 08 / Agent 00:** Installed Windows/Linux WebView accessibility, focus/keyboard, visual appearance, popup clipping, narrow layout and Arabic RTL still require independent QA. JSDOM success does not certify screenshots.

**Final verdict: BLOCKED for Agent 00 merger** pending latest exact-head CI, integration of Agent 01 custom selectors, Rust dependency advisory remediation, and installed-app QA. All additional Agent 05 fixes remain within the original two views, one dedicated test, and this report.
