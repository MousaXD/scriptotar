# Agent 06 — Transcript & AI Studio UX handoff

**Repository:** MousaXD/scriptotar  
**Assigned branch:** `agent-06/transcript-ai-workspace-overhaul`  
**Baseline:** `f741244634b5143602a21a554094a498a0005b11`  
**PR:** https://github.com/MousaXD/scriptotar/pull/93  
**Coordinator:** Agent 00  
**State:** Draft / implementation prepared; exact-head CI and installed-WebView verification outstanding

## Scope and implemented changes

Only `TranscriptView.svelte`, `AiStudioView.svelte`, the dedicated UI test file and this handoff were changed.

**Transcript workspace**
- Removed redundant heading subtext/rail chrome; made transcript title, body text and timestamps easier to read.
- Preserved list/details toggles and their local UI state, search previous/next controls, copy, timestamp navigation and every export format (TXT, timestamped TXT, SRT, VTT, JSON).
- Fetches exact selected transcript; rejects returned IDs or project IDs that mismatch the selected summary.
- Ignores older asynchronous fetch results and resets selected load state when transcript lists become empty.
- Renders the first 140 segments, then extends in 140-segment batches on scroll or explicit show-more action to bound initial DOM work. Search filters the entire selected transcript and can find matches beyond the first batch.
- Shows a small skeleton while the selected transcript is loading. Keeps source-language RTL/LTR direction on reader content; does not invent playback.

**AI Studio**
- Simplified Copy Prompt/BYOK mode switch with pressed-state semantics, removed repetitive explanatory chrome and concealed provider/model settings when not in BYOK mode.
- Keeps manual-first exact transcript choice and visible source ID/lineage; clears a removed transcript instead of leaving stale text ready for a request.
- Provider requests are generation-guarded. Input changes invalidate old runs, immediately unlock the UI for a newer request, and prevent older responses or finalizers from corrupting newer state.
- Mode changes also invalidate pending prompt builds to prevent the wrong-mode prompt surfacing.
- Redacts submitted keys, bearer header values and common secret-bearing URL parameters from surfaced error messages.
- Improved textarea/result legibility, source-loading skeleton and direction detection for Arabic/English typed content.
- Copy Prompt continues to build locally without forwarding API keys/base URLs; BYOK remains an explicit action, credentials remain transient frontend state.

## Dedicated regression coverage

New: `apps/desktop-ui/src/views/TranscriptAiWorkspace.test.ts`

- Long transcript initial bounded rendering; expansion and late search matches.
- Source switch while an older transcript response remains pending.
- Rejection of cross-project transcript responses.
- Arabic transcript direction and timestamp controls.
- A second BYOK request remaining busy when the older request completes.

Existing `ai-source-selection.test.ts`, `gui-overhaul.test.ts`, `App.test.ts` and other frontend tests must also pass. All formats/export flows are preserved in the implementation; browser download integration has **not** been independently exercised by the new tests.

## Validation status and acceptance gates

- GitHub PR creation and isolated branch commits: completed.
- `npm run check`, `npm run test`, `npm run build`: **not locally executed** because the available workspace has no checkout and cannot resolve GitHub/npm hosts. GitHub Actions is the planned authoritative exact-head gate; older runs cancelled by superseding commits are not evidence of a pass.
- Real installed Windows/Linux WebView interaction, narrow screens, Arabic RTL, assistive technology, export downloads and actual provider calls: **not yet certified**. Provider calls require deliberate authorized test credentials; fixtures are not live-provider validation.
- Long-transcript progressive rendering bounds initial DOM but is **not true constant-memory virtualization**. Jumping to distant search matches can mount up to that index. Measure with real large transcripts before release.
- Error redaction is defensive UI hygiene, not a replacement for the host security review.

## Ownership, dependencies and safety

- **Agent 01:** swap to shared controls/design tokens once the design system is available; do not edit global CSS here.
- **Agent 03 / Agent 00:** approve any future backend media playback, source/AI contract or credential handling changes. None are introduced here.
- **Agent 07:** add Arabic/English dictionary strings for the new “Show more” control and review AI Studio's existing untranslated static copy.
- No backend APIs, shared translations, auth, provider transport, schema, security code, main branch or production data changed.
- Agent 00 is the only merge authority. Do not merge without passing exact-head UI checks, review and integrated-app acceptance.

## Follow-up

1. Record exact-head Desktop UI Svelte/TypeScript checks, Vitest and Vite build outcomes; fix any regression on this branch.
2. Exercise all exports, copy/keyboard flows, Arabic/English and narrow installed WebViews.
3. Review combined UI with Agent 01's design-system changes and Agent 07 localization without editing their owned files.
4. Update this handoff with verified final HEAD and CI references before requesting merge.
