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
