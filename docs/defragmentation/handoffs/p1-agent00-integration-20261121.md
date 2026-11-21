# Agent 00 — approved research-to-writing integration handoff (2026-11-21)

**Status: DRAFT / NO-GO. No main merge, source pin roll-forward, release, or paid provider egress.**

## Original trusted provenance (before this PR)

- Protected main: `69e60f91c0b00560332dac06b732fc2259c1d632`.
- Frozen PR #114: `b12843c75764601248a69c0dd89ff753af8df366`.
- Independently staged PR #200: `5aadc5b685a1a0e3ef79fe7e1f128a5f343fc48c`.
- Offline integrated PR #212: `31648bc71cc12a8d32924c0581cfb2849bbac62a`, 229 commits ahead of #200, zero behind; #212 is 251 commits ahead of frozen #114 and zero behind. Nothing was pushed to these refs by this work.
- This PR #215 started **directly from** #212, targeted to its branch; it is a new, unreviewed owner delta. Compare original commits before counting any later status.

### Exact blob identities on original #212

| Specialist | Source SHA | Representative exact original blob match in #212 |
| --- | --- | --- |
| Agent 05 PR #188 | `41584fdcdb3aed0089dfdc07e99220c1186926c1` | `study_writing.rs` `c51cd87a2d58ba343c4b9c8c1909c058bc5c7512`; `study_writing_persistence.rs` `1aadc0504b26bdceb99c05425739b73f6ee18ff4` |
| Agent 03 PR #191 | `5395f02e4faabbfd59360146e0b890446f42ceb1` | `study_research.rs` `611e16e264f4769088a36a5d7f1ad8326ce286cf`; `study_intelligence.rs` `399d5ed8d330e0434b77b66ae37b58302875c28b` |
| Agent 06 PR #194 | `941dffa5e0c403e55bf806ab98349bc6e69ab33a` | `researchReportGuard.ts` `0dce595f9c3e1fee8cd0338ced5f0dd4dd25d264` |
| Agent 04 PR #195 | `ae9eb5bead8b72897b73a8c586877e81a1677a97` | **NOT exact** for `study_verification.rs`: original #212 `a53ead639aa8fdd9a5b2f979c1c62f0ec81dca0c` vs specialist `d4623b09f737467990a212d26479a7de06783a8e`. This must be independently reconciled. |
| Agent 01 PR #192 | `15699123420605a9657e25a850f683095ef4d2e2` | **Not integrated** in #212: `provider_consent.rs` blob differs; strict v13 must be reviewed separately. |
| Agent 02 PR #187 | `cb830c447ac49825dc95f2f5ca7708f953a17cfd` | **Not integrated** in #212: `agent_effects_external.rs` blob differs; late-receipt recovery must be reconciled with Agent 01's shared path. |

Exact blob matching demonstrates source identity for the listed files only, not a blanket safety certification. Original branch ancestry is verified by GitHub compare; PR #215 has **new and unreviewed** executable source.

## Work in this delta

- Added `prepare_approved_three_writing_plan(project, study, run, artifact)`. No browser-supplied research DTO is trusted; the host calls `research_writing_context`, which must replay-check the source content, human-approved v8 run/step, exact artifact and Applied receipt.
- Require three independently attributable source origins, reject unrelated/changed attachments and source revision/fingerprint/content digest errors, and bind approved research artifact ID, source hashes, unverified findings, source identity and limitations into the exact writing prompt and persisted batch.
- The `CurrentStudy::load_for_context` contract reopens the same artifact after each provider step. The canonical v11 commit calls this again **inside its IMMEDIATE transaction**, in addition to existing revision/source-row/evidence guards.
- Production generation and the private production direct-persist entrypoints refuse an unapproved legacy Copy Prompt context. The legacy fixture is test-only.
- Negative source/test cases cover malformed/mixed/cross-artifact evidence IDs, forged attachment fingerprint, malformed SHA-256 and wrong approved artifact UUID.

**No new public IPC, paid adapter, credentials, provider dispatch, workflow source pin, dependency, or migration is included.** This is an offline integration candidate and is not permission to run paid provider calls.

## Exact CI evidence

At #212 SHA `31648bc71cc12a8d32924c0581cfb2849bbac62a`: **9/11 PASS** (Rust foundation, CI, MCP E2E, Desktop UI, security hygiene, GUI visual/performance QA, Tauri migration, provider race, independent security regressions). Linux packaging failed at the step `Verify independently reviewed Linux package executable source scope`. Windows packaging failed at `Verify reviewed Python/runtime/QA inputs before script execution`. Those are source-policy failures, not demonstrated Rust compiler failures; do not bypass source guards.

At PR #200 SHA `5aadc5b685a1a0e3ef79fe7e1f128a5f343fc48c`, Linux and Windows packaging succeeded, but independent installed WebView acceptance failed. This does not approve PR #215's executable deltas.

All PR #215 checks must be read at the **final exact head**. Cancelled/superseded runs do not count. A Rust/Clippy failure on the new code is a genuine blocker even when native package refusal is expected.

## Open release blockers / veto

1. **P1 Agent 01/02 conflict:** PR #192 `provider_consent.rs` and PR #187 `agent_effects_external.rs` overlap. Integrator must retain strict exact-wire v13 CAS/v8+v10 fencing, scope barriers, authenticated late receipts, and legacy retry test correctness. Never resolve by discarding either side.
2. **P1 paid-response recovery:** Agent 02 + Agent 05 must independently establish a durable response/provenance checkpoint before v11 draft acceptance. If provider charged but local output cannot be recovered, preserve an explicit `UNCERTAIN` state and deny re-dispatch.
3. **P1 review:** Agent 10 must separately audit this exact final PR #215 SHA, the originally approved specialist identities and negative source mutations, and must certify a **separate** trusted-source baseline roll-forward before packaging. Owner self-review is insufficient.
4. **P0 security issues:** #129 picomatch parser ReDoS, #153 independently enforced branch/prebuild policy, #155 publisher authorization and #162 trusted-source roll-forward remain open until separate exact-head evidence clears them.
5. **P0/native:** Agent 07 actual Linux WebKitGTK and Windows WebView2 installed-app journey, multilingual access and restart/deny cases; Agent 08 actual native performance startup/SQLite/UI/RSS measurements; avoid substituting backend or Chromium tests.
6. **P1 acceptance:** Full real Study sources -> approved research -> signed four-step provider scope -> three distinct original scripts -> saved immutable claim lineage -> edit/export/reopen -> project switch/cancellation/revocation; use offline fixture provider pending authorization certification.

**Independent security verdict: NOT YET REVIEWED. Release verdict: NO-GO.**
