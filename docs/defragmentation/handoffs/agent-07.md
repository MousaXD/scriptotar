# Agent 07 — defragmentation QA handoff

**Date:** 2026-11-13 UTC+04. **Base:** plan/scriptotar-defragmentation @ 2948918865c1960260a8c602e4dfe261c82cd0d0. **Branch:** agent-07/defragmentation-independent-qa. **Owner:** independent QA only.

## Delivered

1. Source-presence parity inventory across 15 existing feature/security/UI/contracts with strict change-detection tests. These are not runtime proofs.
2. P0–P3 catalog of 25 **not-wired, not-run** approval, recovery, source, provenance, cost, security, Arabic, GUI, and scale cases.
3. Fail-closed evidence-label validator and unit test guarding against false native PASS claims.
4. Operator-usable process-only native startup/RSS sampler and read-only SQLite scale diagnostic, both explicitly NOT_CERTIFIED with no invented observations.
5. Existing Chromium QA runner integration via QA-owned tooling.nodecheck.mjs; no workflow or application changes.
6. Proposal with exact SHA and CI evidence, feature parity table, known source-level continuity gaps, performance methodology, and independent phase GO/NO-GO criteria.

## Independent findings

- **Confirmed by source:** Library's AI/research entries do not hand off exact artifact IDs to the destination view. Report owner: Agent 02 + Agent 03. Reproduce using two saved artifacts and compare IDs; test wrong-project/deleted handling.
- **Clarify and test:** local Copy Prompt uses buildAiPrompt while backend has persisted run path; prompt preview versus saved history must be explicit. Owner: Agent 02 + Agent 06.
- **Existing issue #100 remains blocking:** native installed WebView startup, memory, scale and UI evidence is not provided by browser mock timings. Do not merge based on this QA PR.
- **Previous work preserved:** summary bootstrap, on-demand transcript detail, shell-first skeleton, project-switch safety, Chromium control tests and packaged IPC fixture infrastructure.

## Status

P0-00 **PARTIAL / NO-GO**. P0, P1, P2, P3 exit gates **NO-GO**, awaiting actual integrated implementation and native tests. This branch never runs an AI provider, network media download, SQLite migration or native installed application; no such PASS is claimed.

## Next steps for Agent 00

- Review this PR against planning branch, ensure only QA-owned paths changed, and confirm the new Node→Python guard passes at the **new PR HEAD**.
- Freeze typed study/source/draft interfaces; assign exact-ID navigation failure and Copy Prompt history to their owners.
- Make a required exact-integrated-SHA gate covering QA metadata plus genuine Rust/SQLite/IPC and package acceptance, without weakening existing workflows.
- Collect genuine installed Windows/Linux performance baselines (hardware, package/exe hashes, cold/warm policy, native WebView readiness, real IPC payload, child-process RSS) and 100/1k/10k isolated datasets before performance claims.
- Request an independent re-audit after each P0-P3 integration slice; keep main and releases untouched.

This handoff is NOT a release signoff.

## Independent QA continuation (2026-11-13)

- Following initial draft PR #106, the [c0ee54b QA CI](https://github.com/MousaXD/scriptotar/actions/runs/43443263814) passed 140 Chromium scenarios, all five QA-tooling subtests and the nested Python fixture suite. The [main CI](https://github.com/MousaXD/scriptotar/actions/runs/43443263812), [Windows package](https://github.com/MousaXD/scriptotar/actions/runs/43443263686) and [Linux package](https://github.com/MousaXD/scriptotar/actions/runs/43443263730) also passed at that SHA; **native WebView interactions/performance were not measured**.
- [Integration run 43443263708](https://github.com/MousaXD/scriptotar/actions/runs/43443263708) did **NOT pass** on c0ee54b: `cargo audit` failed with three lockfile vulnerabilities. Filed [issue #111](https://github.com/MousaXD/scriptotar/issues/111) for quinn-proto 0.11.18 (RUSTSEC-2026-0524 and -0523) and quinn-udp 0.5.15 (RUSTSEC-2026-0538). Independent release gate stays NO-GO; dependency owner must assess safe remediation. [Issue #107](https://github.com/MousaXD/scriptotar/issues/107) tracks exact historical Library artifact selection.
- A subsequent QA-only hardening commit requires a truly empty schema-migrated scratch template, rejects existing settings/private records, checks copied snapshots, and validates native evidence metadata structurally. A final QA-only extension samples descendant-process summed RSS under Linux with the caveat that shared memory may be overcounted. No measured native performance is asserted. This handoff applies to the PR branch, not main.
- Agent 00 must use the **final PR SHA**, not any older passing SHA, to decide whether this QA PR is ready to review. Separate product/integrated release certification remains NO-GO.
