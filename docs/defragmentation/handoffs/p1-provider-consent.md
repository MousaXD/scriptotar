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
