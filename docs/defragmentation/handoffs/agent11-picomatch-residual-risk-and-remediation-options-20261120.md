# Agent 11 — picomatch parser residual-risk and remediation options (20 Nov 2026)

**Decision: OPEN #129 / release NO-GO / Agent 11 production remediation BLOCKED.**

This is a source-exact research handoff, not approval to publish, merge a patched package, suppress security tests, or change the independent Agent 10 veto. Agent 00 alone owns integration.

## Immutable production lock and changing integration baseline

- **Newest rechecked integration PR #114 SHA:** `7bb5cfec814abf5502515753cc13409da1f73c4b` (Agent00 merged additional independently scoped P2/native-performance source after the earlier PR158 guard-order change); the checked desktop package-lock Git blob remains **identical** at `eebc9228dd404f0b9f9bd55f2665ab3e00fe9dfa` and still installs picomatch `4.0.7`. **Historical original Linux and Windows package artifacts below are from integration SHA `50ed95608fc742db4a7cf1bdd2ac04fa8c5457e6`, NOT from the newer 7bb... HEAD.**
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
- A **separate later test harness introduced a false-positive** in static lock attestation: [run #44742001158](https://github.com/MousaXD/scriptotar/actions/runs/44742001158) FAIL BOTH OS before `npm ci` with `Unexpected optional or peer edge: node_modules/fdir`. This is a mistake in Agent 11's initial attestation that missed the existing optional peer metadata; **not** evidence of a modified package or new runtime dependency. Corrected test-only `agent11_desktop_npm_scope_attestation.mjs` at `f7b76f0d6ac8873ec70e700fca24a6792ad2f6ed` explicitly pins that exact optional peer edge and fails closed on mandatory peer conversion, importer flag drift, new transitive edges and integrity changes. Source-exact corrected Linux/Windows workflow [#44775812658](https://github.com/MousaXD/scriptotar/actions/runs/44775812658) at test-only SHA `f7b76f0d6ac8873ec70e700fca24a6792ad2f6ed` **COMPLETED SUCCESS BOTH OS**: Linux job `136425191936`, Windows job `136425192576`. Both original logs show **11/11 deliberately adversarial static lock mutation controls FAIL-CLOSED**, original four-edge picomatch dev-only scope attested and actual production Vite/Rollup graph still **174 modules, one 430559-byte JS asset, 0 matched parser module paths**. These only certify the pinned npm lock and original browser build, **not installed binary contents or safe Vite runtime with hostile globs**.
- The census uses the pinned source guard, real locked `npm ci`, genuine original Vite config, actual Rollup `this.getModuleIds()`, compiled JS output checks, and cleans a temporary output directory. It does not use experimental patched parser source. Its browser-only result cannot certify Rust, installers, scripts, or safe use of the Vite build process.

Never equate `npm audit --omit=dev` or absent browser bundle assets with safe execution of a dev tool on untrusted globs.

## Source-verified Tauri bundle input scope (separate from actual installer contents)

At integration SHA `50ed95608fc742db4a7cf1bdd2ac04fa8c5457e6`, `apps/desktop/src-tauri/tauri.conf.json` exact blob `b28cc6a5057032479f94d5b94ed01f2ea23fa03e` declares `build.frontendDist: '../../desktop-ui/dist'` and `beforeBuildCommand: 'cd ../desktop-ui && npm ci && npm run build'`. Its checked-in `bundle.active` is false; packaging workflows generate platform-specific bundle configuration at build time. Thus the **executed npm build is in the installer pipeline even though the application frontend comes from dist**.

The reviewed **Windows** workflow `.github/workflows/windows-tauri.yml` Git blob `86e9c7c808d7748690f8f1dfebffd61078f4cdf3` dynamically sets bundle resources only to `resources/transcription-runtime/` and `resources/mcp-tools/` (lines 141–146), then builds real frontend assets from `apps/desktop-ui` `npm ci` + `npm run build` (lines 162–166) and packages via `tauri bundle`. No `node_modules` resource mapping is declared in that source. This **supports** static source segregation of dev dependencies, but isn't an independent examination of the final NSIS installer bytes.

The reviewed **Linux** workflow `.github/workflows/linux-tauri.yml` Git blob `0bd2bc3a066af0dc9142595adbdc9e9803c7ea7a` similarly sets bundle resources only to `transcription-runtime` and `mcp-tools` at lines 103–108; frontend build `npm ci` / `npm run build` at lines 140–144, `cargo tauri build --bundles deb -- --locked` at 149–150. Its real `DEB-CONTENTS.txt` listing is generated in CI (line 167), but not examined here for an actual `node_modules`/picomatch negative. **Do not claim that a generated Linux .deb or Windows NSIS has been independently verified free of picomatch** on this source review alone.

A separate Agent10 finding already identifies the Linux packaging workflow's source-trust guard executes too late relative to pip/Python/Rust steps; that issue remains independently owned by Agents 00/10 and is NOT fixed by this parser research.

### Original immutable Linux/Windows package inventory test (new, evidence-only)

For historical integration `50ed95608fc742db4a7cf1bdd2ac04fa8c5457e6`, the ACTUAL already-produced and successful package runs and immutable artifacts are:

| OS | Original package run | Original artifact ID and name | GitHub original uploaded ZIP SHA256 |
|---|---|---|---|
| Linux .deb | [#44736655860](https://github.com/MousaXD/scriptotar/actions/runs/44736655860) | `14629728341`, `scriptotar-linux-tauri` | `7ea435e3fe916809129a28ad597878ba5bd58791c5df6fdb766c8bb03b5f9c2f` |
| Windows NSIS | [#44736655842](https://github.com/MousaXD/scriptotar/actions/runs/44736655842) | `14630097202`, `scriptotar-windows-tauri` | `cba3d855e84e11693299ac1025cd37ab92ad3f974699312d153cf7d1bfb10a67` |

The original artifact IDs, exact build-source SHA, and GitHub API artifact upload digests were verified via original action logs and the artifact list. Agent11 created a **separate bounded historical inventory workflow**, `.github/workflows/agent11-historical-installed-package-picomatch-inventory.yml`, test-only commit `bd843df4ebf15333563fc7f61880cbfc6d89e42e`: it uses GitHub read-only artifact access, pins both original SHA and artifact IDs/digests before download, re-verifies original included SHA256 manifest, reads the original .deb archive directory table and actually silent-installs the original NSIS package before listing *installed filesystem paths*. Positive controls require actual app/runtime resources. It rejects present `node_modules/picomatch/tinyglobby/fdir` file paths and records small, independently inspectable inventory artifacts.

**Original source-exact inventory validation:** [run #44784540456](https://github.com/MousaXD/scriptotar/actions/runs/44784540456), actual inventory workflow source `bd843df4ebf15333563fc7f61880cbfc6d89e42e`.

- **Linux .deb independently verified PASS** in original [job #136455771647](https://github.com/MousaXD/scriptotar/actions/runs/44784540456/job/136455771647): original source SHA and immutable artifact ID + uploaded ZIP digest checked; `sha256sum --check SHA256SUMS.txt` passed; recreated `dpkg-deb --contents` listing byte-identical to the original archived `DEB-CONTENTS.txt`; **2,992 .deb entries**, with expected bundled resources and **no named `node_modules`, picomatch, Tinyglobby, or Fdir package paths**. The small independently generated listing is preserved as original [artifact #14652844095](https://github.com/MousaXD/scriptotar/actions/runs/44784540456/artifacts/14652844095).
- **Windows genuine NSIS install inventory independently verified PASS** in original [job #136455772073](https://github.com/MousaXD/scriptotar/actions/runs/44784540456/job/136455772073): original source SHA, immutable artifact ID and post-upload ZIP digest pinned and verified; actual downloaded `Scriptotar-Next-1.4.1-x64-setup.exe` hash matched original Windows `SHA256SUMS.txt`; genuine silent NSIS installation into an isolated Windows runner directory succeeded; **2,831 installed paths** inspected, mandatory `transcription-runtime`, `mcp-tools`, and `scriptotar-desktop.exe` positive controls present and **no named `node_modules`, picomatch, Tinyglobby, or Fdir paths**. Original [Windows inventory artifact #14654341281](https://github.com/MousaXD/scriptotar/actions/runs/44784540456/artifacts/14654341281) preserves the installed paths. **Entire original two-job inventory workflow completed SUCCESS.**
- **Exact source tree carry-forward check:** GitHub's complete, non-truncated recursive trees for historical integration `50ed95608fc742db4a7cf1bdd2ac04fa8c5457e6` and newer integration `7bb5cfec814abf5502515753cc13409da1f73c4b` have **118 `apps/desktop-ui/` entries each and ZERO blob SHA/type/path differences**. Thus all tracked desktop UI build inputs (including lockfile/manifest/config/source) are byte-identical, but this **does not** transfer earlier installer compilation or any separate native/Agent10 release approval to newer integration.
- This workflow is historical only. Even a full path-list PASS cannot prove every packaged executable's bytes lack an embedded parser, certify newest integration source, fix Vite's vulnerable matching at build time, or authorize release.

**Release boundary:** the historical original `.deb` and NSIS installed FILE-PATH inventories are now independently verified PASS on both platforms, but only for exact packaged source SHA `50ed95608fc742db4a7cf1bdd2ac04fa8c5457e6`. This does NOT scan all executable binary bytes or dynamically executed code, does NOT fix unsafe build-time glob processing, does NOT certify newer owner integration `7bb5cfec...`, and does NOT waive Agent10/release/native acceptance vetoes. New source SHA requires new installed/native verification.

## Decision tree for actual remediation

1. **Preferred:** an officially maintained picomatch version whose exact npm tarball, integrity, MIT/license, compiler and source provenance can be independently verified, and which passes original 19 plus expanded adversarial corpus and ALL original upstream matcher semantics without changing positive expected outcomes. Lock exact version and run all security/normal-build/platform tests. Agent10 independently reviews final SHA. No such version verified.
2. **Viable but substantial:** an independently maintained, semantics-preserving non-backtracking glob compiler/matcher that provides the actual documented APIs used by Vite, Vitest and tinyglobby, and that has proof/independent fuzzing on captures, options, negations, path handling, escaped classes and performance. Replace upstream transitive paths with **known trusted, reviewed** package distributions. A limited regex-rewrite proof of concept is *not* this.
3. **Mitigation-only:** keep the upstream original parser and comprehensively constrain externally controllable build patterns using immutable reviewed source trust policies, preserved package checks, and fail-closed CI workflows. Explicitly document that this does NOT fix a parser performance flaw; release decision requires separate Agent10 policy/security signoff and Agent00 authorization. None is present.
4. **Do not do:** unreviewed npm `overrides`, `npm audit fix` downgrades, silent `vitest`/Vite version changes, reclassifying withdrawn advisory as parser fix, suppressing the original 19 tests, changing expected matcher positives to false, synthetic CI success in lieu of real Linux/Windows installed app certification, or merging Agent11's experimental DRAFT PR157.

**Hard requirement: Release and issue #129 remain blocked unless a complete independently evidenced resolution or separately authorized risk decision emerges.**
