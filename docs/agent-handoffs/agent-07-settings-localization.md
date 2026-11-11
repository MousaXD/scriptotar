# Agent 07 — Settings, MCP UX & Localization

## Identity and baseline

- Repo: `MousaXD/scriptotar`
- Agent branch: `agent-07/settings-localization-overhaul`
- Initial baseline and merge-base: `f741244634b5143602a21a554094a498a0005b11`
- Pull request: [#96](https://github.com/MousaXD/scriptotar/pull/96) (draft pending validation)
- Last implementation/test commit audited: `c929cd88c90554e1ef14a904f768f63329e16bf3`
- No edits to `main` or any other agent-owned view, global CSS, Rust MCP code, API client, or shared control implementation.

## Completed

1. Settings UX:
   - Reordered Settings with Interface, Transcription, Media, Storage, Local behavior, and Watchlists as primary preferences; MCP and legacy migration under Advanced.
   - Reduced duplicated eyebrows/long paragraphs, simplified the wizard heading and increased small Settings/MCP control text for readability, and avoided repeating migration state labels; preserved the legacy snapshot safety explanation, candidate selection, retry flow, and result details.
   - Added persistent settings baseline, explicit unsaved state and Discard changes. Save is disabled while clean/busy; the successful backend save resets dirty state even when parent props update later.
   - Snapshot settings before asynchronous save so UI edits during an in-flight save are not misrepresented as persisted. Appearance remains applied only after the backend confirms save.
2. MCP security UX:
   - Kept all permission fields, approved paths/programs, default-deny, explicit powerful-access acknowledgement, connection methods, error display, URL secrecy warning, disconnect/revocation, and connection state.
   - Kept a saved policy separate from the editable draft; normalize approved line entries before comparing. Unsaved permissions visibly block connection/step advancement, with an explicit Discard permission changes action.
   - Acknowledgement resets when scope/permissions change. Existing file-read/powerful grants and pre-existing approved paths/programs automatically reveal the advanced permission and warning panel on reopen. Backend still owns authorization and sensitive confirmation; no bypass or relaxation.
   - Added failure visibility for background connection status refresh and retry for initial load failures. Existing public URL/tunnel identifiers and path input retain LTR direction.
3. Localization and RTL:
   - Added explicit typed `mcp.*` English/Arabic message pairs plus Settings Save/Discard translations; removed duplicate `mcp.setup` definitions found by CI. MCP wizard now uses the Svelte translator directly and opts out of fragile DOM text rewriting.
   - Preserved raw paths, URLs, API keys, ID values, backend-supplied details, and model/setting semantic values; text labels translate without modifying persisted value strings.
   - Added scoped RTL alignment for Settings/MCP and responsive layout adjustments for narrow widths. Local success/copy/status notices now store message states rather than translated strings, so switching languages updates visible notices immediately.

## Tests added/extended

- `components/McpSetupWizard.test.ts`: original default-deny, authorized connect/disconnect, powerful-access approval, OpenAI key and manual HTTPS hostname cases retained. Additional tests cover unsaved permission discard/step gating, Arabic permission flow and live notices, approval invalidation when an approved path changes, visibility of previously approved paths, LTR handling in RTL UI, and keyboard step focus.
- `views/SettingsView.test.ts`: new regression tests for unsaved draft discard, save/persistence and clean state, Arabic interface switch without persisting a different transcription language, legacy migration status and retry wiring, including live language switching for saved notices.
- `i18n/messages.test.ts`: new key parity and placeholder-parity tests for English and Arabic, plus coverage of MCP wizard translations.

## Independent audit findings and corrections

- Exact previous HEAD `f5a60768225b090ed3366c83be6e8fa7e5d0c81b` failed frontend checks due to duplicate `mcp.setup` keys in both language objects. Removed duplicate keys at `556d8afc21791ee4b011e66f721b0ba8be73b562`; subsequent Desktop UI workflow [#42985109055](https://github.com/MousaXD/scriptotar/actions/runs/42985109055) passed Svelte check (0 errors, 0 warnings), Vitest (14 files, 88 tests), and Vite build.
- Audited Settings save snapshot, discard, appearance application, output folder, migration actions, Reactivity of permission drafts, powerful-access acknowledgement, connection/revocation, secret URL display, status/error recovery, keyboard tab focus, RTL paths and layout.
- Fixed stale translated notices on runtime language switch; preserved raw backend errors and user data. Exposed saved path/program allowlists even when file access toggles are off. Added focused regression tests and checked dictionary key parity and matching interpolation parameters.
- Existing settings and permission values/host API signatures remain unchanged. All branch diffs are restricted to Agent 07-owned Svelte views, wizard, i18n dictionaries/CSS, dedicated tests, and this handoff.
- Existing DOM text translation outside the owned wizard is deliberately retained for other agents to address; no cross-agent view modifications.
- Local environment working directory `/` has no Scriptotar Git checkout; GitHub remote branch/commit and PR verified directly. Therefore no local npm run was attempted; validation is through GitHub Actions.

## Validation and merge gate

- **Verified pass at code SHA `556d8afc21791ee4b011e66f721b0ba8be73b562`**: Desktop UI `npm run check` (0 errors, 0 warnings), `npm run test` (88/88 tests, 14/14 test files), and `npm run build`; Security hygiene success. This check preceded later targeted bug-fix tests and is not evidence for the final branch HEAD.
- **Previously audited implementation SHA `33edcfa4acf1ddeebb70388c7b64d1cec4a7b002`**: submitted with additional UI/a11y/i18n regression tests; exact-HEAD GitHub Actions checks were queued when this report was updated. Verify checks again after this documentation commit.
- Previous HEAD `f5a60768…`: Windows installer and Linux package checks passed. MCP E2E Rust/HTTP/stdio stages and Tauri migration Rust-side integration passed before the same frontend type error interrupted the runs; no claim of full workflow success.
- **External release blocker (outside Agent 07 ownership)**: Tauri migration integration Supply chain `cargo audit` flags `libsqlite3-sys 0.30.1`, [RUSTSEC-2026-0459](https://rustsec.org/advisories/RUSTSEC-2026-0459), `CVE-2026-119375` (medium 6.8); advisory recommends `>=0.39.0`. Rust dependencies/`Cargo.lock` are owned elsewhere. Do not suppress or ignore this advisory to turn the check green; Agent 00 should assign a compatible, audited dependency update and full regression test.
- Manual packaged-app/visual gates not proven by jsdom: keyboard/screen-reader review, narrow-window visual layout, real Arabic RTL, and actual migration recovery. Native HTML controls and responsive CSS were reviewed statically and unit-tested where practical.
- **Final readiness verdict: BLOCKED** until all required exact-HEAD workflows pass, the Rust supply-chain advisory is resolved by its owner, and independent UI/RTL manual certification is complete. Do not merge this draft PR before then.

## Dependencies and coordination

- **Agent 01** owns shared reusable select/slider design. This agent intentionally did not create replacement selector components or change global control CSS. Integrator should adopt Agent 01's control contract if compatible.
- Other agents needing translations should request keys for their views; this agent exclusively owns `i18n/messages.ts` and `i18n.ts` dictionaries.
- Existing DOM translation for other agents' views remains in place to avoid cross-agent conflicts; this PR replaces fragile translation specifically within the owned MCP wizard.
- Release QA should verify backend MCP authorization remains fail-closed, run packaged-app checks, and visually review RTL/keyboard behavior.

## Final remediation and certification — 2026-11-11

- **Final production/test implementation commit:** `c929cd88c90554e1ef14a904f768f63329e16bf3` on existing PR #96; the subsequent handoff-only commit does not change application or test code. Obtain the documentation commit's exact SHA from PR #96 head.
- **Failure diagnosis:** [Desktop UI #42986151972](https://github.com/MousaXD/scriptotar/actions/runs/42986151972) showed 90/91 passing and an English success-notice assertion evaluated synchronously after switching from Arabic. The UI was correctly rendering Arabic, so the test now awaits the reactive English notice instead of weakening Arabic assertions.
- **Warning diagnosis:** Removed the unused `.wizard-head p` CSS selector in `McpSetupWizard.svelte`; [Desktop UI #43017156814](https://github.com/MousaXD/scriptotar/actions/runs/43017156814) confirms **0 Svelte errors, 0 Svelte warnings**, **14/14 files, 93/93 tests**, and a successful production build at `c929cd88c90554e1ef14a904f768f63329e16bf3`.
- **Regression found and fixed:** MCP policy saves formerly replaced the entire editable policy when an asynchronous backend response returned. The wizard now snapshots the requested policy, records the persisted response, preserves any newer unsaved edits, keeps the workflow on the permission step, clears sensitive approval, and blocks connection until those newer changes are saved. Added a controlled pending-save test.
- **Settings persistence regression test:** Validates that editing a second setting while Save is in progress leaves it visibly unsaved and preserves the original value in the submitted snapshot.
- **Accessibility:** MCP setup steps now expose an explicitly named `role="group"`, with keyboard focus and `aria-current` test coverage. Original permission-gating, LTR identifiers and paths, label retranslation, RTL, copy/revocation, error states, legacy migration controls and dictionary key/placeholder parity remain covered or statically reviewed.
- **Exact-code-head CI:** [Desktop UI](https://github.com/MousaXD/scriptotar/actions/runs/43017156814) **PASS**; Security hygiene workflow **PASS**. [MCP E2E](https://github.com/MousaXD/scriptotar/actions/runs/43017157019), [CI](https://github.com/MousaXD/scriptotar/actions/runs/43017157336), [Tauri migration](https://github.com/MousaXD/scriptotar/actions/runs/43017156802), Linux package and Windows installer were still in progress at this handoff update. They must be inspected at the ultimate PR HEAD.
- **External blocks:** Issue #97 / `RUSTSEC-2026-0459` / `CVE-2026-119375` is owned by the Rust security agent; no dependencies were changed and no advisory ignored or suppressed. Independently validate a packaged app's Arabic RTL, narrow screen layouts, assistive technology, and real migration recovery before release. Agent 00 handles shared Select/Switch integration and any cross-agent dictionary consolidation.
- **Scoped verdict:** **GO for Agent 07 frontend remediation at code commit `c929cd88c90554e1ef14a904f768f63329e16bf3`**, subject to exact-final-HEAD and integration checks. **BLOCKED for merge/release** until the unrelated supply-chain advisory and remaining integration/manual acceptance gates are independently cleared. PR stays draft. No merge or direct `main` edits.
