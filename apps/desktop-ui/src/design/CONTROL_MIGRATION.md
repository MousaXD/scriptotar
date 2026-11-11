# Workstation control inventory and migration contract

This document is owned by Agent 01 and describes the frontend control foundation, not the behavior of backend commands.

## Design rules

- Main surfaces are neutral graphite/white, without decorative radial backgrounds or repeated card shadows.
- Mint indicates primary action or successful readiness, not decoration. Functional processing, warning and error colors remain distinct.
- Control height is 40px; longer transcript content retains its separate reading scale.
- UI and Arabic text use local/offline font stacks. We intentionally do not request web-hosted fonts. **Exact glyph metrics are not guaranteed across Windows/Linux until a licensed font bundle is approved.**
- Native operating-system file and directory pickers remain native.
- Dark and system modes share semantic tokens. Sidebar markers and activity counters use logical inline offsets; new controls also avoid physical left/right anchoring. Static palette contrast was audited against canvas and raised surfaces (main text, muted and faint are at least 5:1 in both audited palettes), but this does not replace rendered accessibility testing. The system-light native selector chevron now uses a darker SVG stroke with approximately 5.13:1 contrast against the light field surface; this is a closed-select fallback only.

## Shared components (new)

Import from `src/components/ui/<Name>.svelte`:

| Component | Props / integration API | Interaction contract |
| --- | --- | --- |
| `Select` | `label`, `options: {value,label,disabled?}[]`, `bind:value`, `onChange?`, `name?`, `required?`, `invalid?`, `description?` | Styled trigger and listbox; Up/Down/Home/End; Enter/Space; Escape restores focus; Tab exits; options skipped when disabled; outside press closes |
| `Slider` | `label`, `bind:value`, `min`, `max`, `step`, `format?`, `onChange?` | Native range semantics and keyboard, branded track/thumb on WebKit/Firefox |
| `Checkbox` | `label`, `bind:checked`, `description?`, `onChange?` | Native checkbox semantics and spacebar |
| `Switch` | `label`, `bind:checked`, `description?`, `onChange?` | Checkbox input with `role=switch` |
| `RadioGroup` | `label`, `options`, `bind:value`, `name?`, `onChange?` | Native radio controls in a named fieldset |
| `TextField` | `label`, `bind:value`, `type?`, `invalid?`, `description?`, `onInput?` | Real native input with accessible name and validation status |
| `Popover` | `label`, `triggerLabel`, trigger slot and default content slot | Labeled non-modal panel; Escape and outside press close; focus restored |

**Important:** Components expose bindable values/callbacks, but no Rust commands, backend settings semantics, translation keys, or implicit persistence. The caller remains responsible for validation and saving. The Select listbox is not a native required form input; parent forms must validate mandatory selections explicitly. Popover is non-modal: do not use it for confirmation or focus-trapping dialogs.

## Native inventory on initial audited baseline `f7412446`

The audited existing UI still contains the following native elements (migration is intentionally not part of Agent 01's exclusive ownership):

| Owner / file | `<select>` | Checkbox | Radio | Input |
| --- | ---: | ---: | ---: | ---: |
| AppShell | 2 | 0 | 0 | 2 |
| Dashboard | 0 | 0 | 0 | 1 |
| Jobs | 0 | 0 | 0 | 2 |
| Transcript | 1 | 0 | 0 | 1 |
| Research | 3 | 1 | 0 | 3 |
| AI Studio | 4 | 0 | 0 | 8 |
| Library | 1 | 0 | 0 | 1 |
| Settings | 9 | 5 | 0 | 5 |
| MCP Setup Wizard | 0 | 11 | 3 | 17 |

The baseline contains no native `type=range` in those files. AppShell and owned workspace views must migrate their selectors for *consistent opened states*. CSS `appearance: none` can brand the closed native selector but **cannot make OS-owned popup items consistent**. Leave genuine file dialogs native.

## Agent integration instructions

1. Preserve existing handlers and values. Replace each native select only after mapping its `<option>` values to a `Select` options array; use `bind:value` and explicitly retain existing `onChange` side effects.
2. Pass already translated English/Arabic visible strings. Do not put localization inside generic primitives or translate user-generated item names.
3. Use `Checkbox`/`Switch` for boolean controls and `RadioGroup` for exclusive options; retain settings saves and authorization checks from the original view.
4. Select and Popover use viewport-fixed floating placement, with an action that observes window resize and nested scroll capture. Their DOM hierarchy, keyboard focus and outside-click boundaries remain intact. Test clipping/scroll containers, WebView zoom and overlapping focus surfaces in installed Linux/Windows builds.
5. Agent 07 owns localization dictionaries, Agent 08 screenshot infrastructure, and each view owner migrates its own markup.

## Gaps to certify

- Rendered Windows/Linux WebViews are needed to confirm fixed popup placement/flip inside overflow panels, native file picker regressions, font rendering and focus behavior in production.
- JSDOM tests verify roles, changes and event handling; they do not constitute visual or real-screen-reader certification.
- Cross-platform pixel fidelity requires an explicitly licensed and offline-bundled font (currently a deliberate installed-font fallback).
- Native forms still using `select` have platform-controlled *open* menus until their owning agents migrate.
