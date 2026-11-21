# Agent 02 — durable paid-effect late-receipt recovery (2026-11-21)

**Branch:** `agent-02/p1-late-receipt-recovery-20261121` (candidate for integration PR #114; no `main` merge).  
**Safety mode:** billable egress remains hard-disabled. No paid network requests made for this change.  
**Existing foundation:** v8 local effect ledger, v10 scoped external reservation and pre-send CAS, v11 canonical three-script batch, v12 append-only provider-identity registry. Historical migrations are unchanged.

## 21 November continuation — archived restart and multi-process late receipt

The recovery entry point had a scope mismatch: a dispatched effect could be settled after archival but `recover_external_effects` rejected an archived owner during restart. The SQLite implementation now shares `verified_run_settlement_scope` between old-effect reconciliation and restart classification. It still verifies the original run, project, Study, persisted checkpoint shape and plan identity; it never calls `may_execute` for recovery. Existing active-only `may_execute` remains mandatory for new reservation / first send / manual retry.

The generic host reconciliation method now permits an explicitly requested **read-only** provider status probe for an already-dispatched archived effect. It does **not** relax first-send authorization or automatic retry. Provider dispatch and provider probe must use the exact canonical `openai` namespace, not casing/model aliases or an arbitrary vendor string; the only existing mock adapter implements the new trait contract. Future typed provider/account identifiers require independent approval.

Two new SQLite tests persist the real DB file across opens and verify (1) dispatched -> uncertain recovery after both project and Study archival, successful late charge settlement and no second send permit; (2) independently opened SQLite connections racing recovery classification against final receipt accounting always end at one completed, immutable charge. Two host tests verify archived-owner probe settlement without redispatch and reject provider/model namespace aliases before any reservation.

**Important process-ownership distinction:** the recovery classification is not a lease, and another application process may still be actively sending/awaiting a reply. The transition only blocks further dispatch and safely allows an authenticated original-attempt receipt to win the SQLite transaction. Do **not** automatically start a second probe, call `dispatch_once`, prepare a retry or interpret a missing response as nonexecution at application startup. A true cross-process dispatch/mutation linearization still requires Agent 01/00 design and Agent 10 adversarial certification.

These tests are committed but **not certified** until the exact final head has passing Rustfmt, Rust host/DB, Clippy and security evidence; prior green workflows do not transfer.


## Proven integration gap fixed

The v10 DB reconciliation API accepts authenticated final receipts while an effect is `Dispatched` **or** `Uncertain`, including after recovery. However, the host `SqlitePaidEffectFence::settle` previously required `Dispatched` and unconditionally rejected a verified delayed completion after `mark_uncertain` / reopen. This prevented the host from recording a real charge/receipt even when trustworthy provider usage became available later.

The host now permits *settlement only* for an `Uncertain` first attempt, requires exact persisted wire SHA-256, provider, immutable ceilings and authenticated provider request/receipt identity, and recognizes identical `Completed` settlement callbacks idempotently (without a revision bump). A different receipt, request identity, usage or cost cannot replace a terminal completion. The older v10/v12 SQLite transaction and tombstone triggers remain the ultimate authority.

## Recovery transitions / no-send contract

| Durable state | Allowed action | Forbidden action |
| --- | --- | --- |
| No reservation | With fresh consent + approved running step: atomic reserve | External I/O before durable reservation |
| Reserved, before pre-send CAS | Committed `Reserved -> Dispatched` CAS; or cancel without sending | Sending without winning the CAS; automatic replay after a restarted reservation |
| Dispatched, before or after socket I/O | Persist accepted provider ID or verified final receipt; otherwise classify `Uncertain` | Dispatching again (including after crash or timeout) |
| Uncertain (lost ACK, disconnect, crash, timeout) | Provider status lookup, authenticated late receipt -> `Completed`/`Failed`, or authoritative nonexecution proof | Local automatic retries, treating 404/timeout as proof, reusing a changed wire hash |
| Completed/Failed | Read immutable receipt and charges; accept only an exact idempotent completed callback in host settlement | Charging again, changing receipt/usage, re-dispatching |
| Verified NotExecuted | Fresh separate approval may prepare capped second/third attempt | Retry without authoritative nonexecution evidence, or after cancellation |
| Cancelled before dispatch | No send, v8 `not_applied` fence | Reopening claim for egress |
| Cancel requested after dispatch | Record intent; trusted provider proof may still settle a charge | Assuming local cancellation is provider cancellation |

The pre-send CAS intentionally sacrifices availability in ambiguous windows in order to prevent local double-dispatch. `Reserved` after restart is **not** proof that no I/O could ever have been initiated by another process unless the exact dispatch discipline and recovery ordering are independently certified. Where provider status cannot be authoritatively queried, keep `Uncertain` blocked.

## New focused host regression

`authenticated_late_receipt_recovers_uncertain_without_another_send` in `agent_effect_driver.rs` exercises the actual `SqlitePaidEffectFence` with an approved persisted `RunSnapshot` and real SQLite store:

1. Reserve and mark the approved exact effect dispatched.
2. Simulate lost response with durable `Uncertain`; confirm redispatch is denied.
3. Attempt late settlement with the correct effect key but a wrong wire SHA; verify denial and unchanged SQLite revision.
4. Settle authenticated matching late usage and receipt; verify `Completed`, token/cost and attempt ID.
5. Reopen the database and deliver the identical receipt; verify no revision or receipt mutation.
6. Submit a contradictory receipt and attempt to dispatch again; verify both are rejected.

Two additional no-network host/SQLite regressions verify cancellation ordering: (a) cancelling a mere reservation yields a terminal `Cancelled` and v8 `NotApplied` claim, and prevents both a dispatch CAS and a forged late receipt, including after reopen; (b) cancelling after the committed dispatch CAS records cancellation intent and retains a blocked uncertain claim, but a matching authenticated late receipt can correctly record a charge and advance v8 `Unknown -> Applied` without sending again.

A fourth adversarial test, `late_receipt_sqlite_fault_rolls_back_identity_charge_and_legacy_effect`, injects a SQLite trigger that aborts the v8 `Applied` transition after the external receipt update has begun. It asserts the entire IMMEDIATE transaction rolls back: the v12 identity tombstone is absent, v10 remains `Uncertain`, v8 remains `Unknown`, token/cost totals stay zero and the revision is unchanged. After the injected fault is removed and SQLite reopened, one trusted late receipt can settle without dispatching another provider request.

This is a deterministic, no-network host/SQLite regression suite. Existing v10–v12 tests separately cover simultaneous claims, cross-run/cross-project identity, historical upgrade rollback, forged/duplicate provider IDs, retry attempt tombstones and corrupted checkpoints. Their existing passing evidence does **not** certify this new head.

## Archive-safe accounting without archive-safe dispatch

The original `verified_scope` and `may_execute` checks require an active, unarchived Study/project for every new reservation and dispatch. This is correct for authorization but was also being reused by `get_external_effect`, receipt acknowledgement and final reconciliation. If a user archived the source while a provider request was already in flight, those APIs could refuse the only trustworthy late receipt and lose the local charge record.

The new `verified_settlement_scope` validates that the persisted `agent_runs` row, run plan, project, Study and checkpoint structure all agree, without requiring that the historical owner remain unarchived. It is used only for looking up and resolving **existing** effects. The pre-send `may_execute` path still uses the original active-scope check. No new migrations, provider calls or replay permits are introduced.

Host/SQLite regressions now prove that a completed original attempt remains reconcilable after both its Study and project are archived; a merely reserved effect cannot obtain a dispatch CAS after archival; and guessed foreign project/Study identifiers cannot read the archived effect. The historical v8 public effect lookup remains active-scope-only.

**Remaining host integration caveat:** the private host reconciliation API can now use the existing project/Study-scoped ledger after archival, but Agent 00 still owns any operator-visible reconciliation command, credential/session authorization, provider-account confirmation and startup UX. It must remain a trusted status-only probe, not a user-configurable endpoint or new paid request.

## Canonical sealed provider identity

The current production OpenAI wire candidate always identifies its provider as `openai`. The private `SqlitePaidEffectFence::from_approved_context` now rejects any other provider string, including `OpenAI`, `openai:model-alias` and arbitrary vendor labels, because the v12 provider receipt-identity registry is partitioned by **provider** rather than model. A model suffix could otherwise allow the same OpenAI-issued request/receipt ID to be claimed in a separate namespace. The existing regression fixtures use the canonical provider key and assert that model-suffixed aliases are denied.

This is deliberately narrow. The generic `ExternalEffectProvider` host seam now requires its declared canonical provider namespace to match the existing effect and currently admits only `openai`; it is **not** a certified production egress path. Future independently reviewed provider adapters need typed provider/account-aware namespaces with Agent 00/01/10 sign-off. No schema migration, public provider command, credential storage or unapproved transport was enabled.

## Remaining blockers / Agent 00 and 10 handoff

- **Agent 00:** run Rustfmt, Clippy and host/DB suites on this exact branch SHA, then integrate the narrowly reviewed host changes into #114. Ensure the actual shipped provider command still returns the hard-disabled error; do not enable paid egress.
- **Agent 01:** audit atomic source mutation/revocation/cancellation ordering **across processes** between last authorization, durable CAS and actual network send. A process-local mutex is not a cross-process guarantee.
- **Agent 05:** the remaining **paid receipt -> canonical three-script persistence crash window is unresolved**. The v10/v12 effect and v11 three-draft sink commit separately. Design a sealed, durable response-output handoff or a reviewed atomic consume/commit transaction so a recorded charge can be recovered without another paid provider call. Until then keep missing drafts visibly blocked for manual reconciliation; never manufacture output or re-send to repair it. Verify duplicate delivery returns the original draft IDs without new revisions or charges.
- **Agent 10:** independently execute new archived-restart + separate SQLite-connection receipt races, simultaneous completion/cancel/restart adversarial runs, and exact-SHA original tests; verify duplicate success matches the same provider ID, receipt and metered totals. Check the installed-app negative paths.
- **Provider integration:** real provider-side status/receipt authenticity, pricing catalog, bounded usage, correlation from HTTP response to saved identity, and no-automatic-retry wire behavior still require independent verification. A local SQLite ledger cannot alone establish exactly-once **remote billing**.
- **Schema:** v10–v12 migrations are unmodified. Any future metadata/schema extension requires Agent 00 migration-number approval.

**Verdict:** focused host recovery consistency fix, **not independent no-double-charge certification or release authorization**.
