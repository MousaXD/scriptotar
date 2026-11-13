# Continuation checkpoint — start here in the next chat

**Prepared:** 2026-11-13 UTC+04 (planning task; no application code changed).
**Repo:** https://github.com/MousaXD/scriptotar
**Working branch:** plan/scriptotar-defragmentation
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
