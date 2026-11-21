# Agent 11 — upstream picomatch PR #277, exact-source Linux/Windows rejection (2026-11-21)

**Verdict: REJECT as a Scriptotar production parser remediation. Issue #129 stays OPEN; PR #157 stays DRAFT; Agent 10 independent security veto and Agent 00 release NO-GO remain.** This is a research-only, source-pinned test report, not a fixed npm dependency, production override, risk acceptance, or package approval.

## Provenance

| Component | Original immutable identity |
| --- | --- |
| Scriptotar research test commit | `e27df1fdafc3c915c820ab749396a43698321699` |
| Original tested workflow | `.github/workflows/agent11-picomatch-final-v4-upstream-proposals.yml` |
| Original two-OS CI run | [#44916734647](https://github.com/MousaXD/scriptotar/actions/runs/44916734647) |
| Upstream candidate | [micromatch/picomatch#277](https://github.com/micromatch/picomatch/pull/277), unmerged source head `a2ac5967ed7e5547cea5aa031e9f7e81b8972511` |
| Candidate parser file Git blob | `a59932ccf6210970bf73514e30b3336a2adf13fa` |
| Original official locked desktop picomatch | `4.0.7`, `apps/desktop-ui/package-lock.json` Git blob `eebc9228dd404f0b9f9bd55f2665ab3e00fe9dfa` |
| Trusted first-party guard | commit `55ce3ac389b7e1fcab99b3a5acc84b526630af72`, guard blob `5c746abd3caf414ba2c80be825d66983312236c6` |
| Immutable original 19-case probe | blob `e1d4f227c9ee0d4069c823dbd8b5fec711d67228` |
| Independent 12-case security probe | blob `c126c5de32a3bb5f1ff5c322185b8c3a16dde404` |
| Immutable original 10-case matcher probe | blob `f4fa56c4075d02df2eb1fc34a463cdc7958c173c` |
| Original 180-pattern-option × 84-input capture/matcher differential | blob `76b23ead7bd208cda1f3e38d39991f2962d15c07` |
| Original native-RegExp public escape contract | blob `3c1d9b7fa860a8247f73566d7dfc0f5135a3a0c8` |

The workflow checks source/pin identities and runs the independent reviewed guard **before** `npm ci`. It installs the **original** official exact-lock package, records its unmodified matcher baseline, then copies **only** the pinned upstream `lib/parse.js` file into the *disposable* runner's `node_modules/picomatch/lib/parse.js`. The production repository manifest, lockfile, build guard, Tauri sources, main, integration, and published app were **not** patched.

## Actual completed jobs — BOTH intentionally RED

| Mandatory original gate | Windows 2022 (job [136883865034](https://github.com/MousaXD/scriptotar/actions/runs/44916734647/job/136883865034)) | Ubuntu 22.04 (job [136883865380](https://github.com/MousaXD/scriptotar/actions/runs/44916734647/job/136883865380)) |
| --- | --- | --- |
| Untouched baseline matcher control | 10/10 PASS | 10/10 PASS |
| Original matcher control with #277 parser | 10/10 PASS | 10/10 PASS |
| Original immutable 19-case hostile probe | **4 PASS / 15 TIMEOUT — FAIL** | **5 PASS / 14 TIMEOUT — FAIL** |
| Independent 12-case hostile probe | **2 PASS / 10 TIMEOUT — FAIL** | **4 PASS / 8 TIMEOUT — FAIL** |
| Original 180 pattern-option × 84-input finite matcher/capture differential | **4/180 changed — FAIL** | **4/180 changed — FAIL** |
| Native `makeRe` regex `.exec`, `RegExp.prototype.exec.call`, reconstructed `new RegExp(...)` | **9/21 PASS / 12 TIMEOUT — FAIL** | **12/21 PASS / 9 TIMEOUT — FAIL** |
| Overall workflow | **FAILURE**, original strict hard gate | **FAILURE**, original strict hard gate |

The changed four `{capture:true}` combinations are `+(*)`, `+(*)c`, `+(*a)`, and `*(*a)`. Reproducible finite example: `+(*)c` on `abc` originally returns capture `['ab','b']`, but #277 returns `['ab','ab']`. The boolean match may remain true, yet **public capture API compatibility fails**. Do not relax the original positive/capture tests to declare success.

Original 19-case residuals include suffix-star single branch, multi-branch wildcards, nested wrapper stars, repeated character classes, repeated literals, negation/wrappers and long near misses. Original expanded 12 still times out overlapping/ranged classes, `nocase`, `capture`, nested forms and optional recursion. Node-native route timeouts show the emitted RegExp remains unsafe, independently of any public matcher wrapper.

## Narrow patch scope, known incompleteness

The upstream #277 author explicitly proposes simplifying **single-branch leading-star bodies** meeting `/^\\*[^/\\\\()[\\]{}|!@+]*$/` with no `**`, by avoiding redundant outer repetitions. The author explicitly states that multi-branch, nested extglob and bracket-class constructions remain slow, and that `capture:true` changes the final captured repetition. Thus this is a useful narrow parser performance proposal, **not** a general safe, semantics-preserving picomatch replacement under Scriptotar's required contract. Its own upstream 2001 short/positive tests and Linux finite Boolean comparisons are insufficient for Scriptotar's broader security, capture and native-RegExp requirements.

## Integration and release implications

- **Do not merge** #277 into Scriptotar as a security remediation; **do not merge draft PR #157 as a parser fix**. Upstream #277 is also unmerged, not an authenticated/published npm replacement.
- The strict hard gate ran and failed before candidate `npm run check`, `npm test`, `npm run build`, exact replacement-package integrity/license, patched Linux .deb, Windows NSIS, and current installed-native verification. None of those downstream candidate claims can be made.
- A clean `npm audit` caused by withdrawal of GHSA-fc5q-39xc-vjpf does not repair the observed code. The existing dev-only graph, first-party prebuild source guard, and older app package inventory narrow exposure but do **not** resolve build-time execution of arbitrary hostile patterns.
- **Agent 00/10:** retain explicit Option A (P0 release NO-GO) absent separately documented and independently approved narrow Option B threat-model risk acceptance. See [issue #129 decision options](https://github.com/MousaXD/scriptotar/issues/129#issuecomment-7064040515). No such acceptance was given by these tests.

**Status:** independent original cross-OS negative evidence complete for upstream #277. No production change, no fixed release, no security waiver. #129 OPEN / Agent 10 veto ACTIVE / Agent 00 release NO-GO.
