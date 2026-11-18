# Agent 11 — Independent npm supply-chain recovery assessment (2026-11-18)

**Status: BLOCKED UPSTREAM — no eligible patched npm dependency graph; do not merge as a remediation.**

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
