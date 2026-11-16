# CHECKPOINT — 2026-11-16 Agent 00 begins actual P1 integration (candidate; NOT release ready)

- The integration branch was fast-forwarded from `3009989a3aa73248b6432ba0cd84821c59110993` to Agent 02 PR #119 head `9da21c96a62d7a041b64500841b89dc6dc649703` (25 commits, nine affected files), incorporating the canonical additive SQLite v10 external-effect migration, host-private effect driver and corresponding tests. No main merge or paid provider request is enabled.
- Exact-head Agent 02 Rust foundation run `43876690200` passed rustfmt, workspace check, Clippy, full workspace tests, historical v5 upgrade and external-effect crash/concurrency regressions. Other workflows and the combined branch need their own exact-head results.
- Agent 03 PR #120 research source and durable scoped artifact adapter were brought onto the integration branch and privately registered in `services.rs` at `9d3825efa07be78b2e4f80c66e1a79394d746a37`. This compiles the host path for CI but does not expose the user-visible analysis run/read flow or claim it tested.
- See `P1_INTEGRATION_CONTRACT_20261116.md` for owner-owned provider/effect/Study revision/provenance/writing interface gates. SEC-10-01 pre-egress consent revocation race is unresolved; actual paid dispatch stays disabled. PR #122 remains excluded.
- Remaining: full combined CI, consent epoch linearization, production research IPC/approval/reopen, Agent 04 verified-source binding, Agent 05 v11 atomic three-draft persistence, Studio UI/mock/API integration, independent adversarial and installed-native-WebView/performance acceptance. Keep PR #114 draft and main unchanged.

---

# CHECKPOINT — 2026-11-15 pasted structured script → isolated candidate (candidate)

- Added a **local-only, two-stage** ScriptDraft review/explicit acceptance flow following the already approved three-angle JSON. The service re-derives the active Study brief/attached sources/observations, requires exact earlier angle and draft prompts and Study revision, and uses Agent 06's strict `parse_angles` and `parse_draft` before any write. Unsupported factual claims are rejected; remaining model claims are prominently marked for independent review. There is NO provider egress, automatic paid call or claim that JSON validation constitutes fact checking.
- A successful admission also consumes the expected **Study revision atomically**. Replaying the same confirmed request after a lost/duplicated reply now fails instead of silently creating another script. Reopen the Study to reconcile whether it saved; no automatic retries or paid effects. Native/TS regressions cover this denial.
- The acceptance transaction creates an entirely new `creative_drafts` row, its immutable first `draft_versions` entry and links to the same active canonical Study sources, guarded by an IMMEDIATE SQLite transaction and expected Study revision. Original user drafts are not modified. The candidate is editable and can be archived/restored using existing Studio controls. Input/size and max-200 active-candidate bounds apply.
- New typed Rust/Tauri/TS/mock EN/AR flows and tests are pending exact-head validation. **Not done until nine workflows pass.** Structured source links are **context provenance** only, not claim-level verification, a full immutable AI operation ledger, independently verified originality, or a finished three-script automatic agent. Native graphical installed-app and performance certification remain open; keep PR #114 draft and main unchanged.

---

# CHECKPOINT — 2026-11-15 local three-angle response review (candidate)

- A new project-scoped `interpret_study_angles` Tauri endpoint consumes user-pasted response JSON, re-derives the authorized active Study's full bounded writing context, compares the **exact prepared prompt** and Study revision, and runs Agent 06's strict `parse_angles`. Unknown/duplicate source citations, malformed or non-three-angle outputs, other projects and stale/disputed observations are refused without network calls.
- Studio EN/AR displays the three validated creative proposals and lets users select one; the same typed writing core then prepares a local one-script JSON prompt for Copy Prompt. No automatic AI/provider egress, stored model output, draft mutation or claims of independent fact checking. This provides a safe explicit bridge from three-angle prompt to choosing a single script direction, not automatic script generation.
- Added native Rust stale-context/evidence-dispute and wrong-project tests, TS/mock IPC parity, UI end-to-end response validation and no-provider-call assertions. All nine exact-head workflows are REQUIRED before counting this slice certified. PR #114 remains DRAFT/NO-GO, main unchanged; provider approval/effect fencing, three generated persisted scripts, installed graphical WebView certification and 100/1k/10k performance remain blockers.

---

# CHECKPOINT — 2026-11-15 bounded 3-angle request bridge (candidate)

- P1-05: Canonical Studio now exposes **Prepare 3 creative angles** in EN/AR, using Agent 06's typed `prepare_writing_request` with the actual active Study brief, up to eight project-scoped attached sources, literal bounded available transcript excerpts (sampled opening/middle/closing with real segment indices/timestamps where present) and user-created observations marked unverified. Disputed observations are excluded. Research metadata alone is NOT treated as transcript/video fact evidence; limitations are explicit. This only creates a copyable local prompt and makes **zero provider calls**—it does not produce the three angles or scripts, verify claims or enter an autonomous run.
- The eight-source bound applies to both transcript excerpts and user observations; each timestamped excerpt now carries the actual segment ID and start/end seconds (rather than only an array position). A real JSON-segment fixture exercises that provenance, with no fabricated visual or retention claims.
- Existing writing-core prompts isolate untrusted source text. Scope checks deny other projects; native Rust and mocked UI tests cover real transcript inclusion, disputed exclusion, source warnings, explicit Copy Prompt and no `runAi` call. Tauri/client/mock contract and command inventories updated together.
- Exact-head nine-workflow CI and independent native graphical/performance certification remain mandatory. **Do not merge #114 or publish from this branch yet.** Next P1 implementation must parse/validate angles and draft outputs, require explicit provider egress/token/US$ limits with durable effect fencing, then save three separate canonical draft candidates with lineage, not just print a prompt.

---

# CHECKPOINT — 2026-11-15 independent Script candidates (candidate)

- Users can now **archive/restore** draft candidates in EN/AR with explicit confirmation, persistent archived-candidate metadata, exact revision CAS and immutable status version history. The final active script cannot be archived, and archived bodies are never hard-deleted by this workflow. Restart, stale revision, cross-project and last-active denial are covered.
- Users can now **rename** selected draft candidates in EN/AR. The rename operation is now atomic: an IMMEDIATE transaction reads the current saved body, compare-and-swaps title/revision, and appends an immutable version without replaying a stale body. Stale/wrong-project/wrong-Study renames fail; a no-op title retains revision.
- The canonical creative_drafts table already supported multiple drafts, but Studio read/saves always picked the most recently updated draft. Studio now exposes selectable draft candidates, explicit duplication, and separate immutable version histories per candidate, with EN/AR controls. The selected draft UUID and expected revision travel through typed Tauri/TS/mock APIs; SQLite's IMMEDIATE transaction compare-and-swaps both the Study and selected draft, preventing another candidate's content from being overwritten. Creating a duplicate snapshots the source content and draft-source links atomically under its own source revision guard. No additive migration or AI provider requests.
- The draft selector now fetches up to 200 **metadata-only** rows rather than up to 200 full script bodies. Only the selected draft body and version history are hydrated, with a revision consistency check and a large-draft fixture. This avoids multiplying memory/IPC costs as candidates accumulate.
- Switching candidates first saves pending edits and clears the old draft-version preview. New native and mocked tests cover stale revisions, wrong project/Study, copy and edit isolation, and restart. ScriptotarApi/Tauri command parity inventory updated for duplicate_study_draft.
- **Candidate only until nine exact-head workflows pass.** This is hand-edited multi-candidate support, NOT Agent 05/06 three original scripts, independent factual verification, independent final verification, provider consent/effect budget, or real installed WebView native certification. Draft selector lists at most 200 recent active drafts and reopen defaults to most recently updated draft; a user can explicitly select any listed candidate. Keep PR #114 draft; main unchanged.

---

# CHECKPOINT — 2026-11-15 evidence review history (candidate)

- Human unverified↔disputed transitions now append an immutable SQLite v9 review event in the **same IMMEDIATE transaction** as the evidence status compare-and-swap. Failed/stale/foreign-project reviews cannot create events. The latest 50 are returned with each project-scoped evidence read and displayed in a collapsible EN/AR Studio review timeline; no new network or paid AI permissions.
- Prior v6–v8 evidence reviews cannot be reconstructed: migrated databases begin recording review history at v9. Event rows are guarded against cross-study insertion and ordinary update/delete, with cascade cleanup on project deletion. The underlying database retains all events; the UI response includes the last 50 only.
- Includes Rust service tests for persisted history, cross-project denial, stale CAS and direct SQL mutation refusal; mock/app UI tests assert history after reopening an observation. **This candidate is not verified until all nine exact-head Actions workflows pass.**
- Still outstanding: independent verification, multiple drafts per Study, Agent 05/06 original-script orchestration with provider approval/effects, native installed graphical WebView EN/AR + 100/1k/10k perf, P3 polish. Keep PR #114 draft and main unchanged.

---

# CHECKPOINT — 2026-11-15 evidence review and prompt quarantine (candidate)

- New project-/Study-scoped `review_study_evidence` Tauri command and SQLite IMMEDIATE transaction let a human mark an unverified claim **disputed**, or return a disputed item to **unverified**, with a mandatory explanatory note and optimistic status compare-and-swap. Neither transition creates a verified/corroborated assertion. The original claim, exact source and locator remain visible.
- Studio EN/AR inline controls display the state and reason, preserve evidence across restart and invalidate local prompts during review. The evidence-to-Copy-Prompt compiler skips disputed claims even when their source is still attached.
- Typed IPC/mock parity, source isolation, stale review, persisted status/reopen and no-provider regression tests are added. Update test_api_surface_contract inventory for the new IPC surface. This **candidate must pass all nine checks at its exact commit** before it is counted finished.
- Limitations: this is a creator review marker, **not independent fact checking or an immutable review audit log**. Further evidence editing, first-party source verification, Agent 05/06 writing orchestration, paid-call consent/ledger, installed graphical QA and native perf certification remain on #116/#100. Do not merge or release; main unchanged.

---

# CONTINUATION — 2026-11-15 human-controlled historical AI output → versioned Studio draft (candidate)

- Reopened **exact saved AI-run** results (not Copy Prompt-only runs, which have `result: null`) can now be copied into the active Content Study's draft only after an explicit confirmation. The old working script is saved first, and the new content goes through the existing revision-checked atomic Study-save path, creating a fresh immutable draft version rather than overwriting historical versions.
- Only project-scoped, server-validated historical artifacts are eligible. Import is guarded against project switches, in-flight saves and stale history responses; a failed write keeps the unsaved working text visible for retry.
- Added EN/AR labels and a frontend regression that verifies no changes before confirmation, exact imported output/version history, Copy Prompt refusal and reopen after app remount.
- **This does not persist an independent AI-run provenance attachment to the new draft, verify generated claims, or generate new model output.** Full multi-draft provenance UX, P1 consented agent generation, native graphical WebView evidence, P2 perf and P3 polish remain open. Require all nine checks on whichever commit ultimately advances the integration ref; never describe this orphan Git object as integrated before the ref is advanced.
- Keep PR #114 draft, main unchanged, no release.

---

# CONTINUATION — 2026-11-15 canonical evidence → local Copy Prompt candidate

- Added a bounded, project-/Study-validated formatter for the existing persisted evidence references. The Studio local **Prepare writing prompt** path now fetches canonical evidence *after* pending Study saves, includes exact origin IDs/locators, explicitly labels creator observations as **not independently fact-checked**, and omits unavailable/deselected origin material. It never automatically makes an AI provider call or persists API credentials.
- Evidence changes, Study changes and source/brief edits invalidate pending prompt preparations so an old async response cannot silently overwrite newer context. Evidence is prioritized in the local 30k-character context budget; max 16 saved observations, each bounded.
- Unit and UI regressions cover source scoping, foreign-project rejection, truncation, exact-source references and zero paid calls.
- **This is not automated writing or factual verification.** Full Agent 05/06 multi-source insights, paid-provider consent + idempotent effect fencing, revision orchestration, native graphical QA, scale benchmarks and P3 polish still block release. Confirm exact-head nine-workflow CI before marking this candidate green.
- Integration PR #114 remains draft and main stays unchanged.

---

# CURRENT CHECKPOINT — 2026-11-15 Evidence slice (candidate, exact CI required)

- Integration PR #114 includes end-to-end **human-entered evidence observation** APIs and Studio controls. Rust now maps an attached Research/Transcript origin to its canonical StudySource UUID, appends the observation under the active project/Study, and always assigns **unverified** certainty. User cannot arbitrarily promote a saved claim to "corroborated".
- New commands `list_study_evidence` and `add_study_evidence`, typed TS/Tauri/mock parity and English/Arabic Studio panel. Linked source references can be reopened after navigation/restart; an evidence-linked source is not silently detached by the UI. Other canonical source kinds retain locator-only display.
- Independent Rust app-service regression now covers project mismatch, unattached origin rejection, protected source detachment and evidence rehydration from disk. Frontend regression covers exact source, explicit locator, default uncertainty, project denial and reopen. **Do not claim success until all nine exact-HEAD CI workflows have passed.**
- This is **not** independent factual verification, a real graphical WebView acceptance or a working provider research/script-writing agent. Next work remains issue #116 (Agent 05/06 integration), real provider consent/effect-fencing, complete P0 editor features and issue #100 native certification.
- Keep the PR **draft**, main untouched, no public release.

---

# CHECKPOINT — 2026-11-15, exact-head CI audit and draft-history work

- Reviewed integration PR #114 at `fc5caf2384a8def6c0a660ec5299224b00c86acb`: **all nine GitHub Actions workflows succeeded for that exact SHA** (Rust foundation, CI, MCP E2E, Desktop UI, security hygiene, GUI visual/perf QA, Tauri migration, Linux Tauri Package and Windows Tauri Installer). This certifies only the pre-restore source candidate, not live installed graphical/WebView operation or full P1.
- Rechecked issue #115 against `studies_atomic.rs` and `study_gateway.rs`: the integration branch already uses a single IMMEDIATE transaction with Study CAS, brief/source/draft changes and immutable versions. The issue description remains historical until merge; do not reimplement this work.
- New follow-up: Studio has a safe **Restore as new version** action for previous script snapshots, in EN/AR. It writes through the existing project-scoped atomic save path, preserves prior versions and tests reopen after restore. **This new commit requires fresh exact-HEAD CI; no pass is claimed yet.**
- Still outstanding P0: user-facing evidence references/verification, multiple drafts per study and duplication, safe staged delete/undo, independent real installed Windows/Linux graphical WebView E2E, real EN/AR/RTL visual QA and native 100/1k/10k startup/RSS/IPC measures.
- Still outstanding P1: approved real research/tool chain, factual evidence reconciliation, Agent 05 intelligence and Agent 06 original multiple-script generation wired to canonical sources/drafts, bounded provider budgets/consent, cancellation, idempotent effects for **each** new tool and crash-safe recovery. Only the local transcript read is connected and ledger-fenced.
- P2/P3 remain substantive: credible time-series metric intelligence, hardware-measured performance optimization, coherent navigation, redundant-text removal, custom control and locale accessibility pass.
- **Integration branch only. Keep PR #114 draft, main unchanged and no public release until independent evidence and explicit merge/release decision.**

---

# LATEST CHECKPOINT — P1 local agent effect fencing (2026-11-13)

- **Owner branch:** integration/scriptotar-defragmentation-agent00-20261113; **draft PR:** https://github.com/MousaXD/scriptotar/pull/114. Main remains unchanged.
- **Last verified all-green exact SHA:** c21016939b55253c4d249d496b7f31fa72c9d5d0, with nine green workflows including SQLite v8 agent-effects tests and Linux/Windows packaging. This precedes the new live effect adapter.
- **New candidate work:** the approved, zero-cost ReadStudyTranscript production tool reserves a v8 effect key only after the agent Engine has committed its Running-intent checkpoint. It records a durable Applied or NotApplied receipt before reporting completion. Existing claims never permit duplicate dispatch; missing or ambiguous evidence blocks.
- **Recovery:** a user-initiated reconciliation probes the persisted effect and only re-inspects this specifically repeatable *local* read. This is not permission for network, paid or mutating operations. Tests inject a claim failure and restart a reserved read; denial leaves no ledger claim.
- **Resource safety:** the scoped SQLite transcript-attachment and UTF-8 byte-length query avoids loading arbitrary-sized bodies for tool admission; oversized text is refused before a new run.
- **Not complete:** permitted external research tools, independent factual evidence verification, original script generation using Agent 06, real provider-cost/approval policy, native graphical WebView acceptance/performance, and P3 UI consolidation. The app is not a full production autonomous researcher/script writer.
- **Verification:** retrieve current PR #114 HEAD and require all nine workflows at that *new* SHA. Do not present the c210 green baseline as validation for later changes. Draft/NO-GO for merge and release.

---
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
