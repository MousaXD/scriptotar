# Agent 06 — Transcript Reader & AI Studio UX Overhaul

**Repository:** [MousaXD/scriptotar](https://github.com/MousaXD/scriptotar)  
**Exclusive branch:** `agent-06/transcript-ai-workspace-overhaul`  
**Initial baseline:** `f741244634b5143602a21a554094a498a0005b11` (`main`)  
**Existing PR:** [#93](https://github.com/MousaXD/scriptotar/pull/93) (draft; Agent 00 owns merger)  
**Last source-code commit before this handoff refresh:** `2d9f0d3bfbd01d98a23d2eed5d51291b3dc5096d`  
**Final branch HEAD:** refer to PR head, which includes this handoff-only commit  
**Verdict:** **BLOCKED for full integration/release** pending exact-head CI and cross-agent dependencies. Scoped frontend source fixes are implemented.

## Ownership and continuity

Verified branch, PR, committed history, prior handoff and baseline. No local Git checkout exists in this runtime; GitHub repository operations were performed on the existing remote branch. No new branch, duplicate PR, main edits, releases, or merges. Compare against baseline changes only:

- `apps/desktop-ui/src/views/TranscriptView.svelte`
- `apps/desktop-ui/src/views/AiStudioView.svelte`
- `apps/desktop-ui/src/views/TranscriptAiWorkspace.test.ts`
- `docs/agent-handoffs/agent-06-transcript-ai.md`

## Transcript — completed and audited

- Prioritized transcript text/heading over decorative chrome; improved reading scale, spacing and responsive content flow.
- Preserved source metadata in the collapsible details rail using existing localized `ai.sourceLabel`. Preserved transcript list rail, persisted rail visibility, all original metadata, timestamp navigation, keyboard search, next/previous matches, copy text, and TXT/timestamped TXT/SRT/VTT/JSON exports.
- Initial render capped at 140 segments, expanding in batches on scrolling or an explicit button. Full-transcript search still finds distant segments; jumping from a filtered match restores context. This reduces initial DOM work, not constant-memory virtualization; a distant jump can render preceding segments.
- Exact ID/project validation for transcript loading, current-project verification at response time, stale-result generation guards, cancellation of orphaned displayed state, and handling of same-ID source migrations between projects.
- Small loading skeleton with reduced-motion support; retained `dir=rtl/ltr` and content display for English/Arabic.
- Did not invent media playback, local artifact paths, or unsafe file URLs.

## AI Studio — completed and audited

- Clear Copy Prompt/BYOK mode switching with `aria-pressed`, readable source/brief/prompt/result hierarchy, and provider/model settings displayed only in BYOK mode.
- Existing manual-first source picker and exact transcript ID/project lineage; removed-source selection and same-ID moved-project selections reset to manual, clear stale source text, and cannot continue displaying “Exact loaded source”.
- Mode changes preserve previously built prompts and cancel stale in-flight prompt generation. Edits to source/brief invalidate old prompts; earlier provider responses and `finally` blocks cannot replace or unlock a newer in-flight run.
- Local prompt building sends no key or provider URL. BYOK remains opt-in, keys transient in memory, no additional persistence; unused compatible-provider base URLs are excluded for non-compatible providers.
- Prompt preview editing never silently becomes the BYOK request: the existing copy-only/edit-warning contract remains. Local provider remains explicitly unavailable.
- UI error redaction covers submitted keys, URL-encoded variants, bearer headers and common URL credentials. This is defense in depth; Rust security boundaries remain authoritative.
- Reduced-motion source skeleton, readable text, RTL-safe source/prompt/result direction; avoided new app-level rerender work.

## Audit defects found and fixed this continuation

1. **Svelte compiler blocker:** TypeScript callback attempted to capture possibly-undefined `selected` — fixed with locally narrowed selection; removed three obsolete CSS selectors. First evidence: [Desktop UI #42924945243](https://github.com/MousaXD/scriptotar/actions/runs/42924945243), which failed before these fixes.
2. **Same-ID project switching:** transcript reader now revalidates current project on resolution; AI Studio clears source selection and gates exact lineage on both source ID and project.
3. **AI prompt loss:** switching Copy Prompt/BYOK previously dropped an already prepared prompt. Retained completed prompt while invalidating only in-flight builds/runs.
4. **Export/source parity:** added explicit tests for all five export buttons; moved source metadata from the simplified header into the preserved details rail.
5. **Long-transcript late matches:** retained access to full context after search/timestamp navigation; whitespace-only search no longer triggers a no-results state.
6. **Test assertion correctness:** updated new asynchronous assertions without removing expectations or weakening safeguards.
7. **Race/visibility fix:** selected source lineage cannot show “Exact loaded source” after a reset to manual even if legacy reactivity has not yet recomputed its derived reference.

## Tests / CI evidence — distinguish SHAs

- [Desktop UI #42985339925](https://github.com/MousaXD/scriptotar/actions/runs/42985339925) on earlier source SHA `6f44bdd63b31ec690644ede4290fdd1cd9887c34`: **PASS**, Svelte/TS checks, **83/83 frontend tests**, Vite production build. This is **not** proof of the later test additions.
- Expanded tests on SHA `865c9707b73f16b8e24a65e1ea2e786c1ea43651`: [MCP E2E #42986137027](https://github.com/MousaXD/scriptotar/actions/runs/42986137027) reached frontend test stage and reported **86/87 passed, 1 failure** — exact-lineage remained visible after a project swap. Fixed at source SHA `2d9f0d3`; **do not claim later tests passed without the new run evidence**.
- Current dedicated suite has **nine tests**: long transcript paging and late search, old response isolation, wrong-project rejection, same-ID move, all five exports, Arabic RTL/timestamps/source metadata, prompt preservation across modes, AI source invalidation, and overlapping BYOK runs. Existing app, GUI, AI source, i18n, keyboard and backend bridge tests also remain enabled.
- No local `npm run check`, `npm run test`, `npm run build` because this runtime does not have a checkout or external network access. GitHub Actions is the actual validation environment.
- Exact-head CI following the source and handoff changes must be checked before upgrading the scoped readiness verdict. Do not count cancelled predecessor runs.
- Real installed WebView screenshots, zoom/clipping, Arabic RTL, keyboard/screen-reader and production/provider calls are **not** verified by jsdom or installer smoke alone.

## Remaining dependencies and blockers — Agent 00 ownership

1. **Shared controls — Agent 01 PR #89:** the styled `Select` component only exists on `agent-01/design-system-controls-overhaul`, not this branch or `main`. Agent 06's native transcript picker and AI Studio task/rate/provider/source selectors must migrate within these two view files **after Agent 00 integrates the shared component**, preserving option values, `onChange` side effects, source selection, RTL and keyboard operation. Importing it now would intentionally break this standalone branch; copying it would duplicate Agent 01 ownership. Contract: `apps/desktop-ui/src/design/CONTROL_MIGRATION.md` on Agent 01's branch. Agent 08 flagged this on PR #93.
2. **Localization — Agent 07:** supply shared Arabic/English translations for new “Show more” transcript control and remaining untranslated static AI Studio controls; Agent 06 must not modify shared translation dictionaries.
3. **Security — Rust/database/security owner:** integrated RustSec gate flags **RUSTSEC-2026-0459 / CVE-2026-119375** in `libsqlite3-sys` (bundled SQLite); see [Tauri integration #42924945201](https://github.com/MousaXD/scriptotar/actions/runs/42924945201). Do not waive or suppress advisory checks.
4. **Installed app acceptance — Agent 08 / Agent 00:** verify narrow Windows/Linux WebViews, focus, RTL, styled dropdown opening/placement, export downloads and accessibility after control migration. Backend media playback remains explicitly out of scope; Agent 03/00 security review required for any future contract.

## Final verdict

**BLOCKED**, not merge/release-approved. Agent 06's owned UX and race fixes are committed on its original branch with expanded tests. Exact-head CI and the explicit Agent 01 Select + Agent 07 translation + Agent 08 installed-GUI dependencies remain required. Agent 00 alone merges or changes integration scope.
