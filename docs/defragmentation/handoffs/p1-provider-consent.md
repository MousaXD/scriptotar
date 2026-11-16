# Agent 01 — P1 provider consent & budgets handoff

**Date:** 2026-11-16
**Repository:** https://github.com/MousaXD/scriptotar
**Base:** `integration/scriptotar-defragmentation-agent00-20261113` at `3009989a3aa73248b6432ba0cd84821c59110993`
**Branch:** `agent-01/p1-provider-consent-20261116`
**Status:** **Draft integration candidate, paid execution disabled.** No merge/release authority.

## Production implementation on this branch

- `crates/scriptotar-ai/src/provider_consent.rs`: stateful consent boundary with a SHA-256 fingerprint across the exact run, canonical Study scope, prompt, selected attached sources, provider, model, endpoint, trusted price revision, output cap, USD/token/tool/time budgets. Explicit approve/deny, cancellation and expiry. In-memory approvals are one-use and disappear on restart.
- The review preview includes the **literal exact outbound prompt**, destination, model, source IDs, input estimate (display only), conservative input reservation, explicit maximum output tokens, and US$ budget expressed as integer micro-USD. Unknown prices are displayed as unknown, and **approval cannot be granted**. Prices may only come from a trusted host-owned catalog; the implementation does not guess provider rates.
- Validates current project/Study ownership and exact material/prompt context at preparation, approval and dispatch. The request must agree with the DB-derived canonical prompt hash; evidence, source selection, Study revision, pricing revision or request edits invalidate approval.
- Server-side preflight reserves **one** provider tool call, bounded input+output tokens, maximum 120-second wall time and a nonzero micro-USD ceiling. Conservative prompt reservation is UTF-8 bytes plus 512 envelope tokens, not a claim of actual tokenization accuracy.
- `apps/desktop/src-tauri/src/services/provider_consent.rs`: first host adapter reads the canonical Study, brief, attached source metadata and evidence, and re-generates the literal three-angle prompt using the existing Studio writing service. It exposes Rust-only prepare/decide/disabled-dispatch methods. There is **no public Tauri endpoint** on this branch.
- The `PaidEffectFence` contract reserves a single effect identity before dispatch, commits a metered receipt before reporting success and marks provider failures/overages/write failures as **uncertain**. Uncertain effects never automatically retry. In-process consent cannot substitute for a durable paid-effect ledger.
- `ExistingAiServiceAdapter` explicitly returns **unsupported hard bounds**. Existing `AiService<HttpAiProvider>` is intentionally not invoked: OpenAI, Anthropic and Gemini legacy adapters do not all accept a caller-bound output ceiling and no unified verified token/cost receipt exists. `DisabledPaidEffectFence` also refuses all paid effect claims. Existing BYOK and Copy Prompt behavior is not altered.

## Security tests authored

`crates/scriptotar-ai/src/provider_consent.rs` contains deterministic fake-provider/ledger tests for no call without approval, denial, changed prompt/model/selected sources/evidence/budget, foreign project, stale revision, unknown prices, tiny budget, exactly-one claim, disabled production adapter/fence, non-retry after provider error or overspend, cancellation, expiry, missing request-time key, and no approval reconstruction after a restart.

`apps/desktop/src-tauri/src/services/provider_consent.rs` contains a native project-scope fixture. **No local Cargo toolchain is installed in this task environment.** These tests and rustfmt must be verified in GitHub Actions on the exact final SHA before any integration claim.

## Agent 00 — required interface and app-lifecycle integration

1. Review and freeze the approval payload: `run_id`, active `project_id`, `study_id`, Study revision, exact canonical prompt digest, source selection and evidence digest, provider, model, trusted price quote/version, budget and effect identity. Keep Tauri registration, shared DTOs and EN/AR review/denial UI under Agent 00 ownership.
2. Host the `ConsentBoundary` as process state, or replace it with a strictly scoped durable approval record. Reject partial/duplicate approval decisions, never persist keys or attempt to reconstruct approval after restart.
3. Re-derive the canonical writing request and active project/Study under a single host-owned snapshot/lock or database transaction at dispatch. The current first host adapter double-checks before/after reads; **atomic snapshot and project-switch lock across the dispatch authorization boundary remain required before enabling paid calls**.
4. Preserve Copy Prompt and legacy manual BYOK. The new consent policy does **not** protect the legacy `run_ai` call path; if product policy requires consent for manual BYOK too, route that endpoint explicitly through the approved new contract rather than relying on this new module.
5. Build the user-visible approval screen for exact prompt, outbound destination, source excerpts/evidence, token/US$ caps, unavailable pricing and explicit deny. Never count an unregistered Rust helper as an end-to-end usable flow.

## Agent 02 — required durable paid-effect adapter

1. Supply an atomic, project-/Study-/run-/request-fingerprint-scoped ledger claim `reserve_once(effect_id)` **after** the coordinator Running-intent CAS, but **before** any provider I/O. An existing, missing, ambiguous or committed claim must never be used as authorization to resend an expensive request. Exactly one winner under concurrent workers.
2. Persist receipts and actual spend/token metadata transactionally, never secrets. If a request was sent and the receipt cannot be proven, mark **UnknownEffect**, block and require explicit reconciliation with provider-side records. Unlike local transcript read, never re-execute a paid request just to reconcile.
3. Integrate provider-specific network calls that actually send `max_output_tokens` and enforce no retries, a firm timeout, response byte/memory bounds, verified usage/cost accounting and provider-model support. The existing generic `AiService` adapter is **not** suitable for paid execution until those controls are implemented.
4. Define a trustworthy, versioned provider/model pricing catalog including unknown-pricing handling. A user's arbitrary cost estimate must not become a trusted pricing quote. USD caps require proven per-token rate bounds and any provider-specific non-token charges, or must fail closed.
5. Reconcile cancellation and late receipts without replaying; owner and QA must run crash-after-send/claim collision/receipt failure/approval expiration tests.

## Validation / open blockers

- **Source files committed:** pending final SHA; see branch/PR head.
- **Local cargo test / fmt / clippy:** not available (no `rustc`, `cargo`, or `rustfmt` installed).
- **Hosted exact-head checks:** pending draft PR Actions; report final GitHub results separately.
- **Paid execution:** deliberately disabled, not an end-to-end completed provider feature.
- **Public UI/IPC:** not registered, reserved for Agent 00.
- **Key lifetime:** only a `&str` parameter at attempted dispatch, never part of stored consent or hash.
- **Integration, native GUI, performance and release:** not certified. Keep `#114` draft and `main` unchanged.

## Exact validation commands for CI

```sh
cargo fmt --all -- --check
cargo test --locked -p scriptotar-ai provider_consent -- --nocapture
cargo test --locked -p scriptotar-desktop provider_consent -- --nocapture
cargo test --locked --workspace
cargo clippy --locked --workspace --all-targets -- -D warnings
```

## 2026-11-16 — recovery from failing exact-head CI (Agent 01)

**Starting PR #118 SHA:** `bcb3f3a7d08426ee013530236ea2252012fa4da3`. Exact-head GitHub Actions logs showed four failing workflows:

| Workflow | Failure | Recovery |
|---|---|---|
| Rust foundation | `cargo fmt --check`, near `provider_consent.rs:503` | Match rustfmt's compact `reserved_cost` assignment |
| Tauri migration integration | Same Rustfmt failure | Same source fix |
| Main CI | `provider_consent::tests::exact_prompt_model_sources_and_evidence_are_bound` at line 749 | Check immutable approved intent **before** pricing lookup |
| MCP E2E | Same changed-context assertion | Same authorization-order fix |

The original dispatch would query `TrustedPricing::ceiling` on a client-modified model; this returned `UnknownPricing` rather than the required `ChangedContext`. The fix first compares the *entire* approved `ProviderIntent`, then revalidates host-derived Study scope, and then looks up pricing for the stored approved provider/model and checks the stored SHA-256 fingerprint against the current trusted quote. This is a denial-order and integrity fix, **not** a change permitting a paid call. The existing negative test covers prompt, model, source selection, material/evidence digest and budget mutations, and additionally asserts zero provider calls and zero paid-effect claims.

**Fix commits:** `8e1e8d5140194514a2600bb3681fc27f653b3625` (auth + initial formatting), then `e24770f6f230bac07a1d8c741b93e7b5201ce91c` (restores rustfmt-compliant test structure after first fix HEAD reported a remaining test-loop style difference). These are superseded by this handoff update; validate on the **newest PR head only**. The previous 9/9 integration baseline and PR #118 older passing jobs are not current-head evidence. No Cargo/Rustfmt toolchain or network git clone is available locally; GitHub Actions is the verification authority.

### Security audit and remaining blockers

- **Context invalidation / scope:** Exact prompt bytes, Study revision, selected source IDs and host-recomputed material digest are bound; altered submission is now rejected before price lookup. The current host authority obtains Study/brief/sources/evidence/prompt across multiple reads and does not hold an atomic active-project/Study snapshot through the paid send. Agent 00 must supply that host lock/transaction and revalidate before pre-send CAS. No live egress until then.
- **Pricing provenance:** `NoTrustedPricing` remains the production default and cannot grant approval; tests supply a fake catalog only. A signed/versioned host-owned provider/model price schedule with support for non-token charges and authoritative token receipts is not implemented. Catalog-revision drift is bound by the fingerprint; a client price estimate is never a legitimate quote.
- **Canonical fingerprint:** `hash_intent` binds typed intent, provider/model/base URL, complete prompt, source/material digests, limits and the pricing quote. It is **not yet** a fingerprint of the *wire-level* HTTP body or later parameters (system text, sampling temperature, format, tools). Agent 02 must not use the present fingerprint as proof that an unimplemented HTTP payload was reviewed; enforce exact canonical payload serialization before live dispatch.
- **Session/cancellation:** In-memory approvals are single-use, scoped, expire and disappear on restart. Cancellation before reservation denies; after a reservation or ambiguous network result the run is uncertain and must not retry. Clock and scope authority are trusted host responsibilities; an unregistered helper is not a user-facing lifecycle.
- **Concurrency / crash recovery:** A mutex blocks in-process double send and deterministic tests cover eight concurrent attempts. Real cross-process/restart claims, receipts, cancellation races, ACK recovery, and authoritative nonexecution proofs belong to Agent 02's external ledger. The current `PaidEffectFence` string-only bridge cannot safely implement the scoped external-ledger contract without a typed integration review.
- **Legacy/manual BYOK:** Existing `run_ai` is out of this new policy; do not claim it is covered. `ExistingAiServiceAdapter` cannot enforce all model-level hard output/token/receipt controls, so the new policy refuses to send through it.
- **Privacy:** API keys are request-time-only, never stored in consent records or receipts; no new network, silent calls, persisted credentials or public Tauri commands were added.

### Agent 02 coordination and integration sequence

Agent 01 posted [the typed policy/ledger integration review on PR #119](https://github.com/MousaXD/scriptotar/pull/119#issuecomment-6917344870). Agree with Agent 00 on project/Study/run/step + checkpoint and approval identity; canonical full outbound HTTP payload hash; exact selected sources/evidence and quote revision; per-step and cumulative token/time/USD ceilings; and an atomic revocation/active-project check at dispatch. Agent 05's four-step angles + three drafts require separately permitted requests (one angle approval is not blanket approval). Agent 02 owns durable pre-send CAS/receipt/ACK reconciliation; unknown outcomes cannot be retried; Agent 00 registers the versioned migration/driver/IPC and installed-app acceptance. No paid egress until those gates are proven.

**Owner integration:** cherry-pick Agent 01's recovered branch only after all relevant exact-head checks pass and QA reviews the interface; preserve disabled paid adapter, do not merge #114 or modify `main`, and do not ship a release.