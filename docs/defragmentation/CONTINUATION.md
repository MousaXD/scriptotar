# LATEST CHECKPOINT — P1 local inspection integration (2026-11-13)

- **Owner branch:** `integration/scriptotar-defragmentation-agent00-20261113`; **draft PR:** https://github.com/MousaXD/scriptotar/pull/114; `main` remains unchanged. Retrieve and verify the actual current HEAD before continuing; do not infer success from this document.
- **Earlier fully passing baseline:** `218fc97c101da7993c133587c88353dee7319034` passed all nine CI/package workflows for durable v7 checkpoint storage, but predates the new local inspection commands.
- **New production-facing vertical slice:** `read_study_transcript` is the only enabled agent tool. Users select an already-attached transcript in Studio, prepare a durable run, explicitly approve or deny its local read, and reopen exact run state after restart. Each dispatch rechecks active-project, Study and transcript ownership, has a strict byte/CPU/cost limit and sends **no network request**, calls **no AI provider**, and creates **no external side effect**.
- Tauri commands: `start_local_inspection`, `list_local_inspections`, `decide_local_inspection`. Typed ScriptotarApi/Tauri/mock bindings, EN/AR Studio controls and contract tests are included. Installed Linux mock-runtime IPC test now covers approval and two restart cycles using a real transcription artifact. Full graphical WebView acceptance is **not** proved by this mock-runtime test.
- **Scope strictly limited:** This is not yet autonomous research or AI script writing. External tools, factual verification, pay-per-call budget authorization, service-side idempotent-effect ledger, controlled publishing, and writing agent integration are **NOT enabled**. Do not claim P1 GO or release.
- **Next actions:** verify all nine workflows pass at the new exact head; then implement effect deduplication and user-visible tool budgets, wire Agent 05/06 to provenance-backed studies, certify real installed WebView journeys and native performance, and finish P3 professional UI de-cluttering/RTL without removing safety warnings.

---

# CURRENT CHECKPOINT — Agent 00 integration (2026-11-13)

**Use this checkpoint instead of the older planning-only state below.**

- Integration PR: https://github.com/MousaXD/scriptotar/pull/114 (draft)
- Integration branch: integration/scriptotar-defragmentation-agent00-20261113
- Main/released 1.4.1 remains untouched. No release authorization or release-certification claim.
- Seven scoped agent implementation branches have been incorporated without overwriting overlapping paths.
- P0 implementation now includes canonical v6 studies/sources/evidence/briefs/drafts, exact AI/research artifact retrieval, explicit saved Copy Prompt, source-to-Studio navigation, a project-scoped Tauri/TS Study gateway, revision-checked atomic SQLite Study creation/saves, pending-autosave project-switch guards, real-ID provenance/restart/rollback fixtures, and shallow Studio list queries.
- Security: quinn-proto 0.11.19 and quinn-udp 0.5.16 lockfile updates included; root scriptotar-agent crate version aligned to 1.4.1; no ignored audit findings.
- **Verified P0 integration baseline:** commit `22ce84ad3d0792282f6bf82af599967064dc8f8a` passed all nine workflows: Rust, CI, MCP, Desktop UI, security, GUI QA, Tauri migration, Linux installed-package persistence and Windows NSIS smoke. This does **not** certify real installed WebView UX or performance.
- **New P1 checkpoint candidate:** check PR #114 for the latest post-v7 commit and run exact-head verification again. Do not use the older green P0 SHA to certify this addition.
- P1 agent coordinator, research analytics and evidence-aware writing foundations remain **not a functioning production agent**. Agent 00 added SQLite migration v7 and project/study-scoped agent checkpoints with revision CAS, verified append-only audit, restart recovery and late-failure rollback. **Real AppServices tool drivers, service-side idempotent effects, paid provider permission gates, and user-facing run controls are still not wired.**
- P0 still lacks full evidence editing, native packaged WebView journey/restart certification, and independent native scale/latency/RSS evidence (issue #100). P1–P3 exit criteria remain unmet. Do not merge to main or publish yet.
- Exact acceptance still needed on the **post-v7 head**: all nine workflows succeed, full real installed WebView interactions on Windows and Linux, real multi-source Study-to-script journey with evidence, source access limitations, budget/approval tests, native perf 100/1k/10k fixture and EN/AR/RTL real UI validation.
- Read docs/defragmentation/handoffs/agent-00-integration.md for owner decisions and remaining blockers.

---

## Historical planning snapshot (superseded by the checkpoint above)

# Continuation checkpoint — start here in the next chat

**Prepared:** 2026-11-13 UTC+04 (planning task; no application code changed).
**Repo:** https://github.com/MousaXD/scriptotar
**Working branch:** plan/scriptotar-defragmentation
**Open draft planning PR:** https://github.com/MousaXD/scriptotar/pull/102 (do not merge merely because the plan exists)
**Base main SHA at branch creation:** 69e60f91c0b00560332dac06b732fc2259c1d632
**Docs:** [README](README.md) and [P0–P3 roadmap](ROADMAP_P0_P3.md)
**Current implementation status:** PLANNING COMPLETE ONLY; P0-00/P0-01 not implemented, no claims of new QA or released behavior.
**Merge policy:** leave main untouched until reviewed. No release authorization has been given for this plan.

## Why this work exists

Owner reports that the present 1.4.1 app works but feels fragmented: research, download/queue, transcripts, Library and AI Studio behave like individual features. The desired destination is one fully agentic content researcher and script writer that studies successful reels, checks factual sources, and creates original, referenced and editable scripts. Owner additionally wants performance optimization, P0/P1/P2/P3 execution order, final de-cluttering of verbose helper text and GUI polish.

### Latest verified architectural observations

- Existing seven view IDs: dashboard, research, jobs, transcript, ai, library, settings; Svelte App.svelte mounts one at a time.
- AI Studio accepts one explicitly selected transcript or pasted source text; its prompt tasks are fixed ("Viral breakdown", "Hook ideas", "New short-form script", "Structure remix", "Content ideas", "Caption + CTA", "Voice profile", "B-roll shot list"). BYOK supports OpenAI/Anthropic/Gemini/OpenAI-compatible; local AI is not available in UI.
- Existing AI_run schema persists prompt and result; details are not exposed by ScriptotarApi, only summaries. Library AI navigation doesn't restore result identity; Library research nav doesn't focus selected item.
- Backend's prompt builder currently concatenates source text with task instructions; untrusted-source delimiting + exact typed references are a cross-layer upgrade.
- Research uses yt-dlp-backed creator scans for supported social platforms and saves observational metrics; source access varies by machine.
- Existing on-demand transcript detail and summary bootstrap optimizations are already present. Do not redo the 1.4.0 overhaul.
- Rust crates, SQLite migration and sidecar job architecture must be preserved.
- Docs/TECHNICAL_FUTURES_AUDIT.md and docs/AGENT_06_AI_CREATOR_AUDIT.md contain additional context, but have older baselines; distinguish already fixed issues.
- Independent installed-app native perf/visual certification is tracked at https://github.com/MousaXD/scriptotar/issues/100; do not imply certification from mock frontend tests.

## Immediate next action in a new chat

1. Read **this file** then ROADMAP_P0_P3.md and current default-branch HEAD; compare the branch to main.
2. Begin **P0-00 fresh current-head verification + feature parity map**. Verify exact existing code/CI and existing migration revisions. Do not start by rebuilding frontend components.
3. Propose and freeze **P0-01 Content Study, StudySource, EvidenceRef, Brief, Draft/DraftVersion** entities and Rust/TS/SQLite/API contract; identify one integration owner for shared files.
4. Implement P0 as a narrow vertical slice with tests: create study → attach source → reopen exact artifact → persist draft → restart and restore. Keep the existing Research/Jobs/Transcript/AI functionality unchanged and reachable.
5. Then P1 agent coordinator/research/generation; P2 intelligence & measured optimization; P3 remove low-value text, improve custom controls, RTL, styling and distribution.
6. Update this checkpoint with branch HEAD, newly completed task IDs, exact CI results, blocked tasks and next action **after each merged-on-branch slice**. Avoid undocumented parallel changes.

## Prompt to paste into a new chat

Continue Scriptotar's defragmentation roadmap in https://github.com/MousaXD/scriptotar on branch plan/scriptotar-defragmentation. First read docs/defragmentation/CONTINUATION.md, README.md and ROADMAP_P0_P3.md. Verify GitHub HEAD/main and current code before changing anything. Agent 00 owns integration. Start at the first incomplete P0 task; do not redo completed 1.4.0 work. Keep main unchanged unless explicitly approved. Implement source-backed Content Studies and exact navigation with additive migrations, preserve all prior features, run exact-HEAD CI and update the checkpoint with verified progress. Priorities P0 then P1 then P2 then P3 (P3 includes GUI cleanup and unnecessary text removal).

## Task ledger

| Task range | Status | Next step |
| --- | --- | --- |
| P0-00 to P0-05 | Not started | P0-00 baseline evidence, then P0-01 typed contract |
| P1-01 to P1-06 | Not started | Wait for tested P0 exit |
| P2-01 to P2-05 | Not started | Wait for tested P1 exit |
| P3-01 to P3-05 | Not started | Final coherent UI, EN/AR/RTL, installed-app gate |

**Verification in this planning turn:** GitHub source/files and current main commit were inspected; docs only were authored. No local code compilation, automated screenshot QA, installed-app benchmark or media/provider run was performed as part of planning.
