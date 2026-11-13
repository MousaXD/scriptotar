# Agent 05 — research intelligence handoff

**Owner:** Agent 05 (research only)  
**Branch:** `agent-05/defragmentation-research-intelligence`  
**Base PR target:** `plan/scriptotar-defragmentation`  
**Merge/release:** Not authorized. Do not merge.

## Delivered within exclusive ownership

- `crates/scriptotar-research/src/intelligence.rs`: offline metric snapshot normalizer, project-scoped exact-URL/time dedupe, creator/age-matched median comparisons, two-observation view deltas and timestamped literal transcript outline.
- `crates/scriptotar-research/src/intelligence_tests.rs`: fake data for missing counts, real zeros, negatives, invalid dates, rejected URLs, project collision, duplicates, contradictory metrics, missing/age-incompatible creator peers, stale data, counter decreases, empty/partial/denied transcripts, invalid timecodes.
- `crates/scriptotar-research/Cargo.toml` + `src/lib.rs`: expose the new independent module and use the existing workspace `chrono` dependency.
- `docs/defragmentation/proposals/agent-05-research.md`: proposal for Agent 00/01/02 and integration/P2 design.

## What actually runs

All new functionality is pure Rust with local inputs and explicit limitations. `MetricSnapshot::from_research_item` consumes the **existing** `ResearchItem` type. A separate `TranscriptScope` contract accepts explicitly scoped transcript segments; its deterministic output is only literal spoken evidence with timecodes. No new source providers, network I/O, persistence, UI controls, AI model calls, Study ingestion endpoints, frame extraction or video understanding were added.

The existing research provider/yt-dlp behavior and watchlist/queue paths were **not modified**. Provider access denials, cookies, URL validation and source-access constraints remain under existing application policy.

## Verification notes

- Offline deterministic unit tests are included in the crate but are not equivalent to running them.
- No local Rust toolchain is available in the editing environment. Rustfmt, Cargo check, Clippy and workspace tests require the PR's GitHub Actions `Rust foundation` and `CI` checks.
- No real provider scan, installed app, media frame check or live-authenticated platform access was executed in this task.
- As of handoff creation, PR CI verdict is **pending / unverified**. Update after inspecting exact-head checks; do not claim green prematurely.

## Integration blockers and contract requests

1. Agent 01: approve project FK relationships, revision/fingerprint and durable snapshot schema. This branch makes **no** migration.
2. Agent 02: align typed DTOs and project-authorization gate; the self-reported `TranscriptScope` is not sufficient authorization.
3. Agent 04 agent runtime: expose explicitly bounded tool operations after P0 acceptance, preserve partial/denied source status.
4. Agent 03 Studio: display true observation dates, missing/insufficient peers, and excerpt citations without viral/retention claims.
5. Agent 00: review module public interface and rebase/merge only when P0 source contracts are stable and exact-head CI passes.

## Known limitations

- Exact normalized URL is treated as source identity; equivalent alternative social URLs are **not** merged without a future canonical provider ID.
- Dates without recognizable YYYY-MM-DD or `unix:<seconds>` become Unrecognized and cannot qualify for age matching.
- Observational counters alone do not prove real account-wide reach, retention, share rate, post timing causality or guaranteed virality.
- Only opening/middle/closing transcript excerpts are extracted today; hook taxonomy, creative pattern hypotheses, payoffs and CTA detection await an evidence-grounded extractor and review.
- Historical metrics will be available only once a separately approved migration records **real** successive snapshots.

## Next independent actions

Run `cargo fmt --all -- --check`, `cargo check --workspace --all-targets --locked`, `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`, and `cargo test -p scriptotar-research --locked`. Correct findings on this branch only, then update this handoff and PR with actual results. Coordinate P1 wiring with Agent 00 before touching any shared code.
