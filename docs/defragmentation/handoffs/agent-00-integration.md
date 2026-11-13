# Agent 00 integration — Scriptotar defragmentation

**Date:** 2026-11-13  
**Owner:** Agent 00 integration/release gate  
**Repository:** https://github.com/MousaXD/scriptotar  
**Draft PR:** https://github.com/MousaXD/scriptotar/pull/114  
**Branch:** integration/scriptotar-defragmentation-agent00-20261113  
**Base:** plan/scriptotar-defragmentation at 2948918865c1960260a8c602e4dfe261c82cd0d0  
**Main baseline:** 69e60f91c0b00560332dac06b732fc2259c1d632 (Scriptotar Next 1.4.1)  
**Release verdict:** NO-GO pending full P0/P1/P2/P3 acceptance; source integration is not end-user agentic completion.

## Imported and reconciled work

Seven isolated branches were combined by Git tree changes without duplicate edited paths, retaining all unchanged main source code.

| Agent | PR | Integrated source | Status |
| --- | --- | --- | --- |
| 01 data foundation | #108 | schema v6, ContentStudy, source provenance, evidence/brief/draft versions, constraints, tests | Candidate P0 |
| 02 Rust API | #103 | exact AI/research detail, explicit Copy Prompt saving, project-scoped worker methods | Candidate P0 |
| 03 Studio | #109 | Study workspace, source selection, drafts/versions, research/transcript/Library deep links, EN/AR controls | Candidate P0 |
| 04 runtime | #110 | bounded, approval-gated agent core, registered and version-aligned in root workspace | Isolated P1 core; production unwired |
| 05 research | #104 | offline creator-relative snapshots and literal transcript excerpts | Isolated P1/P2 core; production unwired |
| 06 writing | #105 | structured evidence-aware angle/script/review/revision workflows | Isolated P1 core; production unwired |
| 07 QA | #106 | baseline parity, negative evidence gate, native benchmark harness and Chromium tooling | Infrastructure; independent native acceptance outstanding |

## Owner integration fixes on PR #114

1. Resolved affected RustSec quinn-proto/udp advisories using published patched checksums, without audit suppressions. Issue #111 remains open because main still has the old lockfile.
2. Registered agent runtime as a workspace member; corrected 84 formatter hunks, a step-output move in agent tests and version identity drift; enabled real CI validation of the isolated crate.
3. Reconciled Rust IPC and frontend ScriptotarApi/mock/contract tests for get_ai_run, get_research_item and save_copy_prompt. Expanded parity assertions to the true command/method inventory.
4. Introduced project-scoped Content Study create/list/get/save commands and a typed camelCase Studio gateway. Exact Library AI history uses real stored IDs and does not invent provider responses.
5. Added Studio save flushing before switching/creating projects. If a save fails, transition is rejected rather than silently destroying the keyed editor.
6. Added atomic SQLite Study bundle create/update, CAS, validated brief, selected research/transcript origins, protected evidence/job/draft links, immutable draft versions and rollback on later SQL failure. Partial title-only creative briefs are allowed, preserving blank user fields. Defer heavy body/source/version hydration until exact study open.
7. Added Rust acceptance tests: restart/durable version history; invalid-origin and cross-project guards; simulated late insert/update failure rollback; real research+transcript source association; evidence-linked source deletion failure without revision consumption. Existing project migration tests and parity must still pass on one exact head.
8. Fixed Chromium index-dependent view assertions using stable view IDs; kept native/mocked performance signals distinct.

## Test evidence and gates

- The 8acb185 integrated snapshot passed Rust foundation, CI, MCP E2E, Desktop UI, security hygiene, GUI QA, Tauri migration and Linux package. Windows installer was later cancelled by additional commits; do **not** count it as passed.
- A later atomic persistence snapshot dff1319 passed Rust foundation, Desktop UI, security, main CI and GUI QA; other workflows were superseded.
- All final checks must use the **current exact PR head**, not the checkpoints above. Consult the workflow run SHA, Clippy, cargo audit, root workspace tests, installed-app E2E and Windows/Linux packaging artifacts before any GO.

The absence of failure in a mocked Svelte test is not evidence of native performance, API provider correctness, cross-project run durability or a working autonomous agent.

## Remaining critical work

**P0:** prove exact clean-install Windows/Linux restart, real project/Study/source/draft recovery, export and EN/AR/RTL. Expose evidence editing/verification and immutable draft history end-to-end; implement safe staged delete/undo, extended new-project/source continuity, and test 100/1k/10k project database scenarios. Audit the last content UI for misleading badges/subtexts. Avoid assuming a successful installer build is an installed WebView journey.

**P1:** wire the isolated Agent 04 core to a production SQLite checkpoint/effect ledger, permitted AppServices tools, provider consent and cost ceilings, ID-based polling/reconciliation, durable user approvals, cancellation, and crash-safe no-duplicate retries. Adapt Agent 05 observations and Agent 06 evidence-aware writing to validated project-scoped sources and canonical drafts. Surface original multiple candidate scripts, citations, blocked claims and human changes in Studio.

**P2:** persist observational metric snapshots, relevant creator peer baselines and timing limitations; implement credible separate factual research with provenance/access rules and source rights. Run hardware-labeled native startup/IPC payload/RSS/render benchmarks, not fabricated or jsdom timings.

**P3:** consolidate expert tools into Home/Discover/Studio/Library/Settings only after continuity is proven; remove generic descriptions, preserve custom controls, locale quality, native installers and release evidence.

## Integration policy

No force pushes, no modification of main, no release creation, no suppression of failing security gates, no AI-provider key persistence and no unapproved external actions. The draft PR remains the sole owner integration candidate. Independent QA must review the exact final built commit before merge.
