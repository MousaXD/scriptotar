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
| Queue / Jobs | Existing native file picker, safe drag/drop selection, URL/manual-path queueing, cancel/retry and state filters are functional. Some copy still depends on legacy DOM translation. | Preserved source validation boundary; no new file or URL permissions; existing UI interaction tests retained. | Explicit i18n migration and packaged Tauri drop/picker smoke tests; add busy-state cancellation coverage. |
| Transcript | Loading failure removed the normal transcript rail, leaving only an error with no Retry or alternate selection. Externally switching selected transcript could transiently show stale previous content. | Clear old transcript when a new load starts; localized status; Retry button and fallback chooser; regression test of a failed read followed by successful selection. Exports, search and collapsible rails remain. | Media playback blocked on a narrowly scoped Rust/Tauri resource contract (Agent 01); do not use unrestricted `file://` paths. Screen-reader and bidi stress tests pending. |
| Library | Filter/sort controls, kinds, empty states and announcements mixed DOM-driven translation with user metadata. “Newest” comparator performed repeated `indexOf` lookups, scaling poorly. | Explicit English/Arabic labels, kind names, sort options, results and empty states; exclude user metadata from DOM rewriting; map-based stable newest order. | Consider pagination/virtualized rows only when a real backend contract exists; visual/card density and very large library datasets require profiling. |
| Settings | Existing sections, theme/language controls, persistence, output folder selection and migration states are retained. Many settings strings remain DOM-translated; long page needs user testing. | Preserved all settings and navigation access at narrow widths. | Move remaining owned settings copy to explicit messages, and verify packaged theme persistence and keyboard tab order. |
| Research / AI Studio | Existing workflows and visual state are part of the regression baseline. | Unmodified to avoid collisions with Agent 06. | Agent 06 owns these views and should coordinate shared keys with Agent 05. |
| MCP wizard | Embedded in Settings; owned by Agent 03. | Only the AppShell's shortcut label and accessibility were changed. | Agent 03 owns wizard semantics, permissions and strings. |

## Accessibility, language, and UX contract

- **Navigation:** all destination buttons remain rendered; Settings and MCP Setup are again reachable in compact layout. A skip link targets the main workspace.
- **Keyboard:** Ctrl/Cmd+K, arrow navigation, Enter command selection, Escape dismissal, Tab/Shift+Tab containment, and opener-focus restoration are covered by tests.
- **Languages:** the shell additions, Dashboard, Library and transcript loading/recovery use explicit keys in English and Arabic. The remainder of existing views still uses the compatibility translator; migration is **not yet complete**.
- **Bidi and user content:** user-supplied titles, project names, sources, transcript segments and metadata are not forcibly translated. The Library panel excludes DOM rewriting, while selected transcript content retains `data-i18n-ignore` and its own source direction.
- **Failure recovery:** a failed transcript read now has retry and alternate-transcript escape hatches, without claiming the file is playable.
- **Security:** no permissions, Rust/Tauri commands, filesystem URLs, backend services, MCP settings, or destructive actions were introduced.

## Regression tests added

`apps/desktop-ui/src/gui-overhaul.test.ts` now covers: utility navigation accessible names and collapse, palette focus containment/return and reopen, Arabic Library controls without altering transcript titles, and recovery from a failed transcript load. Existing frontend tests continue to cover project switching, native picking, queue operations, exports, RTL content, settings persistence, watchlists, and migration UI.

## Validation requirements

CI is the validation authority; use the `Desktop UI` PR workflow for `npm ci`, `npm run check`, `npm run test`, and `npm run build`, plus the root `CI` and relevant Linux/Windows packaging gates. **Do not infer success from a committed source change.** Add actual workflow URLs, SHAs and conclusions to the PR report after they finish.

Visual before/after screenshots were not available in this chat-run environment. The exact packaging/WebView visual behavior is therefore unverified until run in the project smoke-test environment.

## Remaining defects / handoff

1. **Localization debt:** finish the transition from the existing MutationObserver translation bridge to explicit keyed UI copy for Jobs, Transcript and Settings (Dashboard and Library are migrated). Do not rewrite transcript or user-authored data.
2. **Rendered regression:** automated English LTR/dark, Arabic RTL/dark, system-light, narrow-window and long-content screenshots are still needed. Existing jsdom tests do not prove pixel-level responsive behavior.
3. **Native packaging:** verify real Tauri file picker, drag/drop, focus styles, WebView scrolling and RTL layout in the installed Linux/Windows applications.
4. **Media resource contract:** request secure, narrow Rust/Tauri playback support from Agent 01 before adding media playback to Transcript.
5. **Performance:** for very large Library and Transcript datasets, profile before adding pagination or virtualization; no nonexistent backend APIs were mocked.

Agent 05 must not merge this branch into the base branch. A green CI result is necessary but not sufficient for the manual GUI and RTL acceptance checklist.
