# Agent 10 — independent security/adversarial QA handoff

**Audit date:** 2026-11-16. **Disposition: NO-GO for integration/release approval.** This is a draft security review, not a penetration-test certificate. Do not merge or publish based on upstream green CI alone. No feature-owner production files were changed.

## Immutable review scope

| Work | Exact head reviewed | CI at that head |
| --- | --- | --- |
| Integration baseline PR #114 | `3009989a3aa73248b6432ba0cd84821c59110993` | 9/9 workflows successful |
| Fact verification PR #117, Agent 04 | `9267066ce3576431b666c6c8cb0f20ba8eaad228` | 7/7 successful |
| Provider consent PR #118, Agent 01 | `bcb3f3a7d08426ee013530236ea2252012fa4da3` | 3/7 successful, 4/7 failed |
| Durable effects PR #119, Agent 02 | `273a8ded0285b43aa09da968855ebb4aee86c26b` | 7/7 successful |
| Research PR #120, Agent 03 | `62600e20ec1a8d9f2914fdcb0d9b97a2931c0acd` | 7/7 successful |
| Writing PR #121, Agent 05 | `47fe3965524278844519c36fd2689fd404f634ec` | 7/7 successful |

All five feature PRs were **separate unmerged branches**, not a validated combined product. These statuses were read from the GitHub workflow runs for each exact head. Workflow success proves the configured jobs passed, not installed graphical QA, paid API testing or an independent attack simulation. At this checkpoint PR #114 remained draft. The integration baseline has the existing local-read effect ledger but does not contain the new five PR implementations.

## Blocker findings (prioritized)

### SEC-10-01 — Changed Study after consent check can still reach provider (P1, FAIL — source-code finding)

- **Owner:** Agent 01, with Agent 02 for paid-fence coordination and Agent 00 for host synchronization.
- **Affected head:** PR #118 @ `bcb3f3a7d08426ee013530236ea2252012fa4da3`.
- **Component:** `crates/scriptotar-ai/src/provider_consent.rs`, `ConsentBoundary::dispatch`.
- **Reproduction:** Approve an exact prompt and Study/material digest. Supply a deterministic test fence whose `reserve_once` changes the authoritative Study material digest *after* `validate_context` has returned successfully, before the `BoundedProvider::generate_once` call. The dispatch code does not revalidate the scope after this reservation and before sending. A counting mock provider is still invoked.
- **Expected:** An invalidated scope prevents provider invocation, even when the effect reservation has already committed. Safely settle/mark the reserved effect uncertain; never treat denial as a replay permit.
- **Observed code path:** Validates the scope, obtains the durable reservation, drops the consent-record lock, then invokes `generate_once` without a new scope check. This is a deterministic TOCTOU window; concurrency between DB/active project changes and send is not fenced by the in-memory consent mutex.
- **Impact:** If the host later enables this path, user changes to Study/source access or project selection can race with private prompt egress and paid spend. **Not a current paid-provider exploit:** PR #118 deliberately uses `NoTrustedPricing`, `DisabledPaidEffectFence`, and `ExistingAiServiceAdapter` to deny production dispatch.
- **Adversarial regression:** `tests/defragmentation/security/tests/provider_context_race.rs::changed_study_after_effect_reservation_must_not_egress`. A rejecting-fence control is included. **Not executed locally**: Rust toolchain and feature checkout are unavailable in this audit runtime. The test is expected to fail on this exact feature SHA; run it before accepting the fix.
- **Remediation:** Agent 01 + 02 + 00 should define an atomic approval/dispatch epoch or hold the authoritative lock across the final send boundary, with cancellation/context invalidation enforced at the last permitted point. A single extra recheck narrows but does not eliminate all possible races. Confirm the final integrated transport cannot launch an HTTP request after revocation at the defined linearization point.

### SEC-10-02 — Fact-verification adapter accepts caller-authored draft body at canonical ID/revision (P1, FAIL — source-code finding)

- **Owner:** Agent 04; Agent 00 owns the canonical Studio adapter and binding.
- **Affected head:** PR #117 @ `9267066ce3576431b666c6c8cb0f20ba8eaad228`.
- **Component:** `apps/desktop/src-tauri/src/services/study_verification.rs`, `verify_studio_claims_read_only`.
- **Reproduction:** Create a real saved Study/draft with revision N. Supply a different `ScriptDraft` object with the **same draft UUID and revision N** and arbitrary changed narration or factual claims. The method checks `get_draft` for project/Study/revision/archived status and rereads saved metadata, but runs `extract_draft_claims(draft)` and builds citations from the caller-supplied object's claims rather than from a canonical saved-body snapshot. PR #117's own passing `sqlite_adapter_rejects_foreign_study_and_stale_draft_revision` test constructs a `ScriptDraft` without persisting that same body and receives a successful report.
- **Expected:** Refuse forged draft content or derive claims from an authoritative immutable draft version/content digest within the same trusted snapshot. Verify the complete submitted contents correspond to the saved draft, not just metadata.
- **Impact:** Claim reports may be falsely attributed to a saved draft revision they do not actually describe; future independent-review/publish gates could trust attacker-selected text. **Current limitation:** The adapter is read-only and deliberately has no authenticated independent fact-review ledger, and it returns no independently verified claims. This is a correctness/security integration blocker, **not evidence that production already certifies forged claims**.
- **Adversarial regression:** `tests/defragmentation/security/test_study_claim_boundary.py`; it fails on exact PR #117 when the caller object is used directly for claim extraction. This guard is static and must be complemented by a native Rust test covering different content under a matching canonical ID/revision. It SKIPS on unintegrated baseline and **was not executed in this audit runtime**.
- **Remediation:** Load the canonical draft body/immutable version under a consistent transaction (or verify a persisted digest) and derive the claim inventory from that trusted snapshot. Add a native wrong-body/same-ID/same-revision denial test; reject an update racing verification. Keep the existing fail-closed unverified status until true independent review storage exists.

### SEC-10-03 — Provider consent exact-head CI is red (P0 validation gate, FAIL)

- **Owner:** Agent 01.
- **Affected head:** PR #118 @ `bcb3f3a7d08426ee013530236ea2252012fa4da3`.
- **Reproduction/evidence:** GitHub Actions at this SHA: Rust foundation, CI, MCP E2E and Tauri migration integration failed, while Security hygiene, Windows Installer and Linux Package succeeded. Rust foundation run `43832900049`, job `133182271393` fails at **Rustfmt**. The log reports a formatting diff in `crates/scriptotar-ai/src/provider_consent.rs` near line 503 around `usage.cost_microusd = match reserved_cost(...)`. Exact run: https://github.com/MousaXD/scriptotar/actions/runs/43832900049.
- **Impact:** No exact-head all-green validation. This is an objective CI gate failure, not proof of a security exploit.
- **Remediation:** Run `cargo fmt --all` on PR #118's branch and require all mandated workflows and any newly added security regressions to pass on the **new final head**. Do not quote green CI from earlier commits.

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
| Fact claims, provenance, human reviews | **FAIL (SEC-10-02)** | PR #117 has conservative unreviewed/disputed mapping and does not claim independent fact checks, but canonical draft body is not bound to passed claims. |
| Candidate transaction atomicity / cross-process races | **NOT TESTED** | PR #121 defines `AtomicCandidateStore`; its SQLite sink is a **test fixture**. The three-candidate canonical production transaction has not been registered. |
| Study and draft revision integrity | **FAIL (SEC-10-02)** | Same saved UUID/revision is not sufficient to bind caller-authored claim content. Baseline Studio CAS tests are upstream, not independent attack certification. |
| Secrets, private source data, exports and logs | **NOT TESTED** | Provider key currently passed only at dispatch. No forensic real-log, export redaction, clipboard, OS crash-dump or installed-app filesystem audit. |
| Dependency advisories, workflow pins, installed app | **NOT TESTED** fully | Baseline security-hygiene workflow checks immutable GitHub Actions pins and tracked content; it does **not** by itself establish cargo/npm vulnerability advisory clearance or native WebView security. Issue #100's independent native acceptance remains open. |

**Status convention:** `PASS` is limited to the explicitly described static/existing mocked evidence; `FAIL` marks a concrete code-path or CI gate defect; `NOT TESTED` means no independent negative or native/prod proof, never an implicit pass. This report does **not** establish the combined five-PR system secure.

## Integration requirements for Agent 00

1. Keep #114 and all feature PRs draft/no-release until the relevant owners fix SEC-10-01/02, PR #118 passes exact-head CI, and the five branches are actually composed.
2. Register the independent negative tests in an integrated CI job once the feature modules exist. The nested Rust harness is intentionally separate from the root workspace and is not run by the current root `cargo test --workspace` job. Python discovery requires an explicit command; root CI presently runs narrower Python suites. Never equate skipped/non-executed tests with green certification.
3. For installed-app certification, separately require real Tauri WebView tests and real device evidence under issue #100; Chromium-mock screenshots and packaging workflow success do not substitute.
4. Introduce no paid AI egress until a trusted hard-capped transport, server-owned immutable scope/prompt/approval binding, durable effect receipts, user-visible budget limits, and rejection of stale context are proven on the integrated exact SHA.
5. Protect the history of tests and findings. Review and acknowledge by feature owner; security Agent 10 does not edit other agents' production code and does not approve merges or releases.

## Work limitations

The audit used the GitHub connector to inspect exact-head source, the five PR change lists, and exact-head GitHub Actions metadata/logs. No Rust toolchain or network-accessible checkout was available in the local execution container. The new Rust and Python negative tests were authored but **not executed**. There were no live API calls, real secrets, independent installed WebView sessions, dynamic network fuzzing, or filesystem permission exploitation. Claims above are limited accordingly.
