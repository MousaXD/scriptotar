# Agent 11 — third-party parser alternatives prequalification (20 November 2026)

**Decision: BOTH community alternatives REJECTED for Scriptotar production integration.** This is a security/correctness research handoff only. Issue #129 remains OPEN and the Agent 10 independent release veto remains active. No npm override, tarball promotion, installer changes, or production dependency edits were made.

## Scope and exact upstream source

| Candidate | Source identity reviewed | Package/provenance | Status |
|---|---|---|---|
| `@ryotsu/picomatch` | [`ryotsu-kankichi/secured-picomatch`](https://github.com/ryotsu-kankichi/secured-picomatch) immutable commit `3ab5a7627d480259e3d9ca67fbc14c125d374f0e` (2026-10-29), `lib/parse.js` Git blob `ad31028701bbaf5a4b31b11f70651bcee8aceec2`, `lib/constants.js` blob `a13c4830e13242e00d71e71de7554f6c95d47f44` | Public npm version described in source: `4.0.7-a`. **Original registry tarball not downloaded nor independently matched to GitHub source**. Version is a prerelease-like fork, not an official security release. Upstream official 4.0.7 unmodified in Scriptotar lock. | **REJECT** — original security and capture parity fail |
| `matchnext` | [`2tsoftwareapp/matchnext`](https://github.com/2tsoftwareapp/matchnext) exact `main` commit `49e7837359ad7b5e9d604d9df2e1b067fc8399dd` (2026-11-11), `src/index.ts` Git blob `1ff370d410fe07052a3bdeef725bcdf732e15407`, `package.json` blob `de8cbc715741a7ac202ce0a7b3853f000e68ce15` | `package.json` explicitly `private:true`, version `0.0.0-m1`, only `test` and `typecheck` scripts; no published package provenance established and no complete drop-in interface | **REJECT before running untrusted code** — incomplete API/fixtures, source-level correctness and boundedness gaps |

## `@ryotsu/picomatch`: original independent Linux source-only result

Exact Scriptotar source [workflow `.github/workflows/agent11-community-picomatch-fork-evaluation.yml`](https://github.com/MousaXD/scriptotar/blob/agent-11/p0-picomatch-candidate-evaluation-20261120/.github/workflows/agent11-community-picomatch-fork-evaluation.yml), original runner [#44794428092](https://github.com/MousaXD/scriptotar/actions/runs/44794428092), source SHA `ef06ae8f0b414e666736ce67c5d240776571621f`, Linux job `136490133569`: completed **FAIL** at all mandatory attack/compatibility contracts, not checkout, lock, trusted-guard, or npm setup.

The test starts with Scriptotar's original pinned `picomatch@4.0.7` npm install. It creates the independent **180 pattern-option × 84 short input reference** before replacing only the TWO changed fork source files inside an ephemeral `node_modules`. The original 19-case independent safety probe is pinned to Scriptotar commit `292e408087d8b48bc012e52d1f3e7ab0c8b89650` (unchanged), original Agent 11 10-case upstream matcher parity and separate 12-case expansion are also unchanged. All code runs in killable child processes. The tracked lockfile/manifest remain byte-identical. No public fork package was installed, built, or published.

| Mandatory hard gate | Actual result | Why it matters |
|---|---|---|
| Original 19-case ReDoS probe | **8 PASS / 11 TIMEOUT** | Repeated branches, classes, negation, group quantifiers and long near misses still trigger catastrophic backtracking |
| Expanded 12-case independent option/class adversarial probe | **4 PASS / 8 TIMEOUT** | Ranged and uppercase class overlap, `nocase`, `capture`, disabled recursion remain unsafe |
| Original 10-case historical 4.0.7 matcher positive/negative controls | **10/10 PASS** | This small subset alone would falsely suggest the fork was compatible |
| Independently sampled 180 matcher/option combinations × 84 short input strings, including `capture:true` | **12/180 combinations differ from published 4.0.7** | Several matches remain true while the **capture substrings change**; a real public API behavioral regression |

Examples from the **original GitHub runner logs**:
- Pattern `+(*)c`, input `abc`, `{capture:true}`: original matched with captures `["ab","b"]`, fork matched with captures `["ab","ab"]`. Boolean parity alone misses the regression.
- Pattern `+(a*)`, input `aa`, `{capture:true}`: original capture `["aa",""]`, fork `["aa","a"]`.
- Pattern `+(*a)`, input `aa`, `{capture:true}`: original capture `["aa",""]`, fork `["aa","a"]`.

**Disposition:** source fork rejected as a compatible fixed parser. Do not test the published npm tarball or push to Windows / native packaging for this candidate without a fundamentally new, independently reviewed patch. A license label, npm release and superficial 10/10 result cannot override the 19/12 parser-security and capture-parity hard failures.

## `matchnext`: source-level prequalification blockers (NO third-party code execution)

This alternative has an attractive theoretical architecture (glob AST rather than dynamic regex), but its **current source** does not meet required Scriptotar criteria:

1. **No drop-in picomatch API.** The source exports `isMatch(input,patterns,opts)`, `match(list,patterns,opts)`, `expandBraces`; it does not export the callable `picomatch(pattern)(input)` interface, `makeRe`, `scan`, or full capture/callback behavior needed for package consumers. Its README simultaneously describes a different `match(input,pattern)` result shape than actually exported source. This is a product-integration blocker even without a security flaw.
2. **Unbounded brace expansion occurs BEFORE the matcher step budget.** In `src/index.ts` `expandBraces` (around lines 85–131), numeric ranges are materialized with an unbounded `for (let v = start; ...; v += step) out.push(String(v))` and then recursively `flatMap` expanded (lines ~115–126). `isMatch` calls this on untrusted glob patterns (around line 712) **before** creating the budgeted `Matcher` (line 716). A syntactically tiny range like `{1..1000000000}` thus requests up to a billion strings/iterations without triggering the claimed matching-step cap; the implementation provides no size/expansion guard at this boundary. This is **a source-derived denial-of-service concern**, not a benchmarked proof of an exact runtime threshold. **Do not execute such a payload unbounded.**
3. **Incorrect case-folding for symbols.** `Matcher.norm` (around lines 467–469) uses `nocase ? cp | 0x20 : cp` for ALL Unicode code points, not ASCII A–Z alone. This maps `[` (U+005B) and `{` (U+007B) to the SAME normalized code point under `nocase:true`, although they are not case equivalents. E.g. the lexer would turn escaped glob `\\[` into literal `[`, and the matcher may wrongly accept input `{` with `nocase:true`. This is a source-derived concrete mismatch to correct before production. It also does not implement general Unicode caseless matching.
4. **Provenance/test-claim inconsistency.** README headline says `100% fixture-compatible`, but its own table states 128 of 128 **selected extracted rows** pass, and its text says the full `1,998` upstream test corpus remains a future milestone. Its package is `private:true`, version `0.0.0-m1`, and has no `build` script despite README `bun run build`. Its repository tree has no top-level `LICENSE` file (despite README linking one); MIT origins/provenance need review before redistribution.

**Disposition:** do not graft `matchnext` into `node_modules`, replace npm's transitive picomatch, or run its engine with hostile inputs in this project. Its broad approach could inspire a future safe parser design, but these particular sources are not production-ready and should not be represented as proof of boundedness.

## Engineering requirements to genuinely close issue #129

A future candidate must come with **all** of: a maintained, reviewable MIT-compatible source distribution; independently verified published tarball bytes and integrity; callable picomatch API and options parity including captures and callbacks; bounded syntax expansion, lexer, compiler and matcher work; general extglob/negation/globstar and OS/dot/Unicode correctness; preserved **immutable original 19**, independent expanded **12**, original legacy 10, and bounded 180 differential gates; upstream fixture suite (not just 128 selected rows), generative fuzzing, real Vite/Vitest/Tinyglobby integration checks, and exact SHA Linux/Windows builds. Agent 10 must independently evaluate the final source SHA; Agent 00 owns integration and release. A withdrawn advisory or historical absent browser/installed-package paths do **not** constitute a safe parser.

**Final: Parser unresolved, issue #129 OPEN, release NO-GO, Agent11 PR157 DRAFT/NO-GO.**
