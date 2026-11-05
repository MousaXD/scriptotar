# Agent 05 — GUI, UX and localization audit

**Baseline:** `feature/chatgpt-mcp-ci-20261026` at `517877b26df9028cdd32af69f6f5a88a68b948e9`  
**Implementation:** `agent-05/gui-ux-overhaul`  
**Coordination:** Agent 00 owns review/merge; Agent 03 owns `McpSetupWizard.svelte`, Agent 06 owns AI Studio and Research. No backend API contract was changed.

## Screen-by-screen findings and disposition

| Area | Baseline behavior / usability defect | Agent 05 implementation | Further work |
| --- | --- | --- | --- |
| Shell / navigation | At widths ≤720px, CSS hid the entire utility sidebar, including Settings and MCP Setup. Icon-only navigation relied on title/hidden labels for accessibility. | Restore utility buttons in the compact horizontal rail; explicit accessible names for destinations; localized MCP shortcut; translated skip-to-main link. | Validate actual 760px and narrower WebView layout with rendered screenshots and keyboard walkthrough. |
| Command palette / workspace search | Modal did not contain Tab navigation or return focus to its opener on Escape; focus restoration to the topbar search could cause an immediate reopen. | Focus trapping at Tab boundaries; restore the previous active element; prevent reopening on the restored focus; preserve keyboard commands and click-to-reopen. | Check screen reader announcement on Windows and Linux WebViews. |
| Dashboard / projects | Existing action-first overview and project create/persist flow are useful. Fixed strings still depended on the MutationObserver bridge; no dedicated loading skeleton for each panel. | Explicit bilingual Dashboard copy, RTL-correct navigation arrows, and user-content protection. All project actions, navigation and recent work remain. | Test very long project names / slow queries and panel skeletons. |
| Queue / Jobs | Native file picker, safe drag/drop selection, URL/manual-path queueing, cancel/retry and filters already work but much fixed copy relied on the DOM translation bridge. | Explicit bilingual labels for native picker, queue actions, filters, statuses and empty states. Preserved Rust path validation and job workflows. | Shared JobRow localization and installed Tauri picker/drop smoke tests; add busy-state cancellation coverage. |
| Transcript | Failed loading hid the selector, preventing recovery; stale previous content could flash on selection. Reader chrome mixed DOM translation with user text. | Clear stale content; localized Retry/alternate picker, navigation, reader controls and export feedback; preserve transcript body direction, formats and search. | Safe playback contract from Agent 01; real WebView screen-reader and bidirectional stress tests pending. |
| Library | Filter/sort controls, kinds, empty states and announcements mixed DOM-driven translation with user metadata. “Newest” comparator performed repeated `indexOf` lookups, scaling poorly. | Explicit English/Arabic labels, kind names, sort options, results and empty states; exclude user metadata from DOM rewriting; map-based stable newest order. | Consider pagination/virtualized rows only when a real backend contract exists; visual/card density and very large library datasets require profiling. |
| Settings | Long settings page uses DOM rewriting, with potential confusion between displayed translated option values and persisted semantics. | Explicit bilingual sections, save/status copy, migration states and help text; original Rust/Tauri setting codes retained; MCP wizard left to Agent 03. | Verify packaged persistence, keyboard tab order, language-option presentation and migration candidate behavior. |
| Research / AI Studio | Existing workflows and visual state are part of the regression baseline. | Unmodified to avoid collisions with Agent 06. | Agent 06 owns these views and should coordinate shared keys with Agent 05. |
| MCP wizard | Embedded in Settings; owned by Agent 03. | Only the AppShell's shortcut label and accessibility were changed. | Agent 03 owns wizard semantics, permissions and strings. |

## Accessibility, language, and UX contract

- **Navigation:** all destination buttons remain rendered; Settings and MCP Setup are again reachable in compact layout. A skip link targets the main workspace.
- **Keyboard:** Ctrl/Cmd+K, arrow navigation, Enter command selection, Escape dismissal, Tab/Shift+Tab containment, and opener-focus restoration are covered by tests.
- **Languages:** the shell additions, Dashboard, Jobs, Transcript, Library and Settings chrome use explicit English/Arabic keys. Shared components (notably JobRow), MCP wizard and Agent 06's Research/AI views retain the compatibility bridge; full migration is **not yet complete**.
- **Bidi and user content:** user-supplied titles, project names, sources, transcript segments and metadata are not forcibly translated. The Library panel excludes DOM rewriting, while selected transcript content retains `data-i18n-ignore` and its own source direction.
- **Failure recovery:** a failed transcript read now has retry and alternate-transcript escape hatches, without claiming the file is playable.
- **Security:** no permissions, Rust/Tauri commands, filesystem URLs, backend services, MCP settings, or destructive actions were introduced.

## Regression tests added

`apps/desktop-ui/src/gui-overhaul.test.ts` now covers: utility navigation accessible names and collapse, palette focus containment/return and reopen, Arabic Dashboard, Jobs, Library, Settings and Transcript controls without modifying user titles or stored setting values, and recovery from a failed transcript load. Existing frontend tests continue to cover project switching, native picking, queue operations, exports, RTL content, settings persistence, watchlists, and migration UI.

## Validation requirements

CI is the validation authority; use the `Desktop UI` PR workflow for `npm ci`, `npm run check`, `npm run test`, and `npm run build`, plus the root `CI` and relevant Linux/Windows packaging gates. **Do not infer success from a committed source change.** Add actual workflow URLs, SHAs and conclusions to the PR report after they finish.

Visual before/after screenshots were not available in this chat-run environment. The exact packaging/WebView visual behavior is therefore unverified until run in the project smoke-test environment.

## Remaining defects / handoff

1. **Localization debt:** finish the explicit-translation migration of remaining shared UI components (JobRow, StateBadge, EmptyState and other legacy output) and coordinate boundaries with Agents 03/06. Do not rewrite transcript or user-authored data.
2. **Rendered regression:** automated English LTR/dark, Arabic RTL/dark, system-light, narrow-window and long-content screenshots are still needed. Existing jsdom tests do not prove pixel-level responsive behavior.
3. **Native packaging:** verify real Tauri file picker, drag/drop, focus styles, WebView scrolling and RTL layout in the installed Linux/Windows applications.
4. **Media resource contract:** request secure, narrow Rust/Tauri playback support from Agent 01 before adding media playback to Transcript.
5. **Performance:** for very large Library and Transcript datasets, profile before adding pagination or virtualization; no nonexistent backend APIs were mocked.

Agent 05 must not merge this branch into the base branch. A green CI result is necessary but not sufficient for the manual GUI and RTL acceptance checklist.
