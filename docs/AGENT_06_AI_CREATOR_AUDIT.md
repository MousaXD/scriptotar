# Agent 06 — AI & Creator Intelligence audit

**Branch:** `agent-06/ai-creator-overhaul`  
**Base:** `feature/chatgpt-mcp-ci-20261026` (at `517877b26df9028cdd32af69f6f5a88a68b948e9`)  
**Owner:** Agent 06 · **Coordinator:** Agent 00  
**Status:** Review and CI pending. No agent-initiated merge.

## Existing workflow and privacy contract

- Research profile URLs are allowlisted to Instagram, TikTok and YouTube in `scriptotar-research`; yt-dlp extracts metadata to project-scoped research records.
- The Research UI filters and queues selected source IDs. Metadata values may be missing, and raw counts are not a controlled comparative experiment.
- AI Studio uses eight built-in creator tasks, a manual/copyable prompt, optional BYOK providers (OpenAI, Anthropic, Gemini, OpenAI-compatible), and a disabled future local provider option.
- Copy Prompt does not require a key or a network request. BYOK is user-initiated. User credentials remain request-time data and must never be written to the AI-run history or logs.
- The backend persists project-scoped AI prompts and successful BYOK results in SQLite, but the current AI-run model does **not** store source transcript IDs or template versions. The UI only displays the available history summary; it cannot reopen exact historical results yet.

## Confirmed findings and changes

| Priority | Finding | Agent 06 action |
| --- | --- | --- |
| P0 | Provider HTTP body was fully read via `Response.text()` before checking the 8 MiB cap; chunked/missing-length responses could exhaust memory. | Bounded streaming read; rejects excessive/invalid bodies without exposing response secrets. Unit fixture tests. |
| P0 | yt-dlp stdout line limit was checked only after `read_until` allocated an entire provider line. | Bounded each line read at 2 MiB + sentinel, with a no-newline regression test. |
| P1 | A fallback YouTube ID could inject query parameters into the synthesized source URL. | Accept only alphanumeric, dash, underscore in fallback ID. Revalidate all final URLs using the existing platform policy. |
| P1 | Negative counts and durations could be stored as though valid observations. | Drop invalid negative metrics and durations, preserving legitimate zero values. |
| P1 | AI Studio implicitly loaded the first transcript, risking accidental source substitution. | Start with deliberate manual selection; select exact transcript by ID and show source ID and edited-excerpt status. |
| P1 | AI source reads, prompt builds, and provider requests could return results after input changes. | Reject stale asynchronous results; disable generation while source is loading/failed; surface build failures instead of unhandled rejection. |
| P1 | An edited prompt preview was misleading: BYOK always rebuilt the request from the brief and source, ignoring preview edits. | Explain copy-only edits and block running an edited preview until a fresh build. |
| P1 | Research displayed numeric 0 as missing, and selected stale IDs could remain queued after project data changed. | Preserve zero counts; queue only IDs present in current project items; clear selection on successful queue. |
| P2 | Research provided only raw metrics. | Add an explicitly observational `(likes + comments) / views` indicator and sort; calculate only when all three counts are finite and nonnegative and views are positive. Missing data is not treated as zero. |

## Source lineage boundaries

Current selected transcript identity is exact in **AI Studio UI**, but `AiPromptInput` only transports `sourceText`, not `sourceId`; `AiRun` persists prompt/result without typed source links. It would be incorrect to claim persisted end-to-end lineage. Avoid guessing links from title or URL strings.

**Coordination request to Agent 02 and Agent 01:** design an additive migration and typed source reference (project ID + transcript/research entity ID + source revision/fingerprint, when available); validate ID and project membership in the host service; store source references and created prompt/result artifacts atomically. Keep legacy AI-run reads and migrations intact. Agent 06 should review the contract before editing shared persistence/service code.

**Coordination request to Agent 05:** integrate owned-view i18n and RTL refinements with the shared translation dictionary. Preserve the English labels used by existing interaction tests until both locales are tested.

## Security, provider and fixture validation

- No new outbound connections, credentials storage, telemetry, or auto-provider fallback.
- Provider response limits and redirect denial remain in force.
- Provider fixture tests exercise OpenAI Responses, Anthropic Messages, Gemini generateContent, OpenAI-compatible chat completions, error status, malformed JSON, timeouts, redirects, and redaction. They are **local HTTP fixtures, not paid live-provider certification**.
- Imported transcript/research text remains **untrusted** and is rendered as data in Svelte. The host prompt builder currently inserts source text in the same text channel as task instructions; prompt-injection-resistant source delimiting and source-ID verification require a coordinated change in `apps/desktop/src-tauri/src/services.rs`.
- Research observation ratios do not establish unique-user engagement, causal creative performance, or real-time velocity; no such claims are made.

## CI gates and reproduction

Opening the PR against `feature/chatgpt-mcp-ci-20261026` invokes:

- `Rust foundation`: fmt, check, Clippy, full Rust workspace tests.
- `Desktop UI`: Svelte/type checks, Vitest interaction tests, production build.
- `CI`: repository hygiene, frontend checks/build, Python sidecar tests, Rust workspace tests.
- Other matching GitHub Actions workflows may also run.

Targeted tests for the changed code:
```sh
cargo test -p scriptotar-ai -p scriptotar-research --locked
cd apps/desktop-ui
npm ci
npm run check
npm test -- --run
npm run build
```

No local fixture should be represented as live-provider validation. Keep failures visible and do not alter CI protection to obtain a green check.

## Deferred tasks (do not implement without coordinated ownership)

1. Project-validated persisted source/artifact links and transcript revision stamps (Agent 02 + Agent 01).
2. Historical AI-result detail/open/reuse endpoints and a versioned editable template library, backed by persistent entity IDs.
3. Research-to-AI explicit item/corpus selection and creator comparisons, preserving source confidence.
4. Host-owned untrusted-context quoting/instruction separation and secure BYOK credential-boundary review (Agent 01/03).
5. Full Arabic translation of local AI/research labels (Agent 05).

**Merge rule:** Only Agent 00 may review/integrate; Agent 06 never merges its own PR.
