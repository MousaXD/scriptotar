# Changelog

## Scriptotar Next 1.3.0 — 2026-11-08 (release candidate; unpublished)

- Bumped the integrated owner-overhaul candidate from 1.2.0 to 1.3.0 at the owner's request before promotion to `main`.
- Aligned Rust workspace and lockfile packages, Tauri, Svelte package and lockfile, sidecar identity, MCP acceptance checks, and current public documentation. Sidecar protocol remains version 1.
- Packaging retries transient FFmpeg/tunnel download failures; tunnel assets can use GitHub's official asset API when its download edge fails, retaining pinned digests, size limits, and exact release/filename matching.
- Retains the 1.2.0 candidate's integrated features, fixes, security gates, and disclosed live-provider/manual acceptance limitations. Classic 1.3.0 is a separate archived product line.
- See [1.3.0 release readiness](docs/releases/NEXT_1.3.0_RELEASE_READINESS.md) for the candidate checks and publication status.

## Scriptotar Next 1.2.0 — 2026-11-07 (release candidate; unpublished)

This candidate integrates the seven-agent Rust/Tauri/Svelte Next overhaul. Archived Scriptotar Classic remains at 1.3.0 and is **not** upgraded or replaced.

### Features and experience

- App-owned MCP integration with 14 tools, authenticated stdio/localhost HTTP transport, connection wizard, revocable permissions, and default-deny local file/command boundaries.
- Refined bilingual Arabic/English UI, right-to-left navigation, keyboard/focus accessibility, Library and transcript recovery, and improved project views.
- Safer AI Studio and creator research with explicit source selection, bounded provider requests, validation and retention of supported output choices.
- Stronger SQLite migrations, recovery, legacy import and Rust desktop orchestration, including project summary responsiveness.

### Reliability, security and packaging

- Fail-closed Windows transcription process-tree cancellation and shutdown: uncertain cleanup never releases queued jobs; verified cleanup restores safe recovery.
- Stronger FFmpeg archive integrity, bounded retries, engine/device validation, packaged licenses and self-contained Python/yt-dlp/Whisper runtime.
- Patched Rust dependency advisory and eliminated high/critical npm audit findings without disabling security gates; three documented low-severity paths remain.
- Native Windows subprocess-descendant test; Windows NSIS installation and Linux .deb installation including MCP HTTP auth/revocation, desktop shell launch and repeated restart/persistence tests.
- Release publisher continues to require successful mandatory checks on the exact `main` commit and explicit distribution approval.

### Known limitations and release status

- **Live YouTube E2E is FAILING**: the pinned public source requests bot/sign-in verification before download/inference. Instagram real transcription passed with three segments. TikTok real inference completed with zero speech segments; this is not proof of recognition accuracy.
- Manual/hardware checks for CUDA fallback, Arabic/mixed speech accuracy, authorized live ChatGPT tunnels, full interactive Windows GUI, migrated customer databases and performance remain incomplete.
- FFmpeg upstream archive location remains mutable despite byte-for-byte SHA-256 and size validation.
- This entry describes an **unpublished candidate**. It does not indicate a stable Next release, real YouTube success, or permission to bypass QA. See [release readiness](docs/releases/NEXT_1.2.0_RELEASE_READINESS.md).

## 1.3.0 - 2026-08-21

### Added

- Arabic GUI localization and RTL layout support with instant Arabic/English language switcher.
- Creator Control Room desktop UI overhaul with refined navigation and workspace layouts.
- SQLite-backed full-text search (FTS5) for workspace transcripts with Arabic morphology substring fallback.
- On-demand lazy transcript detail loading and state isolation for improved performance.
- Exact completed transcript lineage linking and navigation from jobs and history.
- Explicit transcript source selector and lineage in AI Studio.
- New project creation flow in dashboard and settings.

### Fixed / Hardened

- Pinned reviewed PyAV 18.0.0 dependency in transcription engine runtime.
- Added CPython 3.12 patchline compatibility in runtime license generator.
- Resolved race conditions in job event notifications and queue cancellation.

## 1.2.0 - 2026-08-09

### Added

- Public creator/profile research using the installed `yt-dlp` engine.
- Research metrics view, sorting, CSV export, and queue-to-transcribe flow.
- Local watchlists with optional refresh while Scriptotar is running.
- Project organization across jobs, research, watchlists, and AI runs.
- Unified local Library for transcripts, research items, prompts, and AI outputs.
- AI Studio with Copy Prompt and Bring Your Own API Key modes.
- OpenAI Responses, Anthropic Messages, Gemini, and custom OpenAI-compatible adapters.
- Linux Secret Service storage for remembered API keys.
- Viral breakdown, hook, original script, structure remix, content ideas, caption/CTA, voice profile, and B-roll prompt templates.
- Spoken-script duration estimator.
- Apache-2.0 licensing and open-source project metadata.
- GitHub Actions test/package workflow.

### Changed

- Upgraded the application UI/database schema without forcing a Whisper engine reinstall.
- Debian package now depends on `libsecret-tools` for optional secure key storage.

### Security / correctness

- AI tokens are not written to `settings.json`.
- Creator scan URLs are passed after `--` to prevent option injection into `yt-dlp`.
- Research CSV export escapes spreadsheet formula prefixes.
- Structure-remix prompts explicitly prohibit copying distinctive source wording.
- Added a migration test for existing 1.1 SQLite databases.
