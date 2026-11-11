# Agent 01 — Scriptotar design system and native controls handoff

**Repository:** `MousaXD/scriptotar`
**Assigned branch:** `agent-01/design-system-controls-overhaul`
**PR:** https://github.com/MousaXD/scriptotar/pull/89 (draft)
**Audited base:** `f741244634b5143602a21a554094a498a0005b11`
**Latest audited implementation commit (before handoff update):** `5eb362269cd97bec61c61dd024741860b0565385`
**Coordinator:** Agent 00

## Work completed

1. Preserved the existing design contract but flattened global workstation chrome: removed all radial/linear gradients from `app.css` and `theme.css`, replaced the sidebar/topbar/panel backgrounds with semantic surfaces, lowered default card elevation, strengthened muted text contrast, and removed 9–10px annotation type from the application CSS. Corrected legacy dark-only backgrounds and text in system-light mode.
2. Rebuilt semantic design tokens for neutral work surfaces, deliberate control dimensions, spacing, radii, type hierarchy and focus. Light system mode and dark mode now use shared tokens. UI and Arabic font stacks work offline and have an explicit fallback policy.
3. Added seven independently reusable Svelte controls under `src/components/ui/`: `Select`, `Slider`, `Checkbox`, `Switch`, `RadioGroup`, `TextField`, `Popover`. New sliders and select options have at least 40px interaction targets. No backend settings, localization catalog or existing view behavior was changed.
4. Styled the legacy native controls at the global CSS layer, including a closed-selector chevron, checkbox/radio/range accent fallback, visible focus ring, disabled states and RTL arrow placement. **This is a fallback only; an operating-system popup cannot be completely themed from CSS.**
5. Added `controls.test.ts` for keyboard and accessible interaction, `floatingPosition.test.ts` for viewport/RTL geometry, and `control-source.test.ts` plus `raw-modules.d.ts` for regression checks on flat surfaces, tokens, focusable selectors, cross-engine slider styling, and legacy native-control isolation.
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
- `apps/desktop-ui/src/components/ui/floatingPosition.ts` (new)
- `apps/desktop-ui/src/components/ui/floatingPosition.test.ts` (new)
- `docs/agent-handoffs/agent-01-design-system.md` (this report)

No `main`, workspaces, AppShell, Rust services, shared API contracts or localization dictionaries were changed. The agent's branch did not exist at start, so it was created exactly at the assigned audited baseline.

## Tests and CI evidence

New checks are authored but not yet confirmed as passed. Exact audited source commit Desktop UI workflow:
https://github.com/MousaXD/scriptotar/actions/runs/42927155011

At the continuation audit this run was **queued**. Other exact-source workflows (CI, Security hygiene, MCP E2E, Linux Tauri Package, Windows Tauri Installer, Tauri migration integration) were also queued or pending. No successful conclusion has been observed. Other workflows on intermediate heads were mostly cancelled by newer-push concurrency; cancellation is not a pass.

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


## Continuation audit — 2026-11-11

**Verified remote working context:** The current container starts at `/` and contains no local Git checkout; git reports `not a git repository`. The existing GitHub PR and assigned branch were verified directly. No branch other than `agent-01/design-system-controls-overhaul` was created or modified. `main` remains the original baseline and no other agent's branch was merged.

**Scope audit:** Reviewed all 18 PR-changed files after continuation, including the global palettes, base CSS, all seven Svelte primitives, floating-position action, component tests, source-contract checks, and the migration/handoff documents. GitHub comparison showed the branch ahead of `main`, not behind. No AppShell, workspace view, Rust/API boundary, or localization catalog appears in the PR diff.

**Defects found and resolved:**

| Finding | Resolution |
| --- | --- |
| Generic `input` rule imposed text-input height/padding/background on native checkboxes, radios and range inputs | Scoped the legacy form defaults to text-like inputs; excluded hidden, checkbox, radio and range; reset slider input border/padding |
| `RadioGroup` lacked a guaranteed shared HTML `name` if not provided | Generated a unique per-instance fallback name so native arrow-key grouping remains intact |
| Scroll-clipped popup menus / very narrow viewport risk | Added `floatingPosition` action for Select and Popover, with viewport clamping, bottom/above flipping, scroll capture, resize reposition and cleanup; capped popover width to match CSS |
| Selected Select value could be announced without its field label | Added explicit `aria-labelledby` pointing to both label and displayed value; tests assert accessible name |
| Popover trigger lacked explicit accessible name and controls link when rendered with custom slot | Added `aria-label`, unique panel ID and `aria-controls` |
| Stale open popover/select after disabled state changes | Close when disabled; ignore a stale focus callback after closure |
| RTL navigation marker and activity count used hard-coded left/right | Switched global selectors to logical inset properties and added regression checks |
| Shared radius diverged between old CSS and new tokens | Derived legacy radius from `--radius-lg` |
| URL/email/password fields could follow RTL base-direction | Explicit LTR input direction on corresponding text fields |

**Audited static contrast:** A WCAG relative-luminance calculation against the audited semantic background/canvas/surface palettes gives at least 5.13:1 for the darkest auxiliary text in system-light mode and at least 5.89:1 for auxiliary text in dark mode across the checked surfaces. These are static token calculations only; they do not certify all legacy hard-coded component colors or rendered font shapes.

**Added regression coverage:** Source tests guard flat backgrounds, semantic colors, native-control size isolation, consistent selector styling and RTL logical positioning. Component tests exercise selection/disabled-option skipping, Escape/outside dismissal, radio group HTML names, native range/checkbox/switch semantics, live-region labels, non-modal popover focus and aria relationships. Floating-position unit tests cover flipping above the trigger, RTL positioning and viewport width clamping. Tests were authored and committed; **they have not actually executed successfully** in this continuation.

**Validation limitation:** The remote Actions runs for `5eb362269cd97bec61c61dd024741860b0565385` were queued/pending at the last check. GitHub runners had not delivered results. The container has no local repository and cannot resolve github.com from the shell, so `npm ci`, `npm run check`, `npm run test` and `npm run build` could not be executed locally. Do not mark these gates green. New handoff-only commits may cause Actions to restart under workflow concurrency; verify exact PR head once the runners resume.

**Independent integration dependencies:** The other agents own migration of existing `<select>` markup (20 baseline instances) and view-level layout changes. Agent 07 owns translation strings; Agent 08 owns rendered screenshot infrastructure. The floating action and Svelte components still need actual Linux/Windows WebView testing for focus, popup clipping and RTL visual positions. Bundled offline font licensing/assets remain unapproved; installed-font fallback is intentional.

**Continuation verdict: BLOCKED from merge** until the exact PR head has successful Desktop UI/type/test/build checks and Agent 00 receives installed-app UI evidence. All code fixes identified in this scope-only audit are committed. Preserve the existing draft PR.


## CI continuation — exact-head diagnostics and correction (2026-11-11)

**Last independently audited source commit:** `a63c9e500f788f1ebbdd81014ea73965e9e85a78`.

**Actual failed CI evidence:**

- [Desktop UI #42927303086](https://github.com/MousaXD/scriptotar/actions/runs/42927303086), on `30c0fdca69ef2669ba45158f727183abebb301f7`, **failed** at `npm run check` with three errors and one warning.
- Error 1: `Select.svelte` exported a TypeScript type from the instance component script, which Svelte disallows.
- Error 2: `RadioGroup.svelte` exported a TypeScript type from the instance component script.
- Error 3: `TextField.svelte` declared `autocomplete` as unconstrained `string`; Svelte's HTML attribute type is narrower.
- Warning: `Select.svelte` placed `aria-invalid` on the trigger's implicit `button` role, which does not support that ARIA property.
- [Scriptotar MCP E2E #42927303013](https://github.com/MousaXD/scriptotar/actions/runs/42927303013) failed at the same frontend `npm run check` step; there is no separate demonstrated MCP security regression in this run.
- [Security hygiene #42927303011](https://github.com/MousaXD/scriptotar/actions/runs/42927303011) passed on the older source commit. This result **must not be treated as an exact-new-head pass**.
- On the older Windows installer job, `Build frontend assets` and `Build locked Windows binary` steps succeeded before a newer commit superseded/cancelled the workflow. These individual step passes do not certify the entire installer.

**Correction committed and pushed:**

- Moved the exported `SelectOption` and `RadioOption` type definitions to Svelte module scripts, preserving consumer-facing type exports.
- Typed `TextField.autocomplete` as `HTMLInputAttributes['autocomplete']` from `svelte/elements`.
- Moved `aria-invalid` to the role=`listbox` element rather than using an unsupported attribute on a button. The invalid visual style and all existing selection/keyboard handlers remain intact.
- No CI steps, scripts, assertions, security checks, or existing product functionality were removed or loosened.

**Revalidation:** [Desktop UI #42929149805](https://github.com/MousaXD/scriptotar/actions/runs/42929149805) is the source-fix rerun for `a63c9e500f788f1ebbdd81014ea73965e9e85a78`. At the last check it was **queued**, as were the other workflows on that commit (the Linux package showed pending). Do not infer that tests passed from their source presence or from the unrelated successful Windows build steps.

**Current verdict: BLOCKED.** Source-level fixes are committed; exact-head Svelte/type check, component tests, production build, packaged-app behavior, WebView visuals and the integration owners' migration of legacy selectors remain necessary. The next handoff-only commit itself also needs fresh exact-head CI according to the coordinator's merge gate.

## Final continuation checkpoint — selector affordance and latest validation

**Last source-fix commit before this handoff update:** `bfe3fc16f47dd72e56a0bb0aa1b91c0e90154a0e`.

The independent accessibility audit also found that the legacy native-select chevron's dark-theme SVG stroke (`#94a1ad`) was reused against the system-light field surface (`#f4f6f7`). Static contrast calculated to approximately **2.43:1**, below the 3:1 non-text affordance target. Source commit `bfe3fc16f47dd72e56a0bb0aa1b91c0e90154a0e` overrides the light-theme selector arrow with `#586b76` (approximately **5.13:1** against that field surface) while preserving the dark SVG and the original native select's keyboard/functionality. Source-contract test coverage and the in-ownership control migration document were updated in the same commit.

The newest source-commit workflows were launched; at this checkpoint all seven required runs were **queued or pending**:
- [Desktop UI #42929889740](https://github.com/MousaXD/scriptotar/actions/runs/42929889740)
- [CI #42929889807](https://github.com/MousaXD/scriptotar/actions/runs/42929889807)
- [Security hygiene #42929889603](https://github.com/MousaXD/scriptotar/actions/runs/42929889603)
- [Scriptotar MCP E2E #42929889701](https://github.com/MousaXD/scriptotar/actions/runs/42929889701)
- [Linux Tauri Package #42929889683](https://github.com/MousaXD/scriptotar/actions/runs/42929889683)
- [Windows Tauri Installer #42929889665](https://github.com/MousaXD/scriptotar/actions/runs/42929889665)
- [Tauri migration integration #42929889725](https://github.com/MousaXD/scriptotar/actions/runs/42929889725)

**Accurate conclusion:** CI on previous head failed at Svelte/type checks and the identified errors were corrected; there is **no green exact-head validation yet** for the updated source. The repository did not have a local checkout in this execution environment; only GitHub Actions can certify the npm quality gates and installed OS tests. This handoff update is itself a new commit and therefore requires an exact-new-head validation per Agent 00's rule.

**Verdict: BLOCKED**, not ready for Agent 00 merge/release. Code corrections within Agent 01's ownership are complete to the extent a source review can determine. Pending exact-head workflow results, screen-reader/Windows/Linux WebView checks, translation integration and migration of the 20 pre-existing native selectors remain external release gates. No branch was merged and `main` was not modified.

## Exact-head validation update — 2026-11-11

On PR #89 source/head `c7adb8375b4aa49a567b1e32903b90a548d280f3`, [Scriptotar MCP E2E run 42930051196](https://github.com/MousaXD/scriptotar/actions/runs/42930051196) **passed**. Verified steps: `npm ci`, `npm run check` (**0 errors, 0 warnings**), `npm run test` (**90 passed / 15 files**), `npm run build`, Rust workspace and MCP E2E. [Security hygiene 42930051003](https://github.com/MousaXD/scriptotar/actions/runs/42930051003) **passed** on this head. At the last verification, standalone Desktop UI 42930051015, CI 42930050999 and Tauri migration 42930051068 were queued; Linux package 42930051004 and Windows installer 42930051010 were running. **Those are not yet certified.**

Repeated audit of all 18 changed files found no further confirmed source defect inside Agent 01 ownership. Global CSS, seven controls, popup positioning, RTL, tests and docs remain in scope. No Rust, existing views, AppShell, security checks, API or translation changes. Local working directory is `/` with no checkout; all source work used GitHub on the assigned branch.

**Disposition: READY FOR AGENT 00 REVIEW of Agent 01 code and test coverage; NOT ready for integration/merge until latest exact-head required CI, packaged Windows/Linux WebView screenshot/keyboard accessibility evidence and other agents' native-selector migrations are cleared.** This documentation commit itself changes HEAD, so reverify CI on the resulting head.


## Independent final audit checkpoint — 2026-11-11 (exact-head CI and dependency escalation)

**Verified repository / branch / PR:** `MousaXD/scriptotar`; `agent-01/design-system-controls-overhaul`; existing draft [PR #89](https://github.com/MousaXD/scriptotar/pull/89) against `main`. Starting verified HEAD: `16b9e15837bba143b1e5d068c73fed5bc506ef12`. The GitHub comparison showed **22 commits ahead, 0 behind**, with exactly **18 changed files, all within Agent 01 exclusive ownership**. The execution container's working directory is `/` and is not a Git checkout, so code and status were inspected through the existing GitHub connection. No `main` edits or other agents' files were made.

### Finished code review

Re-audited Select and Popover keyboard/focus semantics, RTL-aware floating placement, slider/radio/checkbox/switch/text input styles and labels, dark/system-light tokens, and the existing interaction and viewport-position tests. No additional reproducible Agent 01 source defect was identified after the previous fixes. Menu clipping inside transformed WebView ancestors and screen-reader announcements still require real installed-app testing, not speculative source changes. All previously identified fixes are preserved.

### Exact-head validation results at `16b9e15837bba143b1e5d068c73fed5bc506ef12`

| GitHub Actions run | Result | Verified detail |
| --- | --- | --- |
| [Desktop UI #42931669937](https://github.com/MousaXD/scriptotar/actions/runs/42931669937) | **PASS** | Svelte/type, interaction tests and production build |
| [CI #42931669965](https://github.com/MousaXD/scriptotar/actions/runs/42931669965) | **PASS** | Version identity, frontend, sidecar and Rust tests |
| [Scriptotar MCP E2E #42931669865](https://github.com/MousaXD/scriptotar/actions/runs/42931669865) | **PASS** | MCP protocol, frontend checks/tests/build and gateway security |
| [Security hygiene #42931669926](https://github.com/MousaXD/scriptotar/actions/runs/42931669926) | **PASS** | Security hygiene workflow |
| [Linux Tauri Package #42931669998](https://github.com/MousaXD/scriptotar/actions/runs/42931669998) | **PASS** | Installed deb package and runtime, MCP default deny/auth/revocation, desktop shell launch, repeated installed-app IPC/persistence E2E |
| [Windows Tauri Installer #42931669863](https://github.com/MousaXD/scriptotar/actions/runs/42931669863) | **PASS** | NSIS installer bundle, checksum, clean-machine install and packaged runtime smoke |
| [Tauri migration integration #42931669867](https://github.com/MousaXD/scriptotar/actions/runs/42931669867) | **FAIL** | **Supply chain → Rust advisories**; all its other jobs passed, including frontend, Rust workspace, sidecar, integrated Tauri build |

The earlier MCP CI logs also confirmed `svelte-check` **0 errors / 0 warnings**, **90 frontend tests passed across 15 files**, and successful Vite production build. The source has not been altered since that exact-head validation; this handoff-only update will produce a new HEAD that must be revalidated as required by Agent 00.

### Exact release blocker — unrelated to Agent 01

[Failure: Tauri migration integration #42931669867, Supply chain job](https://github.com/MousaXD/scriptotar/actions/runs/42931669867). `cargo audit` fetched **1,446 RustSec advisories**, scanned **495 crates**, and found **one vulnerability**:

- Crate: `libsqlite3-sys` **0.30.1**
- Advisory: **RUSTSEC-2026-0459**, C SQLite **CVE-2026-119375**
- Severity: **6.8 (medium)**; advisory date 2026-10-30
- RustSec supplied resolution: `libsqlite3-sys >=0.39.0` (patched)
- Separate log warnings: **8 allowed warnings** covering unmaintained or unsound dependencies; these are not the vulnerability that caused exit code 1

**Ownership dependency:** Rust dependencies, `Cargo.lock`, database/SQLite compatibility and supply-chain workflow configuration are outside Agent 01. Agent 00 must assign the dependency upgrade to its Rust/database or security owner, verify API/migration compatibility, and rerun the non-weakened supply-chain audit. **Do not ignore or whitelist RUSTSEC-2026-0459, remove the test, or change Rust dependencies from this branch.**

Other integration dependencies remain: the owners of AppShell/Research/AI/Settings/Library/Transcript must migrate the **20 original native selectors** where branded opened menus are required; Agent 07 owns translation keys; Agent 08 owns real dark/light/Arabic RTL and keyboard/screen-reader screenshot certification.

### Verdict

**READY FOR AGENT 00 REVIEW** of the Agent 01 design-system implementation: source ownership respected, all six named in-scope and packaging workflows passed on the prior exact head, no remaining confirmed Agent 01 defect. **BLOCKED FOR MERGE / RELEASE** until the RustSec vulnerability is fixed by the correct owner, required supply-chain CI turns green, and Agent 00 completes visual/integration certification. Maintain the existing draft PR.


## Continuation self-audit — visual form isolation and CSS precedence (2026-11-11)

**Repository / working context:** `MousaXD/scriptotar`, assigned branch `agent-01/design-system-controls-overhaul`, existing draft PR #89, base `main`. The runner working directory is `/`, not a Git checkout; GitHub API access was used. No new branch or PR was created and no other agent's work was merged. The PR's 18 changed paths remain entirely within Agent 01 ownership.

**Audited implementation source:** `46d46d302cb4efa6ce247fc58464b9205ea4076b`, following two targeted source corrections:

1. `372f3512fca6a233fc7957b43efc4255939e05b7` — system-light theme previously styled **all** `input` elements as text inputs, including custom switches, sliders, checkboxes and radios. Scoped light-mode text-field backgrounds/borders to inputs except `checkbox`, `radio`, `range` and `hidden`, and extended source-contract tests.
2. `46d46d302cb4efa6ce247fc58464b9205ea4076b` — a follow-up specificity audit showed repeated `:not()` pseudo-classes raise CSS specificity and could override the `TextField` invalid-state border. Wrapped those exclusions inside `:where(...)` consistently in `app.css`, `design/base.css`, and `theme.css` so the exclusions are kept without overriding component validation styling. Extended regression assertions.

**Source-wide audit of owned files:** Examined the shared Select, Slider, Checkbox, Switch, RadioGroup, TextField and Popover components, their interaction tests, `floatingPosition.ts` and its geometry tests, the global dark/system-light CSS and token declarations, migration contract, and handoff. Rechecked keyboard active selection, role associations, Escape/outside dismissal, popover focus, RTL insets, reduced motion, narrow layout placement, native input behavior and value propagation. No other confirmed functional regression was identified within ownership. Actual transformed-ancestor popup positioning, OS screen readers, native popup migration and rendered bilingual layouts remain integration tests, not simulated local certification.

**Confirmed CI / tests:**

- For the previous exact head `47fcf49b4c49e17ab3f976ae8e96750ab46496f6`, **six workflows passed**: Desktop UI #42934085297, Security hygiene #42934085362, CI #42934085304, Scriptotar MCP E2E #42934085250, Linux Tauri Package #42934085298 and Windows Tauri Installer #42934085279. Linux and Windows packaging and packaged-app smoke are therefore supported by actual workflow success on that head.
- The seventh workflow, [Tauri migration integration #42934085294](https://github.com/MousaXD/scriptotar/actions/runs/42934085294), **failed** only its supply-chain Rust advisories job; Rust workspace, sidecar, Svelte frontend, Rust↔sidecar integration and integrated Tauri build succeeded. The existing `libsqlite3-sys 0.30.1`, [RUSTSEC-2026-0459](https://rustsec.org/advisories/RUSTSEC-2026-0459) / CVE-2026-119375 remains the exact external security blocker. RustSec specifies `>=0.39.0` patched. Agent 01 must not edit `Cargo.lock`, suppress advisory checks or alter another owner's Rust dependencies.
- For the new source fix `46d46d302cb4efa6ce247fc58464b9205ea4076b`, [Desktop UI #42936639255](https://github.com/MousaXD/scriptotar/actions/runs/42936639255) and [Security hygiene #42936639399](https://github.com/MousaXD/scriptotar/actions/runs/42936639399) had **passed** at this checkpoint. The new CI / MCP / Tauri integration / Linux / Windows workflows were still in progress at the time of this report, and must be checked on their exact commit and on this documentation commit. No uncompleted run is counted as passed.

**Ownership dependencies and gate:** Agent 00 must assign the RustSec SQLite upgrade and associated migration compatibility review to its Rust/database/security owner. Owners of the 20 legacy native selectors must migrate their own views without removing persisted handlers. Agent 08 must provide actual installed Windows/Linux WebView dark/light, Arabic RTL, keyboard/focus and popup evidence. Agent 07 owns translations. Do not bypass any test.

**Source verdict:** **READY FOR AGENT 00 REVIEW** — all confirmed Agent 01 code defects corrected and the new frontend checks passed. **Integration/release verdict:** **BLOCKED** by the non-owned RustSec dependency and final visual/integration acceptance; keep existing PR #89 draft. This handoff update itself creates a newer HEAD, which requires exact-head CI confirmation.


## 2026-11-11 final validation — full matrix at audited HEAD

**Verified implementation HEAD:** `9af950d05cae4ec259e09ebbfe77e037bfdc3693` on the assigned `agent-01/design-system-controls-overhaul` branch, open draft PR #89, based on `main` at `f741244634b5143602a21a554094a498a0005b11`. Repository `MousaXD/scriptotar`; the container working directory `/` has no local checkout. GitHub comparison shows **26 commits ahead, 0 behind**; **18 changed files**, all Agent 01 owned. No main changes, other-agent files or branches, security tests, or security configuration were touched.

### Exact-commit completed Actions matrix

| Workflow | Result |
| --- | --- |
| [Desktop UI #42937070351](https://github.com/MousaXD/scriptotar/actions/runs/42937070351) | **PASS** |
| [CI #42937070827](https://github.com/MousaXD/scriptotar/actions/runs/42937070827) | **PASS** |
| [Scriptotar MCP E2E #42937071383](https://github.com/MousaXD/scriptotar/actions/runs/42937071383) | **PASS** |
| [Security hygiene #42937070339](https://github.com/MousaXD/scriptotar/actions/runs/42937070339) | **PASS** |
| [Linux Tauri Package #42937070527](https://github.com/MousaXD/scriptotar/actions/runs/42937070527) | **PASS** |
| [Windows Tauri Installer #42937070474](https://github.com/MousaXD/scriptotar/actions/runs/42937070474) | **PASS** |
| [Tauri migration integration #42937070307](https://github.com/MousaXD/scriptotar/actions/runs/42937070307) | **FAIL — Rust advisory, outside ownership** |

Desktop UI job logs confirm `svelte-check` **0 errors, 0 warnings**, **15 passing test files / 90 passing tests**, and a successful production build. In the failing integration workflow, **Svelte frontend, Rust workspace, Python sidecar, Rust ↔ sidecar integration, and integrated Tauri build all passed**; only `Supply chain → Rust advisories` failed. This is not a frontend regression.

### Outstanding security and integration blockers

`cargo audit` found **RUSTSEC-2026-0459** / **CVE-2026-119375** for `libsqlite3-sys 0.30.1` (RustSec: `>=0.39.0` patched). The existing `Cargo.lock`, Rust SQLite/database compatibility and supply-chain dependencies are owned outside Agent 01. **Agent 00 must delegate a tested dependency upgrade to the assigned Rust/database/security owner. Do not suppress, ignore or weaken the audit.**

The baseline inventory of **20 native selects** in AppShell and other owners' workspace views still requires coordinated migration for consistent opened dropdowns. Agent 08 must independently certify actual Windows/Linux WebView screenshots, responsive/RTL Arabic layout, keyboard and assistive-technology behavior; passing JSDOM/source tests and package smoke do not constitute that visual/accessibility certification.

### Final independent owner-scope verdict

**READY FOR AGENT 00 REVIEW** of Agent 01's code and tests. The full source audit found no additional reproducible Agent 01 defect; prior input-type isolation and CSS `:where` specificity corrections have passed exact-commit frontend tests and builds. **BLOCKED FOR MERGE / RELEASE** until RustSec and integration/visual requirements pass. Preserve the existing draft PR. This handoff-only commit advances HEAD and therefore requires rechecking exact-new-head workflow status before merge.


## Continued accessibility and RTL audit — exact source validation (2026-11-11)

**Assigned repository and branch:** `MousaXD/scriptotar`, `agent-01/design-system-controls-overhaul`. Existing **draft PR #89** remains the sole PR and is based on `main`. The current container is at `/` with **no Git working tree**, so branch, Git history, PR and files were verified using GitHub API rather than claiming local checkout/test results.

### Audit scope, defects, and fixes

The independent review rechecked all seven controls, their CSS and interaction tests, float positioning geometry, global tokens and dark/light/RTL semantics. It identified and corrected **two additional in-ownership accessibility and RTL defects**:

1. **Checkbox / Switch descriptive text was included in the wrapped label's accessible name.** In `Checkbox.svelte` and `Switch.svelte`, each instance now gives its visible label an ID and uses `aria-labelledby` for the control name. The optional help text receives a separate ID referenced by `aria-describedby`, so the accessible description does not contaminate the control name. No click, keyboard, checked-state, disabled-state, or `onChange` behavior was removed. `controls.test.ts` now asserts independent accessible names/descriptions and clicks through both labels. Commit: `c1a587f387145d98fbf26ecf3030e83c42d97cc2`.
2. **Global workspace text and mobile sidebar rules used LTR physical properties.** Six `text-align: left` rules were converted to `text-align: start` in `app.css` for navigation, list and workspace controls; the activity indicator uses `margin-inline-end`, and mobile sidebar border clearing uses `border-inline-end`. Added CSS source regression checks. Commit: `5b4808d1a0390cf5aa824796e21d938264ef73c2`.

Both changes preserve the appearance/functionality of LTR and improve Arabic/RTL correctness. No worktree, files or scripts belonging to other agents were touched; no security assertion or test was weakened.

### Verified validation evidence

The [Desktop UI #42943822806](https://github.com/MousaXD/scriptotar/actions/runs/42943822806) workflow **passed** on the combined source fix `5b4808d1a0390cf5aa824796e21d938264ef73c2`. The actual job log shows **svelte-check: 0 errors and 0 warnings**, **15 test files / 91 tests passed**, and a successful production build (1.45 seconds). [Security hygiene #42943822863](https://github.com/MousaXD/scriptotar/actions/runs/42943822863) also **passed** on that exact source commit.

The same source HEAD's CI #42943822895, MCP E2E #42943823223, Tauri migration #42943822832, Linux package #42943822817 and Windows installer #42943822846 were queued/in-progress at the final evidence check; their success is **not claimed** here. The previous fully checked HEAD `c8099b54f43ab183ba5d34482943caa515f5d567` had **six passing workflows**, while Tauri migration #42939176518 failed **only the Rust advisory supply-chain job**.

### Explicit cross-agent security and integration dependencies

**Release blocker:** `libsqlite3-sys 0.30.1` with **RUSTSEC-2026-0459 / CVE-2026-119375** (RustSec indicates `>=0.39.0` patched) is in `Cargo.lock`, outside Agent 01. Agent 00 must assign a compatible Rust/SQLite dependency upgrade to the Rust/database/security owner and require the unmodified Rust advisory audit to pass; **do not ignore the advisory or modify the audit threshold**.

**View integration:** 20 original native selectors in other owners' AppShell/workspace views require coordinated migration to the shared Select component to remove OS-default opened-menu appearance, with original handlers, localization and security intact. Agent 07 owns translations. Agent 08 must review installed Windows/Linux WebView visuals, Arabic RTL, overflow/transformed-ancestor popovers, keyboard focus and screen-reader behavior; JSDOM tests and installed-package smoke cannot certify those visual/assistive-technology aspects.

### Final scoped verdict

**READY FOR AGENT 00 REVIEW** of Agent 01's source, tests and design-system contract: the identified defects are committed, the exact-source frontend quality gate passed, and all changes remain within ownership. **BLOCKED FOR MERGE/RELEASE** until the non-owned RustSec vulnerability and cross-agent installed-app certification are resolved. Keep the existing PR draft. This handoff-only update advances the PR HEAD, which must be validated again under the coordinator's exact-head CI rule.


## Exact-head completed gate audit — 2026-11-11 at 06:25 GST

**Verified repository:** `MousaXD/scriptotar`; **working directory:** `/` (not a Git checkout); **assigned branch:** `agent-01/design-system-controls-overhaul`; **existing draft PR:** [#89](https://github.com/MousaXD/scriptotar/pull/89); **reviewed source/PR HEAD:** `6814349d3d0a3029f63fb4519a94675be7ea6131`. Compared against `main` at `f741244634b5143602a21a554094a498a0005b11`: 30 commits ahead, 0 behind, and all 18 changed files belong to Agent 01. No unresolved PR review threads were reported. Source remained unchanged after the last checked-in accessibility/RTL fixes.

The complete exact-commit CI matrix for `6814349d3d0a3029f63fb4519a94675be7ea6131` is now **finished**:

| Workflow | Run | Verdict |
| --- | --- | --- |
| Desktop UI | [42944037004](https://github.com/MousaXD/scriptotar/actions/runs/42944037004) | PASS |
| CI | [42944036996](https://github.com/MousaXD/scriptotar/actions/runs/42944036996) | PASS |
| Scriptotar MCP E2E | [42944037049](https://github.com/MousaXD/scriptotar/actions/runs/42944037049) | PASS |
| Security hygiene | [42944037014](https://github.com/MousaXD/scriptotar/actions/runs/42944037014) | PASS |
| Linux Tauri Package | [42944037034](https://github.com/MousaXD/scriptotar/actions/runs/42944037034) | PASS |
| Windows Tauri Installer | [42944037091](https://github.com/MousaXD/scriptotar/actions/runs/42944037091) | PASS |
| Tauri migration integration | [42944036981](https://github.com/MousaXD/scriptotar/actions/runs/42944036981) | **FAIL — Rust advisories** |

Inside the failed Tauri migration workflow, the **Svelte frontend, Rust workspace, Python sidecar, Rust↔sidecar integration and integrated Tauri build** all passed. Only **Supply chain → Rust advisories** failed. On this HEAD, desktop Svelte/type checks and all 91 frontend tests passed, with zero Svelte errors and zero warnings; the production build passed. Native Linux/Windows installer job success is certified by workflow results, but these results do *not* substitute for Agent 08's dark/light/Arabic RTL and assistive-technology screenshot review.

**Root cause external to this branch:** `libsqlite3-sys 0.30.1`, **RUSTSEC-2026-0459 / CVE-2026-119375**; audit recommends `>=0.39.0`. `Cargo.lock` and Rust/SQLite migration compatibility remain the responsibility of Agent 00's Rust/database/security owner. Do not suppress this security check, and do not change another agent's files from Agent 01.

**Further source audit:** Verified actual `main.ts` stylesheet import order (tokens → app → base → theme → RTL); reviewed the owned tokens, controls, keyboard/accessible descriptions, logical RTL text alignment, floating placement, reduced-motion styles, and tests for regressions. Found no new reproducible Agent 01 defect. The 20 original native selectors still require migration by their view/AppShell owners to achieve consistently branded *open* menus; screen-reader and transformed-container popup certification remains Agent 08's integration gate.

**Final owner-scope verdict: READY FOR AGENT 00 REVIEW. Final merge/release verdict: BLOCKED.** Preserve existing draft PR #89. This appendix updates documentation only and creates a new HEAD; certify any merge using that final commit's required checks rather than treating prior-HEAD results as final-HEAD results.


## Final continuation: TextField invalid-state precedence (2026-11-11)

**Repository:** `MousaXD/scriptotar`. **Local working directory:** `/` (not a Git checkout). **Assigned branch:** `agent-01/design-system-controls-overhaul`; existing draft [PR #89](https://github.com/MousaXD/scriptotar/pull/89), unchanged base `main`. This continuation only edited owned `apps/desktop-ui/src/components/ui/TextField.svelte` and `apps/desktop-ui/src/components/ui/control-source.test.ts`, plus this assigned handoff; no other agent files, `main`, tests or security gates altered.

**Independent audit finding and fix:** The system-light theme's global input border rule (`:root[data-theme='system'] input:where(...)`) has CSS specificity equal to the previous scoped TextField invalid-state rule after Svelte scoping. That could allow CSS bundle order to hide red validation borders. In source commit `bed401756161a5580cd807ead2903726866b0653`, the owned TextField invalid-state selector became `.field input[aria-invalid='true']`, providing higher specificity without overrides. Added an in-scope source regression assertion for the selector and absence of forceful priority declarations.

**CI-driven correction:** [Desktop UI #42948400516](https://github.com/MousaXD/scriptotar/actions/runs/42948400516) found one **new test failure**, because a new CSS comment contained the exact prohibited text tested by that assertion. No product code assertion was weakened. The comment was corrected in source commit `1d127c34a6d93ebc2de9988bb371dd582a3fb87d`; the unchanged assertion then passed.

**Verified exact-source result:** [Desktop UI #42948511224](https://github.com/MousaXD/scriptotar/actions/runs/42948511224) **PASSED**, with `svelte-check` **0 errors / 0 warnings**, **92 passing tests in 15 files**, and a successful Vite production build. [Security hygiene #42948511067](https://github.com/MousaXD/scriptotar/actions/runs/42948511067) **PASSED** on the same source commit. CI #42948511164, MCP E2E #42948511089, Windows installer #42948511149, Linux package #42948511127 and Tauri integration #42948511226 were in progress when recorded; success is not assumed.

**Remaining external release gates:** Previous exact-HEAD runs consistently failed the Rust advisory audit on **RUSTSEC-2026-0459 / CVE-2026-119375** for `libsqlite3-sys 0.30.1`, with RustSec patched version `>=0.39.0`; dependency remediation, migrations and `Cargo.lock` changes belong to the Rust/database/security owner. The 20 pre-existing native selects in other owners' views still need migration to branded opened menus. Agent 08 owns installed WebView screenshot, keyboard, transformed-ancestor popup and Arabic RTL / screen-reader certification; Agent 07 owns shared translations. No security check has been bypassed.

**Verdict:** **READY FOR AGENT 00 REVIEW** of Agent 01's implementation, with exact-source frontend and security checks green; **BLOCKED FOR MERGE/RELEASE** until the RustSec advisory and other integration gates are cleared. Preserve PR #89 as draft. This documentation commit advances HEAD; recheck final-HEAD CI before release.


## Closed Select accessibility semantics and re-audit (2026-11-11)

Verified the existing draft PR #89 on its original `agent-01/design-system-controls-overhaul` branch against `main`; the local container working directory is `/` and has no Git worktree, so changes were made via GitHub on the assigned branch without modifying `main` or other agents' files. Prior exact-head `ba12dc257568a0d47b5ee92b38b6190578a4e5a4` completed with **six successful workflows** (Desktop UI, CI, MCP E2E, security hygiene, Linux package and Windows installer); its Tauri migration integration failed **only Rust advisories**, due to `libsqlite3-sys 0.30.1` / RUSTSEC-2026-0459 / CVE-2026-119375. All five other Tauri integration jobs passed. Previous frontend output had **92 passing tests**, zero Svelte errors/warnings, and a successful production build.

**Additional in-ownership defect found:** the reusable Select's trigger was exposed as a generic button, while `aria-required` and `aria-invalid` existed only on the listbox when opened. Closed/select-only workflows therefore did not communicate required or invalid state to assistive technology. The trigger now uses `role=combobox`, keeps a stable accessible name from its visible label rather than folding the current choice into the name, and exposes `aria-required` / `aria-invalid` even when closed. The visual selection and native hidden form value, dropdown focus behavior, keyboard navigation, disabled options, pointer handling and change callbacks remain intact. The owned interaction tests now assert the combobox semantics, required/invalid state, description, selected value, form input propagation, and name stability after selection; the owned control migration contract documents the semantics and explicit parent-form validation requirement.

**Final owner-scope verdict:** READY FOR AGENT 00 REVIEW once the new exact-HEAD frontend checks pass; keep PR #89 draft, BLOCKED FOR MERGE/RELEASE by the non-owned RustSec dependency and external installed WebView/RTL/accessibility integration. Do not bypass any security gate. This update is in the same commit as the source correction, so its exact-HEAD Actions must be verified before claiming tests passed.


## Narrow-screen popover placement audit — 2026-11-11

**Repository:** `MousaXD/scriptotar`. Assigned branch `agent-01/design-system-controls-overhaul`, existing draft PR #89, no local checkout (working directory `/`), and no access to other agent-owned paths. Pre-fix exact-head `20ceff7193684d560edb008e4205d0ad2993d2af` passed Desktop UI (93 tests, no Svelte errors/warnings), CI, MCP E2E, Security hygiene and Linux package; Windows installer was still in progress on last check. Tauri migration failed only Rust advisory `RUSTSEC-2026-0459` in `libsqlite3-sys 0.30.1`, outside Agent 01 scope.

**Additional confirmed in-scope layout bug:** Popover's CSS imposed `min-width: 200px` while the viewport positioning helper could calculate as little as 160px for non-match-width panels. This lets the actual rendered panel extend beyond the measured width, risking overflow near right-side triggers and in narrow windows. The placement helper now treats non-matched popovers as at least 200px before viewport clamping; Popover's CSS minimum also shrinks responsively to `min(200px, 90vw)` to honor narrow windows. Existing select match-width positioning remains unchanged. Added a narrow-viewport geometry test and source CSS contract assertion. These changes preserve trigger handlers, keyboard, focus, scrollbar and RTL behavior.

**Final owner-scope verdict:** READY FOR AGENT 00 REVIEW subject to successful exact-new-head frontend validation. Overall merge/release BLOCKED until Rust/database/security owner resolves the SQLite advisory without bypass, the other view owners migrate their 20 native selects, and Agent 08 certifies installed-app dark/light visuals, Arabic RTL, popup placement and assistive-technology behavior. The source, tests and this handoff were committed together to avoid an unnecessary second CI-triggering commit.


## Search focus-visible audit — 2026-11-11

**Repository/branch:** `MousaXD/scriptotar` / `agent-01/design-system-controls-overhaul`, continuing the existing draft PR #89. Remote GitHub commits and branch verified because working directory `/` is not a local Git checkout. All changes in this continuation remain strictly in Agent 01-owned CSS/test/handoff paths.

**Confirmed keyboard-accessibility defect:** Global search and filter search inputs were assigned `outline: 0` in `app.css`, while their containing `.global-search` / `.search-field` wrappers had no `:focus-within` or `:focus-visible` treatment. This could remove the visible keyboard focus indicator from search fields despite generic input focus rules. Added a 2px, theme-aware `var(--accent-strong)` focus ring and 2px offset to both wrappers using `:focus-within`, preserving the deliberately borderless input and all existing search handlers. Added an owned CSS-source regression assertion that the wrapper ring is present.

**Before this fix:** exact HEAD `45e488286c157f26b646cf7a69cc4942ae2169df` had six completed successful GitHub workflows (Desktop UI, CI, MCP E2E, Security hygiene, Linux Tauri Package, Windows Tauri Installer). Its Desktop UI logs showed **95 passing tests, 0 Svelte errors/warnings and production build success**. Tauri migration integration failed **only the Rust advisory supply-chain step** on `RUSTSEC-2026-0459 / CVE-2026-119375` affecting `libsqlite3-sys 0.30.1`; all other five Tauri migration jobs passed. The dependency owner is Agent 00's Rust/database/security workstream. No checks or assertions were disabled or weakened.

**Readiness:** READY FOR AGENT 00 REVIEW once the updated exact-commit frontend tests and security hygiene pass. BLOCKED FOR MERGE/RELEASE until the external RustSec issue and the other agents' native selector/installed WebView RTL, keyboard, clipping, screenshot and screen-reader certification requirements are satisfied. Keep PR #89 draft. This handoff is committed atomically with the CSS fix and regression test so exact-head verification applies directly.


## Dynamic Select options audit — 2026-11-11

**Repository:** `MousaXD/scriptotar`; original branch `agent-01/design-system-controls-overhaul`, draft PR #89. The active working directory `/` is not a local Git checkout, so the assigned branch, commit history, ownership and GitHub Actions were inspected remotely. Before this correction, exact HEAD `efbf1352dce3b89e7b6cae4fcc94a02968418bc2` had **six successful workflows**, with 95/95 desktop UI tests, 0 Svelte errors/warnings and a successful production build. Tauri migration integration failed **only the Rust advisory** supply-chain job, RUSTSEC-2026-0459 / CVE-2026-119375 in `libsqlite3-sys 0.30.1`. The remaining five Tauri migration jobs were successful.

**Additional independent defect found and corrected:** While Select's listbox is open, a caller can replace or reorder `options` after an asynchronous project/source update. Previously `active` remained a numerical index. If its item moved, keyboard selection could choose the wrong option; if removed, `aria-activedescendant` could refer to a nonexistent option and Enter would do nothing. Select now reconciles a changed options array by the formerly highlighted option's **value**, falling back to the current valid selected option or first enabled option if that value disappears. Disabled-option, keyboard and focus semantics are preserved. Added an owned component interaction regression covering reordering, removal, valid `aria-activedescendant`, and Enter selection, and updated the control-migration guide.

No other agent files were modified; no features, security checks or test assertions were removed. **Scoped verdict: READY FOR AGENT 00 REVIEW** after validating the new exact-HEAD frontend checks. **Overall merge/release: BLOCKED** on the non-owned RustSec dependency fix by the Rust/database/security owner and the view/installed-app integration gates (native select migration, Windows/Linux WebView, RTL and assistive technology). Preserve draft PR #89.


## No-op Select re-selection audit — 2026-11-11

**Branch and original PR:** `agent-01/design-system-controls-overhaul`, draft PR #89 in `MousaXD/scriptotar`. Local working directory `/` has no Git checkout; original branch, commit history, all 18 changed ownership paths, open draft PR, previous handoff, and prior exact-HEAD CI results were verified remotely before editing. Prior HEAD `7f2e9a178df71cfcaefd73718964f25090229913` completed **six successful GitHub Actions workflows**: Desktop UI (96/96 tests, 0 Svelte errors/warnings, build), CI, MCP E2E, Security hygiene, Linux package including installed runtime smoke, Windows installer including clean-machine smoke. Its Tauri migration workflow failed **only Rust advisories** in supply chain (RUSTSEC-2026-0459 / CVE-2026-119375, `libsqlite3-sys 0.30.1`); all five other integration jobs passed.

**New confirmed in-ownership behavior defect:** A user could click the already-selected option in Select and `choose()` would invoke `onChange` again despite the selected value remaining unchanged. Unlike a native select's change event, this caused unintended repeat save/workflow callbacks in consumers. Selecting the current value now closes the popup and restores focus **without** invoking `onChange`; selecting a different enabled option still fires the callback exactly once, updates the selection/form value, and closes. Added a regression test covering selected-item no-op, changed selection and repeated no-op after changing, and documented the expected change-event contract in the control migration guide. No other files, security checks or assertions were modified.

**Final owner-scope verdict:** READY FOR AGENT 00 REVIEW after new exact-HEAD UI checks pass. **Merge/release remains BLOCKED** on the non-owned RustSec dependency remediation and view/installed-WebView dark/light, Arabic RTL, keyboard, popup and screen-reader integration acceptance. Do not bypass the RustSec check; keep PR #89 draft.


## No-op Select callback test reconciliation — 2026-11-11

After commit `1f75e7451a88aa6b5dd6e290eb1f97787edb85ca` pushed the no-op selection fix, the actual GitHub Desktop UI workflow #42968256494 **failed one older interaction test**: `preserves or repairs the active option when a live options list changes` asserted `onChange('a')` even though the previously selected value was already `a`. The new test introduced by the fix passed; the earlier test's callback assumption was incompatible with native select change semantics. The existing test was **retained and strengthened**: after dynamic option removal it now explicitly verifies the fallback is visibly highlighted, pressing Enter does **not** invoke `onChange`, the listbox closes, and the trigger still displays the selected option. The independent new test continues to require a callback **exactly once** when the value changes, plus no callback for both initial and subsequent no-op reselections. This corrects a regression in the test contract rather than suppressing tests, deleting assertions or weakening security checks.

**Final owner-scope verdict:** READY FOR AGENT 00 REVIEW if the correction's exact-HEAD UI checks pass. Overall integration remains BLOCKED until RustSec RUSTSEC-2026-0459 / CVE-2026-119375 in `libsqlite3-sys 0.30.1` is fixed by the assigned Rust/database/security owner and cross-agent installed-app/RTL/accessibility criteria are met. Keep existing PR #89 draft, do not touch main or other agents' files.


## Nested Escape propagation audit — 2026-11-11

**Verified state before change:** original agent-owned branch `agent-01/design-system-controls-overhaul`, existing draft PR #89, last exact HEAD `8a821cf7e86025d19e77e48c7a78d32b1074f0a1`, 18/18 modified paths in Agent 01 ownership, no review threads and no local Git checkout at `/`. Last HEAD had six successful GitHub Actions workflows (Desktop UI **97/97 tests**, 0 Svelte errors/warnings, production build, CI, MCP E2E, security hygiene, Linux package, Windows installer). Tauri migration failed only the **non-owned** Rust advisory `RUSTSEC-2026-0459 / CVE-2026-119375` in `libsqlite3-sys 0.30.1`; its five other jobs passed.

**Confirmed shared-component keyboard defect:** Popover's window-level Escape handler closed the parent panel even if a nested Select/listbox or dialog had already called `preventDefault()` to consume Escape and close only itself. This violated nested-layer keyboard dismissal and unexpectedly discarded the enclosing panel. Popover now checks `!event.defaultPrevented` before handling Escape. A targeted component interaction regression dispatches a consumed bubbling Escape and verifies the parent remains open, then verifies an ordinary unconsumed Escape still closes it and returns focus to the trigger. Existing outside-press and focus-out behavior is unchanged. The migration guide documents this event propagation contract.

**Scoped verdict:** READY FOR AGENT 00 REVIEW after validating the exact new commit's frontend tests and CI. **Overall merge/release BLOCKED** until Rust/database/security owner remediates the RustSec vulnerability without weakening audit, other agents migrate native selectors and Agent 08 validates installed WebView RTL/keyboard/contrast/visual/screen-reader behavior. No other agent files touched and PR #89 remains draft.
