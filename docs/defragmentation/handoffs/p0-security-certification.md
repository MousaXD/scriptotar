## 2026-11-18 — Independent advisory cross-check against upstream registries

Agent 10 independently cross-checked the published vulnerability and release metadata instead of accepting only Agent 11's conclusion. [GitHub's authoritative GHSA-fc5q-39xc-vjpf](https://github.com/advisories/GHSA-fc5q-39xc-vjpf) continues to identify a repeated-extglob ReDoS affecting **picomatch through 4.0.7** without a published fixed package range at this checkpoint. The [npm registry's picomatch versions listing](https://www.npmjs.com/package/picomatch?activeTab=versions) currently labels **4.0.7** as `latest`; no independently verified newer fixed npm release was found. [OSV's current GHSA record](https://osv.dev/vulnerability/GHSA-fc5q-39xc-vjpf) corroborates this. A third-party risk database may differ or revoke a separate advisory; this does **not** override the official GHSA or an unsuppressed reproducible `npm audit --audit-level=high` failure. The direct [Agent 11 exploit/audit run #44313324125](https://github.com/MousaXD/scriptotar/actions/runs/44313324125) separately demonstrated crafted-match timeout against the locked vulnerable package. **Conclusion: advisory still open at this observation; no patch claim, no downgrade, no audit waiver, no release.**

---

## 2026-11-18 — Production Queue alert recovery merged; new integration gate pending

**Live integration source moved again** to `5ed385d09092d2c94a1bd86cb56888d96df0bb5e` after the previously certified `caafb11d9d082af025c8f567a60e8d103730b8e0` checkpoint. The newer Agent 00 commit modifies `apps/desktop-ui/src/i18n/messages.ts`, `apps/desktop-ui/src/views/JobsView.svelte` and its tests. On an actual rejected unsupported `file:` URL response, the Queue now exposes localized EN/AR text, `role="alert"`, `aria-live="assertive"`, `aria-atomic="true"`, `aria-label`, `aria-invalid` and `aria-describedby`; the implementation still sends the URL to Rust `onEnqueueUrl` for authoritative admission rather than silently accepting frontend validation. Unit tests cover English string rejection and Arabic `Error` rejection. **This is a source-level improvement only.**

The freshly dispatched exact-HEAD integration workflows include [native installed acceptance #44313725742](https://github.com/MousaXD/scriptotar/actions/runs/44313725742), [independent security #44313725766](https://github.com/MousaXD/scriptotar/actions/runs/44313725766), and [Tauri migration/npm audit #44313726305](https://github.com/MousaXD/scriptotar/actions/runs/44313726305). At last inspection these were queued; **no new completed PASS is asserted**. The previous `caafb...` 7/7 security PASS is historical and cannot be transferred to `5ed385d09092d2c94a1bd86cb56888d96df0bb5e`. Native Windows input/negative-path behavior and all required installed cross-project/revocation scenarios still require new exact-head evidence.

**Release veto unchanged: DRAFT / NO-GO / paid egress OFF.**

---

## 2026-11-18 — Agent 10 continuation: semantic-denial spoofing and pinned feature matrix

**Independent disposition: DRAFT / RELEASE NO-GO / paid egress OFF.** The following is an additional audit checkpoint; previous dated observations remain historical evidence, not live branch status.

### Exact source / completed execution provenance

| Source | Latest inspected immutable SHA | Status |
| --- | --- | --- |
| Integration PR #114 | `caafb11d9d082af025c8f567a60e8d103730b8e0` | Agent 00 #133 production fix merged; seven integration independent security jobs PASS; installed and npm gates FAIL |
| Agent 07 PR #131 | `6981df8c1bf7592d083d9d3bd011f258755b8850` | screenshot-only automatic PASS corrected, but stale semantic-text risk and real native gaps persist |
| Agent 11 PR #132 | `5b0086d635645a90e8492ed8f624449f65f98364` | report/exploit evidence only, no patched dependency graph |
| Agent 10 PR #124 | test commit `8909c8e39ddc99ecf8179fcfc5a59068002d8cc2`, followed by workflow hardening | new exact-head matrix has NOT completed |

**Narrow independent rerun:** [Agent 10 run #44300422996, attempt 2](https://github.com/MousaXD/scriptotar/actions/runs/44300422996) reran `native-evidence:131` and **PASS** when job `134816208320` checked out audit **old merge SHA** `28baf36c1560dd10afebe10b8029388a8b578d3d` and Agent 07 exact target `d7173290004b7de4b9906dabdb3c0ee8eebbebee`. It confirms the earlier *screenshot-only/unreviewed* auto-PASS regression is fixed on that QA source, **not** that the later semantic-spoofing regression or installed-host acceptance is certified. The workflow attempt as a whole remains **FAIL** because combined job `134816210945` used old integration `f42eff03051c43f4b919763a8950b01758afd36f` and again found missing paid host registration. Do not mix this old combined red with newer [integration run #44307588806](https://github.com/MousaXD/scriptotar/actions/runs/44307588806) (**seven jobs PASS** on `caafb11d9...`).

### New Agent 10 P0 QA-evidence trust challenge

Added `test_unbound_semantic_flag_is_not_request_correlated_proof` to `tests/defragmentation/security/test_native_visual_adjudication.py`. In Agent 07's current Linux runner, a matching AT-SPI node containing the generic tokens `angle` and `invalid` is accepted as proof without proving it is new, specifically associated with the exact malformed JSON submission, or sourced from that public Rust IPC response. The runner writes a report shape consisting of a generic `proof` description, screenshot filename and `atspi_error_text_exposed: True`. The generic `checked()` accepts this caller-controlled shape as PASS after any successful callback.

**Adversarial test:** supply the same semantic-looking proof record from an unrelated/stale native alert, with no request ID, correlated before/after transition, or backend denial identity. Assert that `checked(report, "error_recovery", ...)` cannot certify PASS. This is a **test-only challenge** expected to expose a residual QA-integrity gap in the inspected code; do not claim confirmed executed RED until a fresh workflow runs. Genuine exact-editor readback and correct adjacent accessibility semantics help but do not alone eliminate stale/unrelated text confusion. Request-correlated denial and read-only canonical nonmutation remain mandatory.

**Test-source provenance hardened:** `independent-security-regressions.yml` now checks out the real invoking Agent 10 PR head rather than GitHub's synthetic `refs/pull/124/merge`; asserts its SHA; independently resolves PRs #131, #118, #119, #128, #121 and #117 to immutable SHAs before running feature jobs; uses the same dynamically pinned PR #114 integration SHA for both combined jobs; and post-checks all branch/PR refs for drift before permitting matrix certification. This requires a **fresh workflow run** to validate the new YAML and all eight jobs. Do not assert the new guard is green on source alone.

### Real installed failures at newer exact SHAs

- Integration `caafb11d9...`: [installed run #44307588562](https://github.com/MousaXD/scriptotar/actions/runs/44307588562) **FAIL** on both Linux and Windows. Linux logs `Malformed three-angle JSON produced no visible native rejection`; Windows overall remains incomplete with mandatory NOT TESTED cases, despite old successful GUI controls. Therefore merging Agent 00 PR #133 did **not** establish installed Linux error recovery.
- Agent 07 `d7173290004b7de4b9906dabdb3c0ee8eebbebee`: [installed run #44308361975](https://github.com/MousaXD/scriptotar/actions/runs/44308361975) **FAIL** on both OSes. Linux: valid three-angle follow-up produced **0 native AT-SPI card actions** instead of three; image-only case is correctly **NOT TESTED** when semantic proof absent. Windows: `Media URL` UIA edit control **did not receive actual user text** during unsupported-URL recovery. Both platforms still leave forged foreign-project IPC, genuine multi-source acquisition, source revocation, and full RTL/theme adjudication uncertified.
- Agent 07 `6981df8...` adds unit-contract adjustments including exact input readback; its new installed run must finish before any statement about that SHA. Existing job failures above may not reproduce on newer source, and cannot be carried forward as fresh results.

### Supply-chain blocker strengthened by real exploit evidence

Agent 11's dedicated [Node 22 run #44313324125](https://github.com/MousaXD/scriptotar/actions/runs/44313324125) on `fe13a6a52e4890ca91cda8a35a6202fc35b07e0d` found `picomatch@4.0.7` crafted repeated-extglob inputs **timeout at 1,200ms**, while a benign control completed in 2ms. Clean install/check/tests/build passed, but `npm audit --audit-level=high` failed **8 high findings** and strict patched-version assertion failed. This is operational exploit evidence; do **not** say the compiled desktop runtime is proven attacker-reachable. PR #132 current `5b0086...` is still **report/diagnostic only**, not remediation. Keep original npm supply-chain gate unsuppressed and release NO-GO.

### Ownership and next gates

1. Agent 07: eliminate semantic proof spoofing and make rejection identity/state transitions request-correlated; preserve independent screenshot adjudication. Resolve Linux 0-card recovery and Windows input control failure in installed apps without fake success.
2. Agent 00: prove complete **before-and-after content snapshots** of canonical draft/version, approvals, receipts/effect attempts and historical audit rows during each denied installed-host request. Counts alone are insufficient. Real foreign-project Tauri IPC authorization, source revocation, and allowed multi-source acquisition remain required.
3. Agent 11: independently remediate npm advisory with a verified patched graph or report the upstream blocker, never force an unpatched downgrade or suppress audit.
4. Agent 10: obtain a new complete **current-head eight-gate matrix** using the SHA-pinned workflow and inspect native Linux/Windows artifacts. Any auto-PASS before request-correlated semantic proof is an independent release veto.

**NO-GO remains active. No main merge, no release, no paid-provider egress.**

---

## 2026-11-18 — Live integration/QA head movement: revised immutable evidence

**This addendum supersedes only the *current-state* assertions in the earlier dated checkpoint; earlier run/shas remain historically correct.** GitHub refs changed during the independent inspection. No fresh current-audit certification or installed-host release acceptance is claimed.

- **Integration NOW** `caafb11d9d082af025c8f567a60e8d103730b8e0` (read from actual `git/ref/heads/integration/scriptotar-defragmentation-agent00-20261113`). Agent 00 production [PR #133](https://github.com/MousaXD/scriptotar/pull/133) **is now merged** at this integration SHA. The previous `7af95c...` integration and the claim "#133 unmerged" below were true only at that earlier inspection.
- New integration's [independent security run #44307588806](https://github.com/MousaXD/scriptotar/actions/runs/44307588806) now **PASS** for all **seven** constituent jobs: provider, migration, receipt-v12, writing, verification, combined-v11 and combined. This is a significant real improvement for the production host, but it is **not** Agent 10's newer eight-gate audit including `native-evidence:131`, and not full installed-app release certification.
- New integration [P1 provider-race workflow](https://github.com/MousaXD/scriptotar/actions/runs/44307588613), security hygiene, Desktop UI and GUI visual/performance have already completed **PASS**. Other integration workflows were still queued/in progress at inspection, including installed acceptance and supply-chain migration. No overall release GREEN.
- **Agent 07 NOW** `93a3b7ae8b27d58dbbbe551a3d38e22654e379b8` ([PR #131](https://github.com/MousaXD/scriptotar/pull/131)). Its actual `linux_webkit.py::checked()` **now refuses to auto-PASS** `error_recovery` if `angle_rejection_visual_proof` is missing, `human_review_required=True`, or `atspi_error_text_exposed` is not true. It classifies that condition **NOT TESTED** via `NativeUnavailable`. This is a source-level remediation of Agent 10's earlier fail-open regression, and it retains the remaining native recovery probes. **Do not claim the current branch passed native-evidence yet.**
- Latest Agent 07 [CI](https://github.com/MousaXD/scriptotar/actions/runs/44307630160) and [security hygiene](https://github.com/MousaXD/scriptotar/actions/runs/44307630106) passed. Its [installed WebView acceptance #44307630119](https://github.com/MousaXD/scriptotar/actions/runs/44307630119) was **CANCELLED**, not GREEN, at this source SHA.
- Agent 10 independently requested a **single-job rerun** of the original `native-evidence:131` [run #44300422996](https://github.com/MousaXD/scriptotar/actions/runs/44300422996), job ID `134788406951`. This is **only a narrow regression attempt**: rerun workflow checkout is still associated with its original audit commit, while the target feature ref is resolved dynamically; its log must show the *actual* target checkout SHA before any status is attributed. It **cannot** replace a fresh full matrix from Agent 10's own current audit head, with the integration SHA pinned uniformly.

**Current independent verdict: QA evidence fail-open appears fixed in source, but dynamic exact-head audit rerun and semantic/native authorization are NOT CERTIFIED. Release remains DRAFT / NO-GO / egress OFF.**

---

## 2026-11-18 — Independent QA-integrity veto / exact-SHA rescan

**Decision: RELEASE NO-GO; PR #124 remains DRAFT.** This is an independent audit record, not an installed-host certification. Never enable paid egress or merge to main on this evidence. **All links and conclusions below distinguish the test source from the code being tested.**

### Dynamically resolved revision provenance

- Integration branch `refs/heads/integration/scriptotar-defragmentation-agent00-20261113` (GitHub Git ref read on 2026-11-18): `7af95c574643c14565a5c10d0be2cc78c5e57254` ([PR #114](https://github.com/MousaXD/scriptotar/pull/114)). Do **not** use PR #124's stale `base_sha` `f42eff03051c43f4b919763a8950b01758afd36f`.
- Agent 10 audit source inspected: `a306debb78ab7fddc46a42c64f6f4b65dd63954e` ([PR #124](https://github.com/MousaXD/scriptotar/pull/124)); no completed check for this exact audit head at review time. This report commit itself also requires a **new completed exact-head matrix**.
- Agent 07 current QA source: `9c272a92de8df42a1ecfee5211a4f7ea3bb74e44` ([PR #131](https://github.com/MousaXD/scriptotar/pull/131)).
- Agent 00 production proposal: `e0eda73e62e9288509c92db610e841183cd31ce2` ([PR #133](https://github.com/MousaXD/scriptotar/pull/133)); **not yet integrated into the release branch**.
- Agent 11 supplier report: `75b2fb568b4958cc25bde262643b00c240f04961` ([PR #132](https://github.com/MousaXD/scriptotar/pull/132)); report-only, not a remediation.

### Independent observed matrix — old audit workflow, not current certification

[Run #44300422996](https://github.com/MousaXD/scriptotar/actions/runs/44300422996) is **FAIL**. This run's trigger head was audit `1009cc92bd0653a765f7b489a0a6a3f1215b02b6`; the matrix test checkout itself printed audit **merge checkout** `28baf36c1560dd10afebe10b8029388a8b578d3d`. The target integration checkout printed `f42eff03051c43f4b919763a8950b01758afd36f`. It does **not** certify the later audit `a306...` nor the current integration `7af95...`.

| Independent gate | Observed result on previous audit workflow | Trust boundary / remaining gap |
| --- | --- | --- |
| `provider:118` | **PASS**, isolated mocked provider | Revocation/consent reservations and context race test; not actual paid network execution |
| `migration:119` | **PASS**, isolated | Canonical SQLite/migration and rollback, not installed app authorization |
| `receipt-v12:128` | **PASS**, isolated | Replay/ACK/provider identity/duplicate durability; no real paid transport proof |
| `writing:121` | **PASS**, isolated | Canonical writing atomicity, not full host/no-mutation for every negative UI request |
| `verification:117` | **PASS**, isolated | Unbound body/claim provenance fail closed, no independent publisher attestation |
| `combined-v11:114` | **PASS** against `f42eff...` | Integrated v11, historical DB and draft persistence only |
| `combined:114` | **FAIL** against `f42eff...` | Missing registered approved paid Tauri execution entrypoint and serialized live transport; two independent assertions failed; leave egress OFF |
| `native-evidence:131` | **FAIL** against Agent 07 `bd8187942dd735998bb48024efedce6786423c6d` | `test_image_only_angle_rejection_cannot_be_autocertified`: image-only evidence with `human_review_required=True` was marked PASS; real QA-integrity fail-open |
| Whole matrix / exact-source stability | **FAIL / not yet proven** | Old workflow lacks current shared immutable pin-and-postcheck result; later audit source added one, awaiting fresh run |

**Reproduction:** [native-evidence failing job](https://github.com/MousaXD/scriptotar/actions/runs/44300422996) reports `AssertionError: True is not false : FAIL OPEN: a successful callback marked screenshot-only angle rejection as certified before independent human visual review`. The current Agent 07 `9c272a92...` Linux runner still records `human_review_required: True` after the new-border screenshot fallback and then returns from `error_recovery()`; generic `checked()` blindly assigns `PASS` to any successfully returning callback. This is **source-confirmed still vulnerable**, but its newest exact HEAD has **not** received a new independent native-evidence job at review time. Fix must reject unresolved human review in the actual verdict path, not simply rename or suppress screenshot metadata.

### Actual installed-app acceptance — Agent 07 exact head `9c272a92...`

[Run #44302808654](https://github.com/MousaXD/scriptotar/actions/runs/44302808654) is **FAIL** overall. Linux: **10 PASS / 1 FAIL / 5 NOT TESTED**, with genuine installed Queue URL `file:///scriptotar-native-qa-denied-media.mp4` failing because no native alert was exposed; Windows: **10 PASS / 0 FAIL / 6 NOT TESTED**, including visible UIA URL rejection with unchanged canonical job count and saved candidate readback. The two untested security scenarios include **foreign-project forged IPC authorization, genuine multi-source transcript, revocation**, plus Arabic full RTL and complete visual theme adjudication. Existing screenshots are review evidence, not an automatic visual PASS. Neither OS is release-certified.

Agent 07's read-only negative-path probe currently compares per-project **job count**, and checks persisted Study/candidate bodies, but does not snapshot or diff all **approval, agent effect, external attempt, audit-event, and writing batch rows** around the installed malformed-input and unsupported-URL cases. Source-only confidence in `interpret_studio_angles` or a screenshot does not prove every required installed-host non-mutation boundary. Add read-only before/after canonical table identity and content/hash checks scoped to the same application profile; fail on any unauthorized difference, and include visible native request-correlated rejection. Do not seed or alter production SQLite to satisfy tests.

### Agent 00 production review, separate from native certification

[PR #133](https://github.com/MousaXD/scriptotar/pull/133) improves Svelte localized `invalidAngles` and an accessible live `role=alert`, guards stale async results and saving, and adds a public Rust service test for the exact one-angle JSON. That Rust regression checks unchanged Study revision/draft ID/body/versions/source, evidence count, and snapshots the counts of six ledger-related tables (`agent_runs`, `agent_audit_events`, `agent_effects`, `agent_external_effects`, `agent_external_attempt_history`, `study_writing_batches`). This is **valuable source/unit evidence** but does not cover every approval/old-row hash or a true installed host. It is currently **unmerged into the dynamically resolved integration SHA**. [PR #133 own checks](https://github.com/MousaXD/scriptotar/pull/133/checks) include package/UI tests passing but migration supply-chain failing; do not call release GREEN.

### Agent 11 independent advisory review

[PR #132](https://github.com/MousaXD/scriptotar/pull/132) only adds the signed-off investigation report. On unchanged `7af95...` integration, [npm Supply chain run #44278804900](https://github.com/MousaXD/scriptotar/actions/runs/44278804900) fails high-severity audit: eight affected dependency paths traced to `GHSA-fc5q-39xc-vjpf` (picomatch <=4.0.7). Agent 11 reports **no verifiable patched release** at this checkpoint, so `npm audit fix --force`, an unpatched override, or disabling the audit is NOT certification. Their exact report-only head `75b2...` likewise has a failed Tauri migration integration check. Upstream fix state must be rechecked before changes; the vulnerability remains a RELEASE BLOCKER. Cargo audit warnings and the skipped post-npm Python advisory step must be tracked separately.

### Mandatory closure / next execution

1. Complete `native-evidence:131` on Agent 07's **then-current** SHA, with dynamic integration pin, fail-closed human adjudication, request-specific semantic denial, and no-mutation proof. Existing screenshot fallback is currently RED.
2. Integrate approved real production Queue/angle visibility changes without altering QA assertions; run actual installed Linux and Windows controls and signed evidence at the **same final SHA**.
3. Prove native rejected requests leave all canonical Study, draft/version, approvals, effects, receipts/attempts, and historical audit identities and content unchanged. Check forged IPC and source revocation through real installed-host authorization, not mocked provider races.
4. Resolve unsuppressed npm supply-chain audit, independently re-review dependency advisories; keep paid egress disabled while authenticated paid transport and live receipt/pricing aren't certified.
5. Rerun all eight independent security matrix gates, the exact-SHA pin and postcheck, and complete installed acceptance at a single immutable **final integration** SHA. Treat prior passing tests as historical only.

**RELEASE VETO: ACTIVE / NO-GO / DRAFT / DO NOT MERGE / PAID EGRESS OFF.**

---

# Agent 10 — 2026-11-17 latest independent five-failure database certification

**Completed exact source and executed evidence:** [Agent 10 independent security workflow #44117872802](https://github.com/MousaXD/scriptotar/actions/runs/44117872802), audit `d2498870c674dc9d501b4b5eb39f4fc705b230e6`, integration source `402ce53ef0643cf23b610571617e262ed3582a39` (PR #114). **5 of 6 security jobs PASS, combined FAIL; real SQLite v10 DB 42 PASS / 5 FAIL.** All five independent cases use separately approved RUNNING checkpoints, real v10 Fresh reservation/pre-send CAS, and **NO network or billable provider calls**. These are confirmed *ledger identity/accounting* acceptance defects, not observed external double charges or cross-project data reads. The five exact failing tests:

- `independent_v10_provider_receipt_must_not_settle_two_distinct_runs`: same provider request+receipt credited across two run IDs.
- `independent_v10_provider_receipt_must_not_settle_across_project_boundary`: same provider request+receipt credited across different projects/Studies **even after dropping/reopening the SQLite store**.
- `independent_v10_acknowledgement_id_is_unique_before_final_settlement`: same provider-origin request ACK accepted by two different in-flight effects prior to metered response.
- `independent_v10_receipt_identity_is_unique_even_with_distinct_request_ids`: same receipt ID credited with two different provider request IDs.
- `independent_v10_concurrent_cross_run_receipt_settlement_has_one_winner`: **both** competing SQLite connections settled one provider request/receipt for two different run IDs, violating one winner.

The other five matrix jobs PASS, 10 adversarial provider consent tests PASS, provider wire validation tests PASS and real host Rust tests PASS. Combined also fails `test_approved_paid_provider_dispatch_is_exposed_through_tauri_host`: **there is no registered real consent-approved paid provider Studio Tauri IPC path**. Private sealed-wire host admission remains tested but not public paid HTTP. Owner [PR #114 issuecomment-6953114913](https://github.com/MousaXD/scriptotar/pull/114#issuecomment-6953114913) and durable-effects [PR #119 issuecomment-6953121956](https://github.com/MousaXD/scriptotar/pull/119#issuecomment-6953121956) have the exact CI evidence and recommended **additive v12 global provider-scoped durable ACK+receipt ID uniqueness**, race/attempt/restart guarantees, historical duplicate fail-closed migration handling and same-effect idempotence. No production fix had landed in source `6dfb9430c54152931f4f6c6eed44a56a018f0b70`.

**Owner's latest product-safety improvement:** `223d8d3665f2e69943ffbf68239e0bdb3604eb39` disabled public legacy `AppServices::run_ai` BYOK HTTP sends, retaining only local Copy Prompt; subsequent UI commits disabled the send button and API-key entry and updated frontend red tests. The previous claim that BYOK is a live uncontrolled HTTP bypass is **HISTORICAL, not current on those newer SHAs**. Agent 10 added `test_public_legacy_byok_command_cannot_bypass_sealed_consent` to the real combined host wiring suite, which verifies the registered Tauri `run_ai` delegates to the deny-first method with no direct unbounded HTTP calls; actual Rust host regression must pass as well. New exact-head PR124 workflow [#44118785255](https://github.com/MousaXD/scriptotar/actions/runs/44118785255) was QUEUED at last review (source base `6dfb9430...`). **Do not claim its BYOK regression PASS before the job is completed**.

**Current release:** NO-GO. v10 ledger uniqueness MUST be fixed and rerun, live paid Studio public consent/transport/pricing/receipt/cancel/reconcile must be independently certified, native installed Linux/Windows QA remains FAIL (integration #44112304190; Agent07 #44114536688 also FAIL). Paid egress OFF, PR #124 DRAFT. Audit-only commits never edit owner production files.

---

# Agent 10 — 2026-11-17 public legacy BYOK fail-closed and targeted external receipt concurrency

**Owner source observed:** `ebc5c4c1f86c477275d16680521f47f7135c0a26`, PR #114 draft, not yet independently certified. Owner commit `223d8d3665f2e69943ffbf68239e0bdb3604eb39` removes live `HttpAiProvider`/provider requests from the public `AppServices::run_ai`; BYOK mode returns a non-networking error and Copy Prompt still only stores local prompt metadata under the shared project mutex. The next commit `ebc5c4c...` disables the legacy Studio Run with API button and credential input. **This closes the previously identified directly reachable legacy BYOK bypass IN SOURCE**, but requires exact-head Rust and independent host CI; avoid describing old BYOK live traffic as current on this SHA. The private new sealed Studio paid provider transport **still lacks a registered, certified production approve/send/cancel/reconcile flow**; neither new production code nor any audit tests enabled paid egress.

**New independent release regression in PR #124:** `test_public_legacy_byok_command_cannot_bypass_sealed_consent` in `tests/defragmentation/security/test_combined_host_wiring.py`. It checks the actual Tauri public `run_ai` command delegates to production `AppServices::run_ai`, the latter rejects BYOK before any provider I/O, and no raw `HttpAiProvider`/unsealed `AiService::new` call remains in that function. The owner's actual compiled host Rust `legacy_byok_host_command_fails_closed_without_receipt_or_network` test is still mandatory, not replaced by structure checks. This new independent test is **NOT TESTED** until the audit run uses the new owner SHA.

**New runtime red-team tests in `external_receipt_replay_additional_tests.rs`** test (1) global provider ACK uniqueness *before* metered response, (2) global provider receipt ID uniqueness even with different request IDs, and (3) concurrency through two independent on-disk SQLite connections with exactly one successful chargeable settlement. The existing two tests already prove cross-run/cross-project receipt double credit and survive a SQLite reopen, confirmed in [#44115509272](https://github.com/MousaXD/scriptotar/actions/runs/44115509272) against `402ce53...`. The three newest ACK/concurrency subtests are **NOT TESTED** at the time of this handoff; [workflow #44117872802](https://github.com/MousaXD/scriptotar/actions/runs/44117872802) was IN PROGRESS. Preserve failures until an additive database uniqueness migration is implemented and independently rerun. Do not mistake an older owner 6/6 green run lacking these assertions for a security certification.

**Latest installed acceptance:** integration `402ce53...`, [#44112304190](https://github.com/MousaXD/scriptotar/actions/runs/44112304190) **FAIL** (Linux installed project heading, Windows WebView2/Studio readiness after 90 seconds). Agent07 `24d2047...` [#44114536688](https://github.com/MousaXD/scriptotar/actions/runs/44114536688) also **FAIL** (Linux `Script candidates` list role; Windows PROJECT control despite WebView2+Studio detection). This remains a separate NO-GO release gate. No production source edited by Agent 10, no `main` merge/release, paid Studio egress OFF.

---

# Agent 10 — 2026-11-17 independent security continuation: ACK + concurrent receipt settlement

**Last fully completed exact-head independent evidence:** [security run #44115509272](https://github.com/MousaXD/scriptotar/actions/runs/44115509272), Agent 10 audit `1376aa36148658e7f542c27c5936ff0b2fd716a6`, PR #114 integration **`402ce53ef0643cf23b610571617e262ed3582a39`**. **Five of six jobs PASS**: provider, migration, writing, verification and combined v11. **Combined FAIL**, real v10 DB 42 PASS / 2 FAIL with the *same receipt* credited (1) to two distinct approved runs and (2) across foreign project/Study IDs **after dropping and reopening SQLite**. The real sealed consent and OpenAI fixed-wire unit tests PASS, as do 94 host Rust tests, but production consent-approved Tauri dispatch is still NOT registered and the independent structural test correctly FAILS. `run_ai` BYOK is separate; no live billable provider test is claimed. Older green owner workflow [#44112304916](https://github.com/MousaXD/scriptotar/actions/runs/44112304916) used an old audit checkout with just four structural tests and 42 DB tests; it did **not** include the new cross-run/cross-project security assertions. Do not substitute its green result.

**New independent test additions, audit SHA `d2498870c674dc9d501b4b5eb39f4fc705b230e6`:** all reside ONLY in `tests/defragmentation/security/external_receipt_replay_additional_tests.rs`, injected at CI runtime into the real DB fixture. (A) `independent_v10_acknowledgement_id_is_unique_before_final_settlement` requires provider request ACK identity not be accepted on a second in-flight effect, before a response body is available; (B) `independent_v10_receipt_identity_is_unique_even_with_distinct_request_ids` requires duplicate receipt ID refusal even when provider request IDs differ; (C) `independent_v10_concurrent_cross_run_receipt_settlement_has_one_winner` starts two independent SQLite connections and concurrent reconciliations with one provider request/receipt ID, requiring exactly one winner. All fixture runs are separately approved and reserved with v10 Fresh-only CAS; zero real provider traffic or credentials. These three new tests were **NOT YET COMPLETED** at handoff: [exact-head independent CI #44117872802](https://github.com/MousaXD/scriptotar/actions/runs/44117872802) was IN PROGRESS at latest check. Do not call the new tests confirmed FAIL/PASS until reading the combined job log. Already-confirmed two duplicate-credit tests remain mandatory.

**Owner action:** add immutable, provider-scoped and globally unique (across all project/run IDs) durable ACK/receipt proof, preferably an additive migration (current database schema v11), with transactional multi-connection safety, historical duplicate handling and same-effect identical-terminal callback idempotency. A single `UNIQUE` index on active effect rows might be insufficient if a verified-nonexecution or retry clears provider ID fields; preserve history or use a separate append-only receipt/ACK registry. Never mark effects complete twice with one provider-origin usage record; do not retry uncertain sends. Agent 00 and Agent 02 have confirmed repro links in PRs #114/#119. Agent 10 does **not** change production code.

**Separate native release gate:** [integrated installed QA #44112304190](https://github.com/MousaXD/scriptotar/actions/runs/44112304190) at owner `402ce53...` completed FAIL on both platforms (Linux project heading not found; Windows WebView2 document + Studio control not confirmed within 90s). The newer Agent07 [QA #44114536688](https://github.com/MousaXD/scriptotar/actions/runs/44114536688) at `24d2047bb72b14d1a795f1b629736802cd7991f1` also completed FAIL: installed Linux missing the `Script candidates` AT-SPI list, installed Windows finds Studio/WebView2 but cannot locate the PROJECT control. Different app builds/branches must not share verdicts; no full installed journey certified. Paid Studio egress OFF, no release, PR #124 DRAFT.

---

# Agent 10 — precise next release gate / 2026-11-17 audit addendum

**Audit source change `d6473abf478482d295b00367b8b63524e11afb7d`:** the previously reproduced cross-PROJECT duplicate-provider-receipt test now **drops the first SQLite store and checkpoint handle and reopens the same database from disk** after the first completion, before attempting to credit the same receipt in the foreign project. This tests ledger-wide provider receipt uniqueness across reopened connections, not only one process-local transport instance. [New exact-head independent run #44114976196](https://github.com/MousaXD/scriptotar/actions/runs/44114976196) was **IN PROGRESS** at observation, so do not claim the *reopened-handle variant* has passed or failed until its job log is inspected. The earlier no-reopen cross-project and same-project tests are already **CONFIRMED FAIL** at exactly `402ce53ef0643cf23b610571617e262ed3582a39` in [#44113633278](https://github.com/MousaXD/scriptotar/actions/runs/44113633278): DB 42 PASS / 2 FAIL, independent matrix 5 PASS / 1 combined FAIL.

**Suggested safe repair:** Current DB `LATEST_SCHEMA_VERSION=11`; migrations #10 external-effect ledger and #11 canonical writing are tracked in `schema_migrations` and `PRAGMA user_version`. Implement a **new additive migration #12**, rather than rewriting migration #10 and changing existing install semantics. At minimum, same-provider request/receipt IDs must be globally unique across effect keys/project IDs, checked in a single SQLite settlement/ACK transaction; consider an append-only provider receipt registry keyed by provider/receipt identity to preserve uniqueness even if a verified-not-executed retry clears effect-level receipt columns. Fail closed on preexisting duplicate historical records, preserve same-effect idempotent terminal callbacks, and verify process restart plus multi-process simultaneous settlement. Agent 00 notified in [PR #114](https://github.com/MousaXD/scriptotar/pull/114#issuecomment-6952588508).

**Native installed integration caveat:** owner integration SHA `402ce53...` has [installed WebView run #44112304190](https://github.com/MousaXD/scriptotar/actions/runs/44112304190) still IN PROGRESS, but its installed Linux job has **FAIL** on AT-SPI project heading `NativeQA-402ce53e-A`; Windows job still pending at inspection. This cannot be declared PASS based on more advanced results from Agent07's QA branch. Paid egress OFF, release NO-GO, PR #124 DRAFT, production edits by Agent 10 NONE.

---

# Agent 10 — 2026-11-17 confirmed cross-project AND cross-run receipt double-credit

**Actual executed CI:** [Agent 10 independent workflow #44113633278](https://github.com/MousaXD/scriptotar/actions/runs/44113633278), audit `047eade00c3200a109a410942df91b840f740eb0`, exact integration target `402ce53ef0643cf23b610571617e262ed3582a39`. **Independent matrix: 5/6 PASS.** Provider PASS, migration PASS, writing PASS, verification PASS, integrated v11 PASS; **combined FAIL**. The combined job #134145614495 actually completed the corrected independent wire tests **3/3 PASS**, core consent **20/20 PASS**, actual Tauri-host Rust tests **94/94 PASS**, verification **25/25 PASS**, and the real database **42 PASS / 2 FAIL**.

**Two separate deterministic failure proofs** (injected ONLY in CI into the real SQLite DB test module):
- `independent_v10_provider_receipt_must_not_settle_two_distinct_runs`: a real provider receipt `resp_same_provider_9123` completes two separate legitimate, already-approved and durably-dispatched run IDs *within the same project*. FAIL: `v10 ledger accepted the identical provider receipt for two distinct paid runs`.
- `independent_v10_provider_receipt_must_not_settle_across_project_boundary`: two independently approved RUNNING checkpoints under **different project and Study IDs** each commit the v10 pre-send CAS, then the same receipt `resp_cross_project_9123` is erroneously accepted by both `SqliteStore::reconcile_external_effect` calls. FAIL: `provider receipt reused across projects and credited as two separate paid effects`.

**Security impact:** confirmed **ledger accounting double-credit vulnerability**, *not* observed real duplicate HTTP requests, charging, or unauthorized cross-project reads. Same-effect replay CAS is already enforced and passes, but global provider receipt/ack uniqueness across different effects is absent in current source. Fix is a globally unique provider-scoped request/receipt binding in the durable SQLite reconciliation transaction (and ACK path), keeping same-effect idempotent callbacks, real migration compatibility, uncertain no-replay on crash and foreign-project denial. A new owner CI green on a 4-structural/42-database-test audit checkout does **not** certify these new findings. Production owner Agent 00 notified at [PR #114](https://github.com/MousaXD/scriptotar/pull/114#issuecomment-6952499494) and durable effects Agent 02 at [PR #119](https://github.com/MousaXD/scriptotar/pull/119#issuecomment-6952505179).

**Remaining independent public authorization FAIL:** `test_approved_paid_provider_dispatch_is_exposed_through_tauri_host` returned no certified real paid approval/dispatch `generate_handler!` command. Host sealed controlled policy tests and real `OpenAiResponsesWire` parsing PASS, but there is no production registered consent-approved live paid transport with verified trusted price catalog. Distinct legacy `run_ai` BYOK HTTP remains outside the sealed Studio path. Keep it disabled until approved source/budget/cancellation/reconciliation boundaries pass.

**Exact-head updated audit source:** PR #124 `bec2d23c0d299daf188fe2f39451c47c01b7c1a4`, production baseline `402ce53ef0643cf23b610571617e262ed3582a39`; delta is **only nine security workflow/test/handoff paths**, no production files. Latest [audit run #44113857823](https://github.com/MousaXD/scriptotar/actions/runs/44113857823) was IN PROGRESS at check; the previous run provides exact completed evidence. Owner branch has shared active-project mutex gates for sidecar final content writes, scan research updates, auto-watchlist final writes and legacy import, with new Rust tests; those address earlier origin races but not ledger uniqueness. Native Agent 07 installed-application gate still **FAIL / NOT CERTIFIED**. PR #124 remains draft, no main merge/release, paid Studio egress OFF. **NO-GO**.

---

# Agent 10 — exact owner 402ce53 test expansion and current workflow

**Synchronized owner production SHA:** `402ce53ef0643cf23b610571617e262ed3582a39` (via two-parent audit merge `449548d19a6b1806a5fe0a90068ebc560b315899`), after Agent 00 installed shared host authorization mutex gates on transcription final Source/Media/Transcript commit, watchlist research writes, and legacy imports. **Audit test-only follow-on SHA:** `047eade00c3200a109a410942df91b840f740eb0`. [Exact-head independent six-job workflow #44113633278](https://github.com/MousaXD/scriptotar/actions/runs/44113633278) was **QUEUED** at observation, and [prior #44113514305](https://github.com/MousaXD/scriptotar/actions/runs/44113514305) was **IN PROGRESS**: no PASS claimed before completion.

**Additional test:** The existing confirmed same-project cross-run v10 receipt-replay failure has been extended with `independent_v10_provider_receipt_must_not_settle_across_project_boundary`. That new deterministic test creates two independently approved running checkpoints in distinct project IDs and Study IDs within the **same** real SqliteStore, reserves and dispatches separately, settles one with provider request+receipt `resp_cross_project_9123`, and requires an attempt to settle the foreign project using that receipt to fail while remaining DISPATCHED. This is an independent **cross-project accounting integrity** control, not proof of a foreign-project data-access bypass. **NOT TESTED** until the new CI run reports. Keep both regression tests even after a global receipt uniqueness fix; test same-effect duplicate callback behavior too.

**Known confirmed security fail on preceding code:** [#44107204154](https://github.com/MousaXD/scriptotar/actions/runs/44107204154) combined failed `independent_v10_provider_receipt_must_not_settle_two_distinct_runs` against actual integration `fed7a5a7...`; only five of six independent matrix jobs passed. The newer owner v10 ledger source still lacked global receipt uniqueness at `402ce53...`, so a fix is pending. The corrected three independent zero-network wire tests PASS at `fed7a5a7...`; Tauri public paid dispatch still absent at that SHA. Agent 00 and Agent 02 are notified in [PR #114](https://github.com/MousaXD/scriptotar/pull/114#issuecomment-6952241532) and [PR #119](https://github.com/MousaXD/scriptotar/pull/119#issuecomment-6952282655).

**Native QA remains a separate FAIL:** [Agent07 #44107946419](https://github.com/MousaXD/scriptotar/actions/runs/44107946419) passed installed Windows initial project creation/switching, Study create and candidate edit/save+SQLite verification, but Windows restart/isolation was NOT TESTED; Linux keyboard/relaunch/Arabic/Settings controls failed. No production egress, merge to main or release authorized. Agent 10 production edits: NONE.

---

# Agent 10 — 2026-11-17 CONFIRMED cross-run v10 receipt accounting FAIL

**Security release gate: FAIL.** Executed independent workflow [#44107204154](https://github.com/MousaXD/scriptotar/actions/runs/44107204154) on audit commit `a462a717e650c51346b8a796467a4acad4725eec`, pinned integration target `fed7a5a7b752d5b74f46004bd866102cabdafb7e`. **5/6 independent jobs PASS** (provider, migration, writing, verification, combined-v11); **combined FAIL** with two independent findings. This replaces earlier "cross-run receipt NOT TESTED" statements below for this exact SHA.

1. **Confirmed ledger double-credit risk, not proven external double-billing:** `agent_effects::external::tests::independent_v10_provider_receipt_must_not_settle_two_distinct_runs` injected into the *real* `crates/scriptotar-db/src/agent_effects_external.rs` test module failed. Two different approved RUNNING run IDs each reserved an independent v10 claim, each passed the DISPATCHED pre-send CAS; settling the first with provider request ID/receipt `resp_same_provider_9123` did not prevent `reconcile_external_effect` from completing the second with that SAME receipt. Test log exact message: `v10 ledger accepted the identical provider receipt for two distinct paid runs`, **42 database tests PASS, 1 FAIL**. The external provider was not called and two actual charges were not observed. However, accepting one authenticated provider receipt twice at the authoritative ledger violates cross-run accounting integrity. Root: per-effect status/idempotency key is unique, but no global provider-scoped receipt uniqueness check is visible in the current v10 schema/reconcile implementation. Distinct from same-effect duplicate callback idempotence and source cancellation tests.
2. **Real paid public entrypoint remains absent:** `test_approved_paid_provider_dispatch_is_exposed_through_tauri_host` fails: private sealed host adapters and controlled model transports are not registered in actual Tauri `generate_handler!` for consent-approved real paid Studio dispatch. The legacy `run_ai` BYOK HTTP handler is separate and does not certify this boundary. Keep paid Studio dispatch OFF.
3. **Positive completed evidence:** corrected `provider_wire_boundary.rs` three tests **PASS** (altered fixed JSON/model/store/tools/token-cap, noncanonical bytes, hostile endpoints, forged receipts, zero-network deadline); `scriptotar-ai provider_openai_wire` three tests **PASS**; 10 independent consent race and 20 core provider-consent tests **PASS**; 92 native host Rust tests **PASS** and verification/writing checks **PASS**. No failure in these categories at this exact SHA.

**Agent 00/02 remediation required:** transactionally enforce uniqueness of provider request/receipt IDs **scoped by provider but globally across run, project, Study, retry attempt**, allowing same-effect *identical* late response idempotency, rejecting conflicting/other effect reuse, preserving uncertain/no-replay after crash, and migrating any existing duplicate records without silent data loss. Add at least deterministic cross-project and simultaneous cross-run settlement regression tests. DB uniqueness must survive process restart: in-process one-use transport HashSet is not sufficient for billing integrity. Keep the independent test as FAIL until this production repair is independently executed. Owner informed [PR #114](https://github.com/MousaXD/scriptotar/pull/114#issuecomment-6952241532); Agent 02 informed [PR #119](https://github.com/MousaXD/scriptotar/pull/119#issuecomment-6952282655). **Do not merge or approve release.**

**Newest production source after failing candidate:** Agent 00 advanced to `402ce53ef0643cf23b610571617e262ed3582a39`; the v10 ledger source blob remains `d27d0ace3a9470bc18515dce4bda77856f3d2160` and has no visible global receipt constraint. Owner added valuable shared origin mutex gates for transcription final commits, creator scans, watchlist auto-refresh, and legacy imports; existing host tests exercise lock ordering. Treat those as progress, not as authorization of provider I/O. New owner-head six-job run [#44112304916](https://github.com/MousaXD/scriptotar/actions/runs/44112304916) was still IN PROGRESS at inspection, and does not automatically run this new audit-only cross-run receipt test.

**Native acceptance separate FAIL:** Agent 07 [#44107946419](https://github.com/MousaXD/scriptotar/actions/runs/44107946419) on QA source `eecdbfda08da57f0787b4a8c98a2dc8ec576940e`: Windows installer WebView, create/switch two real projects, Study creation, and edit/save/duplicate with SQLite version-2 assertions **PASS**. Linux still fails native keyboard script candidate selector and relaunch pointer/Arabic/Settings controls. Windows relaunch persistence and wrong-project isolation remain **NOT TESTED**. Full installed GUI journey **NOT CERTIFIED**.

**Release verdict: NO-GO.** PR #124 stays DRAFT, paid Studio egress remains disabled. All production fixes belong to Agent 00/02; Agent 10 modifies only isolated tests/CI/handoff. The exact historical state lower in this document is retained for audit trail; never copy earlier "NOT TESTED" status forward to this now-confirmed v10 result.

---

# Agent 10 — new test corrections and present exact-HEAD CI status, 2026-11-17

**Current Agent 10 security branch SHA at the start of this handoff update:** `cbcb31f7a2a0d82ada0522e4475c34fe1241c450`. The corrected transport assertions now distinguish immutable JSON fixed-field validation from **consent-bound prompt digest validation** in the real `ConsentBoundary`/AppServices seal. The combined runner continues remaining Rust host and v11 suites even after independent transport or v10 receipt-replay FAIL, then returns a nonzero result; no security assertions were suppressed. The new v10 test probes provider-scoped receipt ID double accounting *across two different run IDs* and should be evaluated independently rather than assuming same-effect idempotency covers it.

[Audit six-job #44106404541](https://github.com/MousaXD/scriptotar/actions/runs/44106404541) was **QUEUED** when checked; [older #44104938087](https://github.com/MousaXD/scriptotar/actions/runs/44104938087) was 5 PASS / 1 combined FAIL at earlier parent `9ac69ea6` because the first transport test overclaimed that `send_prepared_once` alone enforces previously approved prompt bytes and the public paid Tauri entrypoint was missing. The test expectation is corrected; no live HTTP was attempted. The latest merged integration audit parent is `105fe912729e3d86078152c3baf2841f1586738e`; Agent 00's source has since advanced. Do not transfer statuses across commits.

**New native Agent 07 evidence:** [#44103511123](https://github.com/MousaXD/scriptotar/actions/runs/44103511123) at `46c8d1cb` failed overall but achieved installed Linux/Windows WebView+Studio PASS, 2-project creation/switch PASS and Study creation PASS on both platforms. Linux stopped on edit/save/duplicate at missing AT-SPI `Saved in this study` control; Windows left edit/save and persistence/isolation NOT TESTED. This supersedes old startup/heading failures; native release gate remains NO-GO.

---

# Agent 10 — additional independent no-network wire and cross-run receipt security gates (2026-11-17)

**Latest tested overlay source parent:** `2f965560072b7b49ac6cfa30274989066825764f` was overlaid onto integration **`105fe912729e3d86078152c3baf2841f1586738e`** as a two-parent security-only merge; production comparison includes only nine security test/workflow/docs paths. [New exact-head six-job run #44105971259](https://github.com/MousaXD/scriptotar/actions/runs/44105971259) was **QUEUED** when this entry was prepared; no PASS/FAIL is inferred from queue state. Agent 00 subsequently moved to `0bbd1ff7f70a3038c6d452f9cc4cff1c997337fb` (one frontend change beyond the audited parent), so do not reuse a pending audit result to certify later code.

**New regression A — exact provider bytes, hard bounds, forged receipts (NOT TESTED yet):** `tests/defragmentation/security/tests/provider_wire_boundary.rs` is copied only into the combined target's independent security crate. Three tests call the actual public `OpenAiResponsesWire` transport: alter fixed JSON model, `store`, tools, max-output tokens and even whitespace; verify `ConsentError::ChangedContext` before network admission. A different-but-canonical prompt deliberately passes *transport* body comparison and is refused only by the higher-level SHA-bound consent boundary, not the transport itself; an earlier overstrong prompt assertion failed in [#44104938087](https://github.com/MousaXD/scriptotar/actions/runs/44104938087) and was corrected at `156ba837`. This was a **test-layer error**, not proof of unauthorized egress; enforce endpoint/provider and token restrictions; reject receipt values not authenticated by the transport. Calls to `send_prepared_once` use deadline Unix time **zero** and an intentionally fake key, so no network requests or spending can occur. The gate also runs in-crate `cargo test --locked -p scriptotar-ai provider_openai_wire`.

**New regression B — v10 duplicate provider receipt across DISTINCT runs (NOT TESTED yet):** `tests/defragmentation/security/external_receipt_replay_additional_tests.rs` is appended transiently *inside the real database module's existing Rust test fixture* by the combined workflow. It prepares two independent approved RUNNING checkpoints in the same project and provider, obtains distinct idempotency reservations, commits a pre-send DISPATCHED CAS on each, completes the first with a fixture `provider_request_id=receipt_id=resp_same_provider_9123`, then asserts the same provider receipt cannot also complete the second. **This is a new expected fail-closed release gate**, distinct from the existing same-effect retry tests. At source-review time, `agent_external_effects` schema appeared to lack global uniqueness for `(provider, provider_request_id)` and `(provider, receipt_id)`; this is an **unconfirmed cross-run accounting risk until CI executes**. A passing check would establish no duplicate cross-run accounting; a failing check must not be suppressed without verifying backend uniqueness across attempts and projects. The test only runs in CI's copy of the database module, not production files.

**Runner integrity:** After a new v10 test fails, the combined gate records nonzero but still executes real Tauri host, verification, canonical writing and v11 tests to preserve independent evidence. The pre-existing Tauri registered paid-dispatch assertion also remains fail-closed; an unregistered provider path is NOT a certified public execution boundary. Both combined-v11 and combined check out the SAME PR-event-pinned integration SHA on each workflow; isolated feature jobs retain their own PR heads.

**Most recently completed Agent 10 security run before these tests:** [#44102210247](https://github.com/MousaXD/scriptotar/actions/runs/44102210247) was **5/6 PASS**, exactly integration `9ac69ea61d6e6f466bdbf07c13e20df68901c7d5`; the combined gate failed only the missing registered consent-approved Tauri command, while host policy, provider wire, v10 database and v11 writing subtests passed. The last completed **6/6 security PASS** predates the registered-path assertion at integration `a92b33f`, [#44094345395](https://github.com/MousaXD/scriptotar/actions/runs/44094345395). A green six-job snapshot before the new test must NOT be treated as full live paid network certification.

**RELEASE NO-GO / paid Studio egress OFF:** no registered public consent-approved real provider flow, verified live price catalog, crash-safe provider billing reconciliation, or complete source/background writer synchronization certification; separate Agent 07 installed graphical WebView acceptance still has not passed. Agent 10 has not merged, released, switched on production egress or edited Agent 00's source.

---

# Agent 10 — 2026-11-17 latest owner source `9ac69ea61d6e` and independent transport coverage

**Owner integration observed SHA:** `9ac69ea61d6e6f466bdbf07c13e20df68901c7d5`. Previous complete [6/6 green independent run #44094345395](https://github.com/MousaXD/scriptotar/actions/runs/44094345395) still belongs solely to `a92b33fed211d5a2f7417c4845c8fdf41d332933`; this newer candidate **has NOT received those same release assertions as PASS**. Agent 00 moved the OpenAI Responses HTTPS implementation into `crates/scriptotar-ai/src/provider_openai_wire.rs` and added an internal `HostOpenAiResponsesWire` adapter in `apps/desktop/src-tauri/src/services/provider_openai_wire.rs`; the newest version removes direct `reqwest` from the desktop manifest and reuses the previously established workspace lockfile graph. **No registered Tauri paid-consent command, trusted production model-price ceiling or independently reconciled provider billing receipt is verified; paid Studio egress remains OFF.**

**The Agent 10 combined runner now explicitly executes** `cargo test --locked -p scriptotar-ai provider_openai_wire` at the **same integration SHA**, in addition to the real host, policy, DB, writing and verification suites. This runs the candidate's no-network tests for the fixed provider endpoint, exact immutable model/prompt/max-output wire payload, invalid/incomplete provider responses, token usage mismatch and one-use in-process TLS response proof. It does **not** send a billable HTTP call or prove crash recovery. Failure remains a release blocker. Keep all tests and assertions; no suppression.

**Last audit exact-head [#44101288566](https://github.com/MousaXD/scriptotar/actions/runs/44101288566)** at prior integration `eb8e6498` was **4 PASS / 2 FAIL**: independent provider, migration, writing and verification PASS; combined-v11 failed before tests at `cargo --locked` (stale dependency graph); combined failed both on the **actual missing registered Tauri consent-approved execution command** and `cargo --locked`. Both combined jobs now use **one pinned exact source SHA** from the PR event, not independently moving refs. Owner subsequently changed the dependency graph; retest lockfile correctness rather than describing stale failure as current at this newer SHA.

**Separate native installed acceptance:** [#44094345602](https://github.com/MousaXD/scriptotar/actions/runs/44094345602) FAIL at old green security SHA `a92b33f` (Linux project heading, Windows Studio readiness), Agent 07 later [#44095877399](https://github.com/MousaXD/scriptotar/actions/runs/44095877399) remains FAIL although Windows Studio now appears. No installed end-to-end Study, restart, RTL or cross-project acceptance. **NO-GO release.** Agent 10 does not change production code.

---

# Agent 10 — 2026-11-17 newer exact owner source and independent gate status

**Audit synchronization target:** integration SHA `eb8e6498f02a2753818ba366e163b568a4190805`. Source added a private `OpenAiResponsesWire` transport with hard wire output limits and strict decoding, plus a separate private canonical four-step writing/orchestration seam and a `Cargo.lock` repair. The **last fully PASS six-job snapshot remains `a92b33f`**, not this newer commit; do not transfer green proof to the new source. The immediately preceding Agent 10 [independent run #44100255675](https://github.com/MousaXD/scriptotar/actions/runs/44100255675) exposed a **real not-yet-registered paid-consent Tauri dispatch** (`test_approved_paid_provider_dispatch_is_exposed_through_tauri_host` FAIL). Its two integrated jobs also encountered a separate stale `Cargo.lock` while the owner was pushing fixes: the combined-v11 job checked out `9e17d45`, and combined checked out `a509eb7` because the old matrix used moving `refs/pull/114/head`. **The new Agent 10 workflow now pins both combined jobs to one event-snapshot integration SHA**, or an explicit SHA on manual dispatch, avoiding mixed-source CI evidence. The lockfile was updated at owner `1de8c51`; repeat exact-head tests before marking it fixed.

The available private OpenAI wire is not an authorized public provider lifecycle: no registered Tauri paid-consent handler or trusted production pricing catalogue is known. Controlled host and v11 persistence are not live external billing certificates. The existing legacy `run_ai` BYOK command is an explicitly different user-key HTTP route; its consent policy scope requires owner decision. Native acceptance remains a separate FAIL/NOT TESTED release gate. **NO RELEASE APPROVAL / paid Studio execution OFF.**

---

# Agent 10 — 2026-11-17 NEW candidate OpenAI Responses transport inspection

**Integration candidate at inspection:** `f6b6e0d1d30c794f2a7fec6fdbfb2185ad083637` (four commits beyond **last fully green six-job source** `a92b33fed211d5a2f7417c4845c8fdf41d332933`). A genuine **Rust `CertifiedWireTransport` implementation now exists** at `apps/desktop/src-tauri/src/services/provider_openai_wire.rs`, registered as a private module in `services.rs`, and its initial local tests are in the source. The previous handoff statement that *no production HTTP implementation exists* is HISTORICAL at `a92b33f`, not true of the new source. However, **no trusted production `TrustedPricing` implementation, public Tauri paid-consent command, or audited end-to-end use of this transport is present**, so the real paid Studio journey is still **NOT TESTED / DISABLED**. Its presence is not certification of egress.

**Source strengths (NOT yet independently certified at `f6b6e0d`):** `OpenAiResponsesWire` sends to the fixed `https://api.openai.com/v1/responses` address; rejects alternate base URLs and incompatible providers; uses `Client::builder().no_proxy().redirect(Policy::none()).connect_timeout(10s).timeout(120s)` and a call-specific deadline; constructs immutable JSON with model, input, `store: false`, `tools: []` and `max_output_tokens` capped at 8192; checks byte-for-byte equality before passing precisely those bytes to `reqwest::blocking::Client`; bounds response reading to 1 MiB; rejects incomplete/non-200, unsupported tool/role/refusal output and mismatched token totals. Receipt proof uses an in-process one-use HashSet derived from an HTTPS response including ID/text/usage. These are suitable protections but require independent transport execution tests and exact-head CI evidence.

**Important trust limit:** TLS-received usage is *not a signed or independently reconciled provider billing record*. The one-use receipt proof is local process memory: after crash/restart there is no stored proof of a delayed provider response. Real `DISPATCHED` effects must therefore remain uncertain without replay until an authenticated reconciliation process exists. Current wire tests exercise deterministic body/response validation and local receipt consumption only; no network failure, HTTP cancellation, malformed transfer encoding, proxy/redirect, request fingerprint race or app crash/restart test through a registered command has been shown.

**Public legacy `run_ai` BYOK remains directly registered and uses `HttpAiProvider + AiService::generate` with a user-entered API key, distinct from Studio sealed consent. Agent 00 must explicitly scope any assertion that `all paid provider calls` are authorized, preserve legacy intended behavior or put it under an explicit policy; Agent 10 will not conflate this with protected Studio dispatch.**

[New exact-head independent security workflow #44099860571](https://github.com/MousaXD/scriptotar/actions/runs/44099860571) was QUEUED at inspection; the last verified **6/6 PASS** remains [#44094345395](https://github.com/MousaXD/scriptotar/actions/runs/44094345395) at `a92b33f`. [Native acceptance #44094345602](https://github.com/MousaXD/scriptotar/actions/runs/44094345602) **FAIL** on that last certified SHA; Agent 07 newer [#44095877399](https://github.com/MousaXD/scriptotar/actions/runs/44095877399) also **FAIL** despite Windows Studio discovery improvement. Keep NO-GO / paid egress OFF.

---

# Agent 10 — 2026-11-17 exact integrated six-job certification checkpoint

**Independent security workflow: [#44094345395](https://github.com/MousaXD/scriptotar/actions/runs/44094345395) ALL SIX JOBS PASS. Integration SHA `a92b33fed211d5a2f7417c4845c8fdf41d332933`.** Audit merge checkout used by that workflow: `68d04432eacc987ea7f0050528f8f21ba7e38a7a`. This successful CI supersedes all older failing claims about the missing fence constructor and the prior intermittent independent consent race. **Never claim those historical failures remain current at `a92b33f`.**

**Scope is controlled-host security, NOT permission to enable paid production HTTP. Release remains NO-GO pending production authorization path and separate native acceptance.** All six jobs targeted the exact integration SHA above (or independent feature PR SHAs as listed), and none used live provider credentials or real billable traffic.

| Boundary | Verdict and exact executed source | Command / CI evidence | Owner and remaining qualification |
| --- | --- | --- | --- |
| Consent, cancellation, altered request, concurrent same-run admission and source change; independent + core policy | **PASS**, 10 independent + 20 core, provider #118 `d6d738ec84ac6eff1aae26edbd79e44995f2e82b` | `cargo test --manifest-path tests/defragmentation/security/Cargo.toml --test provider_context_race -- --nocapture`; `cargo test --locked -p scriptotar-ai provider_consent`; [provider job](https://github.com/MousaXD/scriptotar/actions/runs/44094345395/job/134077398823) | Agent 01/10: isolated controlled policy/fence; no real paid HTTP, and a single passing concurrent run does not prove exhaustive scheduling |
| v4/v5/v10/v11 SQLite migrations, v10 effect CAS and uncertain/restart no-replay at DB level | **PASS**, DB feature #119 `9da21c96a62d7a041b64500841b89dc6dc649703` | `cargo test --locked -p scriptotar-db`; [migration job](https://github.com/MousaXD/scriptotar/actions/runs/44094345395/job/134077396135) | Agent 02: control-plane durable ledger certified, real host uncertain transport recovery not |
| Canonical writing/source references, forged references, foreign-project refusal, rollback | **PASS**, 32 native tests, writing feature #121 `467023db81985cc5af45bb6f28d93f2fd3fbd7af` | `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --test study_writing --locked -- --nocapture`; [writing job](https://github.com/MousaXD/scriptotar/actions/runs/44094345395/job/134077398848) | Agent 05: passes for canonical feature, not an outbound provider gate |
| Typed verification, fail-closed factual promotion, Python review contract | **PASS**, 25 verification Rust tests plus Python boundary, feature #117 `1ffb0fffad71a6fb2980966e1eec106174f543ff` | `cargo test --locked -p scriptotar-ai verification`; `python3 -m unittest discover -s tests/defragmentation/security -p 'test_*.py' -v`; [verification job](https://github.com/MousaXD/scriptotar/actions/runs/44094345395/job/134077396697) | Agent 04: independent human/third-party claim corroboration not claimed |
| Integrated v11 atomic three distinct drafts, immutability, idempotent receipt, cross-project refusal, restart/reopen, rollback | **PASS**, 35 writing tests, integration #114 `a92b33fed211d5a2f7417c4845c8fdf41d332933` | `bash audit/tests/defragmentation/security/run_feature_gates.sh combined-v11 114`; [v11 job](https://github.com/MousaXD/scriptotar/actions/runs/44094345395/job/134077396542) | Agent 00/05: repeat at any later exact candidate |
| Combined **controlled host** policy, real AppServices + v10 reserve/pre-send CAS/settlement, no-replay after fresh in-memory consent | **PASS**, host `91 passed; 1 ignored`, 4 structural checks and full DB/writing/policy suite, integration #114 `a92b33fed211d5a2f7417c4845c8fdf41d332933` | `bash audit/tests/defragmentation/security/run_feature_gates.sh combined 114`; [combined job](https://github.com/MousaXD/scriptotar/actions/runs/44094345395/job/134077396630). `sealed_host_dispatch_reaches_v10_once_and_never_replays` **executed PASS** | Agent 00/10: **not** a registered public Tauri command or authenticated production HTTP |
| Public Studio paid approval/dispatch, trusted model pricing and provider-origin authenticated usage, transport-level max tokens/deadline, no redirects/retries | **NOT TESTED / unavailable** at `a92b33f` | Source audit: private `prepare_provider_angle_consent_sealed` + `dispatch_provider_angle_consent_sealed`, but no `generate_handler!` command and no non-test `CertifiedWireTransport` implementation; normal consent preview uses `NoTrustedPricing`; legacy disabled adapter denies paid Studio send | Agent 00/01: implement real host-owned authority, production bounded byte-sealed provider and verified provider usage/receipt. NO-GO until controlled HTTP tests pass |
| Background origin updates, all project/Study/source/research/transcript mutation locks, cancellation after reservation, crash process/disk reopen, lost acknowledgement, exactly-once billing at provider boundary | **NOT TESTED end to end** at `a92b33f` | Existing source guard uses `AppServices::active_project`; Studio edits/project switch use same lock, but `JobOrchestrator::start_with_notifier(store.clone())`, `spawn_watchlist_refresher(store.clone())`, `scan_creator` and `import_legacy_data` have independent write paths. Their ability to mutate digest-relevant origins during I/O needs adversarial tests. Controlled host test resets `ConsentBoundary`, not process/disk | Agent 00/10: prove all relevant writers version-check or synchronize under one gate; simulate send timeout/lost ACK/crash/reopen and refuse any uncertain retries |
| **Public legacy `run_ai` BYOK endpoint** | **SOURCE-CONFIRMED DISTINCT EGRESS; authorization scope NOT CERTIFIED** | `lib.rs:run_ai` is registered; `services.rs:run_ai` accepts explicit user API key in BYOK mode and calls `HttpAiProvider::new(..)` / `AiService::generate(..)` without `ConsentBoundary`/v10 paid fence. It is NOT the new paid Studio workflow. | Agent 00: explicitly decide whether legacy user-key AI remains a separate consent/product policy; don't certify **all** paid provider traffic via Studio-only tests. Do not mistake a user-provided key for a durable spending authorization |
| **Installed Linux/Windows native WebViews** | **FAIL** on exactly `a92b33f`, separately from security | [installed run #44094345602](https://github.com/MousaXD/scriptotar/actions/runs/44094345602): Linux WebKitGTK UI PASS initial load, create/switch project heading FAIL; Windows UIA document exists but Studio control not confirmed after readiness window. Study/relaunch/project IPC NOT TESTED. Newer Agent07 [#44095877399](https://github.com/MousaXD/scriptotar/actions/runs/44095877399) at QA SHA `cede4c86` shows Windows document + Studio control PASS, yet native acceptance still fails; Linux project heading still FAIL. | Agent 07: fix/diagnose native journey and independently certify actual installed package persistence and isolation |
| **Newest independent Agent 10 audit checks** | **NOT TESTED at the latest audit head** `283f360c96b2dee1220190bc62fea0143a3a9c06` | Completed six-gate CI used audit checkout `68d04432...`, which has **4** structural host checks, while PR #124 additionally has `test_approved_paid_provider_dispatch_is_exposed_through_tauri_host` and improved fail-closed runner. This extra entrypoint check was NOT in the successful run. | Agent 10: synchronize audit-only tests with integration and run exact latest source; DO NOT weaken the missing production Tauri boundary |

## Guardrails and release decision

A **green controlled-host test does not certify real billed requests**. The private provider path binds the serialized bytes to a consent preview, repeats the digest under scope guard, uses the real SQLite v10 Fresh-only reserve + pre-send CAS, and verifies a **test-double** usage receipt. Production HTTPS response authenticity, hard provider request bounds, trusted prices, all mutator synchronization and actual process restart after an uncertain send remain unproved. No tests or docs are permission to enable live egress.

The older missing constructor / sporadic isolated race reports below are retained as *historical*, not present defects at `a92b33f`. **Do not merge or release.** Keep #124 DRAFT and current production paid Studio egress OFF. Agent 00 owns feature changes; Agent 10 only independent test/handoff updates. Re-run the six independent gates and the native installed workflows for each future exact candidate SHA.

---

# Agent 10 — last observed integration HEAD `c2bd18d` (2026-11-17)

Current owner SHA **`c2bd18d61e3f581e9e9263959423bc326d64ee82`** adds a **controlled test of changed serialized wire bytes** after approval in `services/provider_consent.rs`. The test toggles `corrupt_wire` and requires refusal before the controlled transport sends. This is a helpful additional adversarial test, **not yet independently CI-certified at this exact SHA** and not a real paid HTTP path. Previous [security CI #44092349962](https://github.com/MousaXD/scriptotar/actions/runs/44092349962) remains 5 PASS / 1 FAIL on earlier `9b249e2` (isolated concurrent loser outcome). [Next security #44093194240](https://github.com/MousaXD/scriptotar/actions/runs/44093194240) was still in progress against intermediate `19e798b`; [Rust foundation #44093194389](https://github.com/MousaXD/scriptotar/actions/runs/44093194389) **FAIL** on that intermediate SHA (exact cause not yet inspected). Latest build-native acceptance still unproved. No registered Tauri paid-consent entrypoint or production authenticated `CertifiedWireTransport`; release NO-GO, egress OFF. Future exact-head verdict requires new GitHub runs matching `c2bd18d` and independent Agent 10 tests imported/executed.

---

# Agent 10 — new owner exact SHA `19e798b` source inspection (2026-11-17)

**Updated integration head:** `19e798b54acc429621727e49ff520254db2d8882` (two source commits after the last **completed** security run at `9b249e2`). **Security/release remains NO-GO; paid egress OFF.** [New independent run #44093194240](https://github.com/MousaXD/scriptotar/actions/runs/44093194240) was **QUEUED** at review; [native #44093194425](https://github.com/MousaXD/scriptotar/actions/runs/44093194425) **IN PROGRESS**. The completed 5-PASS/1-FAIL matrix at the preceding SHA is preserved immediately below, not transferred to this new SHA.

Agent 00's new `prepare_provider_angle_consent_sealed` now constructs a preview with the **actual serialized wire SHA-256 included before user approval** via `ConsentBoundary::bind_exact_wire_request`; the policy's `dispatch` verifies `runner.prepared_request_sha256` against the approved sealed digest both before reservation and again under the locked pre-send admission scope. `SealedWireProvider` returns the matching SHA of the immutable buffer and sends those bytes. The controlled host test was updated to exercise sealed preparation and approval. This is a positive source-level correction to the exact-HTTP-bytes approval boundary, but **fresh exact-head CI is not yet certified**.

**Unresolved:** `prepare_provider_angle_consent_sealed` and `dispatch_provider_angle_consent_sealed` remain internal Rust methods, not registered user-reachable Tauri commands; no production HTTP `CertifiedWireTransport`, trusted provider price catalogue or authenticated receipt is registered. The prior audit at `9b249e2` still has a concurrent loser error-variant regression (test `concurrent_single_use_dispatch_has_only_one_pre_send_cas_winner`), and the new independent diagnostic from PR #124 remains to be imported. Real crash/reopen and multi-writer source synchronization are not proven. Do not certify on source review or controlled host unit tests alone.

---

# Agent 10 — COMPLETED exact-head independent CI at `9b249e2`, 2026-11-17

**RELEASE NO-GO: [independent security run #44092349962](https://github.com/MousaXD/scriptotar/actions/runs/44092349962) completed with 5 of 6 jobs PASS, 1 FAIL.** Exact target = Agent 00 #114 `9b249e2ba3dbab17113388039f2e854eafbc686b`, audit checkout `b68711bf7268f62c26fc53c25514fac38430423c` (integrated audit tests). This **supersedes the earlier 46cb36e production-constructor failure**: the new controlled sealed host test **executed and passed**, but no production HTTP/Tauri path was certified.

| Boundary | Status | Exact SHA, command and run | Owner / unresolved correction |
| --- | --- | --- | --- |
| **Isolated provider concurrent admission** | **FAIL**: 9/10 independent Rust tests pass; `concurrent_single_use_dispatch_has_only_one_pre_send_cas_winner` failed at line 498, unexpected losing-call error | #118 `d6d738ec84ac6eff1aae26edbd79e44995f2e82b`, audit `b68711bf`; `cargo test --manifest-path tests/defragmentation/security/Cargo.toml --test provider_context_race -- --nocapture`; [failed provider job 134070457052](https://github.com/MousaXD/scriptotar/actions/runs/44092349962/job/134070457052) | **Agent 01 + 10**: identify exact returned error and counts, reproduce deterministically, prove one CAS/send/receipt. No assertion suppressed or weakened. |
| Durable-effects DB migrations, CAS, uncertain recovery | **PASS (DB only)** | #119 `9da21c96a62d7a041b64500841b89dc6dc649703`; `cargo test --locked -p scriptotar-db`; [migration job](https://github.com/MousaXD/scriptotar/actions/runs/44092349962) | Agent 02; host restart-after-uncertain HTTP still untested |
| Canonical writing / foreign project / forged sources | **PASS (feature)** | #121 `467023db81985cc5af45bb6f28d93f2fd3fbd7af`; `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --test study_writing --locked -- --nocapture`; [writing job](https://github.com/MousaXD/scriptotar/actions/runs/44092349962) | Agent 05; preserve version/receipt immutability |
| Fact verification fail-closed | **PASS (isolated)** | #117 `1ffb0fffad71a6fb2980966e1eec106174f543ff`; `cargo test --locked -p scriptotar-ai verification`; [verification job](https://github.com/MousaXD/scriptotar/actions/runs/44092349962) | Agent 04; independent trusted corroboration still not proven |
| **Integrated v11 3-draft SQLite persistence / rollback / restart** | **PASS**, 35 writing tests | #114 `9b249e2ba3dbab17113388039f2e854eafbc686b`; `bash audit/tests/defragmentation/security/run_feature_gates.sh combined-v11 114`; [combined-v11 job 134070456924](https://github.com/MousaXD/scriptotar/actions/runs/44092349962/job/134070456924) | Agent 00; re-run on every further release SHA |
| **Integrated controlled host authorization path** | **PASS (controlled-host Rust + older structural checks, NOT public paid execution)** | #114 `9b249e2ba3dbab17113388039f2e854eafbc686b`; `bash audit/tests/defragmentation/security/run_feature_gates.sh combined 114`; [combined job 134070457117](https://github.com/MousaXD/scriptotar/actions/runs/44092349962/job/134070457117) | Agent 00; `sealed_host_dispatch_reaches_v10_once_and_never_replays` **PASS** in real `AppServices`/v10 SQLite (controlled transport), host `91/91` executed tests PASS. **Do not call this full production certification.** |
| Registered Tauri consent-to-billable-transport, model/budget bounds, authentic provider receipts, verified wire bytes | **NOT TESTED / not implemented for real network** | #114 `9b249e2`: no registered paid-consent Tauri handler, no production `CertifiedWireTransport`, and normal preparation still uses `NoTrustedPricing`; newly added Agent 10 test from PR #124 **has not been imported into this completed run** | Agent 00/01: source approval and ExecutionContext must originate from persisted host authority; then invoke sealed v10 CAS with production bounded transport and authenticated receipts. |
| Concurrent Study origin/background writes, cancellation, real crash during dispatch/restart/no replay | **NOT TESTED through registered host path** | #114 `9b249e2`; isolated mock/DB tests above are not full host race certification. New host test resets `ConsentBoundary`, **not disk/store process** | Agent 00/10: coordinate every digest-relevant mutator or use atomic version gate; simulate timeout, lost ACK, cancellation, crash/reopen, conflicting requests without live paid egress. |
| Native installed Linux + Windows WebViews | **NOT CERTIFIED** on current exact SHA | [native run #44092349958](https://github.com/MousaXD/scriptotar/actions/runs/44092349958) was **IN PROGRESS** when recorded; [older completed #43956319863](https://github.com/MousaXD/scriptotar/actions/runs/43956319863) failed on `46cb36e`, which is not current | Agent 07: require controlled installed Studio journey, project persistence/relaunch, Linux/Windows visual/a11y, project isolation. |
| Toolchain and ordinary CI | **PASS Rust foundation** on latest target | [Rust foundation #44092350051](https://github.com/MousaXD/scriptotar/actions/runs/44092350051) on `9b249e2`; Rustfmt failure on `eaefeb7` corrected by Agent 00 | Agent 00; ordinary checks do not certify security boundaries |

**Important independent race interpretation:** source `ConsentBoundary::dispatch` sets `Decision::Uncertain` during the in-flight send admission window, whereas the failing concurrent test currently insists every losing caller returns `AlreadyUsed`. `Err(UncertainEffect)` is one plausible fail-closed outcome, **not yet observed** because the old assertion did not print the actual variant. The failure occurred before the final atomic counts were checked; **no duplicate send has been demonstrated or ruled out by that failed test**. Agent 10 PR #124 commits `43557b7` + `0a783e4` add error-variant and atomic reservation/CAS/send/receipt diagnostics to the existing assertion **without changing its expected result**; bring those independent test changes into the owner gate to identify the precise race. Do not suppress the test or declare a benign flake without deterministic evidence. [Agent 00 notified](https://github.com/MousaXD/scriptotar/pull/114#issuecomment-6949405278).

**Owner's earlier real missing-fence caller is fixed as a *non-public controlled host* path. Full public Tauri/paid transport and origin-write isolation remain release blockers.** Keep PR #124 DRAFT, paid egress OFF; no merge/release security signoff.

---

# Agent 10 — newest exact integration SHA `9b249e2`, 2026-11-17

**Current owner HEAD at final refresh:** `9b249e2ba3dbab17113388039f2e854eafbc686b`. This is a **rustfmt-only change** to `services/provider_consent.rs` over the sealed controlled-host path at `eaefeb7`; it does **not** add a production HTTP transport or registered Tauri paid-consent command. Agent 00 corrected the [earlier rustfmt failure #44091887127](https://github.com/MousaXD/scriptotar/actions/runs/44091887127) by pushing the formatting commit. Exact-head [security #44092349962](https://github.com/MousaXD/scriptotar/actions/runs/44092349962), [Rust foundation #44092350051](https://github.com/MousaXD/scriptotar/actions/runs/44092350051) and [installed native #44092349958](https://github.com/MousaXD/scriptotar/actions/runs/44092349958) were **IN PROGRESS at this review**. Older combined-v11 PASS at `eaefeb7` and native failure at `46cb36e` must not be silently transferred to this SHA. Keep paid dispatch disabled and security/release **NO-GO** until completed current exact-head evidence exists. The more detailed source findings in the next section remain applicable because the intervening diff is formatting only.

---

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
