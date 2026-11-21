# Agent 02 — charged-provider response durability (staged v14)

**Scope:** PR #187, after original v8/v10/v11/v12 effect and writing schemas. **Not release-certified. Paid dispatch stays disabled.** This is a post-v13 source candidate, not an authority to edit the migration registry ahead of Agent 01 or merge overlapping policy files.

## Exact integration contract

1. Independently review and integrate Agent 01's **v13** source/dispatch barrier (PR #192). Agent 00 must then append **v14** to \`crates/scriptotar-db/src/lib.rs\`'s append-only \`MIGRATIONS\` array as \`agent_authenticated_charged_responses\` using \`crate::agent_effects::external::migration_charged_responses\`, change \`LATEST_SCHEMA_VERSION\` from 13 to **14**, and preserve the original identities and order of migrations 1–13. The current PR intentionally **does not register v14** because its frozen base ends at v12. Fixtures install v14 DDL without modifying user_version, solely to run isolated DB tests; that is not a production migration.
2. The additive \`agent_charged_responses\` table contains one immutable row for the exact \`project_id/study_id/run_id/step_id/idempotency_key/attempt/provider/request_sha256/receipt_id\` and bounded original UTF-8 response bytes, SHA-256, input/output tokens, charged cost and timestamp. A unique run/step key prevents alternate generations for the same step. Raw provider output is limited to **400,000 bytes / 100,000 chars**; the full writing sink retains its own 3-candidate checks.
3. **Trusted adapter API**: after authenticating the provider's identity, model-specific usage, exact raw request, and final receipt, call \`SqliteStore::settle_authenticated_charged_response(AuthenticatedChargedResponse)\`. It opens one SQLite **IMMEDIATE** transaction, checks the run's original saved step and v10/v8 identity, inserts the raw response seal, transitions the exact v10 effect to \`completed\` (v12 provider anti-replay triggers remain active), transitions v8 to \`applied\`, revalidates the seal, and commits. Failed SQL leaves **none** of these changes committed. Repeated exact evidence is read-only idempotent; contradictory bytes/receipt/usage are refused. This API is **not public IPC** and must not be supplied from model text, Studio or an untrusted plug-in.
4. For each of \`angles\`, \`draft-1\`, \`draft-2\`, \`draft-3\`, \`read_sealed_charged_response\` returns authenticated, verified original bytes read-only. A separate Agent 05/Agent 00 adapter can supply those bytes to \`DurablePermissionBoundary::begin\` as \`StepAdmission::Recorded { response_json }\`. It **must not** synthesize \`StepAdmission::Dispatch\` on absent seal. All four recorded responses must match the **same** authenticated run, originally sealed request fingerprints, and the canonical current Study snapshot before any **new** draft creation; never reuse an approval for edited or revoked sources. A v11 committed batch is reconciled read-only by its immutable version-one receipt; do not duplicate it.
5. The v14 trigger rejects any \`completed\` transition for these four steps **without a same-effect sealed response matching receipt, usage, provider, attempt and wire digest**. The historical generic receipt method continues to work for other effect types. A historical/altered \`completed\` writing effect lacking a seal is **UNRESOLVED**, not silently declared recoverable.
6. The trusted receipt **authentication remains external**: comparing a caller-supplied provider name with \`openai\` is not cryptographic proof. Agent 01 and the reviewed wire adapter must attest exact response bytes, response ID, usage, model pricing revision and account ownership. An HTTP timeout, claimed provider name, user-supplied ID, or model-produced JSON cannot authenticate completion.


## Consent receipt + original response hook (Agent 02 continuation)

- `scriptotar-ai::provider_consent::PaidEffectFence` now exposes an additive `settle_with_response(effect_id, provider_request_id, usage, original_response)` hook. The `ConsentBoundary::dispatch` success path invokes it **after** the provider transport's authenticated usage verification and authoritative pricing calculation, and **before** returning a successful response. Existing mock/non-writing fences delegate to their original `settle` implementation by default.
- The production host `SqlitePaidEffectFence` overrides that hook for the four exact step IDs and invokes `settle_authenticated_charged_response` with its own persisted project, Study, run, step, idempotency, canonical provider, sealed wire SHA, verified provider response ID, split usage and **the exact generated text**. It deliberately rejects ordinary receipt-only `settle` for those steps; in a v12 database without the v14 migration, the resulting seal attempt fails closed to **UNCERTAIN**, not `COMPLETED`.
- This change does **not** make `provider_consent` itself a writing orchestrator. Agent 05 still owns `DurablePermissionBoundary::begin`, prompt/request equivalence, saved-step replay and v11 batch commit. Each of the four paid steps needs a separately scoped, exact-wire approved request. Consent must never be inferred from an earlier or different step.
- Agent 01's separate v13 branch changes the lifetime of the authoritative dispatch scope guard; **Agent 00 must manually reconcile** the source overlap in `provider_consent.rs` and `scriptotar-ai/src/provider_consent.rs`. The v13 barrier must not be released before the original authenticated receipt+response transaction, and a post-send crash must leave recovery blocked until independently verified. The submitted v12-based source is *not* v13-certified.
- The test `hard_output_ceiling_and_one_time_claim` now asserts that the original metered provider response is passed to the settlement hook. This is a deterministic fixture only, not proof of a live OpenAI account receipt or a no-double-charge guarantee from the provider.
- Persisting provider outputs creates an explicit local-content retention/privacy obligation. The migration stores raw bytes in Scriptotar's existing private SQLite database; at-rest encryption, key management, and deletion/retention policy are **not added** by this patch and require owner/security review.

## Exact crash/recovery state table

| Crash point / observed state | Durable local result | Allowed recovery | Forbidden action |
| --- | --- | --- | --- |
| Before reservation | No paid effect | New *authorized* work if still eligible | Infer completed call |
| Reserved before dispatch | v8 + v10 reserved | Cancel locally if provably unsent | Treat reservation as send |
| After DISPATCHED CAS, before response | Dispatched/uncertain, no v14 row | Read-only provider reconciliation; retain blocker | Retry paid call or mark no-charge without proof |
| Provider returned response, app dies **before seal** | No durable original bytes; possible remote charge | **UNRESOLVED** until trusted provider reproduces exact response + receipt | Invent result, regenerate, declare safely recoverable |
| During seal/receipt/v8 settlement | Entire IMMEDIATE transaction rolls back | Keep disputed dispatched/uncertain state; authenticated late settle if bytes available | Leave a completed ledger without original bytes |
| Seal+receipt committed before acknowledgement to caller | v14 bytes+SHA and v10/v12/v8 terminal receipt | Read-only replay, no new egress | Second charge, mutate original seal |
| After seal before v11 three-draft commit | Four durable step results (as available), no complete draft batch | Parse recorded bytes only and use canonical v11 atomic commit if context still active/current | Reuse stale consent or overwrite manual work |
| v11 transaction fails mid-batch | Zero new drafts/receipt from failed v11 transaction | Reattempt *local persistence* from validated sealed bytes on current Study | New paid generation |
| v11 batch committed, process dies before UI acknowledgement | Original 3 drafts + immutable receipt | Reopen saved batch read-only | Create duplicate batches |
| Archived owner after dispatch | Original effect may remain charged | Verify and settle original charge/read-only response while archived | New dispatch or new draft creation in archived Study |
| Cancel requested after dispatch | Dispatched/uncertain may still be charged | Authenticated late completion settles accounting | Treat cancel as proof of nonexecution |
| Completed writing effect without v14 bytes | Corrupt/legacy receipt-only outcome | Flag **UNRESOLVED** and investigate provider reconciliation | Fabricate output or call provider to regenerate |

## Focused tests committed in this PR

- All four step IDs: completed response seal+v8/v10 accounting, exact-idempotent redelivery, restart read, denied re-dispatch.
- Response received but lost before SQLite seal: \`dispatched -> uncertain\`; no byte recovery claim and no second dispatch.
- The old receipt-only completion path is denied for writing steps once v14 is present.
- Failure injection into either v10 completion or v8 Applied update: v14 raw bytes, v12 receipt identities and v10/v8 statuses roll back together.
- Original project+Study archival does not prevent authentic late settlement but prevents a fresh send.
- Forged provider namespace, changed wire fingerprint, over-budget token usage, contradictory response and digest tampering are refused.

Tests are deterministic, local SQLite/no-network. The complete merged v13+v14 migration, four-step durable host adapter, authenticated provider proofs, and v11 integrated fault matrix are **not yet certified** by these isolated tests.

## Coordination and review gates

**Agent 01:** review use of \`request_sha256\` against the effective v13 wire seal and frozen canonical scope; keep the token-bound barrier until the receipt+response commit or independently audited orphan recovery. Review the provider-specific authentication/usage boundary; never enable paid egress based on this branch.

**Agent 05:** consume only exact sealed step output through the existing \`Recorded\` admission; validate all four step identities and sealed fingerprints before local v11 commit, and test crash windows/archived Study semantics. Keep source-grounding, originality and immutable draft-v1 checks in the canonical sink.

**Agent 00:** manually reconcile the overlapping Agent 01/02 effect/host files, register v14 AFTER v13, run original v1–v14 upgrade fixtures including user_version/ledger consistency, test crash injection at every boundary on the combined exact commit, and obtain native acceptance. Do not repin publisher/trusted-source gates based on this PR alone.

**Agent 10:** independently inspect the exact SHA for fake receipts, model-qualified provider namespaces, v8/v10/v12/v13 identity mismatches, concurrent SQLite writers, response tampering, late settlement, and provider-result-before-seal unrecoverable window. Do not certify egress without authenticated provider provenance and full v13/v14 integration.

**Verdict: DRAFT / NO-GO.** No provider network call, release, migration history rewrite, or merge to main/integration was performed here.
