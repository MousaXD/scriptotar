# Scriptotar Desktop UI Design Contract

## Product thesis

Scriptotar is a local creator workstation, not a generic analytics dashboard. The interface should feel like calm media equipment: compact, legible, operational, and trustworthy.

The signature interaction is the timestamped transcript rail. Media time, spoken text, search, exports, and later playback should feel like one connected workspace.

## Visual budget

- Graphite/neutral surfaces dominate.
- Mint is reserved for primary actions, active navigation, success, and local-ready signals.
- Blue communicates active processing.
- Amber communicates warnings/interruption.
- Red is reserved for destructive actions and failure.
- Prefer separators, spacing, and background layers over wrapping every block in a card.
- No network-loaded fonts, icons, or design assets.

## Tokens

Semantic tokens live in `src/design/tokens.css`.

Use:

- `--color-*` for semantic colors.
- `--space-*` for spacing.
- `--text-*` for type sizes.
- `--radius-*` for the three allowed radius levels plus pills.
- `--shadow-*` for the three elevation levels.
- `--motion-*` and `--ease-standard` for UI transitions.
- `--font-technical` for timestamps, paths, keyboard hints, IDs, and model names.

New components should not introduce arbitrary hex colors when a semantic token can express the state.

## Density

Scriptotar is a workstation. Compact is good; microscopic is not.

- Normal controls target 40px minimum height.
- Body copy normally uses 13–14px.
- Labels and metadata normally use 11–12px.
- 9–10px is reserved for secondary technical annotation.
- Transcript reading text should remain more generous than surrounding instrumentation.

## Navigation

The desktop information architecture is grouped by workflow:

### Work
- Home
- Research
- Queue
- Transcripts

### Create
- AI Studio
- Library

### Utility
- Settings

The sidebar can collapse to an icon rail and persists that local preference. Keyboard shortcuts remain supported but do not need to occupy permanent visual space.

`Ctrl/Cmd+K` opens the command/search palette. Search must support keyboard movement, Enter activation, and Escape dismissal.

## Localization and RTL

The redesigned shell uses explicit translation keys through `src/i18n/translate.ts`.

The previous MutationObserver translator remains a compatibility bridge for views that have not been migrated yet. Newly redesigned components should use explicit keys instead of adding more DOM-rewrite dependencies.

Never translate or rewrite:

- transcript text,
- user/project/creator names,
- paths,
- URLs,
- API keys,
- provider/model IDs,
- persisted semantic setting values.

Use logical CSS properties where possible so RTL does not require a second geometry implementation.

## Interaction states

Every interactive control must account for:

- default,
- hover,
- keyboard focus,
- disabled,
- busy/loading when applicable,
- success or error feedback when applicable.

Icon-only controls require accessible names. Native HTML behavior is preferred over custom widgets unless the custom behavior adds real product value.

## Motion

Motion is functional, short, and optional.

- Normal transitions: 120–180ms.
- Progress/status animation only when it communicates ongoing work.
- `prefers-reduced-motion` must remove nonessential animation and smooth scrolling.

## Responsive desktop behavior

Target verification widths:

- 1440px and wider: full workstation layout.
- ~1100px: compact rails and reduced auxiliary width.
- ~760px: narrow desktop/tablet fallback with no overlap or unreachable controls.

Scriptotar remains desktop-first. Do not distort desktop workflows just to imitate a mobile app.

## Verification gate

A GUI change is not finished when it merely compiles. Verify:

- Svelte/type checks,
- interaction tests,
- production build,
- keyboard navigation,
- English LTR,
- Arabic RTL,
- dark theme,
- system-light theme,
- empty/error/busy states,
- long transcript/project names,
- narrow-window behavior.

Rendered screenshot regression coverage should be added once the shell and highest-value workspaces stabilize.
