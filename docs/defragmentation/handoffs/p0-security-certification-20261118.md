# Agent 10 — Independent adversarial security certification (18 November 2026)

**Decision: NO-GO / RELEASE VETO.** No authorization for billable provider execution, merge to `main`, or a production release. This is a record of observed CI and inspected source, not a claim of a complete penetration test or successful new live-provider test.

## Immutable evidence identifiers

- **Production integration SHA:** `7af95c574643c14565a5c10d0be2cc78c5e57254` on `integration/scriptotar-defragmentation-agent00-20261113`.
- **Independent security runner/audit checkout SHA observed in all seven jobs:** `244558794d6574ae0a8d48045bd19e2bb79d3e18`. This is the **checkout actually printed in the workflow logs**, not the Agent 10 branch tip. The Agent 10 PR #124 head remains `33ea7b22fac9f89863126799d8f4f88faf38f0b7`. The relevant independent `test_combined_host_wiring.py` and `provider_context_race.rs` files have matching Git blob identities on #124 and integration, but a matching file is not a substitute for exact SHA provenance.
- **Integrated seven-job regression:** https://github.com/MousaXD/scriptotar/actions/runs/44278804909 — **7/7 PASS**, with the combined and combined-v11 jobs explicitly checking out #114 at `7af95c57`. The five feature gates check their separate feature PR heads, **not** `7af95c57` in those jobs.
- **Integrated provider race:** https://github.com/MousaXD/scriptotar/actions/runs/44278805552 — **PASS**, 10 independent race tests and 20 core consent tests logged.
- **npm supply chain:** https://github.com/MousaXD/scriptotar/actions/runs/44278804900 — **FAIL**, Supply chain / npm advisories.
- **Real installed native acceptance:** https://github.com/MousaXD/scriptotar/actions/runs/44278805547 — **FAIL / incomplete**, details below.
- **Agent 07 draft QA follow-up:** https://github.com/MousaXD/scriptotar/pull/131, observed head `1f4711550c2996be53cf0eac154040c19cbda125` — independent changes are not demonstrated as installed and accepted on the exact final integration head.
- **Agent 11 independent supply-chain branch:** `agent-11/p0-supply-chain-recovery-20261118` at `94092d99370bd3101a7fdf7b92369a215e27369b` (observed); diff from integration is **one documentation-only file**, not a dependency remediation. Its new CI was in progress at inspection.

## Gate matrix

| Gate | Test source SHA | Production / feature source SHA | Observed outcome | Security scope / limitation |
| --- | --- | --- | --- | --- |
| Provider consent, revocation and budget #118 | `24455879` | `d6d738ec84ac6eff1aae26edbd79e44995f2e82b` | **PASS** | 10 independent + 20 provider tests on feature checkout; not live egress |
| Durable DB migration #119 | `24455879` | `9da21c96a62d7a041b64500841b89dc6dc649703` | **PASS** | Rust DB suite against feature checkout |
| v12 receipt identity #128 | `24455879` | `23f4e2727dde322c3ab7da8bb2a20f6c88d078bc` | **PASS** | receipt/replay/upgrade negative tests on feature checkout |
| Writing #121 | `24455879` | `467023db81985cc5af45bb6f28d93f2fd3fbd7af` | **PASS** | 32 canonical-writing tests on feature checkout |
| Verification #117 | `24455879` | `1ffb0fffad71a6fb2980966e1eec106174f543ff` | **PASS** | verification + Python claim boundary on feature checkout |
| Combined v11 #114 | `24455879` | **`7af95c574643c14565a5c10d0be2cc78c5e57254`** | **PASS** | current integrated DB and writing; not full host |
| Combined host #114 | `24455879` | **`7af95c574643c14565a5c10d0be2cc78c5e57254`** | **PASS** | 10 independent provider race, 3 wire, 20 policy, 69 DB, 97 host (one ignored), 25 verification, 35 writing and Python structural checks logged; no billable HTTP |
| Independent provider-race workflow | integrated workflow source `7af95c57` | `7af95c57` | **PASS** | 10 race and 20 core consent tests; synthetic/no-cost |
| npm advisories | integration CI at `7af95c57` | `7af95c57` | **FAIL** | 8 high npm findings from one known picomatch advisory |
| Real Linux installed acceptance | native CI at `7af95c57` | `7af95c57` | **FAIL** | 10 PASS, 1 FAIL, 5 NOT TESTED; malformed 3-angle JSON did not yield the required visible accessible rejection |
| Real Windows installed acceptance | native CI at `7af95c57` | `7af95c57` | **INCOMPLETE / job FAIL** | 10 PASS, 0 FAIL, 6 NOT TESTED; workflow correctly refuses to call incomplete acceptance PASS |
| Agent 07 follow-up / Agent 11 fix | separate branch heads | **not integrated and requalified** | **NOT CERTIFIED** | neither branch establishes final installed gate closure or patched dependency lockfile |

**Do not read "7 security jobs passed" as a claim that the overall supply-chain or installed-app security release criteria passed.** A structural Python host assertion confirms registration/call-site wiring, not universal cross-process source mutation exclusion or authenticated paid-provider receipt settlement.

## Independently inspected boundaries

1. **Revocation/cancellation/approval-to-dispatch:** The policy rechecks context after reservation, acquires `ScopeDispatchGuard`, rechecks cancellation under consent mutex, invokes durable `mark_dispatched_once` before `generate_once`, and conservatively marks uncertain outcomes. Agent 10 race tests and integrated host tests pass. **Gap:** `HostScopeDispatchGuard` is a process-local `active_project` mutex; exact synchronization against every external/other-process origin writer has not been demonstrated. No real billable dispatch/cancel/restart sequence was executed.
2. **Duplicate effects, crash replay, ACK identity:** v12 DB and injected negative tests pass, including cross-run receipt, provider alias, historical data collision, rollback and tombstone protection. **Gap:** end-to-end authenticated remote ACK, crash **after actual network send**, and recovery across multiple desktop processes are not certified.
3. **Forged foreign-project Tauri IPC:** Scoped Rust host/service regressions exist and pass. **Gap:** installed native `wrong_project_ipc_denial` is `NOT TESTED` on both OSes; graphical isolation is not a forged invoke denial.
4. **Three-angle AI and evidence:** Rust verification and writing contracts pass; native Linux malformed JSON error recovery **FAILS** as observed. Evidence is not allowed to be auto-verified by merely referencing source IDs. **Gap:** actual installed malformed/unauthorized source negative-recovery proof, foreign evidence rejection through the real native IPC, and absence of unauthorized SQLite writes are not certified as a complete scenario.
5. **Removed/stale Study sources:** scoped source checks and no-stale-write regressions pass in Rust. **Gap:** genuine installed multi-source transcript and revocation tests are `NOT TESTED` on both OSes.
6. **Untrusted pricing, unexpected cost, exhausted budget:** The registered `execute_provider_consent_sealed` entrypoint leads to `execute_openai_provider_consent`; inspected production `certified_paid_egress_enabled()` is **hardcoded `false`**, and `NoTrustedPricing` is passed to public preview/dispatch. This is a positive **fail-closed release safeguard**, not a completed paid feature. No authenticated live price catalog, settled real-provider cost or overspend/retry after remote success is certified.
7. **Tampered citations/drafts and error paths:** the verification, immutable writing, SQLite rollback and v12 integrity suites pass; some checks are static and the full installed negative/error recovery remains unresolved. No comprehensive runtime forensics establishing zero sensitive leakage in all IPC/log error branches was performed.

## Confirmed release blockers and reproduction

### P0 — npm supply-chain advisory not remediated
**Boundary:** untrusted glob pattern -> vulnerable dev/build/test matcher and potentially CI/automation event loop. Impact: high-severity regular-expression denial of service under exploitable pattern input; packaged desktop exploitability **not established**. Reproduce on `7af95c57`:
```sh
cd apps/desktop-ui
npm ci
npm audit --audit-level=high
npm ls picomatch fdir tinyglobby vite vitest svelte-check
```
Actual runner logged **8 high** findings and `GHSA-fc5q-39xc-vjpf` in `picomatch` with propagated paths through fdir, tinyglobby, Vite, Vitest and Svelte tooling; npm audit step exited 1. `npm audit fix --force` proposed a breaking Vitest change; reject blind force, threshold changes or suppressions. Agent 11's source branch changes a document only. External advisory: https://github.com/advisories/GHSA-fc5q-39xc-vjpf . Obtain independently verified advisory-clean dependency graph and rerun original unmodified supply-chain job. **Owner: Agent 11**, integration review Agent 00.

### P0 — installed Linux negative-path behavior fails
**Boundary:** untrusted three-angle JSON -> production Tauri IPC -> accessible user denial, no false approval or draft persistence. Reproduce with installed Linux package built from `7af95c57`: start native app, create/switch project and canonical Study/draft, open Studio's "Prepare 3 creative angles"; paste JSON containing only one angle and click "Validate 3 angles"; demand visible accessible denial. CI reports `NativeFailure: Malformed three-angle JSON produced no visible native rejection`. The check fails **before** proving successful recovery and unchanged canonical draft/version under this scenario. This proves missing *visible rejection evidence*, **not** that the Rust backend actually accepted invalid JSON. Agent 07's PR #131 proposes a different GUI URL-denial/recovery test, not retrospective certification of this failure. **Owner: Agent 00 production behavior, Agent 07 installed QA**.

### P0 — incomplete Windows/Linux native security coverage
**Boundary:** foreign project IDs, revocable sources, genuine multi-source transcript, robust negative/error recovery, RTL and theme visuals. Reproduce `Independent Installed WebView Acceptance` on exact final integrated head; inspect both `result.json` files, demand every required test PASS (not a blanket green on generic GUI CI). Observed Linux: 10 PASS / 1 FAIL / 5 NOT TESTED; Windows: 10 PASS / 0 FAIL / 6 NOT TESTED. Both actual job conclusions FAIL. **Owner: Agent 07**, required production support Agent 00.

### P0 before enabling paid egress — cross-process authority and authenticated cost/receipt proof
**Boundary:** approval -> origin mutations -> immutable wire send -> provider reply/usage -> v12 settlement. `certified_paid_egress_enabled() -> false` is currently the correct behavior. To qualify a future activation, use controlled remote/mock HTTP with a fixed server, deterministic pause at reserve/CAS/send/ACK, and a second process altering the canonical SQLite source; assert **no network send after a revocation that wins before dispatch**, exactly one effect transition, no false-success status, no duplicate bill, no leaked key/prompt in errors or DB. Then separately verify authenticated receipt identity and trusted published model pricing before any real charge. Existing tests do **not** satisfy all these release criteria. **Owner: Agent 00**, Agent 10 retest.

### P1 — Rust warnings and Python audit coverage require independent follow-up
Agent 11's recorded inspection of the failing supply-chain job identifies 10 permitted `cargo audit` warnings, including rustix/tokio/glib; they are **not** a clean vulnerability attestation. The npm failure stops the later Python audit in this job, so `pip-audit` result is **unknown**, not PASS. Confirm severity and exploitability from locked versions, remediate in the responsible workstream and rerun unsuppressed gates.

## Rules for revoking this veto

Agent 00 must land reviewed production/native repairs **on the intended integration branch**, Agent 11 must supply a verifiably clean npm graph or preserve NO-GO, and Agent 07 must produce full installed Linux+Windows `PASS` for previously untested and failed security cases. **Then run all independent security gates against the exact same final integration SHA**, retain audit/test SHA and job logs, inspect secret/error telemetry and verify paid provider remains disabled until a separately certified real-provider path exists. Any source SHA change invalidates prior release certification. Do not squash away independent evidence or edit red results to green.

**Audit conduct:** inspected GitHub source/workflow/job logs and upstream advisory; did not modify production, dependency manifests, audit thresholds, guard code, tests or `main`. No new local compilation, live HTTP billing, multi-process adversarial runtime, or installed-app execution was performed during this independent report-writing pass.