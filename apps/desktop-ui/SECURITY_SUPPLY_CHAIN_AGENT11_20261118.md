# Agent 11 — Independent npm supply-chain recovery assessment (2026-11-18)

## CURRENT CORRECTED ADVISORY STATUS — WITHDRAWN, BUT SECURITY POLICY STILL BLOCKED

**Verified primary source:** the public [GitHub Advisory Database repository](https://github.com/github/advisory-database/blob/main/advisories/github-reviewed/2026/10/GHSA-fc5q-39xc-vjpf/GHSA-fc5q-39xc-vjpf.json) records `withdrawn: "2026-11-18T15:38:56Z"` and `modified: "2026-11-18T15:38:56Z"` for GHSA-fc5q-39xc-vjpf. This is an **actual formal GitHub withdrawal**, not merely an upstream maintainer request. The historical affected range `introduced: 0, last_affected: 4.0.7` and original technical description remain in the withdrawn record; they must NOT be misrepresented as a still-active reviewed advisory. GitHub advisory database commit `7bee34adc2028ebd00c2e87fe43e432cd134fe40` published this revised record at 2026-11-18 15:44 UTC. Earlier sections of this report incorrectly claimed no withdrawal; treat those statements as HISTORICAL AND SUPERSEDED by this correction.

**Registry feed now reconciled — strict npm audit PASS:** At 2026-11-18 **16:59 UTC**, the original `npm audit --audit-level=high` had still reported eight high findings despite the GitHub withdrawal ([historical run #44352653285](https://github.com/MousaXD/scriptotar/actions/runs/44352653285)). By 2026-11-18 **17:24 UTC**, the identical **unfiltered mandatory command** returned exit code 0 and **0 vulnerabilities** on a clean npm-locked install, as proven by the successful audit step and JSON metadata `high:0, critical:0, total:0, issues:[]` in [run #44357988727](https://github.com/MousaXD/scriptotar/actions/runs/44357988727) on exact SHA `f1db4a1abec663eaf707eaee424c608ef9335091`. No npm override, cache manipulation, severity flag, waiver, changed registry, dependency replacement or lockfile change occurred. This was **legitimate advisory metadata propagation**, not a parser patch or evidence that pathological glob patterns were fixed. The original integrated npm security workflow must be rerun on its own final exact SHA before anyone claims that separate release gate PASS.

**Actual technical risk remains demonstrable:** unchanged published `picomatch@4.0.7` reproducibly catastrophically backtracks on crafted pathological globs. GitHub withdrawal changes advisory metadata, **not executable behavior**. Existing public upstream PR260/264 source-only candidates each failed real expanded bounded tests on both Linux and Windows on exact tested SHA. The residual question is whether attackers can influence glob expressions/inputs in the specific build/dev/native deployment context; this requires Agent 10 review and Agent 00's release-policy decision, not automatic acceptance of either npm or GitHub metadata.

**Production reachability — initial limited static evidence PASS on Linux and Windows:** all five watched parser/build-tool entries (`picomatch, tinyglobby, fdir, vite, vitest`) are dev-only (`dev: true`) in the frontend lockfile. The production Tauri config `apps/desktop/src-tauri/tauri.conf.json` points `frontendDist` to `../../desktop-ui/dist` and contains no additional bundle resource entries. The read-only `scripts/audit-desktop-runtime-exposure.mjs` checked actual built JS/CSS/HTML and source-map input metadata and found **no named picomatch/tinyglobby/fdir indicators**, independently **PASS on Linux and Windows** in [run #44357988727](https://github.com/MousaXD/scriptotar/actions/runs/44357988727). This is **limited static evidence only**, NOT a proof of full installed-executable reachability, absence of minified equivalent parser code, or safety against untrusted build-time globs.

**Release decision: BLOCKED PENDING INDEPENDENT RESIDUAL-RISK REVIEW / NO-GO:** the npm advisory withdrawal and strict audit PASS close the **metadata security finding on the verified SHA**. The independent `--assert-patched --extended` laboratory check **still FAILS** on actual published `picomatch@4.0.7` because crafted untrusted glob patterns demonstrably backtrack. Agent 10 must independently evaluate attacker control of glob expressions and candidate inputs during Scriptotar source builds and in installed runtime; Agent 00 must make an explicit risk-policy/release decision. All other required native/security and final integration gates remain separate. Keep draft PR #132 unmerged and **do not remove or disable any failing adversarial assertion just to turn CI green**.

---

**Historical pre-withdrawal assessment below: evidence retained for provenance; this section's old blocker statements are superseded by the corrected current status above.**

## Provenance and reproduction

- Target repository: `MousaXD/scriptotar`; baseline: integration branch `integration/scriptotar-defragmentation-agent00-20261113` at exact SHA `7af95c574643c14565a5c10d0be2cc78c5e57254`.
- Exact failing GitHub workflow: [Tauri migration integration, run 44278804900](https://github.com/MousaXD/scriptotar/actions/runs/44278804900), **Supply chain → npm advisories**. Run head is the exact baseline SHA above.
- In `apps/desktop-ui`, the unmodified GitHub runner executed `npm ci` successfully (127 packages installed; 128 packages audited), then `npm audit --audit-level=high` failed with exit code 1 and **8 high-severity findings**.
- The runner's suggested `npm audit fix --force` would install `vitest@3.1.4` (breaking downgrade from `vitest@5.1.0`). This is **not accepted as a security fix** without a patched `picomatch` version and complete test evidence.

Reproduction, on a clean Node 22 npm installation:

```sh
cd apps/desktop-ui
npm ci
npm audit --audit-level=high
npm ls picomatch fdir tinyglobby vite vitest svelte-check
npm run check
npm test -- --run
npm run build
```

In this assessment, the successful clean install and failing audit are evidenced by the existing GitHub Actions runner rather than a separate independent workstation. The report-only Agent 11 PR is intentionally scoped within `apps/desktop-ui/**` to trigger the **same original GitHub workflow** on the PR's exact head, without editing or bypassing CI.

## Locked graph: direct versus transitive

Direct dev dependencies in `apps/desktop-ui/package.json` relevant to this chain:

| Direct | Declared constraint | Locked | Vulnerability route |
| --- | --- | --- | --- |
| `vite` | `^7.0.4` | `7.3.7` | picomatch, fdir, tinyglobby |
| `vitest` | `^5.1.0` | `5.1.0` | picomatch, tinyglobby, Vite peer |
| `@sveltejs/vite-plugin-svelte` | `^6.1.0` | `6.2.4` | Vite peer and inspector |
| `svelte-check` | `^4.2.2` | `4.7.5` | fdir |

Transitive versions resolved in `package-lock.json`: `picomatch@4.0.7`; `fdir@6.5.0` (peer `picomatch: ^3 || ^4`); `tinyglobby@0.2.17` (requires `picomatch: ^4.0.4` and `fdir: ^6.5.0`).

`vite@7.3.7` directly requires `picomatch: ^4.0.3`, `tinyglobby: ^0.2.15`, `fdir: ^6.5.0`. `vitest@5.1.0` directly requires `picomatch: ^4.0.7` and `tinyglobby: ^0.2.17`. Thus the current graph genuinely includes an affected `picomatch`, even though it enters through development tooling.

## Unique root cause and patch availability

- **One root advisory:** [GHSA-fc5q-39xc-vjpf](https://github.com/advisories/GHSA-fc5q-39xc-vjpf) / CVE-2026-112366, high-severity regular-expression denial-of-service from single-branch repeated extglobs.
- The advisory covers **picomatch through 4.0.7**. The latest published npm version observed on 2026-11-18 was **4.0.7**; there is **no released patched version or official fixed range** at this checkpoint.
- The eight npm findings reflect propagation through `picomatch`, `fdir`, `tinyglobby`, `vite`, `vitest`, `svelte-check`, `@sveltejs/vite-plugin-svelte` and `@sveltejs/vite-plugin-svelte-inspector`, **not eight distinct advisories**.
- A non-security downgrade of Vitest, a manual version pin to another vulnerable picomatch release, overrides to an unpatched version, an unreviewed fork/commit, or a changed audit threshold cannot resolve the root advisory.
- Evidence: [npm published versions](https://www.npmjs.com/package/picomatch?activeTab=versions), [GitHub GHSA](https://github.com/advisories/GHSA-fc5q-39xc-vjpf), [OSV](https://osv.dev/vulnerability/GHSA-fc5q-39xc-vjpf).

## Before/after evidence

| Check | Original integration SHA | Agent 11 source-only report |
| --- | --- | --- |
| `npm ci` | **PASS** (GitHub runner) | Unchanged dependency lockfile |
| `npm audit --audit-level=high` | **FAIL**, 8 high findings / 1 root advisory | **Still vulnerable**; fail closed, no bypass |
| Svelte/TypeScript check | **PASS** (same run: Svelte frontend job) | No source changes |
| Vitest | **PASS** (same run: Svelte frontend job) | No source changes |
| Vite production build | **PASS** (same run: Svelte frontend job) | No source changes |
| Integrated Tauri build | **PASS** (same run: Integrated Tauri build job) | No source changes |
| New exact-head GitHub CI | N/A | Consult Agent 11 PR checks; do not infer green |

**Security impact:** the risky matcher is present in development/build/test tooling. No source-level proof of an attacker-supplied glob reaching picomatch in the packaged desktop runtime has been established. Do not equate dev-tool reachability with zero security risk; CI or any development process accepting untrusted patterns may be exposed to event-loop stalls. This report does not waive the mandatory dependency audit gate.

## Adjacent supply-chain risks (out of Agent 11 ownership)

- On the same run, `cargo audit` **exited successfully while reporting 10 allowed warnings**, including unmaintained crates and known unsoundness. Examples with known patch ranges: `rustix@1.1.4` (RUSTSEC-2026-0486; upgrade >=1.1.6), `tokio@1.53.1` (RUSTSEC-2026-0386; upgrade >=1.53.3 or 1.51.6–<1.52.0), `glib@0.18.5` (RUSTSEC-2024-0429; upgrade >=0.20.0). Rust owners must assess these separately; a passing command does not mean zero warnings.
- **Python advisories were not executed** in this run: the workflow stopped after npm audit failed; therefore Python audit success cannot be asserted. Its scheduled command is `pip-audit -r sidecars/transcription/requirements-engine.txt`.
- No Cargo, Python, production, security workflow, or shared CI files were changed here.

## Required unlock condition

1. Upstream publishes a version of `picomatch` outside the affected advisory range, with a verifiable npm release/integrity record and an upstream security fix, **or** an independently reviewed, supported, advisory-clean dependency replacement graph becomes available.
2. Update only owned npm manifests/lockfile with reproducible dependency resolution; preserve lockfile integrity, Node 22 compatibility and existing major-version constraints where possible.
3. Verify fresh `npm ci`, unfiltered `npm audit --audit-level=high`, `npm run check`, `npm test -- --run`, `npm run build`, and full exact-head GitHub Supply chain job (including Python step).
4. Keep this PR **draft and unmerged** until the full audit passes with no suppression. Notify Agent 00 with the upstream blocker and report link.

**No version changes have been made:** rewriting the lockfile without an upstream patch would be misleading, and blindly forcing a breaking downgrade would not constitute verified remediation.


## Independent Agent 11 exact-head CI follow-up (2026-11-18)

The first Agent 11 PR head is `94092d99370bd3101a7fdf7b92369a215e27369b` (one report-file-only commit relative to baseline). The **unmodified** [Tauri migration integration run 44297361807](https://github.com/MousaXD/scriptotar/actions/runs/44297361807) independently reproduced the problem on the Agent 11 branch:

| Exact-head GitHub job/step | Verified outcome |
| --- | --- |
| Rust workspace | PASS |
| Rust ↔ sidecar integration | PASS |
| Python sidecar | PASS (normal sidecar tests; **not** pip-audit) |
| Svelte frontend: Install/Check/Test/Build | PASS across all four steps |
| Supply chain: Rust advisories | PASS command, **10 allowed warnings** |
| Supply chain: npm advisories | **FAIL** — `npm ci` installed 127 packages and audited 128; `npm audit --audit-level=high` reports 8 high findings and exit 1 |
| Supply chain: Python advisories | **SKIPPED** because npm failed |
| Integrated Tauri build | IN PROGRESS when this report section was written; check the run for its final result |

The second independent audit output reproduces the exact same root advisory `GHSA-fc5q-39xc-vjpf` and same eight affected-path findings. No improved after-audit count can be reported without a patch. **Do not infer a green audit** from the passing frontend, Rust or sidecar jobs.

### Upstream mitigation status

- [picomatch GitHub releases](https://github.com/micromatch/picomatch/releases): most recent release was `4.0.7` published 2026-08-24. The current GHSA still covers through `4.0.7`.
- [Upstream issue #247](https://github.com/micromatch/picomatch/issues/247) discusses delayed fixes and whether the advisory should be disputed. One proposed [fix PR #235](https://github.com/micromatch/picomatch/pull/235) was **closed without merge**. There are other proposals, but **no released, independently verified patch** at this checkpoint.
- The upstream dispute does not itself retract GitHub's published advisory or change the mandatory `npm audit` outcome. A public issue thread also contains a bounded reproduction suggesting exponential slowdown; assess claims independently rather than treating a dispute as mitigation.
- No direct or transitive package manifest was altered, and the existing security workflow has not been suppressed or rewritten.

### Handoff

Agent 00 was notified in [integration PR #114](https://github.com/MousaXD/scriptotar/pull/114#issuecomment-6977582294). This draft PR is a technical security assessment, **not a patch**. Keep it draft until a release-grade patched graph and exact-head audit are available; do not merge it as a fix or clear the production gate.


## Follow-up: independent remediation architecture and bounded reproduction (2026-11-18)

**Release decision remains NO-GO.** This extension documents independently reconstructed lockfile edges, the real Tauri packaging boundary, unmerged upstream candidate review, and a reproducible regression tool. It does not assert that a vulnerable upstream dependency was patched.

### Confirmed upstream publication and candidate fixes

- [Official GitHub releases](https://github.com/micromatch/picomatch/releases) still top out at `4.0.7` (published 2026-08-24); [npm published versions](https://www.npmjs.com/package/picomatch?activeTab=versions) list the same `latest` tag.
- [GHSA-fc5q-39xc-vjpf](https://github.com/advisories/GHSA-fc5q-39xc-vjpf) applies to `picomatch` through `4.0.7`, with **no published fixed range**. A withdrawn Snyk listing or upstream dispute is not a revoked GitHub advisory.
- [Upstream issue #247](https://github.com/micromatch/picomatch/issues/247) is open; as of this review, proposed PRs [#237](https://github.com/micromatch/picomatch/pull/237), [#238](https://github.com/micromatch/picomatch/pull/238), [#246](https://github.com/micromatch/picomatch/pull/246), [#259](https://github.com/micromatch/picomatch/pull/259), [#273](https://github.com/micromatch/picomatch/pull/273), and [#274](https://github.com/micromatch/picomatch/pull/274) remain **unmerged**. The latest upstream main commit inspected is from 2026-08-27, not a security patch release.
- These are **alternative unreviewed approaches**, not an endorsed fix: #237 attempts repetition collapse while preserving many matching semantics; #238 literalizes selected risky patterns and acknowledges uncovered nested forms; #246 adapts a collapse approach to v2; #259 adds broad ambiguity handling with several hundred lines of parser changes; #273 specializes star-literal single-branch cases; #274 uses emitted-repetition metadata and conservative literal fallback. Each has compatibility/security tradeoffs. Do not combine or vendor any blindly.

### Lockfile dependency graph independently reconstructed

At exact original PR HEAD `75b2fb568b4958cc25bde262643b00c240f04961`, `apps/desktop-ui/package-lock.json` has one hoisted `node_modules/picomatch` record, **version `4.0.7`**, `dev: true`, resolved from `https://registry.npmjs.org/picomatch/-/picomatch-4.0.7.tgz`, integrity:

```text
sha512-qcJu88Q2IWqJsDD529JKMdwGm/dvInW4HvQnRwiH9JtihJvzGOscDtHE3x1pBKeUOTysQ8kVmLnJ2kJu7yhcGA==
```

Direct dependency edges in the lockfile:

```text
root -> vite@7.3.7 -> picomatch@4.0.7 (^4.0.3)
root -> vitest@5.1.0 -> picomatch@4.0.7 (^4.0.7)
root -> vite@7.3.7 -> tinyglobby@0.2.17 -> picomatch@4.0.7 (^4.0.4)
root -> vitest@5.1.0 -> tinyglobby@0.2.17 -> picomatch@4.0.7 (^4.0.4)
root -> svelte-check@4.7.5 -> fdir@6.5.0 --peer--> picomatch (^3 || ^4)
root -> vite@7.3.7 -> fdir@6.5.0 --peer--> picomatch (^3 || ^4)
```

The tree contains 177 lockfile package records including root, and all are reachable by manifest dependency traversal. The affected picomatch record and `vite`, `vitest`, `fdir` and `tinyglobby` have `dev: true`. This is a **static reconstruction from the committed lockfile**, not claimed `npm explain` output. The required `npm explain picomatch` and `npm audit --json` must still be captured on a networked clean runner for the selected fix.

### What is and is not reachable

- The Tauri configuration `apps/desktop/src-tauri/tauri.conf.json` runs `npm ci && npm run build` as a **build-time step**, then uses `../../desktop-ui/dist` for `frontendDist`. This is affirmative evidence that the Node dependency graph drives development/build/test tooling rather than being an intentionally bundled Node runtime.
- No packaged-desktop runtime call chain accepting arbitrary user-supplied glob expressions was established. The production build succeeding is not proof that the library cannot be present in build outputs; no compiled-artifact symbol search or dynamic native reachability test has yet been performed.
- Build environments, CI, plugins, and future import/research tasks still require a trust-boundary check. A ReDoS input must flow into the **glob pattern**, not just a filename being matched. Static trusted patterns reduce attack reachability but are **not an advisory fix**.

### Bounded, opt-in regression probe

Added `apps/desktop-ui/scripts/probe-picomatch-redos.mjs` on this branch. It runs two fixed malicious patterns, `+(*a)` and `*(*a)`, against a trailing-mismatch input (`'a'.repeat(36) + 'b'`), plus a safe control. Each case executes in a separate Node subprocess killed after 1,200 ms; max child output is capped at 64 KiB. A timeout is an explicit **failure** in `--assert-patched` mode. No hostile regex runs in the parent process.

```sh
cd apps/desktop-ui
npm ci
node scripts/probe-picomatch-redos.mjs --report
# For a genuine replacement candidate, a mandatory *additional* check:
node scripts/probe-picomatch-redos.mjs --assert-patched
npm explain picomatch
npm audit --json > npm-audit-after.json
npm audit --audit-level=high
npm run check
npm test -- --run
npm run build
```

`--report` exits successfully even on a vulnerable match **by design** and loudly reports it; **never use report-only mode as an audit or release gate**. `--assert-patched` is necessary but insufficient: a successful bounded sample cannot prove arbitrary-pattern ReDoS safety, validate capture semantics, or override `npm audit`.

**Actual validation so far:** The probe's JavaScript syntax was checked locally with Node v22.16.0 (`node --check`: PASS). This sandbox lacks the repo's installed dependencies and network npm access, so executing the probe against the actual locked `picomatch@4.0.7` was **not possible locally**. The existing exact-head GitHub runner, not this local check, remains the independent proof of `npm ci` success and audit failure. No fixed dependency graph has been implemented or tested.

### Remediation options ranked (no authorization implied)

| Rank | Path | Security integrity | Compatibility / maintenance cost | Release recommendation |
| --- | --- | --- | --- | --- |
| 1 | **Maintainer-published and advisory-cleared picomatch patch** | Best: official source, new npm integrity, independent fix tests | Low if a semver-compatible release | Wait for real release; update lockfile, run full checks, retain evidence |
| 2 | **Reviewed maintained security fork with verifiable source commit and npm provenance** | Potentially defensible **only after** independent patch review, regression corpus, integrity and policy approval | Medium/high ongoing patch ownership; may require private registry/auditor treatment | Experimental only until owner approves source, maintainership, signature, test corpus and real unfiltered advisory status |
| 3 | **Dependency-chain replacement without picomatch** | Can eliminate root risk if truly absent | Very high: Vite/Vitest/Svelte replacement and fresh E2E packaging evidence | Avoid as emergency patch; investigate only with dedicated architecture review |
| — | **Time-boxed owner risk acceptance** | Exposure mitigation, **not remediation** | Low code disruption; residual risk remains | Explicit written exception only; current mandatory audit **stays failing** unless owner separately authorizes a transparent policy change |

A self-published `picomatch@4.0.8` under a private registry is **not** trustworthy merely because its version is above the advisory's last published version. A renamed vulnerable package, semver trick, patched lockfile metadata without code modification, blanket ignore or `npm audit fix --force` cannot be certified. A real fork needs source/patch diff and provenance, bounded plus comprehensive glob semantics/security tests, a reproducible build and cryptographic integrity, controlled maintainer policy, and an auditor that genuinely evaluates that exact artifact.

### Before/after and handoff at this checkpoint

| Evidence | Before | After this investigative follow-up |
| --- | --- | --- |
| Declared npm versions / npm lockfile | Unmodified | **Unmodified** |
| npm high advisories | **8 high / 1 root GHSA** | **8 high / 1 root GHSA remains** (same lockfile; not a newly executed after-audit) |
| Published upstream fix | None | **None** |
| Production/native source changes | None | None |
| Regression evidence | Upstream reproductions, npm audit failure | Added bounded opt-in probe; syntax check PASS, package-level invocation pending networked execution |
| Release gate | **FAIL** | **FAIL — no waiver or suppression** |

Original Agent 11 exact-head run at `75b2fb568b4958cc25bde262643b00c240f04961`: [Tauri migration integration #44299922352](https://github.com/MousaXD/scriptotar/actions/runs/44299922352) — Rust workspace PASS, Rust/sidecar PASS, Python sidecar tests PASS, Svelte check/test/build PASS, integrated Tauri build PASS, supply-chain **FAIL** at npm advisory, Python **advisory** audit SKIPPED. The new probe/report commit must earn its own exact-head checks before being cited as validated CI.

**Owner decision:** keep PR #132 DRAFT, no merge/release authorization. Prefer waiting for the maintainer's verifiable patch. If timing cannot tolerate that, Agent 00 must explicitly commission a separately reviewed security-fork experiment or perform a documented risk-acceptance process without claiming remediation. Unresolved external advisories remain blocking under current policy.


## Independent real installed dependency exploit evidence (2026-11-18, follow-up)

**New verified exact-head:** `fe13a6a52e4890ca91cda8a35a6202fc35b07e0d`.

Added isolated, non-release [Agent 11 picomatch exploit evidence workflow](https://github.com/MousaXD/scriptotar/actions/runs/44313324125) under `.github/workflows/agent11-picomatch-evidence.yml`, triggered only for the Agent 11 branch or manually. This does not modify the existing mandatory audit workflow. The new workflow **FAILED as intended** rather than masking the exploit or security gate. Its GitHub job is named `Real installed dependency — expected vulnerable / NO RELEASE`.

### Grounded runner results

| Clean Node 22 GitHub runner step | Actual result |
| --- | --- |
| `npm ci` | PASS |
| `npm explain picomatch`, `npm ls` and exact lockfile provenance | PASS |
| Opt-in subprocess ReDoS `--report` | PASS as diagnostic wrapper; **two vulnerable pattern TIMEOUTs** |
| `npm run check` | PASS, 0 errors / 1 unused-selector warning |
| `npm run test` | PASS (jsdom logs include nonfatal `window.scrollTo` unimplemented warnings) |
| `npm run build` | PASS |
| Static `dist` grep for `picomatch|tinyglobby|fdir` | No parser-name indicator found; **not** a runtime absence proof |
| Strict `--assert-patched` | **FAIL**: both hostile pattern subprocesses timed out |
| Unfiltered `npm audit --audit-level=high --json` | **FAIL**: 8 high, 0 critical, 0 moderate, 0 low |

Two exploit cases independently timed out at the configured **1,200 ms per child** after the clean install:

- `picomatch('+(*a)')('a'.repeat(36) + 'b')` → **TIMEOUT**;
- `picomatch('*(*a)')('a'.repeat(36) + 'b')` → **TIMEOUT**.

Normal control `picomatch('+(a|b)')('abab')` returned in **2 ms**. Separate killable subprocesses prevented a test-runner hang. As the runner truly executed the installed `picomatch@4.0.7`, this is now **confirmed runtime behavior in the Node build/test environment** rather than a merely hypothetical GHSA claim. It is still **not evidence of a user-controlled glob reaching a packaged desktop session**.

The same workflow independently exercised a real build. A negative `grep` for parser-name identifiers in `dist` has limited sensitivity (minification, dynamic code, string elimination); it is not binary-level reachability analysis and not a security waiver.

Audit JSON metadata from GitHub Actions:

```json
{"info":0,"low":0,"moderate":0,"high":8,"critical":0,"total":8}
```

The audit still identifies `GHSA-fc5q-39xc-vjpf` at `node_modules/picomatch`; the eight propagated affected names remain `picomatch`, `fdir`, `tinyglobby`, `vite`, `vitest`, `svelte-check`, `@sveltejs/vite-plugin-svelte`, and `@sveltejs/vite-plugin-svelte-inspector`. This runner proves the **before** vulnerability after an unmodified lockfile reinstall. There is **no patched after-state** and no security fix to claim.

The official [GitHub advisory](https://github.com/advisories/GHSA-fc5q-39xc-vjpf) is updated 2026-11-17 and explicitly lists **affected `<=4.0.7`, patched versions: None**. The latest upstream GitHub release is still 4.0.7 (2026-08-24), with unmerged mitigation proposals. An unmerged upstream pull request is not provenance-verifiable released npm remediation.

### P0 release gate and responsibility

- PR #132 **must remain DRAFT and UNMERGED** as a diagnostic/security report, not a code fix.
- Existing Tauri migration integration continues to **FAIL** at npm audit. The isolated reproduction workflow **also fails**, deliberately and correctly.
- No manifests, lockfiles, production files, native acceptance tests or mandatory security workflows were changed by this follow-up.
- **Decision for Agent 00:** wait for upstream fixed release plus successful clean reproducible audit and compatibility validation, or separately authorize a formal reviewed security-fork experiment. A time-boxed risk acceptance is a distinct policy exception, not remediation; it must never be silently applied to this release gate.


## Separate Python audit and isolated upstream patch experiment (2026-11-18)

**Python audit independently completed, not blocked by npm:** [Agent 11 workflow run 44332583902](https://github.com/MousaXD/scriptotar/actions/runs/44332583902), Python 3.12, pip-audit 2.10.1, with the original unmodified command:

    pip-audit -r sidecars/transcription/requirements-engine.txt

Exit code **0**, output **"No known vulnerabilities found"**, requirements SHA-256:
a701d7587be07615a5e9562c3cfc5e2bf9176d2322494c65ab482155827be1da.

This is a real independent audit of the specified Python requirements on an installed networked GitHub runner. It does **not** turn the mandatory npm stage green, inspect every Python package at desktop runtime, or imply that the combined original workflow completed its sequential Python step. The separate job has no dependency on npm and uses ordinary fail-on-nonzero command behavior; the original .github/workflows/integration.yml and its exact unfiltered npm audit --audit-level=high remain intact.

**Track A:** [npm public versions](https://www.npmjs.com/package/picomatch?activeTab=versions) still mark 4.0.7 as latest, and [GitHub's GHSA](https://github.com/advisories/GHSA-fc5q-39xc-vjpf) still lists affected <=4.0.7, **Patched versions: None**. Because Vite, Vitest and tinyglobby directly require the 4.x package, updating only these consumers without replacing/removing those dependency edges does not create a secure graph. Do not run npm audit fix --force, hide a vulnerable package under another name, or manufacture a clear audit.

**Track B, laboratory candidate only:** scripts/experimental/picomatch-4.0.7-pr237.patch contains a small upstream-derived candidate from [unmerged micromatch/picomatch PR #237](https://github.com/micromatch/picomatch/pull/237), observed upstream HEAD 1bf0d04563cb9ce568cf36efc5f6b7187d9911d5. Its exact committed patch SHA-256 is 10862a65fefb1466c91774f2517b4ce7fd3a7f433b91ac2aa5f360ebf49b7933. Before the experiment, the CI script runs npm ci, checks picomatch@4.0.7 and validates that its installed lib/parse.js matches official upstream tag 4.0.7 Git blob 29207512c60183b4bb9bf5112be3b9b5f4a9a9fd. It then applies the candidate **only to temporary GitHub-runner node_modules**, verifies the file changed, and executes the bounded exploit probe and frontend check/test/build on Linux and Windows.

The candidate is **not published, advisory-cleared, independently security reviewed, or installed through a supported npm dependency graph**. A partial regex performance improvement cannot make the unchanged package-lock.json secure; npm audit still correctly fails. The CI experiment may fail for patch application, semantic compatibility or additional unmitigated exploit patterns. The strict existing audit job independently fails on the unchanged published package. Require a code-owner and independent Agent 10 security review of any proposed final patch, broader hostile glob corpus, supported reproducible packaging and full advisory-clean npm resolution before release consideration.

**No release permission:** This PR stays draft; main and integration production code remain untouched by these experiments. Preserve the hard npm failure.

## 2026-11-18 — extended three-candidate adversarial comparison and synchronized integration

**Final status: UPSTREAM BLOCKED / NO-GO.** There is no officially patched npm version of picomatch outside GHSA-fc5q-39xc-vjpf's affected range (<=4.0.7). The original strict npm audit still fails on eight high dependency-path findings. The desktop package manifest, package-lock, integration audit workflow and production code have **not** been altered to mask this failure.

### Expanded defensive test harness

The opt-in command node scripts/probe-picomatch-redos.mjs --assert-patched --extended executes eleven bounded cases (the original two hostile inputs, one control and eight new adversarial inputs) in separate killable Node subprocesses, each with a 1,200 ms hard timeout. Cases cover single-branch stars, multi-branch wildcards, nested wrappers, ambiguous repeated classes/literals and nested repeated classes. Every timeout is a failure. Successful bounded samples are necessary but insufficient to establish parser security.

### Comparative results on Node 22

| Candidate | Exact upstream HEAD | Linux attack corpus | Windows attack corpus | Frontend compatibility | Decision |
| --- | --- | --- | --- | --- | --- |
| Official picomatch@4.0.7 | Published release, original locked source | Original two hostile cases TIMEOUT at 1,200 ms | Original two TIMEOUT | Previously PASS but vulnerable | REJECT |
| Unmerged PR #237 | 1bf0d04563cb9ce568cf36efc5f6b7187d9911d5 | FAIL: multi-branch, nested and class ambiguity timeout | FAIL: same, plus literal ambiguity timeout | Earlier only limited-probe check/test/build PASS | REJECT |
| Unmerged PR #274 | 3975e4a0e6b4c4fb2c0119863b243ce04612c5e3 | FAIL: ambiguous repeated classes timeout | FAIL: repeated classes and literal ambiguity timeout | Not run after failed probe | REJECT |
| Unmerged PR #259 | fea5ae7ed37ffd67ccdf26086eb8aa2ba2d499ae | 11/11 bounded cases PASS (2 ms each) | 11/11 bounded cases PASS (3–4 ms each) | Both: Svelte check 0 errors/1 warning, 244/244 Vitest PASS, Vite build PASS | EXPERIMENT ONLY |

Actual GitHub Actions runs:
- Original strict npm audit and Python audit plus cross-platform comparison: https://github.com/MousaXD/scriptotar/actions/runs/44336874909
- PR #237 two-platform bypass: https://github.com/MousaXD/scriptotar/actions/runs/44336343310
- PR #274 two-platform bypass: https://github.com/MousaXD/scriptotar/actions/runs/44336541954
- Earlier two-pattern-only test of PR #237: https://github.com/MousaXD/scriptotar/actions/runs/44334181279

All experiments install the original registry-locked picomatch 4.0.7 and check its unmodified official lib/parse.js Git blob SHA 29207512c60183b4bb9bf5112be3b9b5f4a9a9fd. They then verify and apply unmerged upstream parser patches **only inside ephemeral CI node_modules**. Patch Git blobs are PR237 73fefc5cf92ab1250ea3beabecf4f3116ff66a7f; PR274 9314177918b9d5b9a2d1e70bade7c2c9bc574bc8; PR259 8d12d987655d3b7298c64e4bb27899a8291d6097. No experimental patch is distributed, installed by npm dependency resolution, or committed to the application runtime.

PR259 changes approximately 480 lines of parser security/compatibility logic and preserves only some complex glob semantics. Its conservative fallback can change matches. Scriptotar's 244 frontend tests and eleven bounded inputs cannot certify the whole implementation, identify all alternate parser paths, establish compile-time input-complexity bounds or prove compatibility with all consumer tooling. The PR remains unmerged; no maintainer-verified fixed npm release or accepted advisory clean graph exists. DO NOT adopt it for production without upstream/independent security review, fuzz/differential testing and a supported reproducible packaging strategy.

### Remediation dependency request to upstream / Agent 00

1. Upstream maintainer and independent security reviewer select, harden and merge a complete fix; publish a verifiable npm version with package integrity and an advisory-updated fixed range.
2. Once actually available, upgrade the real Vite/Vitest/tinyglobby/fdir graph using supported semver, maintaining package-lock integrity. Do not rename or hide picomatch, force an unrelated tool downgrade, or use unsupported overrides to manufacture an audit PASS.
3. Validate fresh npm ci, all dependency paths, unfiltered npm audit --audit-level=high, expanded/fuzzed malicious corpus, Svelte/Vitest/Vite Linux and Windows, and full exact-head integrated CI.
4. Alternative complete dependency removal would require replacing every relevant Vite/Vitest/tinyglobby path with maintained tooling; no viable such graph is demonstrated.
5. Agent 10 must independently security-review any candidate implementation and its provenance before promotion. A private unpublished patch or source-only test does not authorize a release exception.

**Python advisory stage is independently proven PASS**, using Python 3.12 / pip-audit on the original sidecars/transcription/requirements-engine.txt; it runs in a distinct non-dependent CI job and does not mask the failing npm stage. See run 44336874909.

### History-preserving sync

Two-parent merge commit b022213ee1014192b3b62aea7a9bb0850026f0fd merged exact integration 5ed385d09092d2c94a1bd86cb56888d96df0bb5e into the Agent 11 evidence branch, preserving all predecessor commits, production source and six Agent 11-owned files. Verified GitHub compare: 0 commits behind integration, no overlapping changed paths, no merge conflicts, no production or npm manifest changes.

**PR #132 remains DRAFT / BLOCKED. Main unchanged, mandatory npm audit FAIL, release NO-GO. Do not merge as supply-chain remediation.**

## 2026-11-18 — upstream maintainer policy and final dependency-graph decision

**Material upstream status change:** [micromatch/picomatch PR #259](https://github.com/micromatch/picomatch/pull/259) was **CLOSED WITHOUT MERGE** at 2026-11-18 15:48:15 UTC by its author. The closing comment explicitly referenced the maintainer's position about untrusted regular-expression/glob patterns and withdrew this proposed implementation. Therefore the previously successful bounded CI evaluation of its prior source SHA is **a historical experiment**, not an active upstream remediation candidate.

**Maintainer's explicit view:** In [micromatch/picomatch PR #235, maintainer comment 6315749381](https://github.com/micromatch/picomatch/pull/235#issuecomment-6315749381), the maintainer disputes treating arbitrary complex glob patterns as a package vulnerability and recommends that applications do not accept untrusted regex-like patterns. In that discussion the maintainer additionally stated that he had submitted a dispute asking the CNA to withdraw the advisory. This is a maintainer's stated position, **not** an official withdrawal or a guarantee that the demonstrated pathological execution disappears. As of the last verification, [GHSA-fc5q-39xc-vjpf](https://github.com/advisories/GHSA-fc5q-39xc-vjpf) still covers npm picomatch through 4.0.7, with **no published patched version**. GitHub upstream latest published release also remains 4.0.7 (2026-08-24).

**Exact reproducible dependency edges in Scriptotar desktop-ui package-lock.json:**

| Installed package | Resolved version | Direct dependency constraint on picomatch |
| --- | --- | --- |
| Vite | 7.3.7 | ^4.0.3 |
| Vitest | 5.1.0 | ^4.0.7 |
| tinyglobby | 0.2.17 | ^4.0.4 |
| picomatch | 4.0.7 | advisory affected |

All listed tools are under the desktop-ui root development dependency graph, but a production runtime non-reachability claim would require a separate complete packaged-artifact review; development/build-time exposure is still relevant. Updating Vite or Vitest without removing all picomatch edges or obtaining a genuinely unaffected released version cannot clear the unfiltered audit. There is no demonstrated supported, audited replacement graph. Do not use the maintainer's dispute to infer an advisory is withdrawn or falsely mark a vulnerable locked version fixed.

**Lifecycle improvement at Agent 11 workflow commit 31fc203dbb31c18e0f9c22a228bca98a3eaa1d64:** On normal pushes, the original installed-picomatch job now runs the *expanded* 11-case bounded regression in strict failing mode, then the mandatory unchanged unfiltered npm audit. The independent Python requirements audit remains separately executable and fail-closed. Previously measured PR #237, #274 and now-closed #259 comparisons are preserved as provenance-pinned experimental jobs but run only on explicit workflow_dispatch, preventing repetitive known-failing matrices on every documentation or test change. No candidate test failure was converted to PASS, the original npm audit command and gate were not weakened, and no production packages were modified.

**Unblock alternatives and required authority:** (A) a maintainer-published, independently reviewed, integrity-verified patched version with a registry-resolved graph and updated advisory data, followed by exact-head full CI; (B) a demonstrated maintained, supported build/test dependency graph removing **all** advisory-affected picomatch paths and passing full compatibility/security tests; (C) if the CNA and advisory issuers formally withdraw the advisory, independently evaluate the reasons and residual threat exposure with Agent 10 and Agent 00 before changing any release decision. An owner-authorized security-policy exception remains a separate decision; **none was requested or granted**. Until one defensible path is complete, Agent 11 is **BLOCKED / NOT MERGE READY**, PR #132 stays draft, strict npm audit FAIL and Scriptotar release **NO-GO**.

## 2026-11-18 — independently reconstructed source-exact backports for active upstream candidates

Maintainer-published npm latest remains picomatch 4.0.7. On 2026-11-18 at 15:38:56 UTC the reviewed GitHub advisory database still recorded GHSA-fc5q-39xc-vjpf as HIGH with npm picomatch affected from introduction 0 through last_affected 4.0.7 and no fixed-version event. The maintainer's previous CNA dispute did not remove this active advisory.

### Candidate provenance and exact-file reconstruction

The initial GitHub PR patches were relative to the upstream master parser (9fab7bf); master and the official npm 4.0.7 tag differ. A direct PR264 patch against the installed 4.0.7 parser does not pass exact hunk-context verification. Applying it blindly would create an uncontrolled partial mixture, so that approach was **replaced before CI certification**.

Agent 11 fetched GitHub's upstream commit comparison from exact tag 4.0.7 to the immutable PR HEAD and generated target-version backport patches. Each diff was independently parsed and applied in a pure source-reconstruction check; the reconstructed full lib/parse.js content was compared byte-for-byte against the upstream PR HEAD's actual published Git blob:

| Candidate | Pinned upstream HEAD | Backport Git blob | Reconstructed lib/parse.js Git blob | Source equality |
| --- | --- | --- | --- | --- |
| Open PR #260 | d10c0e5003aabed2d81427dd6c125af50bf920ab | 740e30bdd7b4263672ccf04c0b68c11fb39ebc07 | 1e992b2d41d91aa62a06f69c3e1dab42a727517c | byte-for-byte exact, 9 hunks |
| Open PR #264 | 8144a1a22a8fa1766951411f220cc8c6228cb041 | fad2d74ae69aa2d19fcbec0ef124f992bc83bfb5 | b9eaa6dbc4a52c931e8ec5d7bb216d0eeb8c61a8 | byte-for-byte exact, 15 hunks |

The exact upstream HEAD contains the small post-4.0.7 upstream master changes as well as the PR's security changes; that is intentional for an exact-source comparison, but it broadens the compatibility review surface. The separate original upstream PR diff files remain archival provenance, not the patch inputs used by the final CI. The CI now verifies actual source identity both before and after applying each backport to throwaway node_modules; the application package graph is unchanged.

### Expanded bounded adversarial execution

The existing probe now includes 19 cases when --extended is selected: the original two malformed repeated-extglob attacks, a safe control, eight prior nested/multi-branch/alternation bypass variants, four raw/brace/negation quantifier variants and four 5,000-character near-miss stress cases. Each pattern executes in an individually killable subprocess with the existing fixed 1,200ms timeout. Adding long forms addresses an independent upstream review finding that PR260 can remain pathological on long near-misses even if initial short-input tests pass. Repeated source tests must not be mistaken for universal regex-safety proof.

An independent upstream reviewer compared 17 proposals and reported PR264 as comparatively stronger for the tested adversarial corpus, though it still intentionally changes some complex glob matching semantics through literal fallback. These external observations are not Scriptotar's own completed acceptance evidence and do not constitute maintainer endorsement.

Final exact-head PR260/PR264 Linux/Windows workflow evidence: https://github.com/MousaXD/scriptotar/actions/runs/44348878620 (queued when first read; the most recent source/report commits require an exact new-HEAD rerun). Even if all experiments pass, the original registry-locked picomatch@4.0.7 and unfiltered npm audit remain affected and FAIL. No experimental parser file is included in the actual desktop application or adopted as a pinned npm dependency. No owner-approved security exception, production release or main merge.

**Status remains BLOCKED / NOT READY FOR INTEGRATION.** Upstream must publish a supported, independently reviewed and advisory-cleared fix, a demonstrably clean maintained alternative dependency graph, or obtain a formal advisory change followed by independent Agent 10 residual-risk review. Agent 00 controls all integration and release decisions.


## 2026-11-18 — latest source synchronization and real-package audit boundary

**Integration synchronization:** Agent11 production source was merged again by true two-parent commit `adf671eda69e404c13de448fd5ee92d4ee039750`, first parent `f65f525a828e0597afc740c55ca76fcfab9b92a3` (every prior Agent11 commit), second parent Agent00 integration `63fafe38a2a9280d8b0cec275f185906ee643407` (every latest Agent00 contribution). Compared 15 newly changed integration paths against 12 Agent11-owned changed paths: zero overlaps and zero conflicts. GitHub compare confirms **0 behind** at time of synchronization. No rebase, squash, force push or main merge.

**New static boundary:** `scripts/audit-tauri-bundle-resources.mjs` parses and validates four actual generated Tauri bundle JSON structures embedded in three production/installed-WebView CI workflow sources: Linux packaging, Windows packaging, Linux installed WebKitGTK QA and Windows installed WebView2 QA. Each must map **exactly two** resource roots, `resources/transcription-runtime/` and `resources/mcp-tools/`, and preserve the expected `deb`/`nsis` targets, runtime build/validation and pinned MCP provisioning steps. It rejects unexpected additional resource entries, including `node_modules`, rather than incorrectly trusting the checked-in `tauri.conf.json`, whose `bundle.resources` list is empty.

**Real manifest checker available; still needs actual input:** The same script supports `--manifest <real file listing> --platform linux|windows`, rejects parser/npm-dev tooling paths, requires both runtime and MCP resource roots, and has eight adversarial/clean path-matching fixtures. On an actual completed Linux package job run from repo root:

```bash
# Existing Linux packaging workflow writes this from dpkg-deb --contents.
node apps/desktop-ui/scripts/audit-tauri-bundle-resources.mjs \
  --platform linux --manifest dist/DEB-CONTENTS.txt
```

On Windows after the existing real NSIS installed-package smoke test, `$installDir` is the actual installation directory:

```powershell
Get-ChildItem -LiteralPath $installDir -Recurse -File |
  ForEach-Object { $_.FullName } |
  Set-Content -Encoding utf8 dist/WINDOWS-CONTENTS.txt
node apps/desktop-ui/scripts/audit-tauri-bundle-resources.mjs --platform windows --manifest dist/WINDOWS-CONTENTS.txt
```

**Do not confuse the above CLI examples with completed package manifest evidence.** Agent11's own cross-platform checks independently execute the generated CONFIG source validation; the actual .deb/NSIS manifest invocation must be inserted into Agent00-owned packaging workflows after package/install generation, or separately verified against downloaded exact-SHA artifacts. Only those real invocations can justify a path-list package-content conclusion; even a clean manifest does not prove absence of arbitrary malicious code inside PyInstaller binaries or that attacker-provided glob patterns are unreachable in build tooling.

**Prior same-source code status:** Linux/Windows config/source guard jobs PASS after correcting a source parser mismatch between PowerShell opening `@'` and closing `'@` heredoc markers. The original published picomatch 4.0.7 still exhibits pathological crafted-glob runtime; strict unfiltered npm audit PASS on officially withdrawn GHSA, but real 19-case malicious-pattern assertion FAIL. No release/security exception. Agent10 must separately determine untrusted glob-pattern reachability, and Agent00 must independently validate final installed packages and all remaining native/host security requirements.

**Readiness: BLOCKED as parser remediation and full installed-release certification**, with exact dependency request: actual package-file manifest execution and independent residual-risk decision. Preserve the malicious-pattern failure as real negative security evidence.


## 2026-11-18 — independent real artifact .deb / NSIS inventory, exact SHA and scope

**Latest verified package evidence**: [Agent11 real artifact manifest run #44375979794](https://github.com/MousaXD/scriptotar/actions/runs/44375979794), Agent11 source workflow commit `11799d20e55fb0ff21753f0ddaf5989574c99fc6`. **Both Linux and Windows evidence jobs PASS**. To avoid circular evidence, both jobs fetch original, unmodified actual production package artifacts from independently completed releases **pinned to package source commit `531c5e2fb6f2eb17df80de1cc40a48fc24b551d8`**, check their GitHub Actions run `head_sha` before download, and compare package SHA-256 against the builder's original `SHA256SUMS.txt`. No fabricated manifest, modified product ZIP or mock package was accepted.

| Artifact (actual prior-source installed package) | SHA-256 verified package bytes | Real file-path inventory | Manifest SHA-256 | Result |
| --- | --- | --- | --- | --- |
| Linux .deb from run [#44363867237](https://github.com/MousaXD/scriptotar/actions/runs/44363867237), artifact `14456887927` | `0e209226fce48e8943d28d7d3cdbc7840fdc09d36c90a8eab0bd7b708fc81df1` | 2,992 actual `dpkg-deb --contents` lines; file listing compared byte-for-byte to builder-published `DEB-CONTENTS.txt` | `026f507a9672b235fe10c29ce88cf6958a9e94fa201670f66333514d4d251c93` | PASS |
| Windows NSIS from run [#44363867107](https://github.com/MousaXD/scriptotar/actions/runs/44363867107), artifact `14454989681` | `26a0d5f01afb671152e935e0a60c49dd0d133273bd900b09992f91c1f079760e` | **2,387 actual 7-Zip `Path =` entries**, excluding metadata (19,102 raw listing lines) | `9deb4279f8e261cf353ceb3d966cf4abb926a06c559e38d1d442017645ae9533` | PASS |

Both real package path inventories contain `transcription-runtime/` and `mcp-tools/`, and **zero unexpected watched `node_modules`, `picomatch`, `tinyglobby`, `fdir`, `vite`, or `vitest` package path entries**. Linux used `dpkg-deb --contents` on the actual checked .deb; Windows ran read-only 7-Zip NSIS archive listing (runner's resolved `C:\ProgramData\Chocolatey\bin\7z.exe`) and checked the actual source-builder NSIS SHA-256. The initial Windows tooling attempt incorrectly checked the PowerShell command object with `Test-Path`; this was fixed with the real executable `.Source` and **re-run successfully**, no waiver. The first Windows report counted raw 7-Zip metadata lines; that overcount was corrected to count only actual `Path =` records, verified on the final exact run.

**Strict evidence scope**: These are actual packages built at **historical source SHA 531c5e2, not final production integration SHA**, so no newer installer is certified by inheritance. Inventory checks only file **paths and package-byte hashes**, not compressed binary source code, runtime behavior, untrusted build-tool input propagation or actual installed WebView authorization. A binary could embed other interpreter modules without a `node_modules` filename; do not claim comprehensive reachability proof. GitHub's GHSA-fc5q-39xc-vjpf remains formally withdrawn, mandatory unfiltered npm audit PASS, yet the original published picomatch@4.0.7 still **FAILS 19-case killable ReDoS corpus on real crafted patterns**. Independent Agent10 risk approval and Agent00 final installed-source release approval remain required.

**New tools**: `scripts/audit-tauri-bundle-resources.mjs` now accepts `--platform linux|windows --manifest REAL-FILE-LIST --package REAL-INSTALLER [--expected-package-sha256 HEX]` with streaming SHA-256 and explicit format-aware Windows 7-Zip extraction. `scripts/test-tauri-manifest-inventory.mjs` exercises **eight clearly synthetic CLI positive/negative controls** (clean, parser insertion, missing runtime, wrong package digest for each platform), not portrayed as real installed evidence. Own pinned-artifact workflow `.github/workflows/agent11-package-artifact-inventory.yml` is separate from original production workflows; neither paid dispatch nor original release policy changed.

**Decision: STILL BLOCKED / NO RELEASE.** Agent00 to demand same-full-final-SHA real package manifests and other native/security gates; Agent10 to independently assess whether untrusted parties can select malicious picomatch glob **patterns**, in addition to this supported no-named-parser-path evidence. PR #132 is *not* a patched dependency remediation and must not be merged as if it fixed picomatch's pathological matcher.


## 2026-11-19 — decisive new Agent11 development/CI source-PATTERN exploitation evidence

The previous **source-only suspicion** of a Vite \`import.meta.glob\` build-time sink is superseded by a **REAL bounded Vite build end-to-end reproduction**: [#44409757300](https://github.com/MousaXD/scriptotar/actions/runs/44409757300) at exact Agent11 code SHA \`5b52d11e1e2d8f164c6f60d3f6be77d8941bada4\`, **2/2 Linux/Windows evidence jobs PASS**. The explicit hostile glob **\`./+(*a)\` as a source-code literal** with a separately created near-miss file target causes \`vite.build()\` to **TIMEOUT at 8 seconds on Ubuntu 22.04 AND Windows 2022**. Two independently run control builds complete on each platform: ordinary \`./*.js\` and ordinary \`./*\` with the same unusual filename. Node 22 and installed \`vite@7.3.7\` + \`picomatch@4.0.7\` are version-checked. Every hostile case is killed independently on timeout; the genuine original 19-case parser \`--assert-patched\` remains FAIL.

**Scope:** Confirmed local/CI build-time hostile **PATTERN** sink, not an attack with an ordinary filename alone and not proof that an external party can submit PR source through current repository controls. The observed stall is inside a genuine Vite build; attribution to the precise third-party function needs deeper tracing. PostCSS \`dir-dependency.glob\` remains source-reviewed only. No installed desktop input -> Node picomatch exploit path has been proven; static frontend and original package pathname checks cannot exclude aliased/embedded matcher code. See [Agent11 detailed decision addendum](SECURITY_SUPPLY_CHAIN_AGENT11_DECISION_20261118.md) for the source/effect graph and the proposed independently reviewed **pre-build** PR source guard.

**Branch/version/decision:** Latest diagnostic code used only Agent11-owned script \`scripts/probe-vite-pattern-reachability.mjs\` and workflow \`.github/workflows/agent11-vite-pattern-reachability.yml\`, in addition to Agent11 reports. No dependency manifest, lockfile, parser fork, application runtime, production workflow, integration, release or paid-provider execution was changed. Previous genuine installer package attestation [#44403348988](https://github.com/MousaXD/scriptotar/actions/runs/44403348988) remains exact to \`aeb95293ab025435024cbe335ff393f77984471a\`, **not automatically valid for later source trees**. A future final release must rebuild and independently reattest original Linux/Windows artifacts at the final reviewed SHA. PR #132 **DRAFT / BLOCKED AS REMEDIATION / NO-GO**; Agent10 independently reviews the now-demonstrated build-time threat and Agent00/owner decide whether to await a supported patch, replace the dependency graph or authorize a separately documented, scoped/expiring residual-risk exception.
