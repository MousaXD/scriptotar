# Agent 11 — Independent residual-risk decision package (2026-11-18)

**Disposition: BLOCKED AS REMEDIATION. No patch is installed. PR #132 stays draft; no main merge, no audit bypass and no release authorization.** This is a security evidence and decision record, not a claim that picomatch's pathological behavior has been corrected.

## Evidence identity and meaning

- Repository: [MousaXD/scriptotar](https://github.com/MousaXD/scriptotar); Agent 11 branch: `agent-11/p0-supply-chain-recovery-20261118`; [PR #132](https://github.com/MousaXD/scriptotar/pull/132), base `integration/scriptotar-defragmentation-agent00-20261113`.
- Reviewed starting head: `292e408087d8b48bc012e52d1f3e7ab0c8b89650`; that head's base was `63fafe38a2a9280d8b0cec275f185906ee643407`. **All evidence below is pinned to its own source and workflow SHA, and must be revalidated on a later head.**
- [Exact starting-head Agent 11 run #44376335867](https://github.com/MousaXD/scriptotar/actions/runs/44376335867): real installed `picomatch@4.0.7` hostile 19-case assertion **FAIL**, with the separate strict unfiltered npm advisory command **PASS**, Python pip-audit **PASS**, and Linux/Windows static frontend boundaries **PASS**. Whole workflow **FAIL**, deliberately and correctly. The passing npm audit is not a parser fix.
- [Same-source Tauri integration run #44376352531](https://github.com/MousaXD/scriptotar/actions/runs/44376352531): frontend check, tests, production build, Rust workspace, sidecar, integrated build, and supply chain including npm and Python **PASS**. These are compatibility/metadata gates only, not hostile-glob resistance.
- [Historical genuinely packaged artifact validation #44375979794](https://github.com/MousaXD/scriptotar/actions/runs/44375979794) PASS on real Linux and Windows original packages. The package source is **`531c5e2fb6f2eb17df80de1cc40a48fc24b551d8`**, not the starting Agent 11 branch head and not final integration. Do not transfer package certification across sources.
- The public [GitHub reviewed advisory JSON](https://github.com/github/advisory-database/blob/main/advisories/github-reviewed/2026/10/GHSA-fc5q-39xc-vjpf/GHSA-fc5q-39xc-vjpf.json) records `withdrawn: 2026-11-18T15:38:56Z`. Withdrawal removes that advisory's current audit finding; it cannot remove the observed exponential/backtracking behavior.

## Official upstream provenance and candidate decision

- Official latest [micromatch/picomatch release](https://github.com/micromatch/picomatch/releases/tag/4.0.7) is **4.0.7**, published 2026-08-24. Its annotated Git tag object is `54e4f2ba43c34874f08d1551012c07930b59f0ea`. No newer maintained, officially patched package was identified as of this review. The locked installed version remains `4.0.7`.
- [PR #260](https://github.com/micromatch/picomatch/pull/260): **OPEN, not merged**; evaluated pinned upstream head `d10c0e5003aabed2d81427dd6c125af50bf920ab`.
- [PR #264](https://github.com/micromatch/picomatch/pull/264): **OPEN, not merged**; evaluated pinned upstream head `8144a1a22a8fa1766951411f220cc8c6228cb041`.
- Source-exact backports of both PR heads against original `4.0.7/lib/parse.js` were hash-verified before execution. [Candidate run #44352653285](https://github.com/MousaXD/scriptotar/actions/runs/44352653285): **both PR260 and PR264 failed the expanded bounded corpus on Ubuntu and Windows**, even after correct patch application. **Reject both as comprehensive remediation.** PR264 hit at least three hostile alternatives; PR260 hit at least eleven on Windows. This test is bounded, not an exhaustive proof of safety or insecurity in every other pattern.
- [PR #259](https://github.com/micromatch/picomatch/pull/259) was **closed unmerged**. Earlier limited green tests are superseded by the larger corpus and do not qualify the fork for deployment. Other proposals (#237, #274) also previously failed hostile variants; do not repeat unproductive unpinned patch trials or silently select an unmaintained fork.
- **No published/reviewed parser fix, secure replacement dependency graph, or verified compatible full-corpus patch has been produced.** Any future candidate must prove exact Git commit and content hashes, clean reproducible patch application, 19/19 bounded hostile cases on Linux and Windows, application checks/tests/build, strict original npm audit, and independent Agent 10 review. Passing the 19 cases alone would still be insufficient.

## Threat-source-to-glob reachability matrix

| Boundary | Possible attacker-controlled data | Evidence | Decision |
| --- | --- | --- | --- |
| Developer-local tool execution | Developer-chosen glob pattern / modified project configuration | `picomatch` exists in the dev-only lockfile graph; locally supplied hostile **PATTERN** can drive ReDoS. | **Reachable if developer or tooling submits pathological patterns**; not a demonstrated remote desktop-user exploit. |
| CI processing incoming repository content | PR-modified first-party source/config or third-party build input; untrusted filenames alone are only path values | Linux/Windows clean `npm ci`, Svelte/Vitest/Vite checks run on checkout. The reviewed frontend's `vite.config.ts` uses the Svelte and Svelte testing plugins without direct attacker-supplied glob calls. | **Potential build-time trust boundary**. Only a source/config-to-third-party-pattern call trace, including PR trust policy, could establish exploitation; not established here. |
| Build-time Vite/Svelte/tinyglobby/fdir | Dependency-generated patterns, source-controlled glob macros, workspace metadata | `audit-desktop-runtime-exposure.mjs` checks the reviewed `vite build` contract, 81 first-party frontend config/source files, direct glob API indicators, and built output. It does **not** inspect dynamic internals of all third-party plugins. | **Not proven attacker-reachable; not proven unreachable**. File names by themselves do not demonstrate attacker control of a glob PATTERN. |
| Static shipped frontend | User-entered URLs/transcripts or webview inputs | Tauri `frontendDist` points at the Vite-built `dist`; no direct first-party glob/API references and no named picomatch/tinyglobby/fdir markers in scanned JS/CSS/HTML/maps on two platforms. All watched npm entries are marked dev-only. | **No identified Node picomatch execution path in static frontend**. Heuristic source/text scanning does not exclude minified, aliased or equivalent behavior. |
| Installer-bundled runtime/resources | Installer-supplied files and tools, not necessarily npm packages | Generated Tauri resources map includes `transcription-runtime/` and `mcp-tools/`. Real historical `.deb` and NSIS file-path inventories have no watched Node parser module path. | **No named parser module paths in the inspected historical artifacts**; content-embedded parser bytes, embedded interpreters, and newer packages have not been conclusively excluded. |
| Production user-controlled inputs | URLs, project files, text, IPC values | No demonstrated direct route from these inputs into a picomatch PATTERN in the installed process; DevTools/browser JS runs shipped static assets rather than `npm ci` tooling. | **No positive exploit trace**, not an assertion that production equivalents are impossible. Agent 10 must review installed commands and sidecars independently. |

Key distinction: `picomatch(untrustedPattern)(filename)` requires attacker influence over the **pattern** for this reproduced attack. A hostile *filename* may be a target string but is not automatically a hostile *pattern*. Conversely, compiled output having no literal `picomatch` label is not proof the algorithm or an equivalent matcher is absent.

## Original package provenance (historical source only)

| Real artifact | Source workflow | Verified SHA-256 | Actual path entries | Path-inventory SHA-256 |
| --- | --- | --- | --- | --- |
| Linux .deb | [44363867237](https://github.com/MousaXD/scriptotar/actions/runs/44363867237) | `0e209226fce48e8943d28d7d3cdbc7840fdc09d36c90a8eab0bd7b708fc81df1` | **2,992** `dpkg-deb --contents` entries | `026f507a9672b235fe10c29ce88cf6958a9e94fa201670f66333514d4d251c93` |
| Windows NSIS | [44363867107](https://github.com/MousaXD/scriptotar/actions/runs/44363867107) | `26a0d5f01afb671152e935e0a60c49dd0d133273bd900b09992f91c1f079760e` | **2,387** 7-Zip `Path =` entries (not 19,102 raw metadata lines) | `9deb4279f8e261cf353ceb3d966cf4abb926a06c559e38d1d442017645ae9533` |

Both were independently matched to their builder-produced SHA256SUMS and original run `head_sha`. Both inventories include expected `transcription-runtime/` and `mcp-tools/` paths and zero watched named parser/build-tool paths. **This verifies file-name inventories and package provenance, not absence of parser bytes or runtime behavior.** For a final release, regenerate packages on the **same final integration SHA**, verify original builder digests and manifests, inspect actual extracted package contents (not just names), and trace any suspected matcher's executable entrypoint. Until then final-byte certification is **NOT TESTED**.

## Reproduction and acceptance commands

From an exact-source checkout and on both supported CI runner operating systems, preserve the existing test assertions:

```sh
git rev-parse HEAD
cd apps/desktop-ui
npm ci
npm ls picomatch tinyglobby fdir vite vitest
node scripts/probe-picomatch-redos.mjs --report --extended
node scripts/probe-picomatch-redos.mjs --assert-patched --extended
npm audit --audit-level=high
npm run check
npm run test
npm run build
node scripts/audit-desktop-runtime-exposure.mjs
node scripts/audit-tauri-bundle-resources.mjs
node scripts/test-tauri-manifest-inventory.mjs
```

**Expected current published parser behavior:** the `--assert-patched --extended` command exits nonzero on hostile inputs. This must remain visible and must not be turned into a fake PASS. Real `--manifest` + `--package` audits require authentic archived/extracted package paths and their independently verified source-build hashes; synthetic CLI fixtures do not substitute for packages. Preserve complete runner logs, pattern-level verdicts, source SHA, dependency lock SHA, npm audit JSON, check/test/build outcome, package hashes, manifest hashes and independent reviewer decision for the same code.

## Required explicit owner decision (not yet authorized)

**Agent 10 / independent security veto:** sign a reviewed source-to-pattern reachability assessment for CI, third-party plugin paths, Tauri bundled assets, sidecars and production IPC; review the 19-case failures, binary inspection gaps and controls; record **APPROVE RESIDUAL RISK**, **REJECT**, or **REQUEST EVIDENCE** with scope and rationale. Absence of an exploit trace alone is not approval.

**Agent 00 / integration owner:** receive Agent 10's decision; verify final source SHA, Linux/Windows final installers, original strict audits, full green release gates (including native WebView and GUI), and keep separate production security controls intact. Choose **WAIT FOR FIX / REPLACE DEPENDENCY**, or request a **formal, scope-limited owner risk exception**. This draft report is not a release authority.

**Repository owner (required for an exception):** explicitly authorize or reject any exception in writing, specifying the reviewed commit SHA, dev/build-only scope and excluded runtime assumptions, allowed PR treatment, duration/expiry, compensating controls, further evidence conditions, and rescission triggers (a confirmed reachable attacker-supplied glob PATTERN, an upstream fix, a dependency graph change, or new final package contents). **No exception or approval has been granted here.**

**Final disposition:** `BLOCKED AS REMEDIATION` / `DRAFT` / `NO RELEASE AUTHORIZATION`. PR #132 may convey reproducible independent evidence but cannot be represented as a fixed dependency. Preserve all historical commits.
