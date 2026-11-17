# Agent 10 — updated live owner HEAD `eaefeb7`, 2026-11-17 (CERTIFICATION STILL NO-GO)

**Owner #114 moved during this review** from `46cb36efe2ca4e6ec3f9dd51b7b99c2d59804583` to **`eaefeb7746ee0e9f0856fa800caa3dda656e7e0f`** (two later commits, both confined to `apps/desktop/src-tauri/src/services/provider_consent.rs`). The six-gate 5-PASS/1-FAIL matrix below remains true *historical evidence at the old SHA*, not a verdict on the newest commit. [New exact-head independent run #44091887252](https://github.com/MousaXD/scriptotar/actions/runs/44091887252) was **IN PROGRESS at review**; await completed tests and inspect final exact SHA. Do not claim the old missing-constructor failure persists in the new implementation.

**Meaningful improvement verified by source:** `AppServices::dispatch_provider_angle_consent_sealed<T: CertifiedWireTransport>` is now a real **non-test Rust host method** that constructs `SqlitePaidEffectFence::from_approved_context`, seals the serialized provider request bytes and SHA-256, and feeds `ConsentBoundary::dispatch` a `SealedWireProvider` that forwards immutable bytes through `send_prepared_once`. Agent 00 added `sealed_host_dispatch_reaches_v10_once_and_never_replays`, which exercises a real host `AppServices` + real SQLite v10 reserve/CAS/settlement with a **controlled, no-network** transport, double-dispatch denial and a fresh consent-boundary replay check. This is a materially stronger test than merely instantiating the adapter. **The new test's CI result is pending**, and it must not be counted PASS until run completion.

**Remaining production blocker:** this method is `pub(super)`, has **no registered Tauri command** in `apps/desktop/src-tauri/src/lib.rs`, and has **no production HTTP `CertifiedWireTransport` implementation**. The ordinary `prepare_provider_angle_consent` still passes `NoTrustedPricing`; the default `dispatch_provider_angle_consent_disabled` still passes `DisabledPaidEffectFence`. The controlled `TrustedPricing` and receipt authenticator are **test-only**. Thus a real end-user consent-approved **production** paid dispatch, transport-specific hard limits and authenticated provider receipt remain **NOT TESTED / NOT AVAILABLE**, even if the controlled host unit test succeeds. This is intentionally safe disabled egress, not unauthorized spending.

**New independent check:** Agent 10 PR #124 commit `a0d27895e97d7787556664513295281a072b0f17` added `test_approved_paid_provider_dispatch_is_exposed_through_tauri_host` to the host wiring gate. It requires a registered paid/consent provider dispatch entrypoint and intentionally fails until the real installed Tauri application can reach the guarded method. This is an **entrypoint structural check**; a green result alone will not certify the host, live provider, or crashes. The latest runner improvement (`45a190a`) independently continues runtime Rust suites after structural failure and exits nonzero. Agent 00 should import current Agent 10 gate and retain it as release blocker; the new audit test **has not yet completed CI at this commit**.

**Coverage limits requiring Agent 00's correction and Agent 10's follow-up:**

- The newly added controlled test reloads the **in-memory `ConsentBoundary`** using the **same** `AppServices`/SQLite store. It does not prove process crash, disk reopen, or recovery after uncertain *in-flight* HTTP; run restart/reopen, timeout-after-send, lost ACK and double-bill adversarial tests through the host method.
- Reconfirm who constructs `ExecutionContext` and `ConsentPreview` from **host-owned** checkpoint and source data in the eventual registered command. Current private method takes both as arguments and validates them against intent; tests create the checkpoint directly, so public caller authority is still unproven.
- Verify `CertifiedWireTransport` hard timeout/token cap/redirect prohibition/receipt authenticity with real provider adapters, without real billable traffic in CI. Ensure sealed serialized bytes are *exactly* what the real HTTP implementation sends.
- Source mutator audit is unchanged: project-switch and Studio edits use `active_project`, but background job/research/transcript/watchlist and legacy import paths need proof that no digest-relevant origin write can slip between scope check and send.
- [Latest completed installed-app run #43956319863](https://github.com/MousaXD/scriptotar/actions/runs/43956319863) remains FAIL on **old** `46cb36e`. New exact-head [installed run #44091887271](https://github.com/MousaXD/scriptotar/actions/runs/44091887271) was IN PROGRESS; do not transfer old installed results to the new binary.

**Required decision:** Release **NO-GO**, paid egress **OFF**, PR #124 **DRAFT**. Certify only when exact-owner-head combined + combined-v11 both pass, new controlled host test executes, real reachable Tauri authorization and provider sealing are independently exercised, restart/race cases are covered, and installed Linux/Windows GUI gates pass. This entry supersedes the time-bounded old-SHA summary below without deleting audit history.

---

# Agent 10 — exact-head continued certification, 2026-11-17 (NO-GO)

**Current observed integration HEAD:** `46cb36efe2ca4e6ec3f9dd51b7b99c2d59804583` at review. **Independent test source PR #124:** `0667a831ba6d599d31cf50cf24321576e27ea4f2` before this documentation commit. **No production functionality, `main`, or paid egress changed.** The runtime-combined release gate remains **FAIL**, and authorization to release is **WITHHELD**. This review inspected exact GitHub source plus finished CI logs; it is not a substitute for executing an approved real-host paid-dispatch path.

## Verified independent matrix (source, command, owner, correction)

Completed [independent six-job run #43956319850](https://github.com/MousaXD/scriptotar/actions/runs/43956319850) tested **integration SHA `46cb36e`** (audit merge checkout `f107974c1e3177ac13d42bd77d7ec79240cfc2b8`). All statements below apply only to the cited code, not to future integration updates.

| Boundary / actual outcome | Status | Exact target SHA and command | CI evidence | Responsible owner / correction |
| --- | --- | --- | --- | --- |
| Isolated provider approval, consent revocation, budget alteration, cancellation, one-use and concurrent reservation mocks | **PASS (isolated only)** | #118 `d6d738ec84ac6eff1aae26edbd79e44995f2e82b`; `cargo test --manifest-path tests/defragmentation/security/Cargo.toml --test provider_context_race -- --nocapture`; `cargo test --locked -p scriptotar-ai provider_consent` | [provider job / run](https://github.com/MousaXD/scriptotar/actions/runs/43956319850) | Agent 01 / do not equate mock policy with real host, serialized HTTP or authenticated billing |
| Real SQLite v10 historical migrations, external-effect claims, pre-send CAS, uncertain/restart no replay | **PASS (DB-level only)** | #119 `9da21c96a62d7a041b64500841b89dc6dc649703`; `cargo test --locked -p scriptotar-db` | [migration job](https://github.com/MousaXD/scriptotar/actions/runs/43956319850) | Agent 02 / demonstrate real host invocation and receipt binding |
| Canonical writing and three distinct drafts, cross-project denial and forged references | **PASS (feature-only)** | #121 `467023db81985cc5af45bb6f28d93f2fd3fbd7af`; `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --test study_writing --locked -- --nocapture` | [writing job](https://github.com/MousaXD/scriptotar/actions/runs/43956319850) | Agent 05 / keep canonical links and immutable version-1 contract |
| Verification typed claims and no false factual promotion | **PASS (source and isolated tests, not third-party corroboration)** | #117 `1ffb0fffad71a6fb2980966e1eec106174f543ff`; `cargo test --locked -p scriptotar-ai verification`; `python3 -m unittest discover -s tests/defragmentation/security -p 'test_*.py' -v` | [verification job](https://github.com/MousaXD/scriptotar/actions/runs/43956319850) | Agent 04 / trust/corroboration audit remains separate |
| Integrated v11 atomic three-script persistence, idempotent receipts, immutable first version, rollback, restart and project isolation | **PASS (integrated Rust/SQLite)** | #114 `46cb36efe2ca4e6ec3f9dd51b7b99c2d59804583`; `bash audit/tests/defragmentation/security/run_feature_gates.sh combined-v11 114` | [combined-v11 job](https://github.com/MousaXD/scriptotar/actions/runs/43956319850) | Agent 00 / re-run at every newer exact release candidate |
| **Real consent-approved Tauri host → trusted bounded provider → durable fence** | **FAIL (production integration absent)** | #114 `46cb36e`; `bash audit/tests/defragmentation/security/run_feature_gates.sh combined 114` | [failed combined job](https://github.com/MousaXD/scriptotar/actions/runs/43956319850/job/133611702569), `test_combined_host_wiring.py:102` | **Agent 00** / construct and *exercise* `SqlitePaidEffectFence::from_approved_context` inside a real approved production dispatch, not solely unit tests or unreachable helpers; retain FAIL until exercised |
| Real serialized wire-byte SHA, approved model/prompt/token+cost ceiling, authenticated usage and idempotent external request ID | **NOT TESTED in production** | #114 `46cb36e`; no authenticated bounded live provider path exists | Same failed combined run; code review `services/provider_consent.rs` | Agent 00/01 / implement trusted price catalogue, hard transport bounds, byte-exact hash of final HTTP body, signed/trusted receipt handling; test with controlled transport and no paid traffic |
| All origin mutations and cancellation using the same authority gate across approval→CAS→send | **NOT TESTED; possible bypasses to investigate** | #114 `46cb36e`; `cargo test --locked -p scriptotar-desktop --lib` is **not** sufficient by itself | Real host guard exists; no complete multi-writer stress test | Agent 00 / test background transcript jobs, `scan_creator`, watchlist refresh and `import_legacy_data` against the Study digest and lock; add synchronization or atomic scope-version checks |
| Timeout, uncertain network status, lost acknowledgement, provider failure and restart with no second billable dispatch | **PASS in mocks/DB; NOT TESTED on live host dispatch** | #118/#119 isolated suites above, #114 `46cb36e` | [provider and migration jobs](https://github.com/MousaXD/scriptotar/actions/runs/43956319850) | Agent 00/01/02 / deterministic controlled-provider tests through actual host-owned path; no automatic retry on unknown outcome |
| Installed native Linux WebKitGTK and Windows WebView2 real app flow | **FAIL**, separate release gate | #114 `46cb36e`; native scripts from `tests/defragmentation/native-e2e` in [installed run #43956319863](https://github.com/MousaXD/scriptotar/actions/runs/43956319863) | [Linux job](https://github.com/MousaXD/scriptotar/actions/runs/43956319863/job/133611868740) and [Windows job](https://github.com/MousaXD/scriptotar/actions/runs/43956319863/job/133611869384) | Agent 07 / Linux project heading did not appear after create/switch; Windows UIA WebView document found but Studio control not observed in 90 seconds; Study/relaunch/RTL remain untested |
| Secrets in production logging, full Tauri command permissions and authenticated paid egress | **NOT TESTED end to end** | #114 `46cb36e`; source audit only | No claimed certification | Agent 00/10 / audit executable IPC, logs and transport with controlled credentials and redact all traces |

### Concrete source and workflow evidence

- In `apps/desktop/src-tauri/src/services/provider_consent.rs`, `AppServices::lock_dispatch_scope` holds `active_project` and recomputes the persisted Study, brief, attached sources, evidence and canonical excerpt prompt; `SqlitePaidEffectFence` reserves v10 effects, permits `mark_external_dispatched` once, and blocks uncertain replay. **This is genuine defensive code.**
- Nevertheless the **only non-test consent dispatch** is `dispatch_provider_angle_consent_disabled`, passing `NoTrustedPricing`, `DisabledPaidEffectFence` and `ExistingAiServiceAdapter`. The constructor `SqlitePaidEffectFence::from_approved_context` occurs in host **unit tests**, not a production dispatch call. `prepare_provider_angle_consent` similarly uses `NoTrustedPricing`. The Tauri `generate_handler!` command inventory contains **no registered paid provider-consent dispatch command**. Do not relabel disabled execution as certified paid dispatch.
- The code under `services.rs` starts `JobOrchestrator::start_with_notifier(store.clone())` and `spawn_watchlist_refresher(store.clone(), ...)`; `scan_creator` and `import_legacy_data` also perform store writes without visibly holding `active_project`. Studio editing uses `lock_studio_project`. **No exploit is established** because not every background write necessarily changes the Study digest; ownership must trace and test each relevant origin write under concurrency before authorizing HTTP.
- Exact native evidence at SHA `46cb36e`: Linux reports `installed_webview=PASS`, `create_switch_projects=FAIL` (missing heading `NativeQA-46cb36ef-A`); Windows reports `web_document_seen=true` but `studio_control_seen=false`, installed WebView readiness **FAIL** after 90 seconds. Installed-app artifact IDs **14273505215** (Linux) and **14272752981** (Windows). These are GUI findings, not evidence that provider consent failed.

### Immediate reproduction and owner acceptance requirements

Re-run with two exact source checkouts (`audit` from Agent 10 PR #124 and `target` from Agent 00 PR #114): `bash audit/tests/defragmentation/security/run_feature_gates.sh combined 114` and `bash audit/tests/defragmentation/security/run_feature_gates.sh combined-v11 114`. The Agent 10 runner improvement at `45a190aa92ff213f4e9a8f1a89acdec9b590778d` now allows all Rust host/DB/writing checks to run even when Python host wiring fails; it must **still exit nonzero**. This newer runner requires an exact-head CI execution; the previous completed six-job run used the older runner. Production path completion must include an **executed** Tauri/host test that goes through approved consent, real SQLite pre-send CAS, controlled bounded serialized transport, authentic receipt handling, cross-project/revocation admission and uncertain-restart denial. A positive structural regex check, isolated mocked fence or proof of construction alone is not a PASS.

Keep PR #124 DRAFT and paid dispatch disabled. Agent 00 owns the production fix; Agent 10 independently revalidates exact future SHAs. **NO MAIN MERGE / NO RELEASE CERTIFICATION.**

---

# Agent 10 — 2026-11-17 NEW completed exact-head six-gate matrix

## Native installed-app evidence — separate Agent 07 review, 2026-11-17

**NOT TESTED / NO-GO for native graphical acceptance on the final integrated release.** Agent 07 PR [#126](https://github.com/MousaXD/scriptotar/pull/126) latest observed head `ea6160556fcefc38bfb41e5045ed2c5177463c77` has ordinary CI and security hygiene PASS, but [actual installed WebView workflow #43955788990](https://github.com/MousaXD/scriptotar/actions/runs/43955788990) is **IN PROGRESS** at this exact-head snapshot; it must not be marked passed. Prior [failed installed workflow #43897349298](https://github.com/MousaXD/scriptotar/actions/runs/43897349298) shows Linux WebKit accessibility document/Studio control reached but incomplete project journey, and Windows installed WebView2 content `localhost ERR_CONNECTION_REFUSED` rather than working application UI. More recent Agent 07 harness work and the integration's own installed-run CI require final exact-build evidence, screenshot inspection, project/source/Study/draft persistence after restart, and cross-project IPC negative tests. **Native installed-app failures and limitations remain separate from provider authorization verdict**.

---



**RELEASE DECISION: FAIL / NO-GO. Five of six independent security jobs PASS; the production combined-host authorization job FAIL.** Completed [workflow #43956319850](https://github.com/MousaXD/scriptotar/actions/runs/43956319850). Its job logs report audit merge checkout `f107974c1e3177ac13d42bd77d7ec79240cfc2b8` and exact **integration PR #114 SHA `46cb36efe2ca4e6ec3f9dd51b7b99c2d59804583`**. Agent 00 imported the independent security test sources into this integration head. [Owner notified immediately](https://github.com/MousaXD/scriptotar/pull/114#issuecomment-6929355984).

| Independent security boundary | Verdict | Executed command and exact evidence |
| --- | --- | --- |
| Provider consent / deterministic races / altered request fields / cancellation / concurrent one-use | **PASS (isolated feature)** — 10 independent + 20 provider unit tests, #118 `d6d738ec84ac6eff1aae26edbd79e44995f2e82b` | `cargo test --manifest-path tests/defragmentation/security/Cargo.toml --test provider_context_race -- --nocapture`; `cargo test --locked -p scriptotar-ai provider_consent` |
| Historical v4/v5/v10 migration, at-most-once v10 external effect states | **PASS** — #119 `9da21c96a62d7a041b64500841b89dc6dc649703` | `cargo test --locked -p scriptotar-db` |
| Canonical three-script drafting, source authenticity, input/receipt tampering, rollback | **PASS** — 32 native writing tests, #121 `467023db81985cc5af45bb6f28d93f2fd3fbd7af` | `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --test study_writing --locked -- --nocapture` with independent injected tests |
| Source-based claim verification and foreign-project restrictions | **PASS** — 25 Rust verification tests + Python boundary, #117 `1ffb0fffad71a6fb2980966e1eec106174f543ff` | `cargo test --locked -p scriptotar-ai verification`; `python3 -m unittest discover -s tests/defragmentation/security -p 'test_*.py' -v` |
| **Integrated v11 migration + three editable draft transaction, receipt immutability, versions and restart** | **PASS**, exact #114 `46cb36e` — 35 native writing tests | `bash audit/tests/defragmentation/security/run_feature_gates.sh combined-v11 114`; DB suite plus `study_writing` tests and both independent additions |
| **FULL production host: real consent-approved durable paid dispatch path** | **FAIL**, exact #114 `46cb36e` | `bash audit/tests/defragmentation/security/run_feature_gates.sh combined 114`; **3 structural tests PASS** (provider export, real `ScopeAuthority/ScopeDispatchGuard`, actual `PaidEffectFence`/pre-send CAS), **1 FAIL:** `test_paid_fence_is_reached_by_a_non_disabled_host_dispatch` at `test_combined_host_wiring.py:102`, `AssertionError: durable paid fence exists but is never constructed on a production host path`. [Specific failed job](https://github.com/MousaXD/scriptotar/actions/runs/43956319850/job/133611702569) |

**Meaning of the failure:** the production `SqlitePaidEffectFence` adapter and host dispatch guard now exist, but `SqlitePaidEffectFence::from_approved_context` has no production consent-dispatch caller, and the only existing `dispatch_provider_angle_consent_disabled` uses `NoTrustedPricing`, `DisabledPaidEffectFence`, `ExistingAiServiceAdapter`. This is a **safe disabled path** rather than evidence of an actual unauthorized paid request. The real approval-to-payload-to-CAS-to-bounded-provider chain has **NOT PASSED**; no provider billable requests or secret keys were used. Source/research/orchestrator writer synchronization coverage and installed native WebViews remain separate **NOT TESTED** gates.

**Additional non-weakening test-runner improvement:** Agent 10 branch commit `45a190aa92ff213f4e9a8f1a89acdec9b590778d` changes `run_feature_gates.sh` to record the structural host failure, continue the real Rust host/DB/verification/writing suites, then exit nonzero if host authorization was red. This patch was committed **after** workflow #43956319850; **NOT TESTED** until Agent 00 imports it and exact-head CI executes. This avoids a structural blocker masking separate runtime regressions while preserving a release-blocking FAIL.

**Ordinary CI P1:** [run #43898984028](https://github.com/MousaXD/scriptotar/actions/runs/43898984028) first attempt failed at `progress_persistence.rs:51` (`transcribing progress was not durably observable`); exact same commit rerun succeeded (`sidecar_progress_is_persisted_before_completion ... ok`, 0.27s). Flaky/timing-sensitive observation suspected, root cause unproven; no persistence tests disabled or weakened.

**Required for PASS:** Agent 00 must connect trusted consent to the *real* pre-send durable fence and host authorization gate, attest exact serialized HTTP request/model/prompt/budget, enforce hard provider limits and authenticated usage receipts, prove every relevant source/Study mutation and revocation shares the synchronization boundary, and run deterministic host races and restart/no-replay. Keep paid egress disabled until certified, then rerun **combined** + **combined-v11** on one final exact integration SHA and separately accept Agent 07 native installed-app evidence. No release approval or main merge by Agent 10.

---

# Agent 10 — 2026-11-17 live integration movement / P0 authorization follow-up

**NO-GO remains in force.** PR #114 advanced **after** the completed six-gate run documented immediately below. The successful historical v11 evidence still belongs to `e65e08e732de21dd29725438afc7712be3deb799`, **not automatically to a newer integration SHA**. [Agent 00 was notified with current findings](https://github.com/MousaXD/scriptotar/pull/114#issuecomment-6929238143).

- **Newest integration SHA inspected:** `bcf14edb322d100ee71f8f5e0ad697505f936778`, following `7d62930df6158d00da40e9e344ef46dde6b2f3cf`. Real production `HostScopeDispatchGuard` and `SqlitePaidEffectFence` trait implementations were added, including `mark_external_dispatched` CAS, and Studio's locked writing helper was made accessible. **Source review only; no exact-head completed independent six-gate execution at this SHA.**
- **Unused fence remains a blocker:** the durable `SqlitePaidEffectFence::from_approved_context` constructor is not invoked in the host's non-test provider dispatch path. The only consent dispatch still uses `NoTrustedPricing`/`DisabledPaidEffectFence`/`ExistingAiServiceAdapter` and cannot send paid requests. Existence of a trait implementation is **not** functional authorization. The Agent 10 host wiring test was strengthened at audit commit `171a7d4c0537fc2199f13ac592953f272c407566` to reject a production implementation that is only defined, not constructed and used by an approved non-disabled dispatch. The new audit test is **NOT TESTED** until GitHub Actions executes that exact audit commit against one known integration commit.
- **Shared lock not exhaustively proved:** Studio handlers and project switching acquire `active_project`, but `JobOrchestrator::start_with_notifier(store.clone())`, `spawn_watchlist_refresher(store.clone())`, `scan_creator` and `import_legacy_data` have SQLite write paths not evidently serialized under the same held mutex. Audit whether any such writes can modify materials included in `StudyScope::material_digest` during egress, and fence or version-check accordingly. This is an **open source-level risk assessment**, not a demonstrated exploit.
- **Progress persistence is intermittent, separate P1:** [ordinary CI run 43898984028](https://github.com/MousaXD/scriptotar/actions/runs/43898984028) attempt 1 **FAIL**, same SHA `e15351e`, assertion `progress_persistence.rs:51:5: transcribing progress was not durably observable`. GitHub Actions rerun **attempt 2 PASS** without test/fixture modification (`sidecar_progress_is_persisted_before_completion ... ok` in 0.27s; job `133604010740`). Hypothesis: transient 150ms sidecar progress state and 10ms polling interact with scheduling/SQLite visibility; root cause **NOT ESTABLISHED**. Do not disable or weaken test.
- **Pending new independent Rust tests:** request fidelity (model/prompt/output cap/deadline), altered consent fields, cancellation at paused reservation and concurrent same-run pre-send CAS winner added at `7d6c769dad1718cd9ea05e5b588cc3e4290e7199`. Until exact-commit CI runs these, record as **NOT TESTED**, not PASS.
- **Final release gate:** combined + combined-v11 must pass on the **same post-integration exact SHA**, using current Agent 10 audit source, with actual host-authorization/admission tests and Agent 07 installed evidence. No real spending, main changes, or release approval by Agent 10.

---

# Agent 10 — 2026-11-17 final combined-security recovery, live status

**Decision: FAIL / NO-GO for the complete production host.** Five isolated/combined-v11 gates pass; the production authorization gate fails. **Do not enable paid dispatch, merge for release, publish, or treat a disabled transport as production egress approval.** This entry supersedes older historical snapshots preserved below.

## Refreshed exact-head inventory

| PR/owner | Head verified at this review | Status |
| --- | --- | --- |
| #114 Agent 00 integration | `e65e08e732de21dd29725438afc7712be3deb799` | DRAFT; full combined host **FAIL** |
| #118 Agent 01 provider consent | `d6d738ec84ac6eff1aae26edbd79e44995f2e82b` | Independent **PASS** feature-only |
| #119 Agent 02 durable external effects | `9da21c96a62d7a041b64500841b89dc6dc649703` | **PASS** native DB suite, already merged into integration |
| #121 Agent 05 canonical writing | `467023db81985cc5af45bb6f28d93f2fd3fbd7af` | Independent **PASS** feature-only |
| #124 Agent 10 tests | `e15351ebb3fc343255ab0ce069e496d48d673e0e` was last completed audit source; new independent Rust test additions committed at `7d6c769dad1718cd9ea05e5b588cc3e4290e7199` | New tests **NOT TESTED** until a new exact-head CI run finishes |

## Completed independent matrix: [six-job run 43898983956](https://github.com/MousaXD/scriptotar/actions/runs/43898983956)

The failed combined job logs printed audit merge checkout `03f44fc7f85b84671a7fcbbd3fbdee36db833adc`, target integration `e65e08e732de21dd29725438afc7712be3deb799`. **These are CI-executed SHAs, not a claim that later audit commits passed.**

| Evidence boundary | Verdict | Exact executed command / qualification |
| --- | --- | --- |
| Provider consent, revocation and independent 6-race suite | **PASS**, feature #118 `d6d738ec` | `cargo test --manifest-path tests/defragmentation/security/Cargo.toml --test provider_context_race -- --nocapture`; `cargo test --locked -p scriptotar-ai provider_consent`. Mock/isolated only, no authenticated paid HTTP |
| Durable migration / v4-v5-v10 / v11 schema | **PASS**, feature #119 and integration #114 | `cargo test --locked -p scriptotar-db`; DB source is real SQLite |
| Canonical writing, 3 separate editable drafts, duplicate acceptance same receipt and IDs, tampered input rejection, foreign Study denial, transactional rollback, immutable revision/restart | **PASS** feature #121 and **PASS integrated v11** #114 | `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --test study_writing --locked -- --nocapture` with independent `canonical_writing_additional_tests.rs` and `canonical_v11_integrity_tests.rs` in `combined-v11:114`; 35 integrated writing tests |
| Independent claim/source authenticity | **PASS** feature #117 | `python3 -m unittest discover -s tests/defragmentation/security -p 'test_*.py' -v`; `cargo test --locked -p scriptotar-ai verification`. No combined installed-app verified factual corroboration |
| **Production host authoritative `ScopeDispatchGuard` + `PaidEffectFence` CAS** | **FAIL** integration #114 | `bash audit/tests/defragmentation/security/run_feature_gates.sh combined 114`. Two failed Python tests: `test_actual_host_implements_shared_scope_guard_and_authority` — `no real host ScopeDispatchGuard implementation (mock/default denial is not integration)`; `test_actual_host_has_durable_paid_claim_and_pre_send_cas_adapter` — `host has no PaidEffectFence implementation bridging consent to v10 ledger` |
| Real shared synchronization for project switches, source/brief/evidence/transcript edits, cancellation and revocation during outbound HTTP | **NOT TESTED / no approved host implementation** | Consent source `crates/scriptotar-ai/src/provider_consent.rs` requires `lock_dispatch_scope`, but `AppServices` only implements `current_scope`; no verified same lock across all mutations |
| Actual outbound serialized body, approved model/prompt/budgets, unknown-price refusal and authentic provider usage receipts | **NOT TESTED as real host transport** | Host `services/provider_consent.rs` uses `NoTrustedPricing`, `DisabledPaidEffectFence`, `ExistingAiServiceAdapter`; safe disabled egress, not a live bounded-transport certificate. New capture/mutation mock tests committed after this run are pending |
| Uncertain-effect replay across crash/timeouts and strict one-winner final pre-send CAS | **PASS DB/isolated effects only; NOT TESTED host dispatch** | `mark_external_dispatched` exists in v10 DB and host `agent_effect_driver.rs`, but is not connected to `ProviderConsent::PaidEffectFence` at the actual host |
| Installed Linux WebKitGTK and Windows WebView2 UI/IPC, logs/secrets and release package | **NOT TESTED / not certified at exact integration SHA** | Separate Agent 07 installed-app QA evidence required; no user data or paid keys used in this security audit |

## Additional ordinary-CI failure (separate from P0 authorization)

[CI run 43898984028](https://github.com/MousaXD/scriptotar/actions/runs/43898984028), job `133410134634`, on security PR source `e15351e` fails `cargo test -p scriptotar-orchestrator --test progress_persistence`: `crates/scriptotar-orchestrator/tests/progress_persistence.rs:51:5: transcribing progress was not durably observable` (`sidecar_progress_is_persisted_before_completion`). The test file is **byte-identical** on PR #124 and integration `e65e08e`, including the 300 × 10ms polling loop; its fixture `fixture_sidecar.py` holds the transient 50% transcribing state for only 150ms. Scheduling or SQLite observation timing is one hypothesis, **not yet a proven cause**. A GitHub Actions rerun of the failed job was requested to assess reproducibility. **Do not disable the test, increase tolerances without evidence, or conflate this CI defect with the combined host authorization finding.** Agent 00/orchestrator owner to investigate with durable event ordering and job transitions.

## New independent tests (pending fresh exact-head execution)

`provider_context_race.rs` now additionally checks exact approved provider/model/prompt/output limit/deadline propagation to the bounded provider interface; rejects mutated model, prompt and budgets before reservation; deterministically cancels a paused reservation; and asserts a concurrent same-run dispatch has only one pre-send CAS winner. These are **mock provider/fence security contract tests**. They do **not** prove the exact serialized HTTP body or the DB-backed host implementation. Keep both combined gates as release blockers until Agent 00 supplies that proof.

**Owner notification:** [Agent 10 posted exact reproduction to PR #114](https://github.com/MousaXD/scriptotar/pull/114#issuecomment-6928990118). Agent 00 owns host production wiring; do not modify main or independently merge the feature branches. Require both `combined:114` and `combined-v11:114` **PASS on the same future integration SHA**, plus Agent 07 native acceptance and trusted provider transport/receipt proof before changing this NO-GO.

---

# Agent 10 — final exact-head six-job independent security report, 2026-11-16

**Security disposition: NO-GO. Five of six independent jobs PASS, final combined-host job FAIL.** This report is tied to the completed [run #43897845121](https://github.com/MousaXD/scriptotar/actions/runs/43897845121), which printed independent audit checkout SHA `8fa3effc7def65dac6dd1908f245196aa271f345` (from audit source PR #124 `b1d723ab6ac6d0a00e76071142d6df2a269871e8`). Subsequent audit/report-only commits are not automatically certified.

| Independent gate | Status and exact tested feature SHA | Test command / evidence | Owner and remaining qualification |
| --- | --- | --- | --- |
| Provider consent, revocation, cancellation, linearization mocks and unauthenticated receipt replay | **PASS** at #118 `d6d738ec84ac6eff1aae26edbd79e44995f2e82b` — 6 independent + 20 provider unit tests | `cargo test --manifest-path tests/defragmentation/security/Cargo.toml --test provider_context_race -- --nocapture` and `cargo test --locked -p scriptotar-ai provider_consent`; [job run](https://github.com/MousaXD/scriptotar/actions/runs/43897845121) | Agent 01 / 00; isolated feature/mocks are not real Tauri host transport certification |
| Historical v4/v5 upgrades, v10 effects migration, SQLite ledger rollback, receipts | **PASS** at #119 `9da21c96a62d7a041b64500841b89dc6dc649703` | `cargo test --locked -p scriptotar-db`; [job run](https://github.com/MousaXD/scriptotar/actions/runs/43897845121) | Agent 02 / 00; real database tests, not live provider |
| Canonical three-script writing, forged source lineage, duplicate receipt, uncertainty and transaction rollback | **PASS** at #121 `467023db81985cc5af45bb6f28d93f2fd3fbd7af` — 32 tests | `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --test study_writing --locked -- --nocapture`; [job run](https://github.com/MousaXD/scriptotar/actions/runs/43897845121) | Agent 05 / 00; feature-only but actual SQLite sink |
| Verified claim prohibition on untrusted caller draft, independent origins and high-stakes source restrictions | **PASS** at #117 `1ffb0fffad71a6fb2980966e1eec106174f543ff` — 25 verification Rust tests and Python host-boundary suite | `python3 -m unittest discover -s tests/defragmentation/security -p 'test_*.py' -v` and `cargo test --locked -p scriptotar-ai verification`; [job run](https://github.com/MousaXD/scriptotar/actions/runs/43897845121) | Agent 04 / 00; independent real-world fact certification remains disabled |
| **Integrated v11** historical upgrade/collision rollback, 3-draft atomicity, immutable receipt/version, foreign project, restart | **PASS** at real #114 integration `e65e08e732de21dd29725438afc7712be3deb799` — DB suite + **35 writing tests**, including all 3 new v11 tests | `bash audit/tests/defragmentation/security/run_feature_gates.sh combined-v11 114` runs `cargo test --locked -p scriptotar-db` and `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --test study_writing --locked -- --nocapture`; [job run](https://github.com/MousaXD/scriptotar/actions/runs/43897845121) | Agent 00 / 05 / 10; proves integrated v11 source transaction, **not** paid provider egress or full host |
| **Full combined Tauri host authorization and durable pre-send effect CAS** | **FAIL** at same #114 `e65e08e732de21dd29725438afc7712be3deb799` | `bash audit/tests/defragmentation/security/run_feature_gates.sh combined 114` fails structural `test_actual_host_implements_shared_scope_guard_and_authority` — **`no real host ScopeDispatchGuard implementation`**; `test_actual_host_has_durable_paid_claim_and_pre_send_cas_adapter` — **`host has no PaidEffectFence implementation bridging consent to v10 ledger`**. [Failed run](https://github.com/MousaXD/scriptotar/actions/runs/43897845121) | **Agent 00** owns actual shared authorization boundary, with Agents 01/02. Provider consent module export/registration check PASS, but `NoTrustedPricing` + `DisabledPaidEffectFence` keeps current paid dispatch safely disabled |

### Additional release barriers — NOT TESTED

Installed Linux WebKitGTK and Windows WebView2 native accessibility/IPC security, shared authorization lock under all project/Study/source mutators during real network sends, provider-enforced token and spend limits, authentic live HTTP usage receipts, cross-process crash/acknowledgement behavior, complete secret/config/log forensics and trusted external factual corroboration have not been certified at the combined SHA. Agent 07 / Agent 00 / Agent 01 must provide independent exact-head evidence. The pass on a *mock* outbound provider does not confer authority to enable spending. No API keys or paid network test were used by Agent 10.

### Release requirements

Agent 00 must add a **single authoritative host `ScopeDispatchGuard` synchronization boundary** for active project/Study/source edits and a **durable host `PaidEffectFence` with one-winner `mark_dispatched_once`** and authenticated receipt settlement. Preserve cancellation linearization and deny replay after uncertain dispatch; do not drop the fail-closed gates. After integrating these, run both `combined-v11:114` and `combined:114` against **the same new final integration SHA** plus native installed-app and bounded provider transport acceptance. PR #124, #114 and paid egress remain **DRAFT / NO-GO** until proven.

---

# Agent 10 — latest completed six-boundary security checkpoint (2026-11-16)

**FINAL RELEASE STATUS: NO-GO.** PR #124 is an independent security regression branch, not a release approval. Audit branch head when this addendum was prepared: `b1d723ab6ac6d0a00e76071142d6df2a269871e8`. The newest sixth `combined-v11` job is authored but has **no completed run at this audit SHA**. Preserve the current test failures and keep paid HTTP dispatch disabled.

## Last completed executable matrix — [run 43896067292](https://github.com/MousaXD/scriptotar/actions/runs/43896067292)

CI audit checkout printed `5c8c2b55886626f56679702b10ec66df3d3a6b99`, from PR #124 source head `02e19dcd6d5202dfa8627708921ebb825e242249`. Results are limited to **the checked-out feature SHA**, not a newer PR commit.

| Boundary | Exact feature SHA | Verdict | Native command, failure/result, owner |
| --- | --- | --- | --- |
| Consent, revocation, cancellation, pre-send CAS and unverified receipt no-replay | #118 `d6d738ec84ac6eff1aae26edbd79e44995f2e82b` | **PASS — isolated feature only** | `cargo test --manifest-path tests/defragmentation/security/Cargo.toml --test provider_context_race -- --nocapture` (**6 passed**); `cargo test --locked -p scriptotar-ai provider_consent` (**20 passed**); Agent 01, integration Agent 00 |
| Historical v4/v5 and v10 SQLite upgrades / ledger / rollback | #119 `9da21c96a62d7a041b64500841b89dc6dc649703` | **PASS — effects feature only** | `cargo test --locked -p scriptotar-db`; Agent 02 + Agent 00 |
| Real canonical candidate store, original independent four tests, source fingerprint, forged lineage, duplicate receipt, rollback | #121 `467023db81985cc5af45bb6f28d93f2fd3fbd7af` | **PASS — isolated feature only** | `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --test study_writing --locked -- --nocapture` (**32 passed, 0 failed**); Agent 05 + Agent 00 |
| Source-origin independence, high-stakes claim safeguards and unbound caller-draft quarantine | #117 `1ffb0fffad71a6fb2980966e1eec106174f543ff` | **PASS — verification feature only** | `python3 -m unittest discover -s tests/defragmentation/security -p 'test_*.py' -v`; `cargo test --locked -p scriptotar-ai verification`; Agent 04 + Agent 00 |
| Full production Tauri shared-host dispatch/paid-effect wiring | #114 `3aeaa922a06504f44aa25d62182a9b9966ebc8ac` | **FAIL — release-blocking** | `bash audit/tests/defragmentation/security/run_feature_gates.sh combined 114`; `test_actual_host_implements_shared_scope_guard_and_authority` **FAIL** (host has no `ScopeDispatchGuard`); `test_actual_host_has_durable_paid_claim_and_pre_send_cas_adapter` **FAIL** (no host `PaidEffectFence` and `mark_dispatched_once`). Provider policy export/registration structural test passed; Agent 00 + Agent 01 + Agent 02 |
| Native integrated **v11** historical migrations/collision, immutable receipts/version, three-draft atomicity, restart, forged project | newest #114, checkout must be recorded by CI | **NOT TESTED at newest audit SHA** | New independent `combined-v11:114` executes `cargo test --locked -p scriptotar-db` and `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --test study_writing --locked -- --nocapture`, with original four and new three adversarial native tests appended to true integration checkout. Agent 00 (owner) + Agent 10 (audit) |
| Native installed Windows WebView2/Linux WebKitGTK, real provider usage receipt authenticity, secret/log forensics, real egress budgeting and cross-process lock | no certified combined SHA | **NOT TESTED** | Mandatory installed-app and trusted-adapter acceptance; Agent 00 / 01 / 07 / 10 |

## New adversarial test inventory and trust boundaries

- `tests/defragmentation/security/canonical_v11_integrity_tests.rs`: immutable v11 `study_writing_batches` receipt UPDATE/DELETE, first structured `study_writing_revisions` UPDATE/DELETE, foreign-project valid-shape receipt rejection and DB restart/reopen. Runs against true production SQLite tables in **combined-v11 and combined**, never isolated #121 where v11 isn't registered.
- `test_combined_host_wiring.py`: bounded-failure structural checks for real `ScopeAuthority`, `ScopeDispatchGuard`, durable host `PaidEffectFence::mark_dispatched_once`, exported provider policy and service graph. Full combined job compiles `cargo test --locked -p scriptotar-desktop --lib`. Structural checks are *necessary but insufficient*: they do not prove that every canonical project/Study/source mutator takes one process-wide read/write gate during the outbound send.
- Existing `provider_context_race.rs` now matches the trusted provider request-ID API, tests held guard/source revocation and single-use consent, and rejects replay after an unauthenticated paid acknowledgement. Zero live requests or credentials used; passing this mock contract is **not** real HTTP billing verification.
- Separate `combined-v11` gate was added so a missing paid adapter cannot hide data migration or three-draft regressions. Its PASS would **not** override a full `combined` FAIL, and its checks must be verified on one exact-head run.

## Source-level inspection limitations

At #118 `d6d738e`, core provider consent enforces bounded prompt/response/output tokens, requires verified provider request ID and metered usage and denies unknown pricing via `NoTrustedPricing`; reviewed core source showed no explicit print/logger calls in that module. At #114 `772d10b`, production host adapter still selected `NoTrustedPricing`, `DisabledPaidEffectFence`, `ExistingAiServiceAdapter`, with no real shared dispatch guard and no active paid egress. This is a **narrow inspection only**: no global credential grep, installed binary/log forensic scan, live pricing/transport assessment or cross-process lock proof was executed. Do not mark secret leakage or budget bypass fully cleared from this static review.

**Required next owner action:** Agent 00 must compose real shared authorization guard + durable paid-effect CAS adapter with all canonical mutations, keep disabled until bounded authentic receipts and timing are verified, merge the corrected writing fixture into combined source, and obtain green `combined-v11` **and** `combined` at the same final integration SHA. Release stays NO-GO.

---

# Agent 10 — combined-security refresh, 2026-11-16 (latest checkpoint)

**Release decision: NO-GO.** Independent security regressions are now executable in PR #124, but neither the newer audit changes nor the final *combined host* have a passing exact-head run. Keep production paid egress disabled and PR #114 draft. This checkpoint supersedes older current-head tables farther below.

## Latest refreshed PR heads at audit checkpoint

| Owner | PR | Observed exact SHA | Current interpretation |
| --- | --- | --- | --- |
| Agent 00 integration | #114 | `e965bb3633eef76d5fc30ef37757bb437afb4f4c` | **FAIL / NOT CERTIFIED**; provider-consent implementation absent on this head, regardless of partial private v11 writing integration |
| Agent 01 consent | #118 | `544aa2ee2f63f0a04e79a4eca6e45f84d44bfde4` | Newer than last independently passing provider snapshot; **NOT TESTED** by independent runner at this exact SHA |
| Agent 02 effects | #119 | `9da21c96a62d7a041b64500841b89dc6dc649703` | Merged; independent native migration gate **PASS** at this exact SHA |
| Agent 05 writing | #121 | `6d9e3ca75ebcc65be05678ea5a99cc688d515a96` | Replaced the failing fixture's synthetic source fingerprints with real DB metadata; **NOT TESTED** by independent runner at this exact SHA |
| Agent 04 verification | #117 | `1ffb0fffad71a6fb2980966e1eec106174f543ff` | Independent verification gate **PASS** at this exact SHA, not full host certification |
| Agent 10 audit | #124 | `2100f65b3f93bce7a0eb358367e6dce6d3758e81` | Three more native v11 adversarial tests and combined-host structural/host-library compilation gate; new exact-head CI **NOT TESTED** yet |

## Reproducible exact-commit evidence: independent workflow 43882281063

[Full workflow and jobs](https://github.com/MousaXD/scriptotar/actions/runs/43882281063). The checkout audit SHA printed in those logs is `73ab34b2017da944321ec5fb1a4cc5c2ab465ff3` (CI pull-request checkout); feature SHAs are printed by the runner. The SHA at which a *job executed*, not its subsequent branch head, controls status.

| Case | Result at tested SHA | Command / evidence | Responsible owner |
| --- | --- | --- | --- |
| Post-reservation source revocation, concurrent cancellation and guarded mock dispatch | **PASS (feature-only)** @ #118 `1852dab7a8f7d9032c66095d85db88a6b7e31bba`: 5 independent and 18 provider tests | `cargo test --manifest-path tests/defragmentation/security/Cargo.toml --test provider_context_race -- --nocapture`; `cargo test --locked -p scriptotar-ai provider_consent`; [run](https://github.com/MousaXD/scriptotar/actions/runs/43882281063) | Agent 01; Agent 00 for host linearization |
| Historical v4/v5 SQLite, v10 receipt/cancellation, migration integrity | **PASS (effects feature)** @ #119 `9da21c96a62d7a041b64500841b89dc6dc649703` | `cargo test --locked -p scriptotar-db`; [run](https://github.com/MousaXD/scriptotar/actions/runs/43882281063) | Agent 02 / Agent 00 |
| Canonical writing source identity | **FAIL** @ #121 `81dafac2713b21b8aacf215d0f25c99dc63e7220`: `canonical_gateway_adapter_reuses_real_source_rows_and_rejects_cross_project` failed at `study_writing.rs:714`, expected synthetic fixture fingerprint/revision while SQLite returned canonical source metadata; 31 passed, 1 failed | `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --test study_writing --locked -- --nocapture`; [run](https://github.com/MousaXD/scriptotar/actions/runs/43882281063) | Agent 05; refreshed `6d9e3ca` needs executed proof |
| Independent verification: source lineage and distrust of model-supported claims | **PASS (verification feature)** @ #117 `1ffb0fffad71a6fb2980966e1eec106174f543ff` | `python3 -m unittest discover -s tests/defragmentation/security -p 'test_*.py' -v`; `cargo test --locked -p scriptotar-ai verification`; [run](https://github.com/MousaXD/scriptotar/actions/runs/43882281063) | Agent 04 / Agent 00 |
| Final combined host | **FAIL** @ #114 `88419a813d4deb00978a90b0d8756b47b058842f`: `NOT INTEGRATED: target/crates/scriptotar-ai/src/provider_consent.rs` (exit 4) | `bash audit/tests/defragmentation/security/run_feature_gates.sh combined 114`; [run](https://github.com/MousaXD/scriptotar/actions/runs/43882281063) | Agent 00 / Agent 01 |

## Newly added release-blocking tests on Agent 10 branch

- `canonical_v11_integrity_tests.rs` is appended **only to the combined integration**: real SQLite immutable batch receipt UPDATE/DELETE denial, immutable first structured draft version, cross-project forged writing receipt denial, and exact restart reopen. Existing `scriptotar-db` tests already cover v10-to-v11 malformed table collisions and rollback; the combined gate runs these without changing historical migration definitions.
- `test_combined_host_wiring.py` requires the provider policy to be exported and referenced in central Tauri services, actual host `ScopeAuthority` + `ScopeDispatchGuard` implementations, and a host `PaidEffectFence` implementing pre-send `mark_dispatched_once`. A file merely present in a detached feature branch is insufficient; the combined job also executes `cargo test --locked -p scriptotar-desktop --lib`. These structural assertions are necessary but **not sufficient** proof that every source/project mutator shares the egress lock.
- Existing independent tests still cover revocation at reservation, concurrent source changes, missing guard fail closed, duplicate provider claims, wrong-project lineage, model-faked Supported claims, stale source fingerprints, tampered duplicate runs and three-draft rollback.

**New cases: NOT TESTED until a completed exact-head run records them.** The current integration SHA still lacks provider consent, so even a clean v11-only run cannot override the combined NO-GO.

## Outstanding threats and required proof

- **Agent 00 + Agent 01 + Agent 02:** install one production-authoritative active-project/Study/source synchronization boundary, hold the dispatch scope guard through the real send, enforce durable pre-send DISPATCHED CAS and unknown-pricing denial. A mock guard does not certify production HTTP egress. The source may be intentionally disabled; do not enable real spending to satisfy this test.
- **Agent 05 + Agent 00:** run corrected canonical writing tests at exact latest #121 and then **against combined v11**, including historical upgrades, malformed collision, partial rollback, immutable receipt, duplicate replay, restart, stale Study, foreign project and edited source.
- **Agent 04 + Agent 00:** require independent-origin corroboration from stored immutable source + trusted draft body, not caller-authored text or model claims. No API may promote an unbound caller draft to Verified.
- **Agent 07 + Agent 00:** installed Windows WebView2 and Linux WebKitGTK UI/IPC security, no secrets in logs, bounded paid provider transport/cost receipt authenticity and native package-specific authentication all remain **NOT TESTED**.
- **Agent 10:** run independent matrix again after owner heads advance; record checked-out audit SHA and all feature SHAs, command, logs, and PASS/FAIL/NOT TESTED. A historical green run or source review cannot certify the current host.

---

# Agent 10 — 2026-11-16 independent executable regression update

**Latest audit work:** independent security CI added on PR #124. **NO-GO: combined production security is NOT TESTED.** This supersedes any earlier "current head" rows below; they are retained as dated historical evidence.

## Refreshed owner heads at this update

| PR | Head observed | Integration state | Agent 10 disposition |
| --- | --- | --- | --- |
| #114 — Agent 00 | `88419a813d4deb00978a90b0d8756b47b058842f` | Draft integration, in progress | **NOT TESTED** as a completed combined app |
| #117 — Agent 04 | `1ffb0fffad71a6fb2980966e1eec106174f543ff` | Separate | Source review present; independent exact-head native gate queued |
| #118 — Agent 01 | `5853594ca208088cca59776e9a30c9334d89d54f` | Separate | New shared dispatch-scope trait and fail-closed default; native concurrency gate queued |
| #119 — Agent 02 | `9da21c96a62d7a041b64500841b89dc6dc649703` | **Merged into integration** | Previous v5 break no longer current claimed failure; re-test native v4/v5/v10 now |
| #120 — Agent 03 | `747b41cd5c2431a146a0f43d5b68c5b98b26a415` | Separate | Not yet combined |
| #121 — Agent 05 | `d030af8517786d425dfd0ea8b1aa1541a1c84180` | Separate, changing concurrently | New independent canonical SQLite adversarial tests queued |
| #124 — Agent 10 | `3bf648d90cb89d01cd1eae998474ce2ed07ec1fa` | Independent PR | CI workflow authored/triggered; **no completed new gate result at this snapshot** |

**CI execution link:** https://github.com/MousaXD/scriptotar/actions/workflows/independent-security-regressions.yml

The new dedicated workflow runs five checks from exact GitHub PR refs: #118
provider races, #119 historical SQLite migrations, #121 canonical three-draft
transaction, #117 verification, and **#114 combined integration**. The combined
job exits nonzero if required production feature modules are missing. Exact
commit hashes print in each job log. The main workspace's `cargo test
--workspace` omits the nested provider test crate; this separate check must be
required by Agent 00 and re-run on the final merged integration SHA.

## New independent adversarial coverage

- **Provider authorization + external effect:** existing deterministic
  post-reservation source change, explicit two-thread revocation at the
  reservation completion boundary, successful legitimate guarded dispatch
  where the source write lock cannot be taken during egress, and default
  missing-scope-lock denial. All use fake provider/ledger, zero external egress.
  **Source reviewed:** PR #118 now validates context again and provides
  `ScopeDispatchGuard` plus a required `mark_dispatched_once` CAS. A
  fail-closed interface alone is not proof that the Tauri host and every source
  mutator share one real synchronization boundary.
- **Migration and historical data:** invoke the actual
  `scriptotar-db` native test suite, including previous v4/v5 fixtures,
  v10 migration invariants, and project isolation. The old breakage at
  `dd755bbe` remains **historical**, not evidence that the merged
  `9da21c96` fails.
- **Canonical three-draft writing:** compile the production persistence module
  using Agent 05's own canonical SQLite test fixture and append independent
  assertions for source-fingerprint revocation before commit, forged lineage,
  "supported" model assertions downgraded to unverified, duplicate receipts
  and payload tampering (no fourth draft). These are not mock-only storage
  assertions. PR #121 remains unmerged and may advance again.
- **Claim trust:** core verification native tests cover exact source hash,
  independently reviewed origins, high-stakes claims, stale/foreign reviews and
  forged citations; independent Python host contract refuses Verified for an
  unbound draft. The Python contract is not a substitute for end-to-end native
  permissioned IPC testing.

## Matrix — snapshot before new independent jobs complete

| Security property | Status | Basis |
| --- | --- | --- |
| Provider never egresses after revocation linearizes | **NOT TESTED** on final production path | Rust gate now queued; real shared host mutex not integrated |
| Provider double-claim / dispatch / receipt exactly once | **NOT TESTED** combined | Durable v10 branch merged; new mock/canonical tests queued |
| Real historical v4/v5 upgrade under v10 | **NOT TESTED** independently on merged code | Native full DB test queued; no new exact-head result yet |
| Three canonical drafts atomic + duplicate-proof | **NOT TESTED** independently | Newly authored native tests queued, owner fixture exists |
| Forge source/claim/status cannot promote to verified | **NOT TESTED** independently on combined | Isolated source guard inspected; native core gate queued |
| Cross-project Tauri/MCP IDOR and secret leakage | **NOT TESTED** end-to-end | Additional host/installed-app suite needed |
| Unknown pricing defaults deny paid egress | **PASS (source-level design only)** | `NoTrustedPricing` and no production trusted paid catalog; do not count as dynamic proof |
| Installed Linux/Windows native WebView security | **NOT TESTED** | Agent 07 independent harness belongs to release QA |
| Final combined integration SHA | **NOT TESTED** | Combined CI gate fails closed until modules integrated and it is rerun |

**Do not treat a queued gate, historical green workflow, isolated feature check,
or a disabled paid adapter as an executed production security certificate.**

---

# Agent 10 — independent security/adversarial QA handoff

**Audit date:** 2026-11-16; refreshed after concurrent feature-branch updates.  **Disposition: NO-GO for integration/release approval.** This is a draft security review, not a penetration-test certificate. Do not merge or publish based on upstream green CI alone. No feature-owner production files were changed.

## Exact-head evidence (refreshed after owners advanced their branches)

| Work | Current head at audit refresh | Exact-head result at refresh |
| --- | --- | --- |
| Integration baseline PR #114 | `3009989a3aa73248b6432ba0cd84821c59110993` | 9/9 success, baseline only |
| Verification PR #117, Agent 04 | `0847a06cf5af0d2432ad15495a6cd6110079f954` | 7 queued; prior `9267066` had 7/7 success |
| Consent PR #118, Agent 01 | `cdcd07d61f219e5be08052eea6ccddcafcc3c7e9` | Rust, CI, security succeeded; 4 remaining queued/in-progress; prior `bcb3f3a` had 4 failed |
| Effects PR #119, Agent 02 | `dd755bbea32dfe1afd601a50cb6422d5f77569c4` | Rust, CI, MCP failed; security success; 3 packaging/migration jobs in progress |
| Research PR #120, Agent 03 | `3df1461a6747d9f3583596331522b016bc5b106e` | Jobs queued/in progress; prior `62600e2` 7/7 success |
| Writing PR #121, Agent 05 | `c63cf0c7da5c6f33d57a67b9d1175d45c202efc5` | Jobs queued/pending; prior `47fe396` 7/7 success |

**CI is not certified for the current five-PR set.** Successful workflows on previous SHA cannot be transferred to a newer SHA. Each branch is still separate/unmerged; the integration baseline does not contain the five new feature PRs. Statuses may change as owners push new commits. All findings below identify the exact examined SHA.

## Blocker findings (prioritized)

### SEC-10-01 — Changed Study after consent check can still reach provider (P1, FAIL — source-code finding)

- **Owner:** Agent 01, with Agent 02 for paid-fence coordination and Agent 00 for host synchronization.
- **Affected heads:** PR #118 @ `bcb3f3a7d08426ee013530236ea2252012fa4da3` and **reconfirmed** @ `cdcd07d61f219e5be08052eea6ccddcafcc3c7e9`.
- **Component:** `crates/scriptotar-ai/src/provider_consent.rs`, `ConsentBoundary::dispatch`.
- **Reproduction:** Approve an exact prompt and Study/material digest. Supply a deterministic test fence whose `reserve_once` changes the authoritative Study material digest *after* `validate_context` has returned successfully, before the `BoundedProvider::generate_once` call. The dispatch code does not revalidate the scope after this reservation and before sending. A counting mock provider is still invoked.
- **Expected:** An invalidated scope prevents provider invocation, even when the effect reservation has already committed. Safely settle/mark the reserved effect uncertain; never treat denial as a replay permit.
- **Observed code path:** Validates the scope, obtains the durable reservation, drops the consent-record lock, then invokes `generate_once` without a new scope check. This is a deterministic TOCTOU window; concurrency between DB/active project changes and send is not fenced by the in-memory consent mutex.
- **Impact:** If the host later enables this path, user changes to Study/source access or project selection can race with private prompt egress and paid spend. **Not a current paid-provider exploit:** PR #118 deliberately uses `NoTrustedPricing`, `DisabledPaidEffectFence`, and `ExistingAiServiceAdapter` to deny production dispatch.
- **Adversarial regression:** `tests/defragmentation/security/tests/provider_context_race.rs::changed_study_after_effect_reservation_must_not_egress`. A rejecting-fence control is included. **Not executed locally**: Rust toolchain and feature checkout are unavailable in this audit runtime. The test is expected to fail on this exact feature SHA; run it before accepting the fix.
- **Remediation:** Agent 01 + 02 + 00 should define an atomic approval/dispatch epoch or hold the authoritative lock across the final send boundary, with cancellation/context invalidation enforced at the last permitted point. A single extra recheck narrows but does not eliminate all possible races. Confirm the final integrated transport cannot launch an HTTP request after revocation at the defined linearization point.

### SEC-10-02 — Noncanonical caller draft is explicitly quarantined (PASS for Verified gating; P2 remaining data-binding work)

- **Owner:** Agent 04 and Agent 00.
- **Affected earlier head:** PR #117 @ `9267066ce3576431b666c6c8cb0f20ba8eaad228` accepted a caller-authored ScriptDraft with a matching canonical UUID/Study/revision for claim extraction without content equality. This was a material integration risk.
- **Current mitigation inspected:** PR #117 @ `0847a06cf5af0d2432ad15495a6cd6110079f954` explicitly sets `draft_content_bound: false` for that read-only Studio adapter. `verify_claims` requires `ctx.draft_content_bound` for `VerificationStatus::Verified` and always sets `publication_review_required = true`. The original untrusted claim/report mismatch still exists as a *noncertifying analysis* but **cannot currently become a Verified claim**. Do not describe the earlier trust issue as an unmitigated Verified-promotion exploit.
- **Independent security contract:** `tests/defragmentation/security/test_study_claim_boundary.py` now tests the fail-closed alternative: if caller draft text is analyzed, the host must explicitly mark it unbound and the core must refuse Verified. Test **authored, not run** against a checkout.
- **Further work:** For meaningful trusted saved-draft certification, derive claims from an immutable canonical body or verify a trusted body hash with a consistent snapshot, then add native same-ID/same-revision/different-body tests. Keep current fail-closed status until such work is merged.

### SEC-10-03 — New durable-effects migration breaks a supported legacy upgrade (P0 data migration / FAIL)

- **Owner:** Agent 02 (new external-effect schema/module registration); Agent 00 (central migration compatibility and release gate).
- **Affected head:** PR #119 @ `dd755bbea32dfe1afd601a50cb6422d5f77569c4` (not the earlier green `273a8ded0`).
- **Exact reproduced CI failure:** CI workflow `43873366062`, job `133319993960` fails native Rust test `scriptotar_141_core_v5_database_upgrades_without_losing_projects_or_jobs` at `crates/scriptotar-db/tests/content_studies.rs:617`. Reported error: `database schema version 5 does not match migration ledger (highest 10, count 6); refusing to modify existing data`. The new migration registration changes the expected ledger without preserving historical v5 upgrade support. This is an observed failure on real native SQLite fixture data, not a theoretical test.
- **Impact:** Existing user databases on a supported historical schema may be refused instead of upgraded. This is release-blocking data availability / migration compatibility, even though fail-closed behavior prevents silent corruption.
- **Additional CI failure:** Rust foundation run `43873366184`, job `133319994468` fails Rustfmt in `crates/scriptotar-db/src/root.rs`. MCP E2E also failed at this exact head. Windows/Linux/Tauri checks were still in progress when inspected.
- **Required fix:** Preserve the valid historical migration ledger and integrate the new additive external-effects schema through the canonical versioned upgrade path; restore the supported v5→current fixture and all historical upgrade tests. Run `cargo fmt --all`, `cargo test -p scriptotar-db --test content_studies --locked` and complete all exact-head CI. Do not weaken schema-integrity checks or erase historic data.
- **Evidence:** https://github.com/MousaXD/scriptotar/actions/runs/43873366062 and https://github.com/MousaXD/scriptotar/actions/runs/43873366184.

### SEC-10-04 — Earlier provider-consent Rustfmt failures superseded (historical FAIL, current CI INCOMPLETE)

- **Owner:** Agent 01.
- **Historical head:** PR #118 @ `bcb3f3a7d08426ee013530236ea2252012fa4da3` had four failed workflows, with Rustfmt diff near `provider_consent.rs:503` in job `133182271393`.
- **Current head:** `cdcd07d61f219e5be08052eea6ccddcafcc3c7e9` has successful Rust foundation and CI checks as observed; other jobs were still queued/in-progress. **Do not continue reporting the historical rustfmt failure as current.** Full exact-head acceptance is still NOT TESTED until every required job finishes.

## Additional independent audit evidence and boundaries

| Area | Status | What was checked / what is missing |
| --- | --- | --- |
| Project, Study, source authorization / IDOR | **NOT TESTED** end-to-end | Reviewed scoped host adapters; PR #117 has project/draft membership and revision checks; PR #120 joins canonical source IDs with project/Study. No adversarial integrated multi-process IDOR run. SEC-10-02 is a **FAIL** for caller-body binding. |
| Transcript/source/model prompt injection | **NOT TESTED** | PR #120 treats source text as untrusted metadata; PR #121 uses typed parsers. No live model/tool escalation attempt, Unicode delimiter attack suite, or installed-app validation. |
| Tauri IPC / MCP | **PASS (narrow static review)** | Baseline MCP HTTP uses loopback-only binding, bearer or URL token, Origin allowlist when Origin supplied, bounded request handling and a post-body revocation recheck (`crates/scriptotar-mcp/src/http.rs`). This is not a full MCP permission/path-sandbox penetration test. |
| Provider selection, approval expiry, changed-context invalidation and egress | **FAIL (SEC-10-01)** | Approval and expiry are checked in the model; a revalidation/egress race remains at the final transport boundary. Paid path disabled. |
| Token ceilings, US$ budgets, unknown pricing and keys | **PASS (narrow static review)** | PR #118 has bounded prompt/output, trusted-price cost computation, `NoTrustedPricing` fail-closed, no key in `ProviderIntent`; no trusted real-provider billing/receipt execution. **NOT TESTED** against live API pricing or provider-enforced hard caps. |
| SQLite reservation, simultaneous dispatch, late acknowledgement and duplicate effects | **PASS (existing mocked CI tests)** | PR #119 includes `BEGIN IMMEDIATE` reservation, dispatch CAS and late ACK/replay tests that compiled in CI. **NOT TESTED** independently against combined real-provider retries or SQLite crash injection. |
| Crash recovery, cancellation, provider uncertainty | **PASS (existing mocked CI tests)** | PR #119 includes uncertain-state and provider-proven-not-executed retry coverage; acceptance still depends on trusted provider evidence. **NOT TESTED** against real provider/cancel races. |
| Fact claims, provenance, human reviews | **PASS (narrow fail-closed code review)** | Latest #117 forces unbound caller draft to be unverified and publication-review-required. Authenticated independent fact reviews and canonical draft-body certification remain **NOT TESTED**. |
| Candidate transaction atomicity / cross-process races | **NOT TESTED** | PR #121 defines `AtomicCandidateStore`; its SQLite sink is a **test fixture**. The three-candidate canonical production transaction has not been registered. |
| Study and draft revision integrity | **NOT TESTED** as trusted content certification | Latest #117 detects unbound draft via a hard false flag; it does not certify caller text. Historical same-ID/same-revision mismatch remains a data-lineage issue, not a Verified bypass. |
| Secrets, private source data, exports and logs | **NOT TESTED** | Provider key currently passed only at dispatch. No forensic real-log, export redaction, clipboard, OS crash-dump or installed-app filesystem audit. |
| Dependency advisories, workflow pins, installed app | **NOT TESTED** fully | Baseline security-hygiene workflow checks immutable GitHub Actions pins and tracked content; it does **not** by itself establish cargo/npm vulnerability advisory clearance or native WebView security. Issue #100's independent native acceptance remains open. |

**Status convention:** `PASS` is limited to the explicitly described static/existing mocked evidence; `FAIL` marks a concrete code-path or CI gate defect; `NOT TESTED` means no independent negative or native/prod proof, never an implicit pass. This report does **not** establish the combined five-PR system secure.

## Integration requirements for Agent 00

1. Keep #114 and all feature PRs draft/no-release until SEC-10-01 and SEC-10-03 are fixed, canonical draft-body certification is either disabled or independently implemented (SEC-10-02), all five PRs pass on current exact heads, and the branches are actually composed.
2. Register the independent negative tests in an integrated CI job once the feature modules exist. The nested Rust harness is intentionally separate from the root workspace and is not run by the current root `cargo test --workspace` job. Python discovery requires an explicit command; root CI presently runs narrower Python suites. Never equate skipped/non-executed tests with green certification.
3. For installed-app certification, separately require real Tauri WebView tests and real device evidence under issue #100; Chromium-mock screenshots and packaging workflow success do not substitute.
4. Introduce no paid AI egress until a trusted hard-capped transport, server-owned immutable scope/prompt/approval binding, durable effect receipts, user-visible budget limits, and rejection of stale context are proven on the integrated exact SHA.
5. Protect the history of tests and findings. Review and acknowledge by feature owner; security Agent 10 does not edit other agents' production code and does not approve merges or releases.

## Work limitations

The audit used the GitHub connector to inspect exact-head source, the five PR change lists, and exact-head GitHub Actions metadata/logs. No Rust toolchain or network-accessible checkout was available in the local execution container. The new Rust and Python security contract tests were authored but **not executed**. There were no live API calls, real secrets, independent installed WebView sessions, dynamic network fuzzing, or filesystem permission exploitation. Claims above are limited accordingly.
