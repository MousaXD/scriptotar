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
