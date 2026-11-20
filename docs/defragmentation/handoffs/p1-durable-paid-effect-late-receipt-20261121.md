# Agent 02 — durable paid-effect late-receipt recovery (2026-11-21)

**Branch:** `agent-02/p1-late-receipt-recovery-20261121` (candidate for integration PR #114; no `main` merge).  
**Safety mode:** billable egress remains hard-disabled. No paid network requests made for this change.  
**Existing foundation:** v8 local effect ledger, v10 scoped external reservation and pre-send CAS, v11 canonical three-script batch, v12 append-only provider-identity registry. Historical migrations are unchanged.

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

## Remaining blockers / Agent 00 and 10 handoff

- **Agent 00:** run Rustfmt, Clippy and host/DB suites on this exact branch SHA, then integrate the narrowly reviewed host changes into #114. Ensure the actual shipped provider command still returns the hard-disabled error; do not enable paid egress.
- **Agent 01:** audit atomic source mutation/revocation/cancellation ordering **across processes** between last authorization, durable CAS and actual network send. A process-local mutex is not a cross-process guarantee.
- **Agent 05:** independently verify that a trusted completion's three candidates are inserted as one v11 atomic batch; duplicate delivery of the same run must return existing IDs without altering versions or charging again.
- **Agent 10:** inject adversarial simultaneous completion/cancel/restart tests against the merged exact SHA; verify the host duplicate branch can only return success on the *same* provider ID, receipt and metered totals. Check actual installed-app negative paths.
- **Provider integration:** real provider-side status/receipt authenticity, pricing catalog, bounded usage, correlation from HTTP response to saved identity, and no-automatic-retry wire behavior still require independent verification. A local SQLite ledger cannot alone establish exactly-once **remote billing**.
- **Schema:** v10–v12 migrations are unmodified. Any future metadata/schema extension requires Agent 00 migration-number approval.

**Verdict:** focused host recovery consistency fix, **not independent no-double-charge certification or release authorization**.
