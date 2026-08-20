# Scriptotar GUI/UX Improvement Implementation Plan

Status: implementation plan
Branch: `feature/gui-ux-overhaul`
Base branch: `feature/arabic-gui`
Primary surface: `apps/desktop-ui`

## 1. Mission

Turn Scriptotar Next from a functional developer-style dashboard into a polished local creator workstation for research, transcription, transcript review, and AI-assisted reuse.

The redesign must improve repeated daily use, not merely add decoration. It must preserve the current Rust/Tauri architecture, local-first privacy model, English/Arabic switching, RTL support, keyboard operation, and existing backend behavior unless a small backend contract is explicitly required by a UI feature.

The product should feel closer to a focused media workstation than a generic SaaS admin dashboard.

## 2. Design research used for this plan

This plan was informed by current public agent design skills and comparable transcription/editing products.

### Public SKILL.md patterns worth adopting

1. **Anthropic frontend-design**
   - Choose a product-specific aesthetic direction before coding.
   - Avoid generic card-and-gradient UI patterns.
   - Treat typography, layout, color, and interaction as one coherent system.
   - Make one memorable design choice that belongs to the product.

2. **OpenAI frontend-skill**
   - Product tools should prioritize utility copy over marketing copy.
   - Use restrained composition, strong hierarchy, and fewer unnecessary containers.
   - Avoid card soup and visual clutter.
   - Use motion sparingly and intentionally.

3. **Microsoft frontend-design-review**
   - Judge the interface on frictionless insight-to-action, quality craft, and trustworthy behavior.
   - Accessibility and responsive behavior are part of design quality, not cleanup work.

4. **Microsoft VS Code accessibility skill**
   - Treat keyboard navigation, focus behavior, ARIA, and announcements as default requirements for interactive desktop UI.

5. **Improve UI / design-language skills**
   - Audit the product against its existing design evidence before replacing its identity.
   - Define a reusable design-language contract and explicit visual budgets so later agents do not drift.

6. **Frontend Design Codex / dashboard-design skills**
   - Optimize the first screen for the user's primary workflow.
   - Build normal, loading, empty, error, and narrow-window states together.
   - Verify rendered behavior instead of considering source-code correctness sufficient.

### Comparable product patterns worth borrowing

1. **Buzz**
   - Advanced transcript viewer with search, playback controls, speed control, keyboard shortcuts, and persistent interface state.
   - File/URL transcription is presented as a first-class workflow rather than buried in settings.

2. **Descript**
   - Transcript is the primary editor surface.
   - Supporting panels can be collapsed so the user can focus on the current task.
   - Timeline/playback tooling is available when needed without dominating the default layout.

3. **Whisper Desktop / similar local Whisper tools**
   - Drag-and-drop media import is a natural desktop affordance.
   - Local/private status should be visible but quiet, not repeated as marketing copy throughout the workspace.

## 3. Current-state audit

### What is already strong

- Svelte 5 + Vite frontend is small and understandable.
- Tauri/Rust owns persistence and validates sensitive paths.
- Core workspaces are already separated: Dashboard, Research, Jobs, Transcript, AI Studio, Library, Settings.
- Keyboard shortcuts and visible focus styles already exist.
- Dark and system themes exist.
- Arabic localization and RTL support exist on the base branch.
- Reduced-motion handling exists for the current animated status indicators.
- Job states and failure recovery are explicit rather than hidden.
- `ResearchItem.thumbnail` and `Creator.avatar` already exist in the typed frontend model, even though the current UI often renders letter placeholders instead.

### Main problems to solve

#### A. The visual system still reads as a generic dashboard

`app.css` uses many one-off hex values and repeated bordered gradient panels. Most content is enclosed in similar containers, so important and secondary information have nearly equal visual weight.

The current typography is essentially `Inter`/system UI with very small 9-11px metadata throughout. That is efficient but visually anonymous and can become tiring on desktop displays.

#### B. Navigation treats all seven destinations as equally important

Dashboard, Research, Jobs, Transcript, AI Studio, Library, and Settings appear as one flat list. The app does not visually communicate the primary workflow:

`Research / Import -> Queue -> Transcript -> AI / Library`

Settings is a utility destination but receives the same weight as core workspaces.

#### C. The top bar is becoming crowded

Project selection, global search, language selection, and job activity all compete in one horizontal strip. The Arabic-language control added on the base branch makes this crowding more visible at medium window widths.

#### D. Dashboard is metric-first instead of action-first

The first screen mostly summarizes counts. For a creator workstation, the more useful first screen is:

- start a new transcription or research scan,
- continue active work,
- reopen recent transcripts,
- see anything requiring attention.

Metrics can remain, but should be subordinate.

#### E. Media ingestion is too form-like

Jobs currently starts with separate local-file, URL, and advanced-path controls. It works, but does not feel like a desktop media tool.

Missing affordances:

- drag and drop,
- one unified "add media" area,
- recent/default transcription settings summary,
- clearer queued/working/needs-attention grouping.

#### F. Transcript workspace is the biggest UX opportunity

The current three-column layout is useful, but the center reader is mostly static text. It lacks the interaction patterns that make transcription tools feel professional:

- media playback,
- play/pause and seek,
- playback speed,
- current-segment highlighting,
- click timestamp to seek,
- optional follow-playhead behavior,
- stronger search-result highlighting,
- collapsible side panels,
- compact export menu.

Do not invent a playable file URL. If the frontend cannot safely access the original media through the current Tauri contract, add a narrowly scoped backend command that returns a safe playable asset URL or explicitly mark playback unavailable.

#### G. Research has data the UI is not using

`ResearchItem.thumbnail` and `Creator.avatar` are available in the model, but current views frequently show platform initials or name initials instead of the real assets.

The wide research table is useful on large windows but becomes a horizontal-scrolling data slab on smaller windows. The selected-action control also moves away from the user's attention when the table is long.

#### H. AI Studio is powerful but visually dense

Mode, task, provider, model, credentials, source, prompt, brief fields, actions, status, and result all compete on one screen.

The UI should emphasize a simple path first:

1. choose task,
2. provide source/context,
3. build/copy or run,
4. inspect result.

Provider configuration should be secondary and only expand when BYOK is selected.

#### I. Settings is a long wall of configuration

Settings groups are technically clear, but the two-column sections form a long vertical stack. Save state is only at the top of the view. Migration is important but visually consumes similar weight to everyday preferences.

#### J. Styling architecture will become difficult to maintain during a redesign

Most shared visual behavior lives in one large `app.css`, plus component-local styles and `theme.css`/`rtl.css`. Repeated control patterns are not yet represented by reusable Svelte components.

#### K. Localization implementation is fragile for a large redesign

The Arabic branch currently uses a MutationObserver-driven translation layer that rewrites matching DOM text and selected attributes. It successfully avoids rewriting transcript content, but a large component redesign will create many new strings and dynamic nodes.

Before the GUI grows substantially, migrate user-facing chrome toward explicit translation keys while preserving the current locale store and persisted language behavior. The DOM translator can remain temporarily as a compatibility bridge, then be retired after coverage is complete.

## 4. Design direction

### Visual thesis: "Creator Control Room"

Scriptotar should feel like a calm local media workstation: dark graphite equipment surfaces, readable editorial text, compact data instrumentation, and a restrained mint signal color.

It should not look like:

- a generic analytics dashboard,
- a glassmorphism demo,
- a marketing landing page inside a desktop shell,
- an overdecorated AI product.

### Signature interaction

The memorable product-specific element should be the **transcript rail**: timestamped transcript segments that visibly connect media time, spoken text, search, and downstream AI actions.

This is more meaningful to Scriptotar than decorative gradients or oversized dashboard statistics.

### Color budget

Keep the existing green/mint identity, but make token usage strict:

- neutral graphite background,
- one main surface family,
- mint only for primary action / active navigation / success,
- blue for active processing,
- amber for warning/interrupted,
- red for destructive/error,
- no random per-component hex colors outside semantic tokens.

### Typography

Use a locally bundled, open-source family with strong Latin and Arabic support if package size remains reasonable. Preferred direction:

- IBM Plex Sans for Latin UI,
- IBM Plex Sans Arabic for Arabic UI,
- IBM Plex Mono for timestamps, keyboard hints, paths, model IDs, and technical metadata.

If bundling fonts materially complicates distribution, keep the platform UI stack but still introduce a deliberate type scale and monospace utility role. Never load fonts from a network CDN in the desktop app.

### Density

This is a workstation. It should be compact, but not microscopic.

- normal controls: target 40-44px minimum height,
- body copy: 13-14px,
- labels/metadata: normally 11-12px,
- reserve 9-10px only for truly secondary technical annotations,
- large decorative headings should be rare.

### Radius and elevation

Use at most three radius levels and three elevation levels.

Avoid putting every section inside a floating card. Prefer layout, spacing, separators, and background layers first.

### Motion

- 120-180ms for hover/focus/selection transitions,
- a single subtle entering transition for major view changes if it remains comfortable,
- progress motion only when conveying work,
- fully respect `prefers-reduced-motion`.

## 5. Target information architecture

### Sidebar

Group destinations by purpose instead of one flat list.

**Work**
- Home
- Research
- Queue
- Transcripts

**Create**
- AI Studio
- Library

**Utility**
- Settings at the bottom

Use real icons plus labels. Add an optional compact/collapsed rail for medium-width desktop windows. Persist the rail preference locally.

Do not hide keyboard shortcuts; move them to tooltips/command help instead of displaying a letter on every navigation row at all times.

### Top bar

Primary content:

1. active project selector,
2. global command/search field,
3. concise global status/action cluster.

Move the full language selector out of the constant top-bar spotlight. Keep language easy to reach through a compact globe/menu control and also expose it in Settings > Interface.

Job activity should be represented by a small status control that opens Queue, not a large competing pill.

### Command palette

Extend `Ctrl/Cmd+K` from search-only toward a lightweight command palette:

- search transcripts/projects/creators,
- navigate to views,
- new transcription,
- new research scan,
- open settings,
- switch project.

Search results and commands must remain keyboard navigable.

## 6. Implementation phases

### Phase 0 - Freeze contracts and add design documentation

Deliverables:

- `docs/DESIGN.md` or `apps/desktop-ui/DESIGN.md`
- semantic design token inventory,
- screenshot baseline of every current view in English LTR and key Arabic RTL views,
- viewport matrix for desktop and compact widths.

Document:

- spacing scale,
- type scale,
- semantic colors,
- radius/elevation budget,
- status-state colors,
- motion rules,
- icon rules,
- RTL rules,
- what must never be translated (user content, transcript content, paths, URLs, provider/model IDs).

Acceptance gate:

- no production UI behavior changes yet,
- current tests stay green.

### Phase 1 - Localization and design-system foundation

Files likely touched:

- `src/i18n.ts`
- new `src/i18n/en.ts`
- new `src/i18n/ar.ts`
- new `src/design/tokens.css`
- new `src/design/base.css`
- `src/theme.css`
- `src/rtl.css`
- `src/app.css`

Tasks:

1. Introduce explicit translation keys and a `t(key, params?)` helper/store.
2. Migrate shared shell and reusable component copy first.
3. Keep the existing DOM translator only as a temporary compatibility bridge.
4. Split hardcoded visual values into semantic tokens.
5. Introduce spacing/type/radius/elevation/motion tokens.
6. Normalize focus rings, disabled states, hover states, and form-control sizing.
7. Add a typography utility for technical metadata.
8. Verify dark, system-light, English, and Arabic RTL combinations.

Acceptance gate:

- no MutationObserver dependency for newly redesigned components,
- language switch still persists,
- transcript/project/user content remains untouched,
- no semantic setting values change when labels are translated.

### Phase 2 - Shared component kit

Create small Svelte primitives only where repetition already exists.

Suggested components:

- `Button.svelte`
- `IconButton.svelte`
- `Field.svelte`
- `SelectField.svelte`
- `SearchField.svelte`
- `StatusBadge.svelte` (replace/extend current state badge)
- `PanelHeader.svelte`
- `EmptyState.svelte` (refine existing)
- `ErrorState.svelte` (refine existing)
- `Toolbar.svelte`
- `DropdownMenu.svelte`
- `Tooltip.svelte`
- `Progress.svelte`

Rules:

- do not build a giant abstract component framework,
- preserve native HTML behavior where possible,
- every interactive primitive gets keyboard/focus tests,
- icon-only controls require accessible names.

Acceptance gate:

- duplicated control styling materially decreases,
- no view loses functionality.

### Phase 3 - App shell and navigation

Files:

- `components/AppShell.svelte`
- shell-related CSS
- localization keys

Tasks:

1. Rebuild sidebar with grouped navigation and icons.
2. Add persisted compact/collapsed rail.
3. Simplify topbar hierarchy.
4. Convert global search into command/search overlay with arrow-key navigation, Enter activation, and Escape close.
5. Move language into a compact globe menu while retaining Settings access.
6. Improve active-job indicator.
7. Add useful tooltips for shortcuts.
8. Make compact-window navigation deliberate instead of seven horizontally scrolling text tabs.

Compact behavior recommendation:

- <= 900px: collapsed icon rail or compact top navigation,
- <= 760px: switch to a two-row mobile/tablet shell only if needed for tests; remember this remains primarily a desktop app.

Acceptance gate:

- all destinations reachable by mouse and keyboard,
- `Ctrl/Cmd+K` works from every view,
- Arabic mirrors shell geometry correctly,
- no topbar overlap at tested widths.

### Phase 4 - Home and media-ingestion workflow

Files:

- `views/DashboardView.svelte`
- `views/JobsView.svelte`
- `components/JobRow.svelte`

Dashboard redesign:

Replace metric-first composition with:

1. **Quick start**
   - Add local media
   - Paste URL
   - Start research scan
2. **Continue working**
   - active/transcribing jobs
   - items needing attention
3. **Recent output**
   - latest transcripts
   - recent research
4. secondary compact project stats

Jobs redesign:

- unified add-media surface,
- drag/drop zone for supported local media,
- native picker button,
- URL field in same surface,
- advanced raw path remains available but visually secondary,
- show a one-line summary of the transcription defaults that will be used,
- clearer queue sections or filters,
- failure detail expandable instead of flooding rows,
- sticky batch actions when selections exist.

Acceptance gate:

- a new user can queue local media or a URL without visiting Settings,
- keyboard-only import remains possible,
- drag/drop paths still pass through Rust validation,
- interrupted/failed state semantics remain unchanged.

### Phase 5 - Transcript workspace redesign

This is the highest-value phase.

Files:

- `views/TranscriptView.svelte`
- new transcript-focused components
- potentially a narrow Tauri API addition only if safe media playback requires it

Target layout:

- collapsible transcript list on the left,
- dominant transcript reader in the center,
- optional inspector on the right,
- sticky playback/transport strip attached to the transcript workspace,
- optional compact waveform/timeline only after basic transport works reliably.

Tasks:

1. Add media play/pause when the current backend contract can provide a safe source.
2. Timestamp click seeks media.
3. Highlight current segment during playback.
4. Add playback speed control.
5. Add follow-playhead toggle.
6. Visually highlight search matches inside segments.
7. Add next/previous search result controls.
8. Convert export buttons to one export menu.
9. Add copy-text feedback using a nonintrusive toast/status region.
10. Make metadata inspector collapsible.
11. Ensure Arabic transcript content uses its own direction independent of interface locale.
12. Preserve plain text, timestamp TXT, SRT, VTT, and JSON exports.

Do not implement editing of transcript text in this phase unless persistence semantics are defined first.

Acceptance gate:

- transcript browsing is clearly the strongest surface in the app,
- long transcripts remain responsive,
- search, playback, and timestamp navigation work by keyboard,
- source content is never passed through UI localization.

### Phase 6 - Research workspace redesign

Files:

- `views/ResearchView.svelte`
- supporting creator/media components

Tasks:

1. Render `ResearchItem.thumbnail` when present; use the existing placeholder only as fallback.
2. Render `Creator.avatar` when present.
3. Improve table hierarchy and spacing.
4. Add sticky header for long result sets.
5. Add selected-count action bar that stays visible while scrolling.
6. Improve watchlist-health cards into a compact status strip/list.
7. Make failure details expandable.
8. Add responsive card/list fallback below the width where the research table stops being useful.
9. Use locale-aware number/date formatting instead of hardcoded English number formatting.
10. Preserve platform, sort, queue, and scan behavior.

Acceptance gate:

- thumbnail/avatar data is actually visible,
- no unnecessary horizontal scrolling at the compact desktop target,
- queued selections remain obvious,
- Arabic metrics/dates render coherently.

### Phase 7 - AI Studio and Library

#### AI Studio

Simplify into a progressive workflow:

- task preset rail,
- source/context pane,
- optional brief fields,
- output pane,
- provider configuration drawer for BYOK only.

Add clear trust copy at the point where the mode changes from local prompt construction to external provider execution.

Do not turn AI Studio into a chat UI unless the product behavior actually becomes conversational.

#### Library

- stronger type filters,
- consistent icons,
- better metadata hierarchy,
- row hover/focus treatment,
- compact/comfortable density option only if justified by real use,
- retain direct navigation to the source workspace.

Acceptance gate:

- Copy Prompt mode remains obviously local,
- BYOK credential fields appear only when needed,
- Library remains a fast index rather than becoming another dashboard.

### Phase 8 - Settings and first-run usability

Settings structure:

- General / Interface
- Transcription
- Downloads
- Storage
- Research / Watchlists
- Privacy / Processing
- Advanced / Migration

Tasks:

1. Add a left sub-navigation or segmented settings index for wide windows.
2. Put Theme and Interface language together.
3. Keep a sticky save/revert bar when draft settings differ from persisted settings.
4. Show unsaved-change state explicitly.
5. Move legacy migration into an Advanced section while keeping failure/recovery visible when action is needed.
6. Improve descriptions; remove repeated "Rust owns..." copy unless it directly helps a decision.
7. Add first-run empty-state guidance on Home rather than a modal-heavy onboarding wizard.

Acceptance gate:

- everyday settings are reachable faster,
- migration remains safe and discoverable,
- saving has clear success/error/dirty states.

### Phase 9 - Accessibility, visual regression, and performance gate

Add or strengthen:

- keyboard traversal tests,
- focus-return tests for menus/dialogs,
- ARIA live-region tests for job and copy status,
- WCAG 2.2 AA contrast checks for text and controls,
- RTL screenshot checks,
- reduced-motion checks,
- visual regression screenshots for critical views,
- narrow-window screenshots,
- long-content fixtures,
- empty/loading/error fixtures.

Recommended tooling:

- keep Vitest + Testing Library for component behavior,
- add Playwright for rendered desktop-webview-sized screenshot/interaction coverage,
- optionally add axe-core integration for automated accessibility checks.

Performance targets:

- no unbounded MutationObserver localization work in redesigned surfaces,
- avoid rendering hundreds of transcript/research rows when virtualization becomes necessary,
- no remote fonts/images required for shell rendering,
- no animation that keeps the GPU busy while the app is idle.

## 7. Proposed file organization

```text
apps/desktop-ui/src/
  components/
    shell/
    transcript/
    research/
    ui/
  design/
    tokens.css
    base.css
    components.css   # only if shared CSS remains preferable to component-local styles
  i18n/
    index.ts
    en.ts
    ar.ts
  views/
  app.css            # layout/composition only; shrink over time
  theme.css
  rtl.css
```

Do not reorganize everything in one commit. Move files only when the phase being implemented benefits from the move.

## 8. Testing matrix

Every redesigned phase should be checked against at least:

| Dimension | Required cases |
|---|---|
| Locale | English, Arabic |
| Direction | LTR interface, RTL interface, RTL transcript inside either interface |
| Theme | Dark, System-Light |
| Width | 1440, 1100, 900, 760-ish compact |
| Input | Mouse, keyboard |
| Motion | Normal, reduced motion |
| Data | Typical, empty, long content, failure state |

Critical regression cases:

- translated `<option>` labels preserve typed setting values,
- project names are never translated,
- transcript text is never translated,
- URLs and filesystem paths are never translated,
- provider/model identifiers are never translated,
- active job state remains visible without relying only on color,
- focus is never trapped in search/menu overlays.

## 9. Commit strategy

Use small reviewable commits instead of one visual mega-commit.

Suggested sequence:

1. `docs(ui): define Scriptotar design language`
2. `refactor(i18n): add keyed localization foundation`
3. `refactor(ui): add semantic tokens and shared primitives`
4. `feat(ui): redesign app shell and command search`
5. `feat(ui): redesign home and media queue`
6. `feat(ui): upgrade transcript workspace`
7. `feat(ui): refine research workspace`
8. `feat(ui): simplify AI studio and library`
9. `feat(ui): reorganize settings`
10. `test(ui): add accessibility and visual regression gates`
11. `chore(ui): final responsive and RTL polish`

Run `npm run check`, `npm run test`, and `npm run build` after every phase. Once Playwright is introduced, its smoke/visual suite becomes part of the phase gate as well.

## 10. Priority order if implementation time is limited

P0:

- design tokens and localization foundation,
- shell/navigation cleanup,
- Home/import workflow,
- transcript workspace,
- accessibility/RTL regression protection.

P1:

- Research redesign,
- Settings information architecture,
- real thumbnails/avatars,
- command palette,
- visual regression tests.

P2:

- AI Studio polish,
- Library polish,
- optional waveform,
- optional sidebar density preferences,
- decorative motion.

## 11. Definition of done

The GUI improvement is complete only when:

- Scriptotar has a coherent, documented design language;
- Home leads with useful actions rather than generic metrics;
- media can be queued through a desktop-native workflow;
- Transcript is the strongest and most efficient workspace;
- Research uses real available media/creator imagery;
- the shell works cleanly at compact desktop widths;
- English and Arabic are first-class and RTL is intentionally designed;
- keyboard and screen-reader semantics are preserved or improved;
- dark and system-light themes remain supported;
- user content is never translated accidentally;
- all frontend checks, interaction tests, visual tests, and production build are green;
- the redesign does not weaken Rust/Tauri path validation, local-first privacy, or failure transparency.
