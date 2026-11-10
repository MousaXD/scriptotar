# Agent 07 — Settings, MCP UX & Localization

## Identity and baseline

- Repo: `MousaXD/scriptotar`
- Agent branch: `agent-07/settings-localization-overhaul`
- Initial baseline and merge-base: `f741244634b5143602a21a554094a498a0005b11`
- Pull request: [#96](https://github.com/MousaXD/scriptotar/pull/96) (draft pending validation)
- Last implementation/test commit before handoff update: `f934841403bf8989fd05df35f4ef5bee624cdba1`
- No edits to `main` or any other agent-owned view, global CSS, Rust MCP code, API client, or shared control implementation.

## Completed

1. Settings UX:
   - Reordered Settings with Interface, Transcription, Media, Storage, Local behavior, and Watchlists as primary preferences; MCP and legacy migration under Advanced.
   - Reduced duplicated eyebrows/long paragraphs and avoided repeating migration state labels; preserved the legacy snapshot safety explanation, candidate selection, retry flow, and result details.
   - Added persistent settings baseline, explicit unsaved state and Discard changes. Save is disabled while clean/busy; the successful backend save resets dirty state even when parent props update later.
   - Snapshot settings before asynchronous save so UI edits during an in-flight save are not misrepresented as persisted. Appearance remains applied only after the backend confirms save.
2. MCP security UX:
   - Kept all permission fields, approved paths/programs, default-deny, explicit powerful-access acknowledgement, connection methods, error display, URL secrecy warning, disconnect/revocation, and connection state.
   - Kept a saved policy separate from the editable draft; normalize approved line entries before comparing. Unsaved permissions visibly block connection/step advancement, with an explicit Discard permission changes action.
   - Acknowledgement resets when scope/permissions change. Existing file-read/powerful grants automatically reveal the advanced permission and warning panel on reopen. Backend still owns authorization and sensitive confirmation; no bypass or relaxation.
   - Added failure visibility for background connection status refresh and retry for initial load failures. Existing public URL/tunnel identifiers and path input retain LTR direction.
3. Localization and RTL:
   - Added 85 typed `mcp.*` English/Arabic message pairs plus Settings Save/Discard translations. MCP wizard now uses the Svelte translator directly and opts out of fragile DOM text rewriting.
   - Preserved raw paths, URLs, API keys, ID values, backend-supplied details, and model/setting semantic values; text labels translate without modifying persisted value strings.
   - Added scoped RTL alignment for Settings/MCP and responsive layout adjustments for narrow widths.

## Tests added/extended

- `components/McpSetupWizard.test.ts`: original default-deny, authorized connect/disconnect, powerful-access approval, OpenAI key and manual HTTPS hostname cases retained. Additional tests cover unsaved permission discard/step gating, Arabic permission flow, and approval invalidation when an approved path changes.
- `views/SettingsView.test.ts`: new regression tests for unsaved draft discard, save/persistence and clean state, Arabic interface switch without persisting a different transcription language, legacy migration status and retry wiring.
- `i18n/messages.test.ts`: new key parity test for English and Arabic and coverage of MCP wizard translations.

## Validation and merge gate

- Static source and branch diff reviewed; edits constrained to the assigned Agent 07 frontend files, dedicated tests, and this handoff.
- **Not certified yet**: this environment has no local Git checkout/dependencies and cannot run Svelte check, Vitest, or Vite build. PR-triggered GitHub Actions were queued/pending when the report was written.
- Required exact-head validation: Desktop UI (`npm run check`, `npm test`, `npm run build`); required CI/security/MCP tests; manual narrow-window, keyboard, Arabic RTL, connection error/revocation, and actual migration recovery checks.
- Do **not** merge while a mandatory workflow is queued/failed, or if CI finds a regression.

## Dependencies and coordination

- **Agent 01** owns shared reusable select/slider design. This agent intentionally did not create replacement selector components or change global control CSS. Integrator should adopt Agent 01's control contract if compatible.
- Other agents needing translations should request keys for their views; this agent exclusively owns `i18n/messages.ts` and `i18n.ts` dictionaries.
- Existing DOM translation for other agents' views remains in place to avoid cross-agent conflicts; this PR replaces fragile translation specifically within the owned MCP wizard.
- Release QA should verify backend MCP authorization remains fail-closed, run packaged-app checks, and visually review RTL/keyboard behavior.
