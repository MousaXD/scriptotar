# Defragmentation implementation roadmap — P0 → P1 → P2 → P3

**State at creation (2026-11-13):** all checkboxes below are planned; none are claimed implemented or certified by this branch. Planning-only changes MUST NOT be mistaken for release readiness. **Base:** main@69e60f91c0b00560332dac06b732fc2259c1d632.

## How to execute

- Work in phase order. Each phase is a gate, not permission to implement everything at once.
- Begin with **P0-00** current-head re-audit: confirm what the previous 1.4.x integration already fixed. Historical docs may describe obsolete flaws.
- Split independent work after agreeing on one schema/API contract. Agent 00 owns interfaces/integration, migrations review and merge/release decisions; specialized agents own disjoint file areas. QA reviews across boundaries but should not quietly change production code.
- Each task requires a PR, named owner, fixtures, EN/AR and security regression notes where applicable. Completion means tested functionality, not merely files or screenshots.
- Prioritize a thin vertical slice before breadth: create study → attach one existing transcript → reopen saved draft → attach evidence → export.
- No modifications to main until code review and CI gate. This planning branch may remain unmerged.

## P0 — Unify existing features, make work durable (release-blocking foundation)

**Goal:** the user can move Research → Transcript → Studio → Library without re-entering context or losing artifacts. Not yet autonomous.

### P0-00: Fresh baseline and contract inventory [first task]
- [ ] Verify main HEAD, 1.4.1 release/tag and current open PRs; record exact versions. Do not assume old audits remain current.
- [ ] Map feature flows and source IDs in App.svelte, API client/tauriClient, DTO, services, DB, jobs/recovery, AI Studio, Research, Transcript, Library and MCP.
- [ ] Run/preserve installed-app and fixture validation where possible. Measure cold/warm startup, time to useful workspace, 100/1,000/10,000-record synthetic DB load, IPC payload size, memory and active-job UI smoothness. Record hardware and method; never invent measurements.
- [ ] Create a baseline feature parity matrix for existing imports, URL/video acquisition, watchlists, transcript formats, Copy Prompt/BYOK, EN/AR/RTL and MCP. Update out-of-date architecture references.
**Done when:** a current-head evidence report and regression baseline exist; later tasks have specific owners and shared types.

### P0-01: Canonical Content Study + provenance schema (P0 contract owner)
- [ ] Design typed Study, StudySource, EvidenceRef, Brief, Draft, DraftVersion and related status fields; link back to existing projects/research/media/transcripts/ai_runs, never title matching.
- [ ] Add *additive* SQLite migrations with foreign keys, project-membership triggers/queries, sensible indexes and migration/reopen/rollback tests; keep existing data readable.
- [ ] Define provenance for manual pasted notes separately; include source locator, source kind, text fingerprint/revision, segment/timestamp or document excerpt, collection time and uncertainty.
- [ ] Keep source-to-job, job-to-transcript and study-to-draft relationships explicit. No fake data hydration. Plan deletion/archival behavior before migration.
**Done when:** a newly created study and linked sources survive restart, cannot cross projects, and legacy 1.4.1 data opens unchanged.

### P0-02: Source/artifact detail endpoints + exact navigation
- [ ] Add getAiRun(id) detail/restore and getResearchItem(id) or equivalent project-scoped APIs; correctly reopen selected historical AI output from Library.
- [ ] Add deep links from Research/Library/Jobs/Transcript to exact selected IDs; research selection passes explicit IDs to Studio instead of manual re-selection.
- [ ] Clarify Copy Prompt history: the UI currently calls buildAiPrompt rather than the backend copy-mode persistence path. Offer intentional Save prompt/draft, never silently persist secrets.
- [ ] Keep existing direct research, transcript and queue tools available for power users.
**Done when:** opening an item always displays that exact item after switching views/projects and errors clearly if missing; wrong-project IDs are denied.

### P0-03: Persistent creative brief + editable versioned drafts
- [ ] Brief: goal, platform, audience, language, region, tone, duration, objective, CTA, constraints, sources. Support manual editing without an API key.
- [ ] Autosave/debounced persistence with dirty/error state, optimistic concurrency/revisions, crash/restart recovery and explicit discard; preserve data when switching views.
- [ ] Create/open/rename/duplicate/version drafts; searchable list; TXT/Markdown exports first. Script duration is labeled estimate, not exact.
- [ ] Make draft ownership and source attachments explicit; protect unsent or unsaved BYOK inputs.
**Done when:** the editor can be closed/reopened without data loss or stale writes; drafts have immutable version history.

### P0-04: First unified Studio shell and contextual work
- [ ] Make Content Study the primary page: brief, source tray, findings/evidence area, editable draft and an activity/next-action region.
- [ ] Start with simple hand-directed actions: Add sources, Queue selected, Read transcript, Analyze selected, Write draft, Save/Export.
- [ ] Convert Queue to an app-wide activity drawer/badge without deleting full queue functionality. Keep errors and health/status visible in context.
- [ ] Remove UI-only cross-tab copy/paste; use store-backed study IDs and typed queries. Use stable accessibility labels, responsive layout and EN/AR/RTL from day one.
**Done when:** a user can finish the existing manual workflow entirely in Studio, including interruption/restart.

### P0-05: Boundary, security and regression gates
- [ ] Update ScriptotarApi/Tauri IPC/mock contracts together; avoid raw invoke or SQLite calls in Svelte.
- [ ] Preserve HTTPS and loopback-specific AI endpoint restrictions, bounded response sizes, credential redaction, no persisted API keys and MCP least privilege.
- [ ] Treat social captions/transcripts, comments and web pages as untrusted tool data; enforce source delimiting and never interpret them as agent commands.
- [ ] Add cross-project ID-mismatch, stale async completion, duplicate source, concurrent edits, cancellation, SQLite migration, EN/AR/RTL and job-restart tests.
- [ ] Preserve prior formats/export and installed-package canary behavior.
**P0 exit:** verified end-to-end (no AI provider required): project → study → multi-source selection → one transcript → draft → export → quit/reopen. All provenance and security assertions pass.

## P1 — Deliver bounded agentic research-to-script flow

**Goal:** user sets a goal; the app proposes a plan, executes approved steps using existing tools, verifies claims, creates editable original scripts, and resumes after disruption.

### P1-01: Durable agent coordinator / tool registry
- [ ] Implement a minimal Rust scriptotar-agent domain/application crate or similarly isolated service after P0 contracts; do not put orchestration in a Svelte component.
- [ ] Define a typed state machine: planned → awaiting_approval → running → blocked/needs_input → completed/failed/cancelled; persist per-step checkpoint, tool outputs and idempotency.
- [ ] Tool registry wraps allowed existing services: research scan/list, queue/enqueue, job/read transcript, evidence retrieval, prompt/generate, save draft. No arbitrary shell execution.
- [ ] Explicit API cost/token/tool-call/time limits, request preview, approval and stop controls. Expose progress events with polling reconciliation. Bound retries and stop on denial.
**Done when:** a paused/interrupted run resumes safely without repeated paid requests or duplicate jobs and is inspectable by the user.

### P1-02: Multi-source study ingestion and retrieval
- [ ] Select multiple research items/transcripts/video URLs/manual text and independently credible factual sources (documents/articles where legally and technically available).
- [ ] Normalize/deduplicate by durable canonical IDs/content hashes; on-demand transcript retrieval and bounded source excerpts. Do not concatenate all transcripts into an unlimited prompt.
- [ ] Preserve evidence locator and collected-at timestamp; show data collection/access limitations, missing metrics and source rights.
- [ ] Separate facts from creative-pattern observations; independent corroboration for factual assertions. Clear fallback when a source denies access.
**Done when:** the same study can use >=3 sources; every reported fact points to a real selected source or is marked unverified.

### P1-03: Content pattern analysis (no fake viral scoring)
- [ ] Extract structural beats, hook category, payoff, pacing, opening promise, CTA and transcript-backed rhetorical devices; store structured observations with timecodes where possible.
- [ ] Label transcript-only guesses versus verified visual claims; do not claim frame analysis until frame sampling is built.
- [ ] Compare several videos and provide common patterns and counterexamples without concluding causal performance.
**Done when:** analysis reports are reproducible, inspectable and appropriately uncertain about missing video frames/retention data.

### P1-04: Claim verification and safety review
- [ ] Fact-vs-opinion classification, citation extraction, contradiction/recency flags and human review for high-stakes claims (e.g., law, finance, health).
- [ ] Untrusted-source instruction isolation and prompt injection regression fixtures; explicit provider consent for data egress.
- [ ] Abstain or flag when support is insufficient; limit verbatim copyrighted excerpts and avoid identity/voice impersonation.
**Done when:** fabricated/unsupported claims cannot silently be rendered as verified; citation UI opens exact source segments/docs.

### P1-05: Original script authoring loop
- [ ] Create at least three distinct candidate angles/hooks, brief-driven script beats, on-screen text, captions, shot/B-roll suggestions and optional CTA.
- [ ] Evaluate duration, claims, novelty, evidence sufficiency, audience and style constraints; offer targeted revision without rewriting accepted sections.
- [ ] Save each candidate and revision as real draft versions; attach cited evidence to factual lines and allow manual editing/override.
- [ ] Keep provider choice/BYOK and Copy Prompt mode; no silent paid LLM call. Degrade gracefully when no provider selected.
**Done when:** user can produce, revise, compare and export three original scripts from a studied topic with transparent provenance.

### P1-06: End-to-end agent validation
- [ ] Offline deterministic fixtures for planning, tool restrictions, retries, API failure, user cancellation, corrupt source, project switch and restart recovery.
- [ ] Real media/provider checks only when explicitly enabled and access succeeds; separate fixture green from real-provider certification.
- [ ] Include Arabic writing, RTL mixed content, citations, keyboard path and installed Windows/Linux tests.
**P1 exit:** a single approved agent run completes research → evidence → scripts and can safely resume/retry. No credentials leak, skipped steps or fabricated 'completed' results.

## P2 — Differentiate and scale (not prerequisites for P0/P1)

### P2-01: Trend intelligence based on observations
- [ ] Store time-stamped metric snapshots (views/likes/comments) with provider/source timestamp, uncertainty and missing-value handling.
- [ ] Compare videos against creator-specific baselines, account size and age when defensible; flag selection bias, delayed stats and missing denominator.
- [ ] Trend/velocity estimates only with >=2 comparable time points and confidence labels; never infer retention/causation from views alone.
- [ ] Ranking UI should be explainable, not a mysterious 'virality score'.
### P2-02: Optional visual understanding
- [ ] Rights-respecting frame sampling, on-screen text extraction, shot/scene detection and visual evidence timestamps where legally accessible.
- [ ] Cross-modal summary distinguishes spoken facts and visual claims; user can audit every observation.
- [ ] Respect battery/storage/VRAM, local processing preferences and source permissions.
### P2-03: Creator-specific profiles and knowledge
- [ ] Brand style guide (user-authored or user-owned samples), vocabulary, bilingual dialect, forbidden claims, preferred hooks, audience and channel targets.
- [ ] Reuse trusted findings across studies only with explicit selection and appropriate project isolation.
- [ ] Support optional local/offline AI through a tested provider interface, clearly labeled hardware/resource requirements; no pretense of a working model before integration.
### P2-04: Real runtime scale/performance
- [ ] Query-shaped/paged DB APIs for research/library/AI/jobs, lazy tabs, incremental event updates, indexes profiled with EXPLAIN and cancellation/backpressure.
- [ ] Deduplicate downloads and cached transcripts/analysis by source fingerprint while respecting access rights and project boundaries.
- [ ] Resource-aware download/Whisper/AI scheduling, CPU/GPU/RAM budgets and safe cancellation/recovery; preserve serialized path fallback.
- [ ] Benchmark real installed Tauri startup, time to useful app, 10k+ item search, editor scroll, IPC size and RAM. Keep pre/post traceable.
### P2-05: Careful workflow automation
- [ ] In-app watchlist alerting, scheduled study refresh and new-source suggestions with explicit opt-in and budgets.
- [ ] Queue priority and batch imports, duplicate detection, source availability diagnostics, optional project templates.
- [ ] No claim of unattended operation while a desktop app is closed unless a separately designed service exists; no silent auto-posting.
**P2 exit:** measurable gains on supported hardware and defensible research intelligence; platform failures and privacy boundaries remain explicit.

## P3 — Final polish, text reduction, cross-platform excellence

### P3-01: Remove redundant copy and low-value UI clutter
- [ ] Audit every view; remove duplicate descriptions, generic explanatory cards, repeated metadata labels and unnecessary helper text.
- [ ] Use concise primary actions, informative empty states and contextual help only when needed. Replace tool-centric jargon with creator-centric language.
- [ ] Preserve security/privacy notices, destructive confirmations, progress, error details, accessibility descriptions and actionable source uncertainty.
- [ ] Maintain a before/after copy inventory in English and Arabic with screenshots and user task completion review.
### P3-02: Coherent visual hierarchy and navigation
- [ ] Finalize Home / Discover / Studio / Library / Settings; keep expert actions reachable through contextual panels and command palette.
- [ ] Consolidate queue/progress into global activity drawer; progressive disclosure for technical details, never hide failures.
- [ ] Keep Studio draft central, research/evidence beside it and agent activity unobtrusive. Design for narrow desktop windows first.
### P3-03: Controls and visual consistency
- [ ] Audit for accidental OS-native select/range/check/switch fallback across all views and dialogs; reuse already shipped custom components rather than reinventing.
- [ ] Make custom components accessible via keyboard, focus rings, screen reader roles/labels and touch target sizing.
- [ ] Audit typography, spacing, icons, loading skeletons, empty states, animation restraint and both explicit/system appearance settings.
### P3-04: Arabic, RTL and accessibility
- [ ] Remove unjustified data-i18n-ignore wrappers around actual English UI labels; 100% applicable key parity and UI localization.
- [ ] Bidirectional-safe file paths, URL/timecodes/model names, numeral/duration formatting, Arabic script and code-switching.
- [ ] Automated desktop QA across EN/AR, theme/width, modal focus, accessible controls, reduced motion and 200% zoom; human screenshot review on real installs.
### P3-05: Distribution, regression and documentation
- [ ] Compare all pre-existing feature parity tests, local-media downloads/transcription, migration/rollback, MCP auth, security, installer functionality and recovery.
- [ ] Complete independent native perf and visual certification tracked in #100; follow the current release policy rather than copying stale 1.4.0 flags.
- [ ] Update product onboarding/README to explain studies and evidence-driven writing without exposing implementation detail to ordinary users.
**P3 exit:** no significant regression, concise coherent UX, validated EN/AR + keyboard/RTL and actual installed-app acceptance. Release only after exact-head CI and separate explicit release approval.

## Additional improvements discovered during this 1.4.1 source inspection

| Finding (not all bugs) | Impact | Tracked in |
| --- | --- | --- |
| Library AI entry navigates to AI Studio but does not fetch exact saved result; AI runs store prompt/result in SQLite | History is not genuinely reusable | P0-02 |
| Library Research entry navigates to Research without selecting the item | Context drops at feature boundaries | P0-02 |
| AI Studio source selection is one transcript or pasted text; DTO has source_text but no source IDs | No multi-source provenance despite local selection checks | P0-01, P1-02 |
| AI Copy Prompt UI only builds prompt; backend has a persist-capable copy-mode run | History/save semantics unclear | P0-02 |
| Existing DB analyses table has a transcript reference, but no Content Study / DraftVersion data model | Need connected content artifacts, not just another prompt pane | P0-01 |
| Summary bootstrap removes large transcript bodies but still bundles all active-project metadata and historical AI runs | Large workspaces can still cause eager IPC/SQL cost | P2-04 |
| Research stats are observational, not creator-normalized or historical | Can't honestly label a hit or growth without better benchmark and timestamps | P2-01 |
| Research scanning uses supported platform yt-dlp access; source blocks are environment-dependent | An agent must handle denial, opt-in auth and partial sources | P1-02 |
| Large amounts of UI copy / some data-i18n-ignore regions remain | Language and clutter debt despite prior GUI overhaul | P3-01, P3-04 |
| Existing job recovery can reuse prepared media, but interrupted Whisper inference restarts | Scheduling and caching must preserve true recovery semantics | P2-04 |

Re-check each row at P0-00 against newest HEAD before implementing; prior audit docs from older branches are valuable hypotheses, not authoritative current-state claims.

## Work ownership after the contract freeze (suggested, not assigned)

1. Agent 00 — integration / contract freeze / dependency graph / PR reviews and final go-no-go.
2. Domain + DB — P0-01, P0-03 data migrations / lineage / backward compatibility.
3. Rust host/API — P0-02 endpoint implementation and Svelte seam, no independent DB migration.
4. Studio frontend — P0-04 layout/editor and route selection after API contract.
5. Research & evidence — P1-02/P1-03 ingest, provenance, metrics.
6. Agent runtime — P1-01 orchestration and bounded tool registry.
7. Writing quality — P1-04/P1-05 prompts, revision/evidence and outputs.
8. Independent QA/perf — test harnesses, integration regression and exact-head certification.

Do not run all eight against mutable shared App.svelte, services.rs, types.ts or database migrations. Agree on interfaces first, and give shared files one integration owner. Later P2/P3 may use fresh assignments.

## Required CI and verification

Use exact source SHA; inspect workflows for changed prerequisites:
- npm ci --prefix apps/desktop-ui && npm run check --prefix apps/desktop-ui && npm test --prefix apps/desktop-ui && npm run build --prefix apps/desktop-ui
- cargo fmt --all -- --check; cargo check --workspace --all-targets --locked; cargo clippy --workspace --all-targets --all-features --locked -- -D warnings; cargo test --workspace --locked
- python3 -m unittest discover -s tests -p 'test_*.py' -v
- PYTHONPATH=sidecars/transcription python3 -m unittest discover -s sidecars/transcription/tests -v
- Installed Windows/Linux smoke + MCP E2E + migration / SQLite security + real model/provider canaries where enabled.

No fake PR passes, no removing mandatory gates, no credentials in logs, no secrets or source content in new analytics by default.
