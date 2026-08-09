# Changelog

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
