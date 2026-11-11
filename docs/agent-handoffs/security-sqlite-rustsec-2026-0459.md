# Security Agent — RUSTSEC-2026-0459 SQLite dependency remediation

**Issue:** https://github.com/MousaXD/scriptotar/issues/97  
**Branch:** `fix/scriptotar-sqlite-rustsec-2026-0459`  
**Source baseline:** `main` at `f741244634b5143602a21a554094a498a0005b11`  
**Verdict:** **BLOCKED until exact-HEAD GitHub Actions and packaging pass.** Do not merge automatically.

## Root cause and impact

Scriptotar's `rusqlite = "0.32"` resolves to `rusqlite 0.32.1` / `libsqlite3-sys 0.30.1` with `bundled` and `backup` enabled. The bundled native SQLite can expose uninitialized page-cache memory when opening a locally crafted database and incompatible WAL pair. See [RUSTSEC-2026-0459](https://rustsec.org/advisories/RUSTSEC-2026-0459.html) (CVE-2026-119375). RustSec requires `libsqlite3-sys >=0.39.0`, bundling SQLite >=3.54.0.

Do **not** merge Dependabot PR #44 as the fix: it updates to `rusqlite 0.40.2` but its lockfile still resolves `libsqlite3-sys 0.38.2`, below the patched floor. That PR is a reference candidate only.

## Narrow dependency update

| Package | Before | This branch | Reason |
|---|---|---|---|
| `rusqlite` | 0.32.1 | 0.41.0 | First patched upstream release line; Rust 1.89 minimum |
| `libsqlite3-sys` | 0.30.1 | 0.39.1 | Includes bundled SQLite 3.54.1; patched |
| `hashlink` | 0.9.1 | 0.12.1 | Existing default cache feature compatibility |
| `hashbrown` | 0.14.5 removed | 0.16.1 added; 0.17.1 retained | Transitive from `rsqlite-vfs` / `hashlink` |
| `rsqlite-vfs` | absent | 0.1.1 | Transitive wasm backend dependency |
| `sqlite-wasm-rs` | absent | 0.5.5 | `rusqlite` default feature's wasm dependency |

Unreferenced `ahash 0.8.12`, `hashbrown 0.14.5`, `zerocopy 0.8.56` and `zerocopy-derive 0.8.56` are removed. Existing `foldhash 0.2.0` is reused. All unchanged dependencies retain their baseline versions. Registry checksums and crate metadata came from the official [crates.io index](https://github.com/rust-lang/crates.io-index).

The exact feature declaration `["bundled", "backup"]` is preserved. The workspace toolchain in GitHub Actions pins Rust 1.97.1 (meets upstream Rust 1.89.0 minimum). `libsqlite3-sys 0.39.1` uses MIT; `rusqlite 0.41.0` uses MIT. New transitive crate license details still require the CI/package license inventory check.

## Database and API compatibility

Static inspection of `scriptotar-db` and desktop migration sources found standard `Connection`, `TransactionBehavior::Immediate`, `OpenFlags`, `pragma_update`, and `backup::Backup::new(...).run_to_completion(...)` use. Upstream 0.41 retains these APIs; no source changes were made. The newer `rusqlite` release changes several optional VTab/statement APIs, but no matching call sites were found in inspected owned database and desktop migration sources. Compile/test results must verify this assessment.

Schema migrations are unchanged: main and integration schema versions both end at v5. Existing repository tests exercise v1/v2/v3/v4 upgrades, migration ledger validation, failed-transaction rollback, WAL/read-only snapshots, legacy import/idempotence, project lifecycle, job concurrency and recovery, transcript lineage, AI/research, watchlists and settings. New `sqlite_dependency_compatibility.rs` tests assert a patched runtime SQLite version; exercise WAL, online backup, data restoration; rerun schema migrations; and reject a tampered ledger without deleting project data.

This is an application dependency upgrade, not an on-disk schema migration or database reset. Old database compatibility and file preservation still require exact-HEAD CI; the static review is not proof.

## Dependency chain and validation evidence

Expected inverse tree: `scriptotar-db -> rusqlite 0.41.0 -> libsqlite3-sys 0.39.1` and `scriptotar-desktop -> rusqlite 0.41.0 -> libsqlite3-sys 0.39.1` (also via `scriptotar-db`). A real `cargo tree -i libsqlite3-sys` on the final commit is still required.

Local execution was unavailable in the authoring container (no Cargo/rustc and no external DNS), so **no local cargo audit, fmt, check, clippy, test, or package build is claimed**. This branch relies on GitHub Actions exact-HEAD checks. Requirements:

- `cargo tree -i libsqlite3-sys`, `cargo audit`, `cargo fmt --all -- --check`
- `cargo check --workspace --all-targets --locked`
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`
- `cargo test --workspace --locked` and `cargo test -p scriptotar-db --locked`
- mandatory Tauri migration / Supply chain, Rust foundation, MCP E2E, Linux installed-deb, Windows clean-machine NSIS installation and database smoke workflows.

Do not waive audit findings or approve release without packaging, database-upgrade and exact-HEAD CI evidence. Other agents (especially Agent 03) own source changes; report any API incompatibility to Agent 00 instead of editing their implementation.

## Handoff

The source of truth for final exact-HEAD SHA, CI job results and any follow-up incompatibilities is the linked PR conversation. Owner Agent 00 must independently approve this branch, coordinate with Agent 03 and re-run integrated checks on the final merge candidate. **GO only after all required checks pass**.
