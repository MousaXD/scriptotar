# Scriptotar Defragmentation — product and architecture direction

**Status:** planning only — no feature implementation or merge into main.
**Baseline:** main at commit 69e60f91c0b00560332dac06b732fc2259c1d632 (2026-11-12, Scriptotar Next 1.4.1 hotfix).
**Planning branch:** plan/scriptotar-defragmentation.
**Owner of integration:** Agent 00. This document is the durable start of a new product workstream.

Read in this order:
1. [ROADMAP_P0_P3.md](ROADMAP_P0_P3.md) — executable task backlog, dependencies, acceptance gates and ownership.
2. [CONTINUATION.md](CONTINUATION.md) — how to resume work in a new chat and what has and has not been done.
3. [Existing architecture](../architecture/rust-foundation.md), [technical futures audit](../TECHNICAL_FUTURES_AUDIT.md) and [release quality tracker #100](https://github.com/MousaXD/scriptotar/issues/100) — prior context, not automatically current truth.

## Product goal

Move from a set of adjacent utilities into **one local-first agentic content research and script-writing workspace**. A user states a creative goal; Scriptotar collects relevant content, studies promising short-form videos, corroborates factual claims with separate sources, and produces original, revisable scripts with evidence and production direction. Every step is inspectable, resumable and under user control.

**North-star user story:** “Study reels on this subject, explain which creative structures appear to work (without claiming causality), verify the claims, then write three original 40-second Arabic scripts for my audience with references and shot directions.”

The app should make that flow possible in one **Content Study** inside an existing project, not through manual cross-navigation and copy/paste.

## Working mental model

- **Project:** Existing project/security boundary. Never silently cross it.
- **Content Study:** Persistent goal, brief, selected source links, evidence, analysis, drafts and workflow history.
- **Source:** An explicit URL, local media, existing research item, transcript or factual document with identity/revision.
- **Observation:** A measured or observed property of a source, with timestamp and uncertainty, not a marketing promise.
- **Evidence / finding:** A claim, citation, confidence and source segment or document location, including disagreements.
- **Draft:** Editable script with versions, creative instructions, estimates, fact-review status and export.
- **Agent run:** A durable bounded execution plan with steps, tool inputs/outputs, approvals, cost/resource budget and checkpoints.

### Intended user journey

1. Enter a topic or creative goal, audience, platform, language, duration and style.
2. Discover/paste source videos and select the actual items to use; inspect available metrics.
3. Background acquisition/transcription (reuse existing queue, models, sidecar and recovery).
4. Review selected transcripts, content structures and independently collected factual evidence.
5. Receive a sourced content brief: patterns, opportunities, uncertainties and unsafe/unverified claims.
6. Generate several original draft alternatives; edit, compare, revise and save versions.
7. Export a recording-ready script, optional on-screen text/caption/CTA and an optional shot list.
8. Return tomorrow and reopen exactly the same study, evidence and draft versions.

Video popularity is a signal for choosing what to inspect, not proof of retention, virality causes or factual accuracy. Distinguish social-media creative evidence from independent factual evidence.

### Navigation objective

**Home · Discover · Studio · Library · Settings.** Existing Jobs/Queue becomes a global activity drawer; transcript reading and AI actions live in context in Studio, but remain reachable as tools. Do not remove essential expert workflows. Navigation reduction is an outcome of integration, not an early destructive rewrite.

### Existing strengths to preserve

Rust/Tauri/Svelte boundary, local SQLite/WAL, project scoping, staged transcript bootstrap, event-driven job notification with reconciliation, typed job lifecycle, persistent media recovery, Faster Whisper local inference, research/watchlists, English/Arabic/RTL, exports, secure session/request-time BYOK with Copy Prompt, MCP permission controls, migration compatibility and Windows/Linux packaging.

### Confirmed code-level friction at baseline

- AiStudioView selects one explicit transcript or pasted text; buildAiPrompt/runAi accepts a text blob, not typed multi-source evidence.
- Rust AI prompting is fixed task instructions plus source/brief, not a planner with tools/checkpoints.
- AI results are persisted in ai_runs, but ScriptotarApi exposes summary rather than a detail/reopen path; Library's AI run action navigates to the AI view without choosing the historical run.
- Library research/creator entries similarly navigate to Research without focusing the exact item.
- Copy Prompt UI builds a prompt without using the backend's persist-capable run_ai copy path: ensure history semantics are intentional and tested.
- A partial startup optimization exists (summary transcripts, on-demand getTranscript), but bootstrap still aggregates active-project jobs/research/AI/library; avoid claiming nothing has been optimized.
- Research currently sorts on raw metrics and an explicitly observational interaction/views ratio; there is no durable time-series/creator-normalized creative analysis model.
- The current watchlists and media queue are real capabilities; do not promise that research retrieval is universally accessible or that unsupported services can be bypassed.
- Significant view copy still includes technical/explanatory text. Plan a separate P3 copy/visual hierarchy review; never conceal errors, permissions or privacy notices simply to reduce text.

## Principles / non-goals

1. **No rewrite.** Extend established Rust crates and the typed ScriptotarApi seam; introduce a small agent coordinator only after stable data contracts.
2. **Never fake agency.** AI may plan/recommend, but tool calls execute through typed policy-checked services. Every meaningful step can be inspected.
3. **Source truth.** Project-validated source IDs, evidence references, provenance/versioning and explicit unsupported/unverified states are mandatory.
4. **Privacy by default.** No silent provider calls, credential storage, automatic cookie extraction, telemetry or remote publication. Explicit consent before paid API calls, external uploads or source-account access.
5. **Rights and safety.** Analyze creative techniques; write new language; avoid reprinting lengthy distinctive copyrighted scripts or impersonating a creator. Confirm claims against independent sources.
6. **Fail and resume honestly.** Durable checkpoints, bounded retries, idempotent work, cancellation, audit trail and no misleading completed status.
7. **Optimize measured bottlenecks.** Record real packaged-app CPU/RAM/latency and IPC metrics before rewriting for speed.
8. **No premature video editor, platform posting, unlimited autonomous browsing or heavyweight multi-agent architecture.** First deliver a dependable research-to-script workflow.

## Success definition

A fresh install with one project can create a study, add at least three sources from distinct available sources or fixtures, process selected media, produce a referenced factual/content analysis, save three genuinely distinct scripts, export them, quit and resume without source/draft loss. Missing metrics, provider denial, interrupted transcription, paid-provider refusal and contradictory facts have visible safe outcomes. EN/AR, RTL, keyboard, accessibility, security, project isolation and existing exported file types remain intact.

The phased backlog and engineering-specific acceptance tests are in [ROADMAP_P0_P3.md](ROADMAP_P0_P3.md).
