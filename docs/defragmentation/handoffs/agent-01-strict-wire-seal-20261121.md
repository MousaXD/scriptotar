# Agent 01 — Strict exact-wire consent delta (21 November 2026)

## Source and isolation
- Base: integration PR #114, immutable parent `b12843c75764601248a69c0dd89ff753af8df366`.
- Agent 01 source branch: `agent-01/strict-wire-seal-delta-20261121`; original PR #118 is preserved and not rebased.
- Scope: production policy in `crates/scriptotar-ai/src/provider_consent.rs` and compatibility updates to the existing **independent** no-network race suite in `tests/defragmentation/security/tests/provider_context_race.rs`. No schema or provider egress configuration changes.

## Gap addressed
Before this delta, a direct caller of `ConsentBoundary::decide` and `dispatch` could obtain a priced approval without calling `bind_exact_wire_request`. The host's sealed OpenAI route does seal bytes, but the reusable authorization boundary also needs to reject unsealed requests.

Approval now requires a sealed SHA-256 of the exact outbound request and dispatch requires the provider's prepared immutable byte digest to match. This check occurs before `reserve_once`, before the durable DISPATCHED CAS, and before any provider I/O. Changed digest => ChangedContext, zero claims, zero send. A missing digest cannot fall back to optional checking.

## Regression scope
- New: priced-but-unsealed approval denied, no claim/send.
- New: changed prepared wire hash denied, no claim/send.
- Existing: cancellation, expiry, wrong project/Study, changed sources/evidence, race with source edits during reservation, guard held during dispatch, replay and restart.
- The independent Agent 10 race suite's mock providers now report a deterministic synthetic SHA-256 and seal their fixture approval. No assertions or negative cases were deleted. This compatibility edit is **not** independent approval; Agent 10 must review and re-run their own suite.

## Still blocked
Production paid execution remains hard-disabled (`certified_paid_egress_enabled() == false`), with `NoTrustedPricing` and no trusted live catalog. A process-local `active_project` mutex has not been independently proven to fence all direct SQLite connections and cross-process origin mutations. Authoritative authenticated usage receipts, complete host mutation coverage, UI activation and native installed-app acceptance require Agent 00/02/06/10 certification.

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
