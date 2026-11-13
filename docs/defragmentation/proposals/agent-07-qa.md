# Agent 07 — independent QA / P0-00 baseline

**As inspected:** 2026-11-13 (UTC+04). **Evidence baseline SHA:** `2948918865c1960260a8c602e4dfe261c82cd0d0` (planning). **Main SHA:** `69e60f91c0b00560332dac06b732fc2259c1d632`. **QA branch:** `agent-07/defragmentation-independent-qa`. This report is independent source/CI review, not an installed-product certification.

## Verification status and source of truth

- Repository refs were read from GitHub REST Git refs. Main remains the Scriptotar Next 1.4.1 hotfix SHA identified in the planning docs. The desktop UI package.json version is 1.4.1. GitHub Releases queried on 13 November exposed rolling prerelease `tauri-next-latest` (published 12 November 2026) and stable `v1.0.0`; **do not conflate a rolling preview tag with an immutable v1.4.1 release tag**.
- The planning HEAD's GitHub Actions query returned **CI run 43439348331 SUCCESS** and **Security hygiene 43439348357 SUCCESS**, both for exact SHA 2948918. This **does not** cover Rust foundation, GUI visual, sidecar, MCP E2E, packaged Windows/Linux or live-provider canaries on that exact SHA. New PR exact-head CI must be inspected independently.
- Existing `apps/desktop-ui/qa/visual-regression.mjs` collects 140 Chromium screenshots and reports browser-mock metrics; approved image baseline is a separate, manual action. This is **not** a native Tauri benchmark or a full installed WebView interaction test.
- Existing `docs/INSTALLED_APP_E2E.md` defines Linux .deb + subprocess fixture + real Rust/Tauri IPC via mock-runtime, launched installed executable, repeat/restart assertions; Windows installer smoke includes GUI process survival and backend initialization. Its documentation explicitly says WebView clicks/DOM assertions are not automated there. Linux/Windows executions were not reproduced during this audit.
- Current 1.4.x frontend already implements summary-only bootstrap with on-demand `getTranscript`, non-blocking shell/skeleton/error recovery, event-based jobs with periodic reconciliation, source-ID validation, safe project-switch reconciliation, and custom control QA. These are **preservation requirements**, not unimplemented performance tasks.

Evidence: [planning docs](https://github.com/MousaXD/scriptotar/tree/plan/scriptotar-defragmentation/docs/defragmentation), [CI 43439348331](https://github.com/MousaXD/scriptotar/actions/runs/43439348331), [Security 43439348357](https://github.com/MousaXD/scriptotar/actions/runs/43439348357), [installed-app boundary](https://github.com/MousaXD/scriptotar/blob/plan/scriptotar-defragmentation/docs/INSTALLED_APP_E2E.md), [browser QA](https://github.com/MousaXD/scriptotar/blob/plan/scriptotar-defragmentation/apps/desktop-ui/qa/README.md).

## Feature preservation matrix (1.4.1 → defragmentation)

Status **SOURCE/EXISTING TEST** means code and/or a checked-in test was inspected, **not an end-to-end run**. An unobserved installed flow remains NOT RUN.

| Existing flow | Current contracts / checked-in regression | Required preserved behavior | Native observation |
| --- | --- | --- | --- |
| Project creation / switch | ScriptotarApi createProject/selectProject; App project generation and reconciliation; project-management tests | Correct active project, restart, no stale cross-project data | NOT RUN |
| Research / watchlists | saveWatchlist, scanCreator, queueResearch; ResearchView statuses | Scan/access failure explicit, refresh health, correct project and selected source | NOT RUN |
| Local and URL media | enqueueLocalMedia/enqueueUrl; Tauri file-drop tests | User-controlled queue, source validation, no auto enqueue on file drop | NOT RUN |
| Jobs / recovery | retryJob/cancelJob; Rust queue and installed fixture flow | Correct interrupted/cancel/retry state, real persisted outcomes | NOT RUN |
| Transcription | packaged sidecar, existing fixture + live canaries | Local inference, exact transcript ID, error/restart truthfulness | NOT RUN |
| Transcript reader / exports | getTranscript/searchTranscripts, transcript view; existing PDF not implied | Preserve TXT, timestamped TXT, SRT, VTT, JSON and timestamps | NOT RUN |
| AI Copy Prompt / BYOK | buildAiPrompt/runAi, explicit transcript selection and 4 providers | No hidden API request or persisted credential; edited prompt and result behavior clear | NOT RUN |
| Library / history | LibraryView.onOpen and App.openLibraryItem | Open the **exact** selected persisted artifact | **KNOWN GAP** for AI/research |
| Settings | getSettings/saveSettings, settings tests | Local persistence, migration/error visibility, custom controls | NOT RUN |
| MCP permission controls | getMcpPermissions/saveMcpPermissions; stdio/HTTP gateway E2E | Default deny, scoped grants, revocation and no privilege widening | NOT RUN |
| EN/AR, RTL, keyboard, resize | i18n/rtl, Svelte tests + 140 browser QA matrix | Real direction, localized controls/errors, no overflow or focus loss | BROWSER-MOCK ONLY |
| Windows/Linux installation | NSIS/.deb workflows and installed smoke documentation | Real package launch, migrations, sidecar paths, uninstall | NOT RUN ON SHA |

A machine-readable **source-presence-only** guard is in `tests/defragmentation/fixtures/feature_parity.json`. Passing it only shows checked-in entrypoints still exist. It deliberately does not assert user-visible behavior. Review/add a functional test whenever a source marker is renamed or moved.

## Reproducible findings and follow-up ownership

**QA-P0-01 · Exact artifact detail lost at Library boundary (P0 blocker, Agent 02 + 03):** On the baseline App.svelte, `openLibraryItem` recognizes Project and Transcript IDs, but for AI run it sets `activeView = 'ai'`; for Research/Creator it sets `activeView = 'research'`. No exact run/source ID is passed. Reproduction once populated: create or select two distinct AI runs/research items, enter Library, open the older entry, observe that the view changes without restoring that entry's saved details. Expected: project-scoped detail API and explicit selected artifact ID; wrong-project/deleted IDs must fail clearly. This is **source-confirmed missing linkage**, not a claimed installed-app reproduction.

**QA-P0-02 · Copy Prompt save-history semantics (P0 requirement, Agent 02 + 06):** AiStudioView calls `api.buildAiPrompt` for a local Copy Prompt build; backend exposes `runAi` and persists run history separately. Test that a mere preview does **not** silently pretend to be saved; provide an explicit save operation when required. Verify no secret persistence and no remote call in Copy Prompt. Repro with mock provider/network spy and SQLite history before/after; do not infer observed DB behavior from the Svelte code alone.

**QA-P0-03 · Missing native performance baseline (P0-00 blocker, integration owner):** No cold/warm measured **installed** WebView startup, useful workspace time, main-plus-sidecar memory, real bootstrap IPC transfer, large real SQLite workspace or concurrent background-jobs frame data was collected for SHA 2948918. Existing browser-mock numbers cannot close this gap. Track with existing [issue #100](https://github.com/MousaXD/scriptotar/issues/100); do not duplicate release certification claims.

**QA-P3-01 · UI copy / RTL manual assessment (P3 owner):** The existing ResearchView and AiStudioView still carry multiple hardcoded English helper/state strings. Example: ResearchView's “Watchlist health”, “No saved watchlists in this project yet.”; AiStudioView's “Prompt” note and transcript-ID diagnostics. Inspect EN/AR rendered screenshots and manual accessibility before copy reduction; retain actionable errors, consent and privacy notices.

## Performance acceptance instrumentation

No measurements are asserted here. The benchmark entrypoints are:
- `python3 tests/defragmentation/benchmarks/native_session.py --executable /usr/bin/scriptotar --source-sha <full-sha> --trial cold --seconds 20 --output /tmp/scriptotar-cold.json`. It runs **the named executable** and samples Linux main-PID RSS. It explicitly labels shell survival as not UI readiness, excludes sidecars, and cannot certify an installed package from a path alone.
- Run `--trial warm` after separately documenting OS page-cache state and restart policy. Record hardware, clock, model/sidecar, package SHA256, executable SHA256 and exact commit in signed-off evidence.
- `python3 tests/defragmentation/benchmarks/sqlite_readonly.py --db /path/to/isolate.sqlite3 --source-sha <full-sha>` runs **read-only aggregate diagnostics**, not real startup/IPC. Use QA-owned seed_workspace.py to create 100, 1,000 and 10,000-record fixtures from a verified blank, app-migrated scratch template; use immutable copies, same hardware and repeated runs. It emits no user transcript text.
- **Real** SQLite bootstrap timing and serialized IPC payload must be instrumented at the actual Rust/Tauri command boundary by the backend owner or an external native harness, with sizes, call counts and project ID recorded safely. The QA scripts intentionally do not fabricate an IPC sample.
- Full installed-app gate: Windows and Ubuntu .deb or NSIS package verification; actual package install, real WebView visible and interactive, cold/warm workspace and navigation probe, child processes included in memory, background job impact, locale/RTL and source/approval dialogs. Attach exact-head CI URLs, package hashes, machine specs, screenshots/traces and raw JSON. Mock/browser/UI and Rust fixture results get **separate evidence rows**.

No arbitrary performance PASS threshold is set without real baseline distributions and a documented tolerance/variance budget. A one-off speed number alone cannot certify regression.

## P0–P3 acceptance fixtures / GO-NO-GO criteria

The unexecuted scenario catalog in `tests/defragmentation/fixtures/acceptance_cases.json` covers P0 project/recovery/history/export/permissions/RTL, P1 approvals/checkpoint/idempotency/cost/Arabic/provider failure, P2 provenance/contradictions/injection/large datasets, and P3 native controls/reader/accessibility. Every catalog case is explicitly NOT_WIRED / NOT_RUN: schema validation is **not** a product acceptance PASS.

| Phase | Independent status now | Conditions to change to GO |
| --- | --- | --- |
| P0-00 baseline | **NO-GO (partial)** | Full reproducible installed baseline, exact SHA, hardware/OS, parity evidence and final integration signatures |
| P0 feature integration | **NO-GO / not implemented at baseline** | Thin study→source→draft→restart flow, ID isolation, legacy data, exports, EN/AR, SQLite migration and exact-head native smoke |
| P1 agent/runtime | **NO-GO / future** | Durable approvals, bounded retries, cancel/restart, idempotent tools, provider/cost consent, adversarial fixtures |
| P2 insight/performance | **NO-GO / future** | Truthful metrics, contradictions, provenance, 100/1k/10k data and native measured regression comparison |
| P3 UI polish | **NO-GO / future** | Accessibility, responsive/RTL, keyboard and screen-reader checks, control appearance, visual baseline approval and installed-package QA |

**Security invariant:** source transcripts/captions/comments are untrusted data, never instructions. Every claimed finding needs a project-scoped locator and auditability; approval must precede paid calls, remote uploads and account access. No provider secrets, raw user text or high-volume transcript bodies belong in performance reports.

## QA automation integration boundary

QA-owned `apps/desktop-ui/qa/tooling.nodecheck.mjs` now executes the Python fixture and parity contract suite as a subtest in the existing Chromium QA workflow, in addition to existing screenshot-verifier tests. Its path-based trigger only covers changed `apps/desktop-ui/**`. Agent 00 must add a separate exact-head required gate for Rust/data-only work and call the new acceptance adapters once the typed P0 API is frozen; QA does not edit cross-agent workflows or weaken existing mandatory gates.

**Independent conclusion:** strong preexisting engineering baseline, but **NOT CERTIFIED** for the defragmentation release or native performance. The QA artifacts are reproducible guardrails and a test plan, not implementation proof.
