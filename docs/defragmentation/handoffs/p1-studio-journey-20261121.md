# Agent 06 — Studio journey parity and native acceptance handoff (2026-11-21)

**Audited integration:** `integration/scriptotar-defragmentation-agent00-20261113` @ `b12843c75764601248a69c0dd89ff753af8df366`  
**Focused follow-up:** `agent-06/p1-studio-journey-guard-20261121` (PR draft against integration)  
**Product owners:** Agent 00 integration, Agent 01 consent, Agent 02 paid effect durability, Agent 03 research, Agent 04 fact checking, Agent 05 script generation, Agent 07 installed native QA.

This is a code-level parity ledger, **not** certification of installed desktop behavior. The UI must continue to distinguish local offline research, Copy Prompt, user-imported AI material and a genuinely approved provider run. Do not enable a paid generation CTA without the authoritative production backend.

## End-to-end feature parity ledger

| User step | Current product-level state | Evidence and remaining work |
| --- | --- | --- |
| 1. Create/select project | Integrated; guarded app-level project transition | `App.svelte` calls Studio `flushPendingSave`; confirm installed project isolation |
| 2. Attach >=3 sources | UI allows multiple saved research/transcript references | Study source persistence and provenance; confirm 3+ real accessible sources in installed test |
| 3. Launch bounded research with consent | Integrated local, zero-paid-egress research run | `list/start/decide/read_study_research_*` Tauri commands; approval, denial and cancel |
| 4. Review evidence/uncertainty | Integrated source observations and user dispute | This PR validates read-only returned report source identity, collected snapshot and observation lineage; human dispute is not independent verification |
| 5. Write original brief | Integrated editable CreativeBrief with autosave/CAS | Verify localized input and unsaved edit preservation across project/Study switching |
| 6. Choose provider/model | Preview-only UI seam | The optional `StudioAgentRunGateway` is **not** production Tauri IPC; never confuse with real consent/dispatch |
| 7–8. Preview exact egress, cost, approve/cancel | Blocked for paid generation | Agent 01/02/00 must wire authenticated exact prompt/source scope, budget, single dispatch and durable receipts before CTA |
| 9. Three original generated drafts | Canonical v11 persistence exists, production generation is blocked | Agent 05/00 must expose actual authorized 3-candidate run, real source/claim metadata and truthful native read |
| 10. Compare candidates | Integrated saved-body 3-column/selection view | This PR checks exact candidate *and Study* revision and refuses mixed-revision read results |
| 11. Inspect references/claim warnings | Partial | Existing unverified/disputed evidence + pasted-model review counts. Agent 04/05/00 must bind saved DraftVersion fingerprint and verified source locators |
| 12. Edit/revise/restore/save | Integrated versioned editor, manual candidate operations | Test persistence with exact IDs, including dirty editor switch and archived drafts; model-targeted revision still blocked |
| 13–14. Quit/relaunch and reopen three drafts | Native base scenarios run previously; full three-script journey not certified | Latest merged QA harness #126 has some installed proof, not full 3-source/provider/candidate restart proof on this branch |
| 15. Export selected script | Existing TXT and Markdown, plus receipt-linked 3-script Markdown | Export must read exact persisted candidates; frontend receipt/mixed-revision negative tests exist |

## Focused changes in this PR

- Add `researchReportGuard.ts`: the Studio render boundary rejects reports that do not match the completed, source-scoped run; detached, revoked, replaced or wrong-collected source; duplicate or foreign source; or observation evidence linked to another origin. Rust remains authoritative.
- Refuse mismatched source-ID selections returned by the prepare/approve flow before displaying consent or a report; disable approval for no-longer-current sources and clear stale approval controls before a potentially failing refresh.
- Present the exact saved source IDs/locators before local AnalyzeEvidence approval; reject a nonzero-cost research plan and disable preparation after a failed source-list refresh. The mock now lists durable runs newest-first, matching production SQLite.
- Freeze Study brief, source picker, script editor, import and export while an incoming Study read is unresolved, with an English/Arabic accessible skeleton. Temporarily unmount the old local research and candidate-comparison controls; restore editing after the new Study loads or the read fails.
- Add deferred Study-switch test proving the outgoing draft and source links are preserved; add latest-run reopening, local cancellation, zero-cost authorization, Arabic scope and failure recovery tests.
- Show actual English/Arabic labels for native research run states and source kinds; declare RTL direction and accessible `aria-busy`/loading skeleton.
- Reject mixed Study revisions when opening a receipt-linked candidate comparison.
- Add isolated provenance tests and integrated local research/three-candidate negative tests; preserve existing Copy Prompt/history/editor workflows.

## Backend and installed-app blockers

1. **Agent 01/02/00:** exact trusted provider authorization, egress disclosure, tokens/cost ceiling, pre-send CAS, uncertain-effect reconciliation, no hidden replay, and production Tauri run contract. The optional `StudioAgentRunGateway` is not wired to Rust. Keep the paid execution UI disabled.
2. **Agent 05/00:** real three-angle/three-script orchestration from authorized immutable research into v11 canonical saved drafts and real source-linked claim lineage. Manual JSON validation is not an autonomous generation success.
3. **Agent 04/00:** show and navigate source/segment evidence and warnings bound to the exact saved draft fingerprint. Human disputes are not independent fact verification.
4. **Agent 07/00:** rerun on final exact integration SHA: Linux installed WebKitGTK and Windows installed WebView2, 3+ real sources, approved local research/denial/cancel, source revoke and forged IPC rejection, Arabic RTL keyboard/screen reader, candidate comparison, dirty saves, real quit/relaunch, all three drafts, and TXT/MD export. Do not count simulated browser success as native proof.

**Previous independent native baseline:** Agent 07's merged #126 / run 44196667168 reports 10 executed passes, 0 failures and 6 **NOT TESTED** on each of Linux and Windows. The six missing cases include foreign-project forged IPC, multi-source transcript, revocation, error recovery, Arabic RTL verification and independent theme inspection. This is *not* a full passing gate or evidence for the present new SHA.

## Validation checklist — record actual results, not intentions

- [ ] `cd apps/desktop-ui && npm ci && npm run check && npm test -- --run && npm run build` on this exact branch SHA.
- [ ] Rust command/API inventory and `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets`, `cargo test --workspace --locked` on the final integration SHA.
- [ ] Local research source-scope and foreign observation negative tests pass.
- [ ] Candidate comparison rejects foreign IDs and wrong Study/candidate revisions, with no provider side effects.
- [ ] EN/AR controls, RTL direction, focus and loading/error announcements reviewed in *installed* WebView.
- [ ] Project/Study switching preserves unsaved edits; restart restores all saved drafts; exact selected content exported.
- [ ] Agent 07 publishes independent Linux/Windows screenshots, automation artifacts and test logs at one exact integrated SHA.
- [ ] Agent 00 reviews consent/effect/provider and closes all release gates before considering a main merge.

**No backend reimplementation, no production paid egress, no main merge, no release authorized by this PR.**

## 2026-11-21 continuation — asynchronous reopen and same-Study revocation

- **Desktop UI original failures:** exact-head  `743ade13f26e8ea6791e9f2bc538262b61cd9393` failed two tests reopening saved Studies. Both clicked the **disabled** `Open study` picker while its asynchronous index was still loading. Test commit `29179e9ffc71e656e459a6810d083636b4507b18` now waits for the enabled state and accessible saved option, without weakening the busy-state guard or replacing keyboard semantics.
- **Saved-revision boundary:** commit `04e9dc28ac236307b834a1936199771602777370` keys the local research panel by project, Study and **persisted Study revision**, instead of only project/Study. A newly saved source change or revocation discards the old report and approval UI, reloads canonical attachments/run state, and lets the backend reject stale plans.
- **Negative regression:** commit `33ba80515c107e8bbbbdb9afed85c7247291498f` extends Studio source-revocation acceptance: an awaiting-approval local research plan is visible before revocation; after the revision changes its previous approval is disabled, sources have been refetched, and no research output or paid run was produced.
- **Boundary:** a change made by another process while this Study remains open requires an explicit refresh/reopen or future native invalidation event; a frontend display guard cannot replace host-side current-source authorization.
- **Agent 07 handoff delta:** on installed Windows WebView2 and Linux WebKitGTK, start a locally approved pending analysis, revoke an attached source via Studio, verify that the old approval cannot be clicked and no stale report is shown; then quit/reopen the Study and verify the denial persists. Repeat after a normal saved brief edit to check refreshing does not drop draft bytes. The broader 15-step journey still requires a certified paid backend to test any provider generation.
- **Validation:** local Cargo and npm execution unavailable in the connector-only environment. Every test change is committed, but **exact-head GitHub Actions and independent installed-app results must be read from the current head, not inferred from earlier runs**. Until those pass, the PR is draft and the merge remains NO-GO.

## Same-Study unsaved-edit guard — 2026-11-21

- After a local brief/source/draft edit, Studio's local research panel now receives `pendingEdits` from the editor. Until the changed Study has been saved, **Prepare**, **Approve** and **Reconcile** are disabled at both the UI and handler levels. A previously loaded report is hidden, not promoted as current evidence. Deny and cancel remain available for safe interruption.
- After the new authoritative Study revision is saved, the panel remounts on that exact revision and re-reads canonical source availability and the durable local-run state. Agent 03's source-content binding and host approval CAS remain authoritative.
- EN/AR save-before-approval guidance is visible while pending; the existing Copy Prompt path is unchanged.
- New component and whole-Studio tests cover state flip during a pending local approval, zero output before authorization, and hiding an already completed report while the visible Study is dirty. Relevant commits: `3750297f71f4fdf4b2ec9e6db4c8dcd0e3824d1b`, `3106df03f2c33d81eabc842b06b05fbff33d810d`, `483555757f1d75aa9cc0b5184851f4bddd4a4609`, `4484d0f15e7a9e7af411fbf269b0302f7239b772`.
- Agent 07 should additionally create an awaiting-approval local research run, edit its brief/source selection without saving, and verify that a keyboard or accessibility activation cannot start the old plan. Repeat after saving/reopening and with Arabic RTL. This guard is an extra UI protection; external database mutations still require backend revalidation.
- **No exact-head pass claimed:** frontend and native GitHub Actions are automatically superseded by subsequent commits; require the final head's original checks and installed-app evidence. Source-scope locks on Linux/Windows packaging must not be bypassed without Agent 10's exact-hash source certification.

## Agent 06 continuation — final frontend defensive scope (2026-11-21)

- **Verified original checks:** exact PR #194 head `6835c8727b8f06c44a9a7926420b8113a244c4f7` completed **six successful non-packaging workflows**: Desktop UI, CI, Scriptotar MCP E2E, Tauri migration integration, Security hygiene and GUI Visual/Performance QA. The Windows installer and Linux package workflows fail at intentional independent-source review gates, before package acceptance. This is **not** native installed certification, and not a green final release gate.
- **New unverified head:** commits after that checkpoint further reject malformed/null source and research-run lists, enforce unique canonical source IDs, require persisted run-step IDs to match their authorized plan-step IDs, and refuse malformed report/attachment inputs without rendering approval. Added component and guard regressions for duplicate/null source entries, malformed runs and mismatched durable step IDs. Exact-head CI must be checked again after these commits.
- **Earlier frontend hardening:** local reports now enforce native-compatible 96 KB JSON budget, 160 Unicode scalar excerpt limit and bounded observations before rendering, with negative tests. English/Arabic research report accessibility has a localized landmark and explicit RTL. Candidate comparison tracks the entire saved/archived draft summary set and source scope (including the fourth receipt-linked candidate), not only three summaries; stale comparisons are discarded. These are display/UX guards, **not** source-authenticity certification.
- **Agent 03 contract:** canonical `study_sources.id` identifies selection; an attached source is not necessarily independently attributable writing evidence. Three-source writing requires **three distinct original transcript origins with Attributed excerpts**, confirmed by canonical research-to-writing context. Do not claim three authentic writing citations based on a three-row attachment count alone.
- **Native acceptance delta:** verify malformed source/run failure is safe and noninteractive, Arabic report landmark/focus, dirty Study approval disabled, source revocation during awaiting approval, and stale fourth-candidate comparison after revision/restore on both installed Windows WebView2 and Linux WebKitGTK. These remain **NOT TESTED** on the changed branch.
- **Ownership:** Agent 06 modifies only desktop UI/test/handoff. Agent 00 owns safe integration; Agent 10 independently certifies source stage and paid boundary; Agents 01/02/03/04/05 own host authorization, durable effects, source-derived research, immutable draft verification and authentic generated candidates. Keep PR #194 draft, paid dispatch off and `main` untouched.


## Agent 06 continuation — authenticated local report display, research-to-writing handoff

- The completed-report display now checks **both** the selected canonical sources and a completed, approved, zero-cost persisted research step with a nonempty saved `artifact_id`. A forged completed status without its durable approved output cannot render source-backed findings.
- The display admission now requires observation locators to match the stored canonical StudySource locator, and segment IDs/timecodes to have valid shapes (safe integer milliseconds). Source-decorated/fabricated locators are not evidence. The authoritative Rust extractor uses `source.locator.clone()`; this UI check intentionally matches that contract.
- After an authorized `readStudyResearchReport` succeeds, Studio displays the exact saved artifact ID, with an explicit EN/AR warning that it is **not provider authorization**. It does not construct or persist any model-ready writing context, grant paid consent, or register a new IPC.
- Added guard regressions for false approval, missing saved artifact, mismatched step, nonzero local token estimate, forged locator, malformed segment and fractional time. The full research-review fixture checks artifact visibility after approval.
- **Handoff to Agents 03/05/00:** expose a host-authenticated writing-plan API that accepts only project/Study identity plus the selected approved *research run/artifact handle*. On the host, load the original report via the existing authenticated `research_writing_context` service, independently revalidate the run/Study/source snapshot and derive evidence internally. Do not accept a client-built evidence list, arbitrary report JSON or client-generated research fingerprint as authority. Return a read-only exact model/request/data/budget preview, and still refuse paid dispatch until Agent 01/02/10 gates are independently certified.
- **Agent 07 installed handoff:** reopen completed research on installed Linux/Windows and confirm the saved artifact ID is stable; reject an unapproved/completed spoof, forged observation locator and changed source, including on Arabic RTL. Preserve actual host errors without showing stale material.
- Browser/UI tests and GitHub workflows must be checked on the **last commit**, not a previously successful head. The production research-to-writing command, paid provider execution, native acceptance and main merge remain **NO-GO**.
