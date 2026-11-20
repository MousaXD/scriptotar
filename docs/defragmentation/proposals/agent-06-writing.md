# Agent 06 — isolated intelligent script-writing proposal

**Status:** Implemented as an independent Rust library foundation; not connected to Content Studies, the Tauri services, provider execution, persistence, or the agent runtime.
**Branch:** agent-06/defragmentation-script-intelligence
**Target:** plan/scriptotar-defragmentation
**Ownership:** crates/scriptotar-ai/** and this proposal / Agent 06 handoff only.

## Baseline findings

- The current UI has eight task presets: Viral breakdown, Hook ideas, New short-form script, Structure remix, Content ideas, Caption + CTA, Voice profile, and B-roll shot list.
- The current host service constructs a single Markdown-oriented prompt from a free-form source-text field and user brief; it does not bind durable source IDs or evidence. BYOK execution, Copy Prompt semantics and history writes remain in the Tauri service and are unchanged by this branch.
- The scriptotar-ai crate already offers an AiRequest/AiResponse, AiService, AiProvider and endpoint policy for OpenAI, Anthropic, Gemini and compatible endpoints. Reusing this existing interface avoids credential ownership or network code in the writing layer.
- ProviderKind::Local exists as a configurable compatible endpoint path, but the present UI does not expose a tested local generation workflow; this proposal makes no local-LLM support claim.
- Source text is untrusted; putting it in a JSON data envelope and telling a model to disregard embedded commands is defense in depth, **not a prompt-injection security boundary**. Host-level permissions, API egress confirmation and output validation are required.

## Contract (provisional; no P0 schema assumed)

The module is exposed as scriptotar_ai::writing and uses serde-serializable types independent of SQLite/UUID or the UI.

| Layer | Types / behavior |
| --- | --- |
| Intent | CreativeBrief (goal, platform, audience, language, region, dialect, tone, duration, objective, CTA, constraints), StyleGuide (user-authored vocabulary/pacing/avoid-list) |
| Sources | EvidenceRef (opaque ID, source locator, excerpt, kind, declared status), ResearchFinding (creative pattern / factual assertion / counterexample, evidence IDs, limitations) |
| Composition | AngleOption with distinctive hook/approach and timed OutlineBeat; ScriptDraft with ScriptBeat roles, narration, on-screen text, shots, CTA, caption and FactualClaim |
| Generation | WritingInput + WritingTask (propose angles, draft, review, revise beat) → prepare_writing_request → existing AiRequest |
| Parsing | parse_angles, parse_draft, parse_review, parse_revision_proposal require strict JSON input and validate links/limits |
| Protection | apply_beat_revision checks optimistic expected narration and manual-edit guard; modifies only the requested narration |
| Quality | audit_draft and compare_angles return advisory findings and a speaking-duration estimate without numerical viral scores |

**Important distinction:** a declared EvidenceStatus::Corroborated is metadata provided by a caller; it is *not* verification by scriptotar-ai. Only host-validated independent documentation can be eligible for a script's supported factual claim. A model cannot invent evidence IDs or unilaterally grant itself the right to overwrite a manual edit.

### Proposed P0 integration contract (Agent 00/01/02 approval required)

1. Host loads Content Study sources using exact project-scoped persisted IDs, revision/content fingerprints, locator/timestamp and collected-at time. Verify project ownership and provenance before constructing any EvidenceRef. Never infer trust from a web domain or a creator popularity metric.
2. Agent 05 supplies research observations and source annotations. Treat video metrics and transcripts as creative information, not independent factual corroboration. Explicit human/document-review authorization is required before mapping facts to Corroborated.
3. Agent 04 selects and records WritingTask steps and requires user authorization before paid provider execution. The writing crate produces an AiRequest only; it does not make network calls, store keys or invoke a local model.
4. Agent 02 may adapt the data and use the existing AiService interface. JSON parsing failures, source-version mismatches, cross-project IDs and unsupported claims must be visible errors, never marked successful.
5. Agent 01 stores immutable DraftVersion and evidence relations with optimistic concurrency. Revision proposals are staged; the host applies a validated BeatRevision only on user acceptance, and persists a new draft version. Preservation tests must cover project switch, interruption and concurrent manual editing.
6. Agent 03 consumes these types through approved shared DTOs (not from guessed generated JSON), retains Copy Prompt, and exposes citations and advisory notes without hiding missing evidence or key/session consent.

## Input limits and outputs

- 5–600-second requested duration; two to five alternative angles; up to 40 evidence items and 30 findings; 1,200 characters per excerpt; 100,000-character prepared prompt ceiling.
- Factual claims marked supported require a reference to a *declared corroborated independent document*. A declared social transcript alone cannot satisfy this check.
- Model output is parsed as structured JSON and IDs are checked against the supplied evidence, but the library does not independently authenticate those IDs. Reject malformed/unbounded or unsupported structured responses; do not pretend they are valid drafts.
- The draft editor owns accepted narration and manual edits. A model can return a proposed replacement, not an override flag, and apply_beat_revision checks stale content before change.
- Duration uses a transparent rough 150 words/minute for English and 125 words/minute for Arabic. Other languages currently use the English heuristic. Speech cadence and on-screen pacing require manual review.

## Quality and security expectations

The following are *review cues*, not assertions of correctness: generic hooks, similar angles, repeated narration, divergent estimated duration, missing declared claims/citations, unsupported/uncertain factual claims, CTA/brief alignment and eight-word normalized source overlap. Lexical copying checks can miss paraphrased infringement and flag ordinary phrases; they are not copyright clearance.

Transcripts, social captions, research notes and articles remain untrusted, even if a model response claims higher privilege. The prompt envelope contains typed JSON to help separate source content from application instructions, but an LLM can still be prompt-injected. Authorization, tool calls and project access stay strictly outside the language model. Provider execution and data egress require the same explicit user action as existing BYOK.

No API keys are new model fields. No filesystem/network or credential capture is introduced. No unexpected provider calls are made on request preparation. No direct social-service calls are made.

## Deterministic examples from committed fixtures

- Independent document evidence ID event: an event notice declares a Saturday 6pm library gathering (illustrative fictional fixture, **not a real-world verified event**).
- Social-transcript pattern evidence ID pattern: a script structurally opens with a surprise and ends in a payoff, with no visual/retention inference.
- Manual note evidence ID injection contains a fake high-priority order to disclose an API key; the note stays data and does not change library behavior.
- Alternative angles in tests/fixtures/angles.json: a contrast-based neighborhood surprise; a first-person micro-story; a three-step quick guide. All are original test writing, not excerpts from a creator.
- Draft in tests/fixtures/draft.json: an original neighborhood hook, one evidence-linked event-detail beat, one CTA. The event beat is marked as a manual edit and resists unauthorized replacement.
- An English prompt retains goal/platform/audience/constraints. An Arabic prompt retains Arabic text and خليجية dialect instructions. Both are prompt fixtures, **not certified provider-generated Arabic output**.

## Unsupported and deferred

- No real media access, source retrieval, web/document verification or visual analysis; no claim to infer retention or guarantee engagement.
- No execution agent, streaming provider response, paid provider call, auto-publishing, draft persistence or migrations.
- No historical-run navigation, editor autosave, project scope validation or generated GUI.
- No verified local/offline LLM and no provider calibration; existing eight tasks remain available unchanged.
- No single-package or installed Windows/Linux certification from local unit tests.

## Test plan and readiness

Target check: cargo test -p scriptotar-ai --locked; cargo fmt --all -- --check; cargo clippy -p scriptotar-ai --all-targets -- -D warnings. Current repo's CI should also check regressions in the existing provider fixtures. Real-provider tests need opt-in and must not be confused with offline tests.

**Readiness:** ready for P0 contract review and isolated tests; not ready to merge into a working Content Study, ship as an autonomous writer, or release. Agent 00 alone decides integration.

## Post-review hardening (continuation)

Additional isolated checks now reject model responses above 100,000 characters and draft outputs that declare a different language from the requested brief (including common English/Arabic name/code aliases). Generated draft beats cannot assert editor-controlled manual-edit flags. Model review findings always remain human-review suggestions even if the model supplies a false negative review flag. Angle outline beats must be nonempty and have positive bounded estimated durations.

The existing targeted-revision function now drops the modified beat's stale narration evidence links and downgrades any associated previously supported factual claims to **unverified**, clearing their old claim citations. This prevents an editor-approved rewrite from inheriting evidence that only justified its prior wording. Unchanged beats, claims, and no-op revisions retain their provenance. The app must still obtain fresh citations and a user-approved new DraftVersion before marking revised claims supported.

These are structural checks on declared metadata; the module does not detect the actual spoken language, independently verify a factual assertion or reliably detect all copied phrasing. Integration and permissions remain Agent 00/01/02/04 responsibilities.
