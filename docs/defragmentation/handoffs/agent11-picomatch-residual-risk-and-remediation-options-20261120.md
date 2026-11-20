# Agent 11 — picomatch parser residual-risk and remediation options (20 Nov 2026)

**Decision: OPEN #129 / release NO-GO / Agent 11 production remediation BLOCKED.**

This is a source-exact research handoff, not approval to publish, merge a patched package, suppress security tests, or change the independent Agent 10 veto. Agent 00 alone owns integration.

## Immutable production lock and changing integration baseline

- Latest verified integration PR #114 SHA: `50ed95608fc742db4a7cf1bdd2ac04fa8c5457e6`. This integration merge incorporates guard-order workflow PR #158, **not** a parser fix.
- Desktop package-lock Git blob `eebc9228dd404f0b9f9bd55f2665ab3e00fe9dfa` at that SHA: single npm `picomatch@4.0.7` package, MIT license, official tarball `https://registry.npmjs.org/picomatch/-/picomatch-4.0.7.tgz`, SHA512 integrity `sha512-qcJu88Q2IWqJsDD529JKMdwGm/dvInW4HvQnRwiH9JtihJvzGOscDtHE3x1pBKeUOTysQ8kVmLnJ2kJu7yhcGA==`.
- Original source trust guard on integration is Git blob `5c746abd3caf414ba2c80be825d66983312236c6`, identical to pinned Agent 11 guard SHA `55ce3ac389b7e1fcab99b3a5acc84b526630af72`. It scopes reviewed first-party inputs and rejects unreviewed source changes *before* allowed builds; it is **not a repair** to the picomatch RegExp parser. Independently identified execution-before-guard issues in other workflows must remain separately tracked by Agent 00/10.

## Real transitive dependency and upgrade facts

Source-exact official package metadata, not merely npm-audit metadata:

| Package | Exact source commit/blob | Relevant dependencies |
|---|---|---|
| `vite@7.3.7` | `vitejs/vite` tag `v7.3.7`, `packages/vite/package.json` blob `df79dd206238d151e4f4f792973b7c3b57f2461c` | Direct `picomatch:^4.0.3`; `tinyglobby:^0.2.15`; `fdir:^6.5.0` |
| `tinyglobby@0.2.17` | Actual Scriptotar **npm lock snapshot** at blob `eebc9228dd404f0b9f9bd55f2665ab3e00fe9dfa` | **Locked metadata:** `picomatch:^4.0.4`, `fdir:^6.5.0`; both resolve to picomatch `4.0.7` and fdir `6.5.0` in this lock. A later `SuperchupuDev/tinyglobby` `main/package.json` blob `4a28470b86ce772d4106d0c7fe6183217bc298ee` instead declares `picomatch:^4.0.7`: do not confuse mutable repository HEAD with the published lock snapshot. |
| `vite@8.3.1` | `vitejs/vite` tag `v8.3.1`, `packages/vite/package.json` blob `a4a112d4554b5d2273267654e1776ce8138d478d` | STILL direct `picomatch:^4.0.7`; `tinyglobby:^0.2.17` |

**Therefore upgrading from Vite 7.3.7 to Vite 8.3.1 does not remove picomatch or prove its safety.** It would also be a breaking tooling change requiring its own full build/QA review. Do not make it a vulnerability-only bump.

**Exact npm lock dependency facts (corrected after runner evidence):** `apps/desktop-ui/package-lock.json` contains **177 package records**; root `dependencies`, `optionalDependencies`, and `peerDependencies` are empty and all 9 root packages are `devDependencies`. The locked `node_modules/picomatch` is `dev:true`, version `4.0.7` and has the original official npm SHA512 integrity. Its three regular `dependencies` incoming edges are `vite@7.3.7` (`^4.0.3`), `vitest@5.1.0` (`^4.0.7`), and `tinyglobby@0.2.17` (`^4.0.4`). **Important fourth edge:** `fdir@6.5.0` declares `peerDependencies.picomatch: '^3 || ^4'` with `peerDependenciesMeta.picomatch.optional: true`; fdir is also `dev:true`. All FOUR source-locked referencing packages are development-only. This is static package reachability, **not** proof of safe build-tool execution.

Official upstream `micromatch/picomatch` latest verified GitHub release as of 20 Nov 2026: **4.0.7** (published 2026-08-24). Upstream issue `micromatch/picomatch#247` remains OPEN, with maintainer/community disagreement about whether user-supplied globs are a library vulnerability. GHSA-fc5q-39xc-vjpf was withdrawn on 18 Nov. That changed `npm audit` to 0 high findings on the *identical vulnerable* lock, not the observed catastrophic backtracking behavior.

## Independently reproducible parser evidence (development / build-time)

The **original immutable 19-case exploit probe** is sourced from Scriptotar `292e408087d8b48bc012e52d1f3e7ab0c8b89650`, without weakening its behavior or timeouts.

- Actual source-exact locked 4.0.7 baseline [#44596156345](https://github.com/MousaXD/scriptotar/actions/runs/44596156345): Linux 17/19 TIMEOUT, Windows 18/19 TIMEOUT.
- Upstream #260/#261/#262/#264/#267/#274 source-exact proposals individually failed mandatory original 19 cases; [candidate comparison](https://github.com/MousaXD/scriptotar/actions/runs/44727083710).
- Additional #258/#266/#269/#270/#273 individually failed mandatory original 19 cases, and #264 stayed the closest (Linux 17/19 PASS; Windows 16/19 PASS) while retaining 10/10 of the distinct original matcher-positive controls; [second-wave comparison](https://github.com/MousaXD/scriptotar/actions/runs/44734086909).
- CLOSED UNMERGED upstream #275 broad ambiguity analyzer passed 19/19 cases both OS [#44730425071](https://github.com/MousaXD/scriptotar/actions/runs/44730425071), but broke **8/10** original positive-matching assertions on each OS [strict source-exact validation](https://github.com/MousaXD/scriptotar/actions/runs/44731226195). Reject as a drop-in.
- Agent11's deliberately limited and **test-only** PR264+class-algebra normalizer passed original 19/19, legacy matcher 10/10 and frontend check/test/build on Linux/Windows [#44735530250](https://github.com/MousaXD/scriptotar/actions/runs/44735530250), but independent expanded 12-case tests [#44736249700](https://github.com/MousaXD/scriptotar/actions/runs/44736249700) found **eight new 1.5-second timeouts** on BOTH Linux/Windows, involving uppercase/ranged classes, case-folding, captures, and explicit recursion setting. Agent10 separately reproduced capture/recursion bypasses [#44737440248](https://github.com/MousaXD/scriptotar/actions/runs/44737440248). Reject the prototype. DO NOT add one more ad-hoc source whitelist to claim general repair.

The trusted source guard is valuable exposure mitigation but cannot make the actual parser safe when a hostile glob reaches Vite. Independent Agent10 source-compiler proof exists in `tests/defragmentation/security/agent10_vite_scope_reachability.mjs`.

## Precise new distinction: shipped browser bundle versus build-tool execution

One may be able to prove `picomatch` is **not shipped to browser JS** while demonstrating it is **executed by the build tool**. These are independent questions, and only the second currently has demonstrated hostile input reachability.

Agent11 has added a separately executed *read-only* Rollup graph census:

- Test source: `tests/defragmentation/security/agent11_bundle_module_census.mjs`
- Linux/Windows workflow: `.github/workflows/agent11-picomatch-browser-bundle-census.yml`
- **Confirmed original source-exact result:** [run #44739758034](https://github.com/MousaXD/scriptotar/actions/runs/44739758034), test SHA `9b478dfbcad630be1f247d9f6d8ccaf85b37ea3a`, Linux job `136300135056` PASS and Windows job `136300135249` PASS. Both genuine Vite builds enumerated **174 modules**, generated **one JavaScript asset (430,559 bytes)**, and identified **zero `picomatch`, `tinyglobby`, or `fdir` modules** in the compiled browser module graph. This is limited to browser output for that exact reviewed source; the build-tool invocation itself remains vulnerable.
- A **separate later test harness introduced a false-positive** in static lock attestation: [run #44742001158](https://github.com/MousaXD/scriptotar/actions/runs/44742001158) FAIL BOTH OS before `npm ci` with `Unexpected optional or peer edge: node_modules/fdir`. This is a mistake in Agent 11's initial attestation that missed the existing optional peer metadata; **not** evidence of a modified package or new runtime dependency. Corrected test-only `agent11_desktop_npm_scope_attestation.mjs` at `f7b76f0d6ac8873ec70e700fca24a6792ad2f6ed` explicitly pins that exact optional peer edge and fails closed on mandatory peer conversion, importer flag drift, new transitive edges and integrity changes. Source-exact corrected Linux/Windows workflow [#44775812658](https://github.com/MousaXD/scriptotar/actions/runs/44775812658) **QUEUED at this writing**; do not transfer prior graph success to the unverified new static attestation.
- The census uses the pinned source guard, real locked `npm ci`, genuine original Vite config, actual Rollup `this.getModuleIds()`, compiled JS output checks, and cleans a temporary output directory. It does not use experimental patched parser source. Its browser-only result cannot certify Rust, installers, scripts, or safe use of the Vite build process.

Never equate `npm audit --omit=dev` or absent browser bundle assets with safe execution of a dev tool on untrusted globs.

## Decision tree for actual remediation

1. **Preferred:** an officially maintained picomatch version whose exact npm tarball, integrity, MIT/license, compiler and source provenance can be independently verified, and which passes original 19 plus expanded adversarial corpus and ALL original upstream matcher semantics without changing positive expected outcomes. Lock exact version and run all security/normal-build/platform tests. Agent10 independently reviews final SHA. No such version verified.
2. **Viable but substantial:** an independently maintained, semantics-preserving non-backtracking glob compiler/matcher that provides the actual documented APIs used by Vite, Vitest and tinyglobby, and that has proof/independent fuzzing on captures, options, negations, path handling, escaped classes and performance. Replace upstream transitive paths with **known trusted, reviewed** package distributions. A limited regex-rewrite proof of concept is *not* this.
3. **Mitigation-only:** keep the upstream original parser and comprehensively constrain externally controllable build patterns using immutable reviewed source trust policies, preserved package checks, and fail-closed CI workflows. Explicitly document that this does NOT fix a parser performance flaw; release decision requires separate Agent10 policy/security signoff and Agent00 authorization. None is present.
4. **Do not do:** unreviewed npm `overrides`, `npm audit fix` downgrades, silent `vitest`/Vite version changes, reclassifying withdrawn advisory as parser fix, suppressing the original 19 tests, changing expected matcher positives to false, synthetic CI success in lieu of real Linux/Windows installed app certification, or merging Agent11's experimental DRAFT PR157.

**Hard requirement: Release and issue #129 remain blocked unless a complete independently evidenced resolution or separately authorized risk decision emerges.**
