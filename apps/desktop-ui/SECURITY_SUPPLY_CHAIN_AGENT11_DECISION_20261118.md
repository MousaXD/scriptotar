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


## Exact PR-head package provenance extension (2026-11-18)

New read-only Agent11 jobs use the original Linux/Windows packaging workflow runs for the SAME PR head SHA and branch, selected by GitHub Actions metadata. They poll for successful completion, fail closed on absent/failed runs, verify original artifact SHA256SUMS, and independently recompute real .deb/NSIS archive path inventories using the existing validator. Historical pinned-artifact jobs remain intact. These additional jobs are not credited as passing until their exact-head results exist. A PR packaging workflow can checkout a synthetic merge ref even when run metadata shows a given PR head SHA, so this check cannot attest the full checked-out source tree. File names and installer hashes are NOT a binary-content scan, exploit-reachability proof or release certification. Agent10 threat analysis and Agent00 exact-final-integration validation remain required.

## Completed previous-head Linux and Windows genuine package provenance

At immutable prior Agent 11 SHA `a57e6c8bb9be4a48e64e16fe7c81bfe7d4c192bf`, the separate [exact-source genuine package workflow #44387006766](https://github.com/MousaXD/scriptotar/actions/runs/44387006766) **completed all 4/4 jobs PASS**, including both *actual original same-source* Linux and Windows artifact checks; this is NOT certification for any later code commit.

| Original package | Original build run | Real package SHA-256 | Actual inspected paths | Recomputed manifest SHA-256 |
| --- | --- | --- | --- | --- |
| Linux `Scriptotar-Next-1.4.1-amd64.deb` | [#44387024413](https://github.com/MousaXD/scriptotar/actions/runs/44387024413); artifact `14466973554` | `08ff3b0a94c9c1859ab3e8ed6d0db715aee89745d46420318683c7d54ff4c1f1` | **2,992** original `dpkg-deb --contents` entries | `6ef71f021d066af4f5db712e2e2a2bc16b82e19703836aa57d317addacb09cd6` |
| Windows `Scriptotar-Next-1.4.1-x64-setup.exe` | [#44387024220](https://github.com/MousaXD/scriptotar/actions/runs/44387024220); artifact `14465899558` | `4a1e4fab92b54b978931f439519a0b72dea7c48536e6c0489d6bf591387fef32` | **2,387** actual 7-Zip `Path =` entries | `e5beeb0860542cdf81340c73f8cfd83ca8191b678aeb369b919a10c1f79f1396` |

Both original packager jobs completed successfully on `head_sha=a57e6c8`, package bytes matched builders' SHA256SUMS, exact-source identity was rechecked before downloading, generated Linux/Windows/installed-native Tauri config checks passed, expected transcription/MCP resource trees were present and the validator reported `issues: []`. This checks original archive path manifests and package integrity, **not compiled/embedded binary parser signatures**, definitive checked-out merge-tree identity, final Agent00 integration package bytes or production attacker-supplied glob-pattern reachability.

The independent package provenance workflow now triggers for changed desktop frontend sources, Vite config, Tauri native source and platform packaging workflows in addition to its own files. Its exact-head results are a separate condition; do not inherit this previous-head PASS.

## 2026-11-19 — immutable original packaging checkout-tree proof

The real original [Linux package run #44389844184](https://github.com/MousaXD/scriptotar/actions/runs/44389844184) (job 135104208113) and [Windows installer run #44389844190](https://github.com/MousaXD/scriptotar/actions/runs/44389844190) (job 135104252103) both logged checking out **the same generated PR merge commit** `f671a868ec2510ef86e8580d95c796a06bb931c3` via `refs/remotes/pull/132/merge`. Its verified immutable Git parents are integration `cc55b4b19926b59e0e028f8facb0e053afa0b8b1` and Agent 11 head `5e9220fcafe4153626f9cfaeccb04e69cd1fd73b`. Both the checked-out merge and reviewed head have **exact same Git tree `cbbf8485002ce80a21a32d40cbbaa8ae1e50b3b2`**. Thus these historic packages were built from source tree bytes identical to reviewed head, although the checkout *commit* was a PR merge. The [4/4 real package verification jobs #44389832492](https://github.com/MousaXD/scriptotar/actions/runs/44389832492) separately corroborate genuine installer bytes, original SHA256SUMS and actual .deb/NSIS path inventories at that historical SHA.

New `scripts/attest-pr-package-checkout.mjs` now applies a **fail-closed original checkout attestation** in both independent current-PR-head package inventory jobs. It uses the original *successful* package job logs to recover the exact fetched PR merge SHA, independently verifies the actual checkout reference, original run identity, unique job, expected head as second merge parent, and **same immutable Git tree as reviewed head**. Nine synthetic self-tests exercise wrong PR/ref, wrong SHA/parent/tree and failed jobs, but are not real package evidence. An original workflow head_sha alone no longer suffices for current-source tree qualification. The new CI job must pass on the current full SHA before a new result is accepted.

This improves exact-source provenance, **not** parser safety: production-generated resources, archived binary bytes, hidden/renamed parser code, third-party build-time glob patterns and end-user exploit reachability require separate review. Picomatch remains unpatched, 19-case hostile regression fails, and owner-approved residual-risk exception has not been given. **BLOCKED AS REMEDIATION** persists.


## 2026-11-19 — PR142 source-identity mismatch found and corrected without downgrading attestation

At previous Agent11 source `328ae2a1ca8ecde09f93c6994f064780f97e46eb`, the original Windows [packaging job #44400549405](https://github.com/MousaXD/scriptotar/actions/runs/44400549405) completed successfully, but the independent [source-tree attestation #44400533091](https://github.com/MousaXD/scriptotar/actions/runs/44400533091) correctly **FAILED**. After fixing the original ANSI-coded GitHub job-log transport issue, the checked-out PR merge Git tree was `ce476222e9d0a7ce50973a98a9cd0ec1471d2cfa`, different from Agent11 HEAD tree `b4e41cf102875b251513761d94b5103dddddcbfb`. Exact original Windows checkout was immutable merge commit `339a45a4e33b12f0ea36954ef61ab55680b5431b` with parents Agent00 integration `346cc83afed2886938c79b06cb49f56d3827c966` and Agent11 `328ae2a1ca8ecde09f93c6994f064780f97e46eb`. GitHub compare confirmed exactly **three PR142 changed files** between these Git trees: `.github/workflows/gui-visual-qa.yml`, `.github/workflows/mcp-e2e.yml`, and `tests/defragmentation/benchmarks/test_performance_suite.py`. This is actual source drift, **not** a parser exploit or a false source integrity check.

The proper remediation was a normal two-parent Agent11 branch merge of latest integration, not an exception/allowlist. New history-preserving Agent11 commit `9ad81d36a72418099fac11e2931e5841b3bd48ca` has parents `328ae2a1...` and `346cc83...`, **0 behind integration**, with Git tree `ce476222e9d0a7ce50973a98a9cd0ec1471d2cfa` matching previously examined PR merge bytes. This retrospective source-identity match still **does not count as same-HEAD package CI PASS**. Exact-source jobs must run again; release blocked.

The independent package inventory workflow's push-path filters now also cover GUI/MCP workflow changes and benchmark test sources, ensuring future PR142-like base/tree drifts launch same-head source attestation rather than being silently omitted. The strict comparator is unchanged; no original release workflow, npm gate, vulnerable parser or native test is modified.


## 2026-11-19 — NEW independently reproduced Vite build-time hostile-PATTERN sink

**Finding: BUILD/CI SOURCE-TO-EFFECT REPRODUCED on both operating systems; not a desktop-runtime exploit.** Independent [exact SHA \`5b52d11e1e2d8f164c6f60d3f6be77d8941bada4\` Vite E2E #44409757300](https://github.com/MousaXD/scriptotar/actions/runs/44409757300) finished **2/2 evidence jobs PASS** on Ubuntu 22.04 and Windows Server 2022, Node 22, locked \`vite@7.3.7\` + \`picomatch@4.0.7\`. The positive security finding is deliberately distinct from CI's successful execution of the bounded test.

- New isolated \`apps/desktop-ui/scripts/probe-vite-pattern-reachability.mjs\` creates a fresh temporary project and invokes the REAL \`vite.build()\` pipeline with a JavaScript entry containing the **literal** \`import.meta.glob('./+(*a)')\`. A separate filesystem filename \`'a'.repeat(36) + 'b'\` is a near-miss **match target**, never misidentified as the pattern. Real source-pattern build **TIMEOUT at 8,000 ms on both platforms**, killably contained in independent child processes. The output proves a Vite build-time stall with malicious checked-in source; it does not isolate which third-party matcher call consumed the time or prove remote exploitability in deployed desktop binaries.
- **Positive clean control** \`import.meta.glob('./*.js')\` with a real \`normal.js\` file: Vite build COMPLETED on both systems and included the expected module. **Unusual FILENAME control** \`import.meta.glob('./*')\`, retaining the long near-miss filename but WITHOUT a hostile pattern: Vite build COMPLETED on both and enumerated that filename. This distinguishes attacker-controlled **PATTERN** content from hostile filenames alone.
- Source/effect chain: submitted source/config *if permitted to modify source literals* -> \`vite.build()\` -> Vite 7.3.7 \`import-meta-glob\` plugin -> \`parseImportGlob\`/normalized literal + \`getCommonBase\` (\`picomatch.scan\`) -> \`tinyglobby.glob\` / picomatch dependency graph -> bounded build stall. This is now an **observed end-to-end development/build-tool denial of service**, beyond merely a source-code-identified sink. It is **not a demonstrated externally submitted hostile PR against Scriptotar's real CI**, because the test deliberately creates temporary source fixtures and does not submit code to a production repository branch. The PostCSS \`dir-dependency.glob\` sink is separately source-reviewed but not end-to-end reproduced.
- GitHub PR CI can process untrusted source changes depending on repository permissions/review/trigger policy. The existing first-party frontend scanner reports no direct glob macros in reviewed production source, and conventional CI job timeouts cap abuse; neither fact repairs picomatch or proves future PR source is safe. A **pre-build** source/config glob-pattern control (run before any Vite build and independently enforced in applicable PR workflows) is a candidate compensating control for Agent00/Agent10 review, but the current post-build static inventory alone cannot prevent a malicious submitted pattern from stalling Vite first. Do not edit other agents' workflow gates without coordination.
- Initial exploratory dev-server workflow [#44409540903](https://github.com/MousaXD/scriptotar/actions/runs/44409540903) had Linux controls PASS + hostile timeout, but Windows's temporary dev URL loader produced an unrelated setup error, **not** a negative security result. The corrected cross-platform real-build experiment [#44409757300](https://github.com/MousaXD/scriptotar/actions/runs/44409757300) supersedes that setup issue.
- The **original independent 19-case \`--assert-patched --extended\` parser regression still FAILS** and must stay fail-closed. The standalone Vite workflow uses \`--report\` for evidence ONLY, prints the existing timeouts, and does not change npm audit, the lockfile or any release condition. Previously verified original Linux/Windows installer hash/tree attestation belongs to older exact HEAD \`aeb95293...\`; later Agent11-only test/doc commits are different trees and cannot inherit final release-package certification.

**Updated decision to Agent 10:** developer/PR-build-time hostile literal glob **reachability and real impact now reproduced on BOTH platforms**. Independently decide the practical CI trust policy, mitigation adequacy, binary/runtime exposure and whether a scoped owner exception is permissible. **Updated decision to Agent00:** enforce any approved pre-build pattern policy without accepting a vulnerable dependency as patched; run fresh exact-final-source package attestations and native acceptance before any release. **Final status stays BLOCKED AS REMEDIATION / DRAFT / NO RELEASE AUTHORIZATION.**
