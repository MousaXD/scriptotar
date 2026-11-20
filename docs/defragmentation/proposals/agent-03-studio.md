# Agent 03 — Unified Studio UI and contract proposal

**Branch:** `agent-03/defragmentation-studio-ui` (base `plan/scriptotar-defragmentation`)  
**Scope:** `apps/desktop-ui/src/**` plus this proposal and `handoffs/agent-03.md`. Agent 07 owns `apps/desktop-ui/qa/**`. No Rust, SQLite, Python or root changes.

## Current architecture verified

At the planning baseline the UI has seven top-level screens: Dashboard, Research, Jobs, Transcript, AI Studio, Library and Settings. `App.svelte` mounts one screen at a time; the global search already has some stable transcript IDs, whereas Library research and AI actions historically discarded the target IDs. `AiStudioView` consumes one transcript or a manually pasted blob, and the existing `ScriptotarApi` exposes `getTranscript` and `buildAiPrompt` but not study mutations or historical AI run detail. The prior Svelte overhaul already shipped an accessible custom `Select`, local appearance setting, keyboard palette, transcript on-demand hydration and event-driven job updates. Preserve those assets.

## Design direction

The primary composition is **source tray + central brief/editor + evidence preview**, with current jobs/failures in context. Research row actions and transcript reader actions pass *exact typed IDs* to Studio. The evidence panel fetches a transcript on demand, validates its project ownership and discards stale async responses. Research metadata is labeled observational, not proof of causality. Script editing and prompt preparation are distinct: preparing a prompt is a local, copy-only action and does not trigger a paid provider call.

During P0 we retain visible Queue/Transcripts/AI expert navigation and palette commands, and add Studio. Final five-item Home / Discover / Studio / Library / Settings consolidation is a P3 gate, not a destructive P0 rewrite. Global activity preview reads real jobs and includes failed/interrupted detail; full Queue remains available.

## Proposed frontend boundary — needs Agent 00 contract freeze

`src/studio/contract.ts` currently defines an **isolated frontend adapter** for mock-driven component validation, **not** a new production Tauri IPC claim. Agent 00 and Agents 01/02 must reconcile names/types with the canonical Rust/SQLite schema:

| UI operation | Existing API | Required durable backend API |
| --- | --- | --- |
| Preview transcript by exact ID | `getTranscript(id)` (existing) | Retain project membership assertion |
| Discover source metadata and queued jobs | `bootstrapSummary`, `listJobs` | Paged/list queries later in P2 |
| List/open study | none | `listStudies(projectId)`, `getStudy(projectId, studyId)` |
| Create study | none | `createStudy(projectId, brief)` |
| Edit brief, source links and draft | none | `saveStudy(projectId, studyId, expectedRevision, patch)` or split transactional equivalents |
| Reopen an exact AI result | AI summary only | project-checked `getAiRun(projectId, runId)` / `getAiArtifact` detail |
| Source evidence locators | transcript segments exist | provenance DTOs for research, document, excerpt, offset and confidence |
| Source queue and AI tools | `queueResearch`, `buildAiPrompt`, `runAi` | typed study association and consent for P1 agent runs |

The present optional `StudioGateway` accepts a `ContentStudy` snapshot with `revision`, `brief`, `sources[]`, `draft` and immutable `draftVersions[]`. It requires project validation on each read and mutation, and an expected-revision failure (not last-write-wins). Source IDs are kind-prefixed; duplicate and foreign-project transcript IDs are rejected in the UI and MUST be checked again by Rust. Saved AI results require exact ID + project match; a missing endpoint shows an explicit error instead of a fabricated history result.

### Newly verified parallel contract proposals (2026-11-13)

- Agent 01 [PR #108](https://github.com/MousaXD/scriptotar/pull/108) proposes migration v6 with **separate** `ContentStudy` (study title/goal), `CreativeBrief` (revision), `CreativeDraft` (revision + immutable versions), and independently referenced/tombstoned `StudySource` and `EvidenceReference`. The simplified frontend `saveStudy` port is a **test fixture facade**, not a schema instruction. Agent 00 must map it to approved project-scoped, optimistic-concurrency-safe transactions; do not flatten distinct draft/brief revisions into one unsound write.
- Agent 02 [PR #103](https://github.com/MousaXD/scriptotar/pull/103) implements project-scoped `get_ai_run({ projectId, aiRunId })` and `get_research_item({ projectId, researchItemId })` in an independent Rust branch. `UiAiRunDetail.result` is nullable; the UI already distinguishes saved prompt from generated response. These commands are not in the planning branch's production frontend API yet.
- The Agent 02 branch cannot pass API-surface parity until Agent 00 directs an approved frontend `ScriptotarApi`/Tauri-client/mock mapping and the independent QA command inventory is updated. Do **not** bypass this regression gate.

**Integration owner action:** install a real gateway in `App.svelte` only after Agent 00 accepts the DTO/method names and Agent 02 implements Tauri handlers. Do not implement the interface with sessionStorage/localStorage, guessed IPC commands, or cross-project name/title matching.

## Implementation prepared on this branch

- Studio route and keyboard shortcut S; existing screens remain reachable.
- Studio page with title, creative goal, audience, platform, language, duration and style.
- Per-source checkbox selection from Research and current-project transcripts; exact ID handoff from Research and Transcript pages.
- Inline transcript segments, on-demand fetch, stale-request suppression and project-bound validation.
- Editable script area, TXT/Markdown export, version viewer when a gateway supplies history.
- Mock-driven create/open/save, revision-checked update with debounced writes while the component is mounted.
- Project-local job activity preview in Studio and global topbar activity popover, with failure details.
- Saved AI-run detail surface that explicitly reports when the backend endpoint is unavailable.
- Bilingual Studio labels, mixed Arabic/English `dir=auto`, responsive three/two/one-column layout and visible keyboard focus via shipped controls.
- Local prompt preparation uses existing `buildAiPrompt`; no provider call, no credential persistence.

## Gaps / dependencies not to mark complete

1. **P0-01/P0-03** Rust/SQLite durable studies, source provenance and versioned drafts are not implemented by this frontend branch. Until integrated, the live Studio displays **unsaved workspace**, and exports are the only durable output.
2. **P0-02** historical AI detail remains unimplemented in the existing production API; the UI supports it through an optional typed port.
3. **P0-03** whole-app quit/restart guarantees, autosave flush during view/project switching, collision recovery, and immutable version certification require the real backend and full integration tests. Mock success is not proof.
4. **P0-04** the source-to-script path is present, but end-to-end cross-source evidence and background operations need the agreed study associations. Existing AI tools still use single-source inputs.
5. Independent installed-app screenshots/performance benchmarks and Agent 07 QA are separate gates; no numbers are asserted here.

## P3 preliminary copy/visual audit (do not destructively apply yet)

| Existing view | Redundancy to audit at P3 | Preserve |
| --- | --- | --- |
| Dashboard | Repeated metric/explainer cards | actionable failure/status and recent work |
| Research | instructional text and metric legends repeated in filters/rows | metric provenance, missing values, access/denial notices |
| Jobs | explanatory headers duplicated with row states | progress, failure messages, retry/cancel |
| Transcript | repeated instructions near search/export | source identity, timestamps, export formats |
| AI Studio | step headings and privacy prose repeated across sections | session-only key boundary and provider consent |
| Library | filter instruction fragments and generic descriptions | exact IDs, project, unsupported artifact errors |
| Settings | multi-paragraph descriptions for routine toggles | permission scope, MCP security and destructive warnings |

Audit English and Arabic together with screenshots after P0 contract integration; do not remove a11y labels or vital privacy/security/failure text for visual minimalism.

## Verification gate

Run `npm ci --prefix apps/desktop-ui`, `npm run check --prefix apps/desktop-ui`, `npm test --prefix apps/desktop-ui` and `npm run build --prefix apps/desktop-ui` on the exact branch commit. Review Agent 07 Chromium screenshots (owned externally) when produced. Agent 00 is responsible for Rust/DB/installed-app canaries and merge decisions.

Tests on this branch cover source ID validation, mock study reopen/versioning, stale async transcript hydration, wrong-project denial, EN/AR controls, source-to-Studio navigation, global activity and exact historical AI-run opening when detail is provided. Test results are recorded in the handoff, not inferred from their existence.
