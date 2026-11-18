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
