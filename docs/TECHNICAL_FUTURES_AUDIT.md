# Scriptotar Technical & Futures Audit

Audit baseline: `feature/arabic-gui` at `ce37ba5abafdb091d95ebd0f616c869d81090c71`.

This document is the planning baseline for `feature/technical-futures`. It audits the current Scriptotar product, identifies technical and product gaps, and prioritizes work that turns the existing pieces into a coherent creator-research and content-development workflow.

## Executive assessment

Scriptotar Next is already much more than a transcription GUI. It is a local-first creator research workspace with media acquisition, persistent transcription jobs, transcript review/export, creator watchlists, local search/library, and optional AI workflows.

The strongest part of the product today is its engineering foundation: typed Rust domain boundaries, a Rust-owned SQLite database, persistent job state, explicit Tauri IPC, a packaged Python transcription sidecar, provider-specific AI/research services, migration safety, and broad CI coverage.

The main weakness is product integration. Research, queueing, transcripts, library, and AI are all useful, but they still behave too much like adjacent tools. Stable entity relationships, deep links, reusable artifacts, richer analytics, and workflow automation are the next leverage points.

A second weakness is scale readiness. The frontend currently bootstraps a large workspace snapshot, performs global search in memory, polls jobs every second, refreshes operational state periodically, and the orchestrator serializes media work. Those choices are acceptable for a 0.1 preview, but should not become permanent architecture.

A third weakness is localization completeness. The Arabic branch has a real i18n/RTL foundation, but substantial user-facing English remains hard-coded across Dashboard, Research, Jobs, Transcript, AI Studio, and Settings. Arabic support should be treated as a product-quality feature, not only a translation dictionary.

## What Scriptotar serves today

The current product loop is:

1. Create/select a local project.
2. Scan public creator/profile metadata and save watchlists.
3. Compare raw media performance signals.
4. Queue selected URLs or local media.
5. Download/process media and transcribe it locally with Faster Whisper.
6. Review timestamped transcripts and export TXT/SRT/VTT/JSON.
7. Use transcript text in Copy Prompt or BYOK AI workflows.
8. Keep projects, jobs, transcripts, research, watchlists, settings, and AI-run history in a local SQLite workspace.

This positions Scriptotar best as a **local-first creator intelligence and content-development desktop workspace**, not merely a video downloader or transcription utility.

## Current quality scorecard

These scores are based on source, architecture documents, and CI behavior, not a full manual UX session on every packaged OS build.

| Area | Assessment | Notes |
| --- | --- | --- |
| Architecture / boundaries | 8.5/10 | Strong Rust crate ownership and thin IPC boundary. |
| Local-first privacy | 9/10 | Local processing, Copy Prompt mode, request-time BYOK credentials, no mandatory account/telemetry. |
| Persistence / recovery | 8.5/10 | SQLite/WAL, typed job states, restart recovery, safe Classic migration. |
| CI / engineering hygiene | 8/10 | Broad frontend, Rust, Python, integration, security, runtime and package workflows. |
| Core workflow coherence | 7/10 | Strong pieces, but entity lineage and exact-item navigation are incomplete. |
| Research intelligence | 5.5/10 | Useful collection/filtering, mostly raw metrics rather than derived intelligence. |
| Transcript workspace | 6/10 | Good reader/search/export, missing editing, playback, speakers, persisted artifact actions. |
| AI workflow | 6/10 | Good provider/privacy boundary, weak source selection, artifact lineage and reuse. |
| Arabic / RTL foundation | 6.5/10 | Solid directionality/i18n scaffolding, but translation coverage is still incomplete. |
| Scale / performance readiness | 6/10 | Full bootstrap snapshots, polling, frontend text search, serialized orchestration. |
| Distribution maturity | 6/10 | Next is preview; Windows signing, macOS and extra Linux formats remain unfinished. |

**Overall:** approximately **7/10 for a 0.1 preview**. The foundation is ahead of feature completeness.

## What is already strong

### Architecture

- Rust owns domain state, persistence, orchestration, AI/research policies, migration, and IPC-facing services.
- `scriptotar-core`, `scriptotar-db`, `scriptotar-jobs`, `scriptotar-media`, `scriptotar-orchestrator`, `scriptotar-ai`, and `scriptotar-research` have clear responsibilities.
- Svelte uses a `ScriptotarApi` seam instead of directly opening SQLite or launching processes.
- The Python sidecar protocol is versioned and shell-free, with explicit ready/cancel/shutdown behavior.
- Active job transitions are typed and persisted.
- Classic migration snapshots the source safely instead of mutating the old database in place.

### Product behavior

- Local file picker and native file-drop support.
- URL transcription and public creator/profile research.
- Persistent jobs with retry/cancel/interrupted recovery.
- Watchlist health and retry status.
- Transcript search with timestamp navigation.
- TXT, timestamped TXT, SRT, VTT and JSON export.
- Copy Prompt mode that does not require an AI key.
- BYOK support for OpenAI, Anthropic, Gemini and OpenAI-compatible endpoints.
- Command palette and keyboard navigation.
- English/Arabic locale controls and RTL transcript rendering.

## High-value current gaps

### P0: Product coherence and correctness

#### 1. Persist the active project

The documented behavior still falls back to Inbox after restart. Persist the active project ID and recover safely when a project has been deleted or archived.

#### 2. Finish project management

`ScriptotarApi` already exposes `createProject(name)`, but the current shell does not expose a visible project-creation flow.

Add:

- create project;
- rename project;
- archive/unarchive;
- delete with clear destructive confirmation;
- project description editing;
- recent/pinned projects;
- optional project templates later.

#### 3. Use stable entity relationships everywhere

The UI should never infer a transcript from matching title/source strings.

Add explicit relationships such as:

- `Job.completedTranscriptId`;
- research item → queued job IDs;
- transcript → source research item / source media;
- AI artifact → exact source IDs;
- creator/watchlist → imported research items.

Then make Dashboard, Library, Research and AI open the exact selected entity instead of merely navigating to a view.

#### 4. Native artifact ownership

The transcript UI currently cannot open a persisted output directory.

Add backend-owned artifact metadata and native commands for:

- open output folder;
- reveal source media;
- Save As / export location;
- regenerate/export artifact;
- clean artifact/cache safely.

Do not make the webview invent durable filesystem paths.

#### 5. Complete Arabic localization

Arabic support is currently mixed with hard-coded English strings in major views.

Required quality gate:

- every user-facing string goes through translation keys unless explicitly marked technical/brand content;
- English and Arabic dictionaries have identical key coverage;
- RTL is tested at app-shell and component level;
- bidirectional text isolates URLs, paths, timestamps, model names and code fragments correctly;
- back/forward icons and layout affordances mirror where semantically appropriate;
- pluralization/number/date formatting uses locale-aware APIs;
- long Arabic strings do not clip controls;
- mixed Arabic/English transcript text remains readable.

Add a CI check that detects new raw user-facing UI strings or requires an explicit i18n-ignore annotation.

### P0: Scale and responsiveness

#### 6. Replace polling with host events

The app currently polls active jobs every second and operational state every 15 seconds.

Prefer Tauri events/subscriptions for:

- job state/progress;
- watchlist refresh state;
- migration state;
- model/runtime state.

Keep a low-frequency reconciliation refresh as a safety net rather than the primary transport.

#### 7. Stop shipping the whole workspace over IPC

`bootstrap()` currently carries projects, creators, research, jobs, transcript text/segments, AI runs, library data and operational status.

Move toward query-shaped APIs:

- workspace summary;
- paged research;
- paged library;
- transcript list summaries;
- transcript detail on demand;
- paged jobs;
- AI history summaries;
- explicit entity detail endpoints.

This reduces startup time, IPC payload size, memory use and expensive full refreshes.

#### 8. Move global search into the backend

The frontend currently searches full transcript text and other entities in memory.

Use the existing SQLite search infrastructure for a unified search endpoint with:

- pagination;
- ranked results;
- entity filters;
- snippets/highlights;
- project scope/all-project scope;
- exact entity IDs for navigation.

Semantic search can be layered later without replacing lexical FTS.

#### 9. Parallel job scheduling

The orchestrator currently serializes media work.

Introduce a resource-aware scheduler with conservative defaults:

- configurable transcription concurrency;
- separate download/transcription slots;
- CPU/GPU/RAM-aware admission;
- per-device defaults;
- queue priority;
- pause/resume;
- graceful shutdown/recovery.

Do not simply spawn unlimited Whisper workers.

## P1: Jobs and media pipeline

- Accept and queue all valid files from a multi-file drag/drop operation, not only the first path.
- Paste/import multiple URLs at once.
- Optional playlist/channel extraction preview before queueing.
- Per-job overrides for model, language, translation, quality and device.
- Queue reorder / priority controls.
- Pause/resume queue and pause/resume all.
- Estimated remaining time and current throughput.
- Clear failure diagnostics with copyable redacted logs.
- Retry only the failed stage where safe.
- Duplicate-media detection using source identity/hash.
- Download-only and transcribe-only job modes.
- Audio-only local import support as a first-class workflow.

## P1: Transcript workspace

### Media-linked transcript editor

Add a local player synchronized to transcript segments so clicking a timestamp seeks the media and playback follows the active segment.

Add:

- editable segment text with persistence;
- split/merge segment controls;
- undo/redo;
- find/replace;
- bookmarks/highlights/notes;
- speaker labels/diarization when available;
- word-level timestamps/confidence when available;
- low-confidence review mode;
- custom dictionary/glossary for names, brands and Arabic terms;
- side-by-side original/translated transcript;
- subtitle line-length and reading-speed warnings;
- Arabic subtitle preview with RTL-safe line breaking.

### Export improvements

Add native Save As plus:

- DOCX;
- PDF;
- CSV/TSV segment export;
- bilingual subtitle export;
- configurable subtitle line wrapping;
- reusable export presets.

## P1: Research intelligence

The current Research view captures useful raw signals. The next step is to calculate intelligence instead of only displaying counts.

Add derived metrics:

- engagement rate;
- views per day / velocity;
- creator-baseline-normalized outlier score;
- percentile against recent creator posts;
- comments-to-views and likes-to-views ratios;
- duration-adjusted performance;
- recent acceleration/deceleration where data permits.

Add analysis surfaces:

- creator comparison;
- post-performance distribution;
- top outliers;
- duration vs performance;
- posting cadence;
- new-since-last-scan changes;
- saved filters/searches;
- tags/notes/collections;
- CSV/JSON research export;
- research detail drawer with source metadata and direct queue/transcribe action.

### Content pattern intelligence

Once transcripts exist, derive reusable local features such as:

- hook type;
- opening sentence;
- topic/category;
- CTA type;
- approximate speaking rate;
- script length;
- recurring phrases/entities;
- content structure tags.

Keep generated/inferred fields separate from source metadata and preserve provenance.

## P1: AI Studio

### Fix source selection

AI Studio currently starts from a generic/first transcript rather than a deliberately selected source.

Add a source picker for:

- exact transcript;
- research item;
- creator corpus;
- multiple selected transcripts;
- project notes/artifacts.

Navigation from Transcript/Research/Library should support **Use in AI Studio** and preserve exact source IDs.

### Make AI outputs first-class artifacts

Instead of storing only an AI-run log, introduce artifact types such as:

- insight;
- hook set;
- script;
- caption/CTA;
- voice profile;
- shot list;
- content brief.

Each artifact should retain:

- source entity IDs;
- prompt/template version;
- provider/model;
- generation parameters;
- creation timestamp;
- parent/variant relationship.

### AI quality and safety controls

- Prompt template library and custom templates.
- Versioned templates.
- Regenerate/variant/compare flows.
- Context-size estimation and trimming warnings.
- Token/cost estimates for remote providers when calculable.
- Configurable provider/model defaults rather than a fragile hard-coded default model.
- Provider fallback only when explicitly configured.
- Treat imported captions/transcripts/research text as untrusted source data and delimit it clearly from system/task instructions to reduce prompt-injection risk.
- Redact secrets from local AI diagnostic logs.

### Local AI

The type system already advertises `Local (coming later)`.

A future local provider boundary should support one or more local inference servers/runtimes without coupling the UI to a single vendor. Keep the provider interface identical to remote execution where practical.

## P1: Arabic-specialized product improvements

Arabic can become a product advantage rather than only a translated interface.

Add:

- Arabic dialect hinting/presets where supported by the speech pipeline;
- mixed Arabic/English code-switch review;
- Arabic punctuation normalization options;
- Arabic/Latin digit display preference without altering source text;
- named-entity/custom glossary correction;
- bilingual aligned segment view;
- Arabic subtitle line-breaking and reading-speed preview;
- Arabic-first prompt templates for hooks, captions and scripts;
- per-project language profile;
- robust bidi isolation for usernames, URLs, timestamps and technical terms.

Never silently rewrite the source transcript. Normalized/translated forms should be separate derived artifacts.

## P1: Models, storage and diagnostics

Add a runtime/model manager that shows:

- installed Whisper models;
- model sizes;
- download state/progress;
- cache location;
- delete/re-download;
- disk usage;
- detected CPU/GPU/runtime capability;
- recommended model/device based on hardware;
- a local transcription benchmark/test clip.

Add a diagnostics page with:

- app/version/build/channel;
- database schema version;
- sidecar protocol/runtime versions;
- FFmpeg/yt-dlp versions;
- hardware capability summary;
- recent redacted failures;
- exportable redacted support bundle.

## P1: Distribution and update maturity

Before calling Scriptotar Next stable:

- Windows signing and predictable publisher identity;
- updater channel with signed metadata/artifacts;
- safe rollback/recovery strategy;
- Next AppImage and/or Flatpak if maintained intentionally;
- macOS signing/notarization/package lane;
- installed-app smoke tests for each supported package;
- real-model canary coverage with a tiny deterministic media fixture;
- release provenance/SBOM where practical;
- dependency/advisory gates that distinguish actionable vulnerabilities from unreachable/accepted risk.

## P2: End-to-end creator workflow

The largest future opportunity is turning separate tools into a content pipeline:

`Research → Select → Transcribe → Analyze → Brief → Script → Shot list → Export package`

Possible workspace states/entities:

- Research Item
- Transcript
- Insight
- Brief
- Script
- Hook Variant
- Shot List
- Caption
- Export Package

Every stage should retain lineage to its sources.

### Clip intelligence

- Mark promising hook/claim/story moments from transcript timestamps.
- Suggest clip ranges without destructively editing source media.
- Export selected ranges through FFmpeg.
- Burn or attach generated subtitles.
- Generate subtitle-safe vertical previews.
- Keep all generated clips linked to source transcript segments.

### Storyboard / B-roll planning

Turn a script into timestamped sections with:

- spoken line;
- B-roll suggestion;
- on-screen text;
- source reference;
- estimated duration;
- status/checklist.

### Brand / voice profiles

Local reusable profiles could contain:

- target audience;
- tone constraints;
- forbidden phrases;
- preferred CTA styles;
- glossary/brand spellings;
- language/dialect preferences;
- sample scripts.

AI runs can reference a profile instead of repeatedly pasting instructions.

## P2: Automation engine

Add opt-in local rules such as:

- when a saved creator scan finds a new post above an outlier threshold → queue transcription;
- when transcription finishes → run a chosen local/remote analysis template;
- when a job fails repeatedly → surface a notification and stop retrying;
- when model/cache disk usage exceeds a threshold → warn, never delete automatically without policy.

Rules should be explicit, inspectable and locally persisted.

For true scheduled behavior while Scriptotar is closed, use a separate deliberate OS background-service design. Do not imply that an in-app timer works when the app is not running.

## P2: Power-user interfaces

- Scriptotar CLI using the same Rust services/domain contracts.
- Optional localhost API with explicit enablement/authentication.
- Import/export portable project bundles.
- Workspace backup/snapshot/restore.
- Batch processing manifests.
- Plugin/provider boundary for future research sources, AI providers, exporters and transcription engines.

Plugin execution must be sandboxed/permissioned deliberately rather than loading arbitrary code into the main process.

## Technical refactors before feature explosion

### Frontend decomposition

Several Svelte files are already large. Before adding major features, split stateful surfaces into smaller tested components/stores, for example:

- `AppShell` → sidebar, project picker, command palette, topbar/activity;
- `TranscriptView` → transcript list, reader, details, export menu, search controller;
- `ResearchView` → capture, watchlist health, filters, table/cards;
- `SettingsView` → section components;
- `AiStudioView` → source picker, provider config, brief, prompt, artifact result.

Do not over-componentize trivial markup. Split on state ownership, reusable behavior and test boundaries.

### Typed frontend state/query layer

As bootstrap is decomposed into query-shaped APIs, introduce explicit loading/error/stale state per resource rather than a single large mutable snapshot.

### Structured local observability

Use correlated IDs through Tauri → service → orchestrator → sidecar so one job can be traced through local logs. Add log rotation and redaction rules.

### Resource cleanup

Define retention policies for:

- downloaded source media;
- failed partial artifacts;
- exported artifacts;
- model cache;
- thumbnails;
- temporary files;
- logs.

Expose disk usage before adding automatic cleanup.

## Suggested implementation order

### Phase 1: Make existing behavior coherent

1. Finish Arabic string coverage + CI localization guard.
2. Persist active project.
3. Add project create/rename/archive/delete UI and backend operations.
4. Add stable entity IDs/relationships and exact-item deep links.
5. Add native artifact path/open/export APIs.
6. Add batch file/URL queueing.

### Phase 2: Scale the current app

7. Introduce backend pagination/query endpoints.
8. Move global search to SQLite/FTS.
9. Replace primary polling with Tauri events + reconciliation.
10. Add resource-aware concurrent orchestration.
11. Add model/cache/runtime diagnostics.
12. Decompose the largest Svelte views along the new state boundaries.

### Phase 3: Upgrade the core value

13. Media-linked editable transcript workspace.
14. Derived creator analytics/outlier scoring.
15. Research detail/comparison surfaces.
16. AI source picker and first-class generated artifacts.
17. Arabic-specialized transcript/subtitle tooling.
18. Local AI provider boundary.

### Phase 4: Turn it into a workflow platform

19. Artifact lineage pipeline.
20. Clip intelligence and subtitle rendering.
21. Brand/voice profiles.
22. Automation rules.
23. Portable project backup/export.
24. CLI/optional local API/plugin boundaries.

### Phase 5: Stable distribution

25. Signing/updater/provenance.
26. Additional maintained package formats.
27. macOS release lane.
28. Installed-app and real-model release gates.

## Merge gates for this branch

Before merging technical/future implementation work back toward the Arabic GUI line or main:

- Rust fmt/check/clippy/tests green;
- Svelte check/tests/build green;
- Python sidecar tests green;
- Rust ↔ sidecar integration green;
- migration/recovery tests green;
- security/supply-chain gates green or documented accepted exception;
- Arabic + English translation key coverage green;
- no regression to Classic packaging unless deliberately scoped;
- relevant installed-package smoke tests green for changes touching packaging/runtime;
- docs updated when IPC/schema/persistence behavior changes.

## Product direction to protect

Do not turn Scriptotar into a cloud account product by accident. Its strongest identity is that research metadata, transcripts, projects and generated work can remain local, while network access is explicit for source acquisition and optional AI providers.

The best future is not “more AI buttons.” It is a local creator intelligence graph where every source, transcript, insight, script and export remains linked, searchable, reproducible and user-controlled.
