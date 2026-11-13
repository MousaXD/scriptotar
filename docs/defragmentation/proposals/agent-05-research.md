# Agent 05 — Content research intelligence proposal

**Branch:** `agent-05/defragmentation-research-intelligence`  
**Target:** `plan/scriptotar-defragmentation`  
**Scope:** P1-02/P1-03 preparatory library; P2-01 observation primitives; P2-02 contract planning only.  
**Contract status:** Proposed for Agent 00/01/02 review, **not integrated** with Content Study persistence, API, Studio, or agent runtime.

## Inspected baseline

- `scriptotar-research/src/lib.rs` already has strict allowed social hosts, URL revalidation after provider scan, capped yt-dlp output, user-selected browser-cookie configuration, nullable numeric observations, and `ResearchItem` conversion.
- Current core `ResearchItem`: `id`, `project_id`, optional `creator_id`, `source_url`, platform, nullable counts, optional date/duration, `scanned_at`. There is **no historical metric snapshot table**.
- Research UI filters and sorts on raw views, likes, comments, publication time and (likes + comments)/views, and uses watchlist statuses. This observational rate already excludes missing/zero views; it is not retention.
- `docs/SOURCE_ACCESS.md` documents environment-dependent YouTube/TikTok/Instagram source denials. Local access, valid download, inference, and accurate transcript are different checks; no provider access is promised.
- P0 domain/API and Study contract is still separately owned. This branch must not invent persistent study foreign keys or change shared DTOs.

## Implemented standalone Rust API

All public types/functions below live under `scriptotar_research::intelligence`. No network or provider call is made.

| API | Intent and guardrails |
| --- | --- |
| `MetricSnapshot::from_research_item(&ResearchItem)` | Validate allowed URL, RFC3339 observed-at, nonnegative counts; retain missing vs zero and unrecognized upload date. |
| `deduplicate_snapshots(Vec<MetricSnapshot>)` | Deduplicate exact canonical URLs at equal observed timestamps *within a project*; reject contradictory metrics and identical research-item IDs across projects. Do not drop identity-bearing URL queries. |
| `MetricSnapshot::observed_interactions_per_view()` | Only output a descriptive ratio when views > 0 **and** likes/comments are both present. |
| `compare_creator_views(target, peers, as_of, max_stale_days)` | Use same project/creator/platform, distinct source URLs, latest observation per peer URL, age window 0.5x–2x of target video age, at least **3 valid peers**, no stale/future observations; yield unavailable reason otherwise. Compare views to median, not to an unexplained virality score. |
| `observed_view_growth(older, newer)` | Two actual sequential timepoints of same project/source, two view counts, strictly increasing time and nondecreasing count. Return observed delta and views/day or a typed limitation; no extrapolation. |
| `extract_transcript_outline(scope, segments, access)` | Deterministically produce bounded opening/middle/closing **spoken excerpts** with segment IDs and timecode millisecond ranges. Distinguish Partial/Denied/Unsupported, empty speech, invalid segments. No fake visual, hook type, CTA, payoff or inferred structure. |
| `TranscriptScope`, `EvidenceRef`, `CreativeObservation`, `SourceConfidence` | Suggested evidence payloads for Agent 00 to map to persistent project-checked Content Study provenance after contract approval. |

### Observed metrics vs uncertain claims

- Views, likes and comments from yt-dlp are **observations**, not audited platform analytics. Counts can be delayed, inconsistent or suppressed by an inaccessible source.
- Stale is explicit: callers supply an `as_of` instant and recency cap. No UI clock or historical backfill is fabricated.
- Publication date may be missing, recognized or unrecognized. The creator comparison intentionally abstains if target age is unavailable or the peer sample is too small.
- The creator-relative views ratio is descriptive, not evidence of audience retention, engagement cause, platform boosting, or a future hit.
- Growth is only the *difference between two observed snapshots*. It is not guaranteed growth of unique people or normalized reach.

## Evidence and creative extraction contract

`TranscriptScope` carries the real project/research/transcript IDs and a caller-supplied immutable transcript revision. `TranscriptSegment` requires a unique segment ID, ordered start/end times in **milliseconds**, and bounded text. `EvidenceRef` retains the source scope, segment ID, timecodes, and up to 160 Unicode characters of excerpt. This is a locator, not a whole copyrighted script.

The offline outline is literal positional evidence, not semantic analysis. A later human-reviewed or AI-assisted extractor can attach *hook hypothesis, opening promise, narrative beat, pacing, payoff, CTA placement and rhetorical device* as structured **hypotheses** with these exact evidence references, explicit transcript-only modality, separate confidence, and counterexamples. Pacing inferred from words per second is a spoken-text estimate, never a visual edit/cut rate. No visual evidence or frame inspection is implemented here.

Before persistence, Agent 01/02 must verify each `project_id`, `research_item_id`, and `transcript_id` belongs to the same project and that the revision matches the stored source. A self-reported Rust struct is not database authorization. Factual assertions require independently corroborating sources; repeated reels are not independent fact verification.

## Source access, compliance and P2 design

Do not add cookie extraction, authentication bypass, new platform promises, unrestricted URLs, or direct external requests in this crate. Provider-denied/unsupported input returns a typed failure; partial transcripts may yield partial *spoken* evidence with a warning. The existing research provider and original URL validation remain unchanged.

**P2 snapshots proposal (not a migration):** Agent 01 should consider a project-scoped unique key over `(project_id, source_id, observed_at, provider)`, plus collector timestamp, raw provider timestamp when available, metric availability/quality flags, and immutable provenance. Require a second real observation before velocity calculations; never synthesize histories from re-scans of old UI state. Preserve contradictory provider reports rather than averaging them silently. Evaluate creator/channel membership, dedupe canonical IDs and account-size denominators before generalizing.

**P2 visual proposal (not implemented):** rights-aware optional local frame sampling, visual evidence locators, bounded storage/CPU/GPU, explicit permission and confidence; separate transcript-excerpt claims from verified frames. Cannot claim visual understanding today.

## Integration handoff / dependencies

1. **Agent 01** freezes Content Study and EvidenceRef schema, project FK checks, deletion/revision behavior and optional snapshot migration.
2. **Agent 02** maps project-scoped DTOs and query pagination; do not expose unchecked IDs or provider credentials.
3. **Agent 04 (runtime)** may call these pure functions through approved tools, never execute arbitrary URLs or infer missing evidence.
4. **Agent 03 (frontend)** can surface unavailability, peer count, observed-at, confidence and traceable source segment, never a `viral score`.
5. **Agent 00** reviews types and adds the cross-layer seam only after P0 contracts and CI.

Test coverage in `intelligence_tests.rs` uses fake local data. No live provider/media/installed-app certification is implied. See the handoff for exact validation status.
