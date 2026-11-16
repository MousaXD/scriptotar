# Agent 02 — P1 durable external effects and idempotency handoff

**Branch:** agent-02/p1-durable-effects-20261116  
**Base:** integration/scriptotar-defragmentation-agent00-20261113 at 3009989a3aa73248b6432ba0cd84821c59110993  
**Scope:** database-side paid/external effect boundary, service integration seam and regression tests. NO main merge, tag or release.

## What already existed — preserved

- v7 SQLite project/Study-scoped agent runs, checkpoint CAS and append-only audit.
- v8 agent_effects immutable claim, revision protection and status transition guard.
- Running-intent checkpoint and explicit approvals in scriptotar-agent.
- ReadStudyTranscript local-only, repeatable adapter in services/agent_inspection.rs.

None of those are replaced. No provider request is enabled just by compiling the new ledger.

## New code

- crates/scriptotar-db/src/agent_effects_external.rs: real SQLite IMMEDIATE reservation and dispatch CAS, bounded provider receipts, persistent cancellation and uncertain states, recovery and explicit proof-only retry. Linked to the v8 claim via foreign key. Includes native regression tests.
- crates/scriptotar-db/src/agent_effects.rs: includes and re-exports the new DB module, without changing v8 behavior.
- apps/desktop/src-tauri/src/services/agent_effect_driver.rs: Agent 01 policy and Agent 05 provider-transport interfaces, real database fence used before network I/O, trusted provider status probe, cancellation request and manual retry admission. This file is NOT registered as a service or IPC endpoint; Agent 00 owns registration.

## State machine and receipts

- Reserve: an approved Running checkpoint with exact project, Study, run, step and engine idempotency key; atomically insert existing v8 claim + new external-effect row. Unique fingerprint is **64 lowercase hex SHA-256** of the canonical request bytes (computed by Agent 01, not raw prompt text). Provider label, approved token and cost ceilings are immutable. A repeated key always returns Existing and must not cause another call.
- Mark dispatched: separate IMMEDIATE transaction and revision CAS; commit before provider I/O. Once this marker exists, even a crash before socket write is treated as potentially billable.
- Settle: verified receipts record bounded tokens, micro-USD, provider request ID and receipt ID without prompt/output/secrets. Completed and charged-failure states are distinct. The legacy v8 fence is settled within the **same transaction** as the external receipt.
- Uncertain: timeout, 5xx, disconnect, missing acknowledgement, crash after dispatch or unavailable provider status. Recovery moves Dispatched -> Uncertain and legacy v8 Reserved -> Unknown; never repeats network I/O.
- Reconcile: only a trusted provider adapter may supply authoritative receipt/cancellation or proven nonexecution. A missing provider ID or eventually consistent 404 is NOT proof of nonexecution.
- Cancel: Reserved -> Cancelled (no call); Dispatched/Uncertain set cancel_requested without claiming remote cancellation succeeded. Late provider receipt may still win. Terminal receipts cannot be overwritten.
- Retry: only explicitly after provider-verified nonexecution and fresh authorization, with revision CAS and append-only prior-attempt evidence. Maximum three attempts. A new attempt remains Reserved and must be separately dispatched. Old callback attempts are rejected. No automatic retries.
- Source/path checks: foreign project/study/run/step IDs, changed fingerprints, wrong budgets, expired/archived projects, corrupt checkpoint and unapproved steps fail closed. Local ReadStudyTranscript is excluded from the external adapter.

The SQLite durable claim prevents duplicate *local dispatch* across processes. It cannot claim exactly-once external execution without provider-side idempotency or reconciliation. A provider with neither capability is a NO-GO for automated paid retry.

## Required Agent 00 changes before activation

1. Register the additive agent_external_effects and agent_external_attempt_history schema through the central versioned migration ledger (next schema version) rather than relying indefinitely on the present atomic first-use CREATE TABLE compatibility bootstrap. Preserve all old migrations exactly; test upgrade from v9 and forced rollback.
2. Publicly re-export ExternalEffect, ExternalEffectKey, ExternalEffectLimits, ExternalEffectStatus, ExternalReservation and ExternalResolution from crates/scriptotar-db/src/root.rs.
3. Register the new private services/agent_effect_driver.rs module from services.rs; add only new shared command/DTO surfaces after a contract review. Keep existing local transcript inspection unchanged.
4. Call recover_external_effects for each active run during startup/recovery, and surface Uncertain as blocked in Studio with explicit reconciliation affordance, not a Try Again button.
5. Include native runner and full installer/GUI acceptance on one exact SHA. No release until independent QA signs off.

## Agent 01 / Agent 05 contract

**Agent 01:** provide ExternalEffectPolicy implementation that checks user consent, exact provider identity, prompt preview/egress, source access, expiry/revocation, per-run token and micro-USD caps on every dispatch. Compute canonical SHA-256 over the *actual* outbound payload, including model, temperature, system context, selected sources and parameters. Reject changing a pending request after approval. No API keys or prompts in DB receipts. Provider transport should have explicit timeout and local/remote idempotency support.

**Agent 05:** after Engine Running-intent CAS, call dispatch_authorized_external_effect once. Handle Existing as blocked/probe, never as permission to send; use reconcile_external_with_provider with actual provider status. Do not treat local application cancellation as remote cancellation. After restart recover pending effects and seek user-driven reconciliation for ambiguous ones. No automatic replay of paid requests. Provider outcomes must come from trusted adapter responses, not user-pasted JSON.

## Native tests added (not yet executed)

Unit tests under agent_effects_external.rs cover: pre-reservation absent state; crash after reservation; crash after dispatch; crash after receipt; duplicate callbacks; late response after uncertainty; lost acknowledgement; concurrent two-thread claim; fingerprint mutation; foreign project; overbudget; corrupt checkpoint; reserved/dispatched cancellation; stale attempt; provider-proven nonexecution and bounded retry. Existing agent_effects.rs integration tests and unchanged local transcript acceptance must be rerun.

**Validation limitation:** this execution environment has no cargo/rustc and no direct GitHub clone access. No local native test, Rustfmt or Clippy success is claimed. GitHub Actions exact-head checks are required and must be inspected. This is draft implementation for review, not release certification.

## Operational limitations for review

- In-flight cancellation is best-effort; a remote charge can still occur after the user requests stop.
- If a provider lacks an authoritative receipt or idempotency lookup, Uncertain remains blocked until human/provider resolution.
- Dispatched receipts are metadata-only; the actual external provider call must be implemented by Agent 01, not this DB boundary.
- Direct raw SQL/database tampering is out of trust scope; project/Study identity is also enforced in the service and parent table.


## 2026-11-16 continuation — provider acknowledgements, safe retry and cancellation

- **Previously certified SHA:** `68da0872328e2ac6841b9a92c97b5f68ce64ce35` passed all 12 GitHub checks (Rustfmt, workspace check/Clippy/tests, CI, supply chain, MCP, Windows installer and Linux packaging among them). The **latest HEAD has new code** and needs independent exact-SHA certification.
- **Pending provider ID:** `SqliteStore::record_external_ack` persists a unique, bounded provider request ID on a Dispatched/Uncertain effect without falsely recording completion. Identical duplicate acknowledgements are idempotent; conflicting request IDs fail closed. On restart, the provider can be queried by its durable ID. Only a trusted adapter may supply receipts.
- **Retry dispatch now reachable:** `AppServices::prepare_external_after_verified_nonexecution` is followed by an *explicit*, separately authorized `dispatch_prepared_external_retry`. The normal `dispatch_authorized_external_effect` rejects Existing and cannot silently replay. Retries are bounded by both three attempts and the checkpoint's `max_attempts`.
- **Legacy v8 fence synchronized:** cancellation before dispatch sets both the external status Cancelled and v8 status NotApplied atomically. Cancellation after dispatch only sets intent and blocks the v8 effect as Unknown until a trusted provider receipt arrives. An in-flight charge can still occur; never label it cancelled without provider evidence.
- **Agent 00 immediate requirement:** register `crate::agent_effects::external::migration_external` as the next core SQLite schema migration, re-export public DB types in `crates/scriptotar-db/src/root.rs`, and register `services/agent_effect_driver.rs` from `services.rs`. The first-use CREATE TABLE compatibility bootstrap in the isolated draft needs replacement by versioned migration **before enabling provider execution**. The unregistered host driver is not yet compiled by CI.
- **Agent 01 immediate requirement:** implement `ExternalEffectPolicy` and a real `ExternalEffectProvider` with canonical request hashing, explicit provider consent, revocation, egress disclosure, bounded budgets, provider-side idempotency and authoritative status probes. Use `ProviderEvidence::Accepted { request_id }` only for accepted-but-incomplete requests.
- **Agent 05 immediate requirement:** call first-attempt dispatch once after checkpoint Running intent; on Existing/Uncertain probe without replay. On provider-proven nonexecution and a fresh user approval, prepare a retry and call the *explicit retry dispatch* method. Never dispatch a previously Dispatched or Uncertain attempt.
- **Validation limitations:** native installed graphical WebView testing, live paid provider canaries and service-driver compilation are not certified by the isolated branch. New database regressions exercise restart, acknowledgements, request-ID conflicts, cancellation's v8 fence, cross-process dispatch CAS, corruption, delayed callbacks and bounded retries.

## 2026-11-16 recovery integration candidate (Agent 02-owned branch only)

- Coordinated integration contract with Agent 00 on PR #114 and exact-byte approval/receipt identity contract with Agent 01 on PR #118; no integration/main refs changed.
- Registered **private** `agent_effect_driver` in the actual Tauri host module for compilation (no IPC exposure, no real provider adapter, no outbound requests). This uses an inline module/include so the previously isolated adapter is included by Rust compilation; Agent 00 can normalize its formatting when integrating.
- Added minimal public DB type exports and candidate additive **v10** `agent_external_provider_effects` after v9, using the existing migration function. The per-request CREATE TABLE bootstrap and missing-table-is-empty fallback were removed. Retained IF NOT EXISTS **inside the versioned migration only** to permit upgrading databases exposed to the old draft bootstrap; malformed tables must still fail closed.
- New regression tests cover restoring a v9 database to v10 and reopening once, simulated v10 migration collision/rollback without losing v9 projects/studies, and two separately opened SQLite stores racing the dispatch CAS. Native tests must be certified at the exact new HEAD, not the old green SHA.
- Agent 00 review is still required before integrating shared-file changes, and Agent 01 must freeze canonical provider request SHA, durable scoped approval identity, metering and authentic receipts. This is **compile-time host wiring only**, not product-visible paid execution or release approval.
