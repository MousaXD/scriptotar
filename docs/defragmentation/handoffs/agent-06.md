# Agent 06 handoff — script intelligence

**Assigned branch:** agent-06/defragmentation-script-intelligence  
**PR target:** plan/scriptotar-defragmentation  
**Owner:** Agent 06, no merge/release authority  
**Scope:** AI crate plus isolated writing tests/fixtures and Agent 06 documentation only

## Implemented

- New scriptotar_ai::writing Rust module with typed Brief, StyleGuide, EvidenceRef, ResearchFinding, AngleOption, OutlineBeat, ScriptDraft, ScriptBeat, ShotSuggestion, FactualClaim, ReviewFinding and revision proposals.
- Local-only structured prompt construction using the existing AiRequest type, with explicit untrusted-source instruction separation and no additional provider network surface.
- Strict structured result parsing for multiple angles, draft, review and revisions; bound sizes, reject invented citations, reject model-labeled supported facts without declared corroborated independent evidence.
- Revision safety: only one requested beat's narration is modified, with optimistic content comparison and explicit manual-edit override requirement. Provider output cannot request that override itself.
- Advisory hook, repetition, speaking-duration, citation, uncertainty, possible source-overlap, CTA and angle-distinctness checks. No numerical performance/virality prediction.
- Deterministic English, Arabic, prompt-injection, fake-evidence and manual-edit fixtures.
- Existing provider implementations, request-time keys, Copy Prompt path and eight legacy task presets remain unchanged.

## How to integrate (after P0 contracts are signed)

1. Agent 01/02: provide verified project-scoped source identity, revision, trust provenance and versioned draft DTOs, and adapt to WritingInput. Do not accept source-supplied corroborated claims without authorized review.
2. Agent 05: convert source-access-aware research observations into ResearchFinding, explicitly distinguishing factual evidence from stylistic/video observations.
3. Agent 04: treat prepare_writing_request as a plan step; gate calling existing AiService and persist approval/cost/checkpoint results.
4. Agent 02/03: parse provider response and stage candidates in the Content Study; allow manual correction of unsupported JSON and explicit copy mode without API execution.
5. Agent 00: require end-to-end project isolation, crash recovery, EN/AR, provider consent, installed-app QA and independent review before merge or release.

## Tests

crates/scriptotar-ai/tests/writing.rs tests structured prompts, distinct angles, fake citations, supported-claim metadata, revision staleness, manual-edit protection, attempted prompt instruction override, advisory quality and Arabic language/dialect preservation. tests/fixtures/{angles,draft}.json show the shape of expected data. The event notice is fictional.

## Remaining risks

- Corroboration is caller-provided metadata. This crate validates declared evidence links but cannot verify facts, authenticity or project scope.
- JSON prompt framing is defense in depth, not proof that model instructions cannot be hijacked; host authorization must remain independent.
- Duration and originality quality findings are lexical heuristics; they do not establish engagement outcomes or legal originality.
- Provider replies may be invalid JSON or omit annotations; surface a recoverable validation error, not a successful run.
- No P0 draft schema/wiring, offline LLM execution, provider integration testing or installed app validation is performed by this implementation.

**Integration verdict:** ISOLATED FOUNDATION / NOT PRODUCTION-READY until owner-reviewed shared contracts and end-to-end tests.

## Continuation hardening

- Model outputs now have a separate 100,000-character parse ceiling, with empty/zero-duration outline beat validation.
- Draft language metadata must agree with the brief (English/Arabic aliases recognized); actual language detection remains unsupported.
- New generated drafts cannot set manual-edit ownership; model review findings are always treated as needing human review.
- Approved narration changes clear stale beat citations and downgrade that beat's existing factual claims to unverified (claim citation links cleared). A no-op revision leaves provenance unchanged.
- Added regression coverage for manual flags, language mismatch, response bounds, revised factual support and no-op edits.
- Keep this draft PR unmerged until exact-head CI validation and the P0 source/draft contract is approved.
