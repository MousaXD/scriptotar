# Agent 01 — Scriptotar design system and native controls handoff

**Repository:** `MousaXD/scriptotar`
**Assigned branch:** `agent-01/design-system-controls-overhaul`
**PR:** https://github.com/MousaXD/scriptotar/pull/89 (draft)
**Audited base:** `f741244634b5143602a21a554094a498a0005b11`
**Latest implementation commit (before handoff):** `3710e861416be8e0adbfc8727061fb8ed7790bdb`
**Coordinator:** Agent 00

## Work completed

1. Preserved the existing design contract but flattened global workstation chrome: removed all radial/linear gradients from `app.css` and `theme.css`, replaced the sidebar/topbar/panel backgrounds with semantic surfaces, lowered default card elevation, strengthened muted text contrast, and removed 9–10px annotation type from the application CSS.
2. Rebuilt semantic design tokens for neutral work surfaces, deliberate control dimensions, spacing, radii, type hierarchy and focus. Light system mode and dark mode now use shared tokens. UI and Arabic font stacks work offline and have an explicit fallback policy.
3. Added seven independently reusable Svelte controls under `src/components/ui/`: `Select`, `Slider`, `Checkbox`, `Switch`, `RadioGroup`, `TextField`, `Popover`. No backend settings, localization catalog or existing view behavior was changed.
4. Styled the legacy native controls at the global CSS layer, including a closed-selector chevron, checkbox/radio/range accent fallback, visible focus ring, disabled states and RTL arrow placement. **This is a fallback only; an operating-system popup cannot be completely themed from CSS.**
5. Added `controls.test.ts` for keyboard and accessible interaction and `control-source.test.ts` plus `raw-modules.d.ts` for regression checks on flat surfaces, tokens, focusable selectors, and cross-engine slider styling.
6. Published `src/design/CONTROL_MIGRATION.md`, including the 20 existing native selectors in the audited app shell and view files and the migration contract for other agents.

## Files touched

- `apps/desktop-ui/src/app.css`
- `apps/desktop-ui/src/theme.css`
- `apps/desktop-ui/src/design/tokens.css`
- `apps/desktop-ui/src/design/base.css`
- `apps/desktop-ui/src/design/CONTROL_MIGRATION.md` (new)
- `apps/desktop-ui/src/design/raw-modules.d.ts` (new)
- `apps/desktop-ui/src/components/ui/Select.svelte` (new)
- `apps/desktop-ui/src/components/ui/Slider.svelte` (new)
- `apps/desktop-ui/src/components/ui/Checkbox.svelte` (new)
- `apps/desktop-ui/src/components/ui/Switch.svelte` (new)
- `apps/desktop-ui/src/components/ui/RadioGroup.svelte` (new)
- `apps/desktop-ui/src/components/ui/TextField.svelte` (new)
- `apps/desktop-ui/src/components/ui/Popover.svelte` (new)
- `apps/desktop-ui/src/components/ui/controls.test.ts` (new)
- `apps/desktop-ui/src/components/ui/control-source.test.ts` (new)
- `docs/agent-handoffs/agent-01-design-system.md` (this report)

No `main`, workspaces, AppShell, Rust services, shared API contracts or localization dictionaries were changed. The agent's branch did not exist at start, so it was created exactly at the assigned audited baseline.

## Tests and CI evidence

New checks are authored but not yet confirmed as passed. GitHub Actions Desktop UI run:
https://github.com/MousaXD/scriptotar/actions/runs/42924312254

At report creation this run was **queued** for the implementation commit. Other workflows on intermediate heads were mostly cancelled by newer-push concurrency; cancellation is not a pass.

The required `npm ci`, `npm run check`, `npm run test`, `npm run build` and installed-app visual checks cannot be claimed as passed until exact-PR-head CI and packaged Windows/Linux WebView verification finish.

## Open items / known limits

- Native open dropdown menus in pre-existing view markup remain operating-system-rendered; migrating those markup files is deliberately forbidden for Agent 01. Existing `select` controls are styled only in their closed state.
- `Select` is a non-native listbox implementation. Screen-reader and actual WebView keyboard/focus certification is required. It exposes `aria-required`; parent forms must explicitly validate required selections before persisting.
- `Popover` is non-modal and should not be used for permissions, confirmations or destructive dialogs.
- The offline font stack uses installed Noto/DejaVu when available, then the generic host sans/mono. For pixel-stable Windows/Linux typography, ship explicitly licensed local font assets with the appropriate owner approval.
- AppShell and workspace views still contain local CSS and decorative structures outside this file ownership; visual integration requires their owning agents to update the markup rather than adding broad global hacks.
- Real screenshot comparisons, Arabic reading layout, contrast, actual keyboard focus, Windows/Linux native font fallback, and packaged media/file picker smoke tests belong to the integration acceptance gate (Agent 08 and Agent 00).

## Required integration steps

1. **Agent 00:** Merge only after exact-head Desktop UI/type/unit/build checks pass, check styles against all concurrent workspace PRs, and obtain installed-app evidence. Keep this PR draft until then.
2. **View owners:** Follow `src/design/CONTROL_MIGRATION.md`: replace existing native selectors with `Select` where open-menu consistency is required; preserve persisted values and handlers, translate strings at call site, and test selection keyboard/RTL.
3. **Agent 07:** Supply locale keys for labels/options in owned translation dictionaries, never in these primitives.
4. **Agent 08:** Screenshot/control matrix for dark, system-light, Arabic RTL, narrow widths, open select and popover, slider, checkbox/radio/switch and installed Windows/Linux WebViews.

## Release disposition

**Not yet ready for merge.** Source implementation is on the assigned branch and PR #89, but exact-head CI and installed-app visual/accessibility certification are pending. No claim of a screenshot-verified professional UI is made here.
