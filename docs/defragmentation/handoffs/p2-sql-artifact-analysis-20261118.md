# Agent 08 — independently verified migrated-SQL artifact analysis

**Date:** 2026-11-18. **Scope:** Python SQLite warm read-only query profiling of *synthetic* 100 / 1,000 / 10,000 canonical Studies, migrated using real Scriptotar Rust schema v12. **Never classify these as native Tauri IPC, WebKitGTK/WebView2, editor responsiveness, or application startup timings.**

## Evidence integrity and reproduction identity

- Exact benchmark source: `01d915574cb117e3bbc47480efb96030097e735f`, [completed two-OS workflow #44299533922](https://github.com/MousaXD/scriptotar/actions/runs/44299533922).
- Linux ZIP artifact **14425855103** SHA-256: `13a578b3e6cefb29d3e46c23b1b1f344a430ca3ae53d54cbdc46f794a8f06a50`.
- Windows ZIP artifact **14423834139** SHA-256: `2d450a4d7dcaf8ffb2cf1ec7b7c4d44b133a1e16ad5372c8abea62f77d61176e`.
- Both artifact ZIP bytes were independently downloaded and verified against those published SHA-256 digests. Each archive includes `migrated-sql-report.json`, `sql-{100,1000,10000}.json` with raw timing samples, `EXPLAIN QUERY PLAN`, and fixture manifests.
- Samples per query-size-OS combination: **11**, warm Python SQLite `SELECT + fetchall`, p95 **nearest rank**, no OS cache cold-start control, no matched hardware beyond hosted environment. Linux: Python 3.12.16 and SQLite 3.37.2, 4 logical CPUs. Windows: Python 3.12.10 and SQLite 3.49.1, 4 logical CPUs. Different hosts and SQLite versions prevent causal cross-OS comparisons.
- Fixture DB sizes in both environments: ~4.2 MB at 100; ~14.92 MB at 1,000; ~122.27 MB at 10,000 Studies. Canonical v12 database rows are **synthetic** and study counts do not establish an installed-app workload.

## Actual warm SQL distributions (milliseconds)

Each cell is `p50 / p95-nearest-rank` calculated from **11 raw samples**, not desktop interaction latency.

| SQL query and synthetic Studies | Linux | Windows |
|---|---:|---:|
| Study search (up to 200 metadata), N=100 | 0.2835 / 0.3191 | 0.3405 / 0.4126 |
| Study search, N=1,000 | 1.8682 / 1.9124 | 2.1657 / 2.3012 |
| Study search, N=10,000 | 18.0529 / 18.1564 | 22.3999 / 27.4326 |
| Transcript metadata (up to 50), N=100 | 0.1150 / 0.1437 | 0.1212 / 0.1735 |
| Transcript metadata, N=1,000 | 0.7725 / 0.8492 | 0.9210 / 1.0296 |
| Transcript metadata, N=10,000 | 11.7849 / 12.0149 | 17.6649 / 18.3786 |
| Draft selector, N=100 | 0.0081 / 0.0098 | 0.0185 / 0.0252 |
| Draft selector, N=1,000 | 0.0083 / 0.0243 | 0.0185 / 0.0230 |
| Draft selector, N=10,000 | 0.0082 / 0.0226 | 0.0189 / 0.0217 |
| Single ~1 MiB transcript full body, N=10,000 | 0.1737 / 0.7835 | 1.1795 / 1.2962 |

The query plans for **Study search** use `idx_content_studies_project_updated` plus `USE TEMP B-TREE FOR ORDER BY`; **transcript metadata** joins indexed `sources`, `media`, `transcripts` and also uses `USE TEMP B-TREE FOR ORDER BY`. The draft selector uses `idx_creative_drafts_study_updated` with a temporary sort on the final ORDER BY term. These plan strings were extracted from the raw archives, not inferred from visual screenshots.

## Ranked *SQL investigation* candidates, not approved optimizations

1. **Study search/sorting.** Its synthetic warm query p50 rises from 0.2835 to 18.0529 ms on Linux and 0.3405 to 22.3999 ms on Windows between 100 and 10,000 Studies. The recorded temporary sort is a plausible contributor; profile the real Rust query parameters, row selectivity and matching index/ordering before changing schema. Only a native UI/IPC trace can establish that this affects user-perceived Studio readiness.
2. **Transcript metadata join/order.** Its p50 rises from 0.1150 to 11.7849 ms Linux and from 0.1212 to 17.6649 ms Windows. Review join/order selectivity, page limits and the temporary B-tree in the *real* read path after native trace correlation; do not replace with fake transcription evidence.
3. **Metadata vs full transcript payload.** Current synthetic draft-selector latency is stable at all three sizes; one ~1 MiB transcript read is not a substitute for real WebView IPC payload transfer or editor-render timing. Measure command payload bytes and both sides of IPC first.

## No false performance sign-off

These archived SQL timings are useful *hypothesis evidence*. They neither measure nor certify installed package cold/warm startup, useful Studio readiness, native UI operations, process private memory, real WebView IPC, or reproducible speedups. Platform variations mean it is invalid to conclude that Linux Scriptotar is faster than Windows. No regression threshold or optimization is proposed as a production change without native benchmark proof and Agent 00 approval. Native performance status: **NOT TESTED / NO-GO**.
