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
