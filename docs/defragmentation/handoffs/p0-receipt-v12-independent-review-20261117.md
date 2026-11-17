# Agent 10 — v12 provider identity independent security review (17 November 2026)

**Gate: NO-GO.** This is an evidence checkpoint, **not** release approval. Original five tests are unchanged. Paid provider egress stays OFF. No production or `main` files were edited by Agent 10.

## Exact previously executed baseline

- Independent audit PR #124 head: `a816ec761903e274e033540b4f90d27fc62a153d` (workflow's audit checkout prints merge ref `d47d407e58329d743549183e9537a65c22c48e9b`).
- Owner integrated source: `efb38cf186fb76138323b0d8d1136ed727739974`.
- [GitHub Actions run #44119319865](https://github.com/MousaXD/scriptotar/actions/runs/44119319865); [combined job #134166001293](https://github.com/MousaXD/scriptotar/actions/runs/44119319865/job/134166001293).
- Independent command in CI: `bash audit/tests/defragmentation/security/run_feature_gates.sh combined 114`. The CI script injects tests only into a disposable target checkout; it runs `cargo test --locked -p scriptotar-db` and real Tauri host and wire tests.
- Result: **5/6 jobs PASS; combined FAIL**. Real SQLite DB **42 PASS / 5 FAIL**, plus missing production paid Tauri entrypoint. These are accounting / ledger double-credit acceptance defects. No actual external duplicate charges or provider API requests were generated.

| Finding | Verdict on exact baseline | Evidence / expected invariant | Owner |
|---|---|---|---|
| `independent_v10_provider_receipt_must_not_settle_two_distinct_runs` | **FAIL** | Same provider receipt completes two separately approved runs; loser must remain non-completed | Agent 02, Agent 00 |
| `independent_v10_provider_receipt_must_not_settle_across_project_boundary` | **FAIL** | Same receipt accepted after SQLite reopen under another project/Study | Agent 02, Agent 00 |
| `independent_v10_acknowledgement_id_is_unique_before_final_settlement` | **FAIL** | Duplicate accepted request ID across two live dispatched effects before final response | Agent 02 |
| `independent_v10_receipt_identity_is_unique_even_with_distinct_request_ids` | **FAIL** | Different provider request IDs settle with identical provider receipt ID | Agent 02 |
| `independent_v10_concurrent_cross_run_receipt_settlement_has_one_winner` | **FAIL** | Two independent SQLite connections both credit the same receipt | Agent 02 |
| `test_approved_paid_provider_dispatch_is_exposed_through_tauri_host` | **FAIL** | No registered consent-approved paid `generate_handler!` command, despite private sealed host adapters | Agent 00 |
| Isolated provider consent and revocation | **PASS** | PR #118 source `d6d738ec84ac6eff1aae26edbd79e44995f2e82b`; independent 10 tests + 20 core tests in same run | Agent 01 |
| Historical v4/v5 and v10 standalone DB | **PASS** | PR #119 `9da21c96a62d7a041b64500841b89dc6dc649703`, real `cargo test --locked -p scriptotar-db`; **does not include five newer injected checks** | Agent 02 |
| Canonical writing | **PASS** | PR #121 `467023db81985cc5af45bb6f28d93f2fd3fbd7af`; 32 native tests | Agent 05 |
| Claim-source verification | **PASS** | PR #117 `1ffb0fffad71a6fb2980966e1eec106174f543ff`; 25 native verification tests | Agent 04 |
| Combined v11 | **PASS** | #114 `efb38cf186fb76138323b0d8d1136ed727739974`; 35 real writing and native DB tests; separate from combined provider gate | Agent 00 |

## New unconfirmed repair and added independent gates

- Agent 02 recovery: [draft PR #128](https://github.com/MousaXD/scriptotar/pull/128), branch `agent-02/p0-v12-provider-receipt-uniqueness-20261117`. Inspected source through `c1cdff88d0b56747d2a999c7ee59e3591d07a92b`; may advance. Additive v12 registry `(provider,kind,identity)`, SQLite trigger/unique index, receipt tombstones, atomic fail-closed backfill and owner-written cross-project, concurrency, retry, collision migration tests. **NOT TESTED by completed Agent 10 v12 gate at this checkpoint.**
- Agent 10 branch after extra work: `e71cf18c691ca2b769d31016c257d232e6614b5d`. New matrix job `receipt-v12:128` is added to [independent security workflow](https://github.com/MousaXD/scriptotar/actions/workflows/independent-security-regressions.yml); queued, **NOT TESTED** at this checkpoint.
- Exact new command: `bash audit/tests/defragmentation/security/run_feature_gates.sh receipt-v12 128`. It checks out PR #128, injects the original five unchanged tests plus additional positive/adversarial tests and independent v11→v12 migration checks into the real DB Rust fixture, then runs `cargo test --locked -p scriptotar-db`. Owner files remain unchanged.
- After Agent 00 integrates v12, the **combined:114** gate conditionally injects the independent v12 migration fixtures as well; combined and combined-v11 must run on the **same corrected integration SHA**. Old runs are not transferable.

| Additional test / finding | Verdict | Requirement | Owner |
|---|---|---|---|
| `independent_v10_same_effect_receipt_replay_after_reopen_is_idempotent` | **NOT TESTED** | Same effect, same callback after restart does not allocate second receipt/charge | Agent 02 |
| `independent_v10_provider_scoped_identities_do_not_block_distinct_providers` | **NOT TESTED** | Real separate providers may use equal provider-local IDs | Agent 02 |
| `independent_v10_old_attempt_ack_cannot_be_reclaimed_after_proven_retry` | **NOT TESTED** | Historical ACK remains owned by first effect after proof-backed retry | Agent 02 |
| `independent_v12_provider_case_alias_cannot_double_credit` | **NOT TESTED** | Reject noncanonical `OpenAI` or canonicalize to `openai`; no false second owner | Agent 02 |
| `independent_v12_retry_cannot_claim_same_provider_request_from_older_attempt` | **NOT TESTED** | Same effect must not reuse attempt-1 provider ACK as attempt-2 acceptance proof | Agent 02 |
| `independent_v12_valid_historical_v11_receipt_backfills_and_replays_without_double_credit` | **NOT TESTED** | Real v11→v12 backfill preserves charged receipt and accepts original idempotent callbacks | Agent 02 |
| `independent_v12_cross_project_legacy_receipt_collision_rollback_is_atomic` | **NOT TESTED** | Malformed historical cross-project double-credit causes full migration rollback, no deletion, ledger still v11 | Agent 02 |
| Historical v11 proof-backed retry cleared old ACK before v12 | **NOT TESTED / source-review risk** | Migration cannot reconstruct erased old IDs from v10 current row; fail closed or explicitly quarantine ambiguous history | Agent 02, Agent 00 |
| Registered Tauri approval+trusted OpenAI Responses body/pricing/usage | **NOT TESTED** | Public production command must execute only approved sealed consent, trusted wire bytes, hard cost limits and authenticated receipt | Agent 00 |
| Cross-process source/revocation synchronization, crash/uncertain replay | **NOT TESTED** | Process-local active-project mutex is insufficient as proof for independently writing MCP process | Agent 00, Agent 01 |
| Actual installed-app WebViews | **FAIL as full acceptance, separately scoped** | [Agent 07 installed run #44118342637](https://github.com/MousaXD/scriptotar/actions/runs/44118342637): Linux and Windows jobs failed; incremental Linux journey successes must not be mistaken for full installed certification | Agent 07 |

## Signoff conditions

Require 1) exact new PR #128 independent v12 DB **PASS** including positive tests; 2) correction and execution of any newly failing adversarial cases; 3) additive migration safety on v10/v11 historically colliding rows; 4) owner integration exact-SHA **combined** and **combined-v11** PASS with a real registered approved Tauri path and no legacy BYOK bypass; 5) independently authenticated cost, source/revocation synchronization, replay, installed native, and HTTP transport evidence. All tests use controlled local/no-network paths until true live provider dispatch can be separately and safely certified.

**Keep #124 and #128 draft; do not merge into main, release, or enable paid egress.**

## Additional OpenAI cross-role and pre-v12 retry hazards

- **NOT TESTED — OpenAI response-ID cross-role claim:** `independent_v12_openai_response_id_cannot_cross_request_receipt_roles`, independent test-only commit `a65ef21812fef80db5c107ad5e15b750157bf48b`. The host `SqlitePaidEffectFence::settle` maps the same authenticated OpenAI response ID to both `provider_request_id` and `receipt_id`; a `(provider,kind,identity)` registry must not let one effect's request/ACK ID be appropriated as another effect's receipt ID. Exact CI gate `receipt-v12:128`; Agent 02 schema/Agent 00 wire integration owner. Test status unverified until completed run.
- **NOT TESTED — old ACK erased before v12:** `independent_v12_historical_v11_retry_cleared_ack_cannot_reenable_billable_dispatch` added at audit SHA `bf58feea9878dde6b16c69b815e512c56b8b6d83`. The fixture creates a real provider ACK in attempt 1, records verified nonexecution and a reserved attempt 2, then reconstructs v11 (dropping the v12 registry). After migration it must **either** fail database open atomically **or** quarantine the ambiguous effect so pre-send CAS cannot grant a new paid dispatch. Current v10 old-attempt history contains nonexecution evidence only, not earlier cleared request ID; v12 backfill cannot establish uniqueness from the current-row NULL. Owner Agent 02, integration Agent 00.
- Both are **additional independent negative cases**, not statements that real provider billing duplicated. **No security signoff while their status is unknown or failing.**

## 2026-11-17 continued audit — SQLite anti-replay tombstone immutability

- Independent audit source commit `3f662351c895757b2d9b5471347759f9c04f27a4`. New test `independent_v12_provider_identity_claims_are_append_only_under_raw_sqlite` appended in `external_receipt_replay_additional_tests.rs` without changing the original five tests. It uses a *real reserved/approved/dispatched* effect, records a provider ACK, then attempts an ordinary SQLite `DELETE` and `UPDATE idempotency_key` on the v12 registry claim. Both must be rejected and the original row must survive. **NOT TESTED** in completed CI; current v12 schema lacks the expected `BEFORE DELETE/UPDATE` triggers, indicating a source-level failure risk (Agent 02 ownership).
- New repair PR #128 inspected at `b21ad022f33b48a2203717405d7d956ff53f8825`: registry now has `COLLATE NOCASE` provider-scoped cross-request/receipt ownership and additional historical v4/v5/v10 fixtures. The cross-role and alias gaps are addressed at the schema-design level only. Source review does not certify correct execution and the old ACK/new attempt issue is still open pending independent `receipt-v12:128`.
- Agent 02's earlier repair source `e9f8007a0ed4ab90dbe505164c15169cc00a2a8b` failed `cargo test --locked -p scriptotar-db` (standalone historical v5/v10 tests; Rust foundation [run #44133733441](https://github.com/MousaXD/scriptotar/actions/runs/44133733441), job #134217486956). Current `b21ad022` contains fixture corrections, **pending exact-SHA tests**.
- Latest observed independent seven-job [run #44134429115](https://github.com/MousaXD/scriptotar/actions/runs/44134429115) still queued at review; do not infer repair results. Latest audit test-only SHA above has its own [Actions checks](https://github.com/MousaXD/scriptotar/pull/124/checks). Agent 00's production Tauri paid dispatch remains **NOT CERTIFIED**; keep live paid egress off.
- [Agent 02 action item](https://github.com/MousaXD/scriptotar/pull/128#issuecomment-6955481526) and [Agent 00 security veto](https://github.com/MousaXD/scriptotar/pull/114#issuecomment-6955482359).

## 2026-11-17 host command false-positive prevention

**Agent 10 test-only source:** `0310fae5814f9bb816cacbcb15bb43008d21c041`. Existing host structural test already fails absent Tauri paid command. Added second necessary-but-not-sufficient test `test_registered_paid_host_has_real_serialized_openai_transport` that refuses to certify a dummy registered command: it requires a non-test sealed provider dispatch call, a production `HostOpenAiResponsesWire::new` construction and implementation of `CertifiedWireTransport`, and delegation from the registered command into `AppServices`. [Owner notice](https://github.com/MousaXD/scriptotar/pull/114#issuecomment-6955515969).

**Current verdict: FAIL on prior integrated v11 source `efb38cf186fb76138323b0d8d1136ed727739974` for missing public paid Tauri entrypoint (run #44119319865).** New structural test exact-head execution is **NOT TESTED** until independent CI finishes. A future structural PASS alone will **not** certify real HTTP use, pricing, revocation, authenticated receipt or cross-process source synchronization; keep egress disabled. Latest observed integration branch still draft.
