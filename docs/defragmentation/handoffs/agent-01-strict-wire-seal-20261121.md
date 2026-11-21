# Agent 01 — Strict exact-wire consent delta (21 November 2026)

## Source and isolation
- Base: integration PR #114, immutable parent `b12843c75764601248a69c0dd89ff753af8df366`.
- Agent 01 source branch: `agent-01/strict-wire-seal-delta-20261121`; original PR #118 is preserved and not rebased.
- Scope: production policy in `crates/scriptotar-ai/src/provider_consent.rs` and compatibility updates to the existing **independent** no-network race suite in `tests/defragmentation/security/tests/provider_context_race.rs`. The original wire-seal change had no schema alteration; the later additive v13 cross-process barrier is documented below. Provider egress configuration remains unchanged.

## Gap addressed
Before this delta, a direct caller of `ConsentBoundary::decide` and `dispatch` could obtain a priced approval without calling `bind_exact_wire_request`. The host's sealed OpenAI route does seal bytes, but the reusable authorization boundary also needs to reject unsealed requests.

Approval now requires a sealed SHA-256 of the exact outbound request and dispatch requires the provider's prepared immutable byte digest to match. This check occurs before `reserve_once`, before the durable DISPATCHED CAS, and before any provider I/O. Changed digest => ChangedContext, zero claims, zero send. A missing digest cannot fall back to optional checking.

## Regression scope
- New: priced-but-unsealed approval denied, no claim/send.
- New: changed prepared wire hash denied, no claim/send.
- Existing: cancellation, expiry, wrong project/Study, changed sources/evidence, race with source edits during reservation, guard held during dispatch, replay and restart.
- The independent Agent 10 race suite's mock providers now report a deterministic synthetic SHA-256 and seal their fixture approval. No assertions or negative cases were deleted. This compatibility edit is **not** independent approval; Agent 10 must review and re-run their own suite.

## Still blocked
Production paid execution remains hard-disabled (`certified_paid_egress_enabled() == false`), with `NoTrustedPricing` and no trusted live catalog. A process-local `active_project` mutex alone is insufficient to fence direct SQLite connections; the new v13 durable barrier is a draft candidate and has not yet received independent security certification. Authoritative authenticated usage receipts, complete host mutation coverage, UI activation and native installed-app acceptance require Agent 00/02/06/10 certification.

Do not claim release readiness from this delta. No API keys, credentials, transcript content or external paid calls are included.

## Exact-head validation (required)
```sh
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace
cargo test --manifest-path tests/defragmentation/security/Cargo.toml --locked
```
GitHub Actions and original independent-race tests must be checked on the **final exact HEAD**, not a previous baseline.

## 21 November follow-up — visible price revision and real Studio mutation test

The consent preview now exposes `trusted_pricing_revision: Option<String>` (TypeScript `string | null`). A missing revision blocks approval, and changed revision at dispatch returns `ChangedContext` before durable reservation. This is disclosure/contract hardening only, not a live price catalog. Added a native host test proving that real Studio brief edits cannot commit while a provider scope guard is held; the guard release then permits the edit and invalidates the approved scope. No cross-process SQLite safety claim is made.

## 21 November follow-up — canonical transcript revocation race

The host regression `dispatch_guard_blocks_real_transcript_revocation_and_invalidates_scope` now uses a real project-owned `sources -> media -> transcripts` fixture, attaches it through the Studio bundle, holds the exact provider scope gate, and races the public `revoke_studio_source_access` method. The revocation must not complete before guard release; once committed, its previous scope must be inadmissible. This proves this particular **in-process** host path only. It does not stop independent SQLite writers or certify paid-provider execution.

## Continued Agent 01 — v13 exact-effect cross-process authorization gate

- **Why:** Agent 10's independent PR #202 compiled and reproduced a canonical transcript edit from a different SQLite connection while the prior host authorization mutex was held. Stale-scope detection **after** the edit is not a safe pre-send linearization boundary.
- **Additive migration v13:** `provider_dispatch_barrier` has a durable singleton token, project/Study/scope binding, timestamp and **optional exact effect identity** (run, step, idempotency key, attempt, provider namespace, sealed body SHA-256). Unbound records are only for existing offline host/DB negative tests; the production `ScopeAuthority::lock_effect_dispatch_scope` **requires a bound attempt** from `PaidEffectFence::dispatch_attempt`.
- **Atomic admission:** `SqliteStore::acquire_provider_dispatch_barrier_for_effect` opens a SQLite `BEGIN IMMEDIATE` transaction, verifies that the exact v10/v12 effect is still **RESERVED**, uncancelled and matches all immutable identity fields and wire digest, and inserts the singleton barrier before committing. Then the host recomputes the canonical scope and the generic policy rechecks its original sealed bytes, pricing revision, expiry and cancellation before the single v10 `DISPATCHED` CAS and provider I/O.
- **Cross-connection DML freeze:** v13 INSERT/UPDATE/DELETE triggers cover canonical project, Study, sources, media, transcript, research, evidence, drafts, writing outcomes and agent-run snapshots. The effect's `cancel_requested` update is separately denied under a held barrier; v10 pre-send dispatch, v12 provider request acknowledgement and authenticated receipt settlement remain writable.
- **Receipts:** the `ScopeDispatchGuard` is retained through durable authenticated settlement, not released when the provider merely returns. A standalone policy test pauses settlement and checks competing scope mutation cannot pass until commit.
- **Crash / ordinary-drop handling:** The persisted token is never TTL-expired. Normal RAII drop releases a bound barrier only when its exact original attempt is still `RESERVED` (no successful pre-send CAS) or has a durable terminal outcome and provider receipt/nonexecution evidence. `DISPATCHED`, `UNCERTAIN`, missing/changed attempts and partial identity remain frozen. A killed owner leaves its barrier in the database even after a late terminal receipt; **there is no automated or operator unlock API yet**. Agent 02 and Agent 10 must review a token-and-effect-specific, authenticated, transactional recovery release contract; a local crash is **not** evidence that a remotely charged request never executed.
- **Honest upgrade fixtures:** tests that reconstruct v5/v9/v10/v11 from an existing v13 file must physically remove the v13 table, triggers and migration ledger entry before setting the legacy `user_version`. Forged versions must still fail closed.
- **Known residual attack surface:** An actor able to execute arbitrary DDL can remove triggers or swap the barrier. The current mechanism is suitable only under a trusted, permission-isolated SQLite writer threat model. No independent Windows/Linux cross-process hostile test has certified it, and the global single-flight freeze can affect unrelated projects' performance and cancellation UX.
- **Migration ownership:** Agent 05's separate durable four-outcome response store must **not** register a second migration v13. Agent 00 owns selecting an independently reviewed additive v14 (or a single coordinated v13), with no retrospective rewrite of released migrations.
- **Release decision:** Provider egress remains `certified_paid_egress_enabled() == false`, `NoTrustedPricing` has no live trusted quote, and no complete paid-output-to-v11-batch transactional provenance exists. Agent 10 controls independent exact-head approval. PR #192 stays **DRAFT / NO-GO**.

## Remaining cancellation sequencing and release contract (NO-GO)

- v13 denies independent `agent_external_effects.cancel_requested` updates once the freeze is committed. A cancel that commits before admission is rejected by the atomic RESERVED/uncancelled lookup, and the v10 `DISPATCHED` CAS checks cancellation again.
- The existing host `SqlitePaidEffectFence::mark_uncertain` calls `request_external_cancel` for an unused RESERVED effect. While the v13 freeze remains active this generic cancellation call is correctly rejected to avoid a second-process race, and its return value is currently ignored. Although the original idempotency key still prevents an unreviewed replay, that pre-send cancellation may remain unrecorded.
- Agent 02 must own an **effect-bound, token-authorized, single-transaction pre-send cancellation + gate release** method, or a reviewed equivalent, that proves no DISPATCHED CAS before the event. An unauthenticated caller must not be able to use it to release another in-flight charge. The normal resolver must leave charged, disputed and uncertain attempts frozen until independently authenticated recovery.
- The proposed same-SHA CI suite and independent Windows/Linux two-process tests have not been certified. Keep `certified_paid_egress_enabled() = false`.
