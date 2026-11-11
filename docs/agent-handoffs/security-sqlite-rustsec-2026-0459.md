# Scriptotar SQLite supply-chain remediation — final security-agent handoff

**Issue:** [#97](https://github.com/MousaXD/scriptotar/issues/97)  
**PR:** [#98](https://github.com/MousaXD/scriptotar/pull/98)  
**Owned branch:** `fix/scriptotar-sqlite-rustsec-2026-0459`  
**Original main baseline:** `f741244634b5143602a21a554094a498a0005b11`  
**Fully validated source/test commit:** `30575059e6c69a98150c4da744022cb7b41e90c1`  
**Verdict:** **GO for Agent 00 independent integration review; NOT permission to merge or release.**

This document is a follow-up documentation-only update to the already validated source commit. The latest PR HEAD must have its own green checks before merge; see the PR for the current exact-HEAD status. The original source and test changes are unchanged.

## Root cause

The application's workspace `rusqlite = "0.32"` and its `bundled` feature resolved to `rusqlite 0.32.1 -> libsqlite3-sys 0.30.1`. RustSec [RUSTSEC-2026-0459](https://rustsec.org/advisories/RUSTSEC-2026-0459.html), CVE-2026-119375, describes uninitialized memory disclosure from crafted SQLite database and WAL data. The patched `libsqlite3-sys` floor is **0.39.0**, bundling SQLite **3.54.0+**. This vulnerability failed the mandatory Rust advisory job on the old lockfile.

Existing Dependabot [PR #44](https://github.com/MousaXD/scriptotar/pull/44) is **not a fix**: it upgrades `rusqlite` to 0.40.2 but its lockfile still resolves `libsqlite3-sys 0.38.2`, below the safe floor. Do not merge #44 for advisory remediation.

## Applied dependency update

| Package | Before | This PR | Notes |
|---|---|---|---|
| `rusqlite` | 0.32.1 | **0.41.0** | Upstream Rust 1.89.0 MSRV; CI Rust 1.97.1 meets it |
| `libsqlite3-sys` | 0.30.1 | **0.39.1** | Patched; bundled SQLite 3.54.x |
| `hashlink` | 0.9.1 | 0.12.1 | Updated for rusqlite caching |
| `hashbrown` | 0.14.5 | 0.16.1 and 0.17.1 | 0.17.1 already existed; transitive adjustment |
| `rsqlite-vfs` | absent | 0.1.1 | New wasm graph transitive |
| `sqlite-wasm-rs` | absent | 0.5.5 | New rusqlite wasm default-feature transitive |

Orphaned `ahash 0.8.12`, `hashbrown 0.14.5`, `zerocopy 0.8.56`, and `zerocopy-derive 0.8.56` were removed; existing `foldhash 0.2.0` was reused. Unrelated dependencies were held at baseline versions. Registry checksums match the crates.io index. The workspace's `rusqlite` feature declaration stays **`["bundled", "backup"]`**. No application backend implementation, schema migration, installer workflow, source data, or audit policy was edited.

**Lockfile inverse dependency audit (derived from Cargo.lock):**

```text
scriptotar-db 1.3.0 ─────┐
                         ├─> rusqlite 0.41.0 ─> libsqlite3-sys 0.39.1
scriptotar-desktop 1.3.0 ┘
```

`scriptotar-desktop` also pulls `scriptotar-db`; other Rust crates depend transitively on the DB crate. Only one `libsqlite3-sys` version exists in the new lockfile. This inverse chain was independently calculated from the committed lockfile; **the literal command `cargo tree -i libsqlite3-sys` has not been captured** because the GitHub connector cannot execute arbitrary shell commands and local Cargo is unavailable. Agent 00 should capture that command on the integration candidate if a literal command transcript is required.

## Compatibility and data safety

Both existing migrations and new tests retain main and integration schema version **5**. Source review located the used `rusqlite` APIs: `Connection`, `OpenFlags`, immediate transactions, PRAGMAs, and `backup::Backup::new(...).run_to_completion(...)`. Their compatibility is now corroborated by passing locked compilation and the complete Rust tests.

New `crates/scriptotar-db/tests/sqlite_dependency_compatibility.rs` covers patched bundled SQLite runtime version (>=3.54.0), WAL mode, online backup/quick-check, project/job/settings restoration, idempotent reopening of migrations, and rejection of tampered migration ledgers without erasing project data. Existing tests additionally cover historical schema upgrades, repeatability, failed-migration rollback, transaction and concurrency behavior, recovery, legacy import and snapshot safety, project lifecycle, transcripts, research, watchlists, AI state, and MCP. Installed-app CI checks clean database initialization, packaged frontend/backend, and persistence (Linux repeatedly, Windows clean install).

**No database reset, migration deletion, relaxation of fail-closed checks, or removal of bundled SQLite occurred.** The production data format is unchanged.

## Security audit and warnings

The [successful Supply chain job in workflow #43017701668](https://github.com/MousaXD/scriptotar/actions/runs/43017701668) ran `cargo-audit 0.23.0`, fetched **1,453 RustSec advisory records**, scanned **494** locked crates, and passed with **zero reported vulnerabilities**. It emitted **nine warnings** (not vulnerability failures) involving the existing unmaintained `proc-macro-error` and `unic-*` crates and unsoundness warnings affecting `glib 0.18.5`, `rustix 1.1.4`, and `tokio 1.53.1`. There were eight warnings in the earlier failing audit ([#42931619259](https://github.com/MousaXD/scriptotar/actions/runs/42931619259)), which reported the removed SQLite vulnerability. The ninth is **RUSTSEC-2026-0486 for `rustix 1.1.4`**, and the baseline `main` lockfile already contains `rustix 1.1.4`. That warning does not originate from this dependency change. It may warrant separate coordinated dependency remediation; do not misreport zero warnings.

Supply chain also passed `npm audit --audit-level=high` (three low-severity findings remain) and `pip-audit` (no findings). No `cargo audit` ignore, suppression, exception, or workflow modification was introduced. `rusqlite` and `libsqlite3-sys` are MIT-licensed; the existing package distribution pipelines check their required license inventory artifacts. No upstream license-exception waiver was added.

## Fully completed exact-HEAD CI — source/test SHA `30575059e6c69a98150c4da744022cb7b41e90c1`

| Workflow | Result and validation |
|---|---|
| [Rust foundation #43017701772](https://github.com/MousaXD/scriptotar/actions/runs/43017701772) | PASS: `cargo fmt --all -- --check`, `cargo check --workspace --all-targets --locked`, strict `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`, `cargo test --workspace --locked` |
| [Full CI #43017701607](https://github.com/MousaXD/scriptotar/actions/runs/43017701607) | PASS: frontend/sidecar/workspace regression suite |
| [Security hygiene #43017701469](https://github.com/MousaXD/scriptotar/actions/runs/43017701469) | PASS |
| [Scriptotar MCP E2E #43017701556](https://github.com/MousaXD/scriptotar/actions/runs/43017701556) | PASS: MCP CLI, HTTP, auth and integration |
| [Tauri migration integration #43017701668](https://github.com/MousaXD/scriptotar/actions/runs/43017701668) | PASS: supply-chain audit, database migration/recovery, sidecar/frontend/Rust workspace, integrated desktop build |
| [Linux .deb #43017701584](https://github.com/MousaXD/scriptotar/actions/runs/43017701584) | PASS: package creation, installation, real application startup, multiple IPC/persistence test repetitions, MCP smoke, distributable license inventory; `scriptotar-linux-tauri` artifact |
| [Windows NSIS #43017701836](https://github.com/MousaXD/scriptotar/actions/runs/43017701836) | PASS: locked binary build, NSIS bundle, clean-machine install, installed SQLite initialization, runtime/MCP smoke, uninstall, `scriptotar-windows-tauri` artifact |

All seven workflows concluded **success** on this code commit. The prior formatting failure on `edf32d86...` was fixed in the validated source/test commit.

## Ownership and final decision

The modified files are exactly: root `Cargo.toml`, `Cargo.lock`, one *new* `crates/scriptotar-db/tests/sqlite_dependency_compatibility.rs`, and this handoff document. No Agent 03 or other agent's implementation files were touched.

**Security-agent determination: GO for Agent 00 review.** The library vulnerability and packaged compatibility are validated; the issue remains open and the PR remains unmerged. **Owner-level release determination: BLOCKED** pending independent cross-agent integration, regression certification on the eventual merged SHA, and owner approval. Agent 00 alone should decide merge order, version/changelog and release. Neither a passing isolated PR nor this handoff authorizes a direct merge to `main`.
