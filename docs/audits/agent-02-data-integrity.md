# Agent 02 — Scriptotar database, jobs and recovery audit

**Branch:** `agent-02/database-recovery-overhaul`  
**Target:** `feature/chatgpt-mcp-ci-20261026`  
**PR:** https://github.com/MousaXD/scriptotar/pull/74  
**Status:** Validation pending; do not merge until exact-head CI passes.

## Safety findings and remediations

| Severity | Finding | Remediation |
| --- | --- | --- |
| High | Legacy backup copied only the SQLite main database file. Committed WAL frames could be absent from backups. | Back up through the SQLite online backup API to a disposable snapshot; verify `PRAGMA quick_check`; read imports from the verified snapshot. |
| High | Legacy imports used deterministic record IDs but `ON CONFLICT DO UPDATE` overwrote existing jobs, sources, media, transcripts, research and AI rows when source data changed or another legacy file reused an ID. | Reject pre-existing target IDs before any updates; rollback the import transaction on collision. |
| High | A native project with the same case-insensitive name as a legacy project was silently adopted as its destination. | Reject all collisions with pre-existing project names on new imports, including deterministic IDs; exact repeat imports are skipped using recorded snapshot provenance. Do not merge unrelated projects automatically. |
| Medium | Legacy imports treated every unknown input type as a local file and every unrecognized AI mode as BYOK; unexpected statuses became interrupted. | Accept historical Classic values explicitly and reject unknown values transactionally. |
| Medium | Legacy source main-file size and whole-second mtime were used as an import fingerprint. WAL-only updates could be ignored. | Stream a deterministic content fingerprint over the complete online snapshot, including committed WAL data. |
| High | Publishing a legacy backup with `rename` could replace a pre-existing path after an existence-check race, including a dangling symlink on Unix. | Atomically publish the verified snapshot with an exclusive hard link; reject existing symlink/nonregular or mismatched backups, always clean up the temporary snapshot. |
| Medium | An inconsistent core or integration migration ledger could pass a MAX/COUNT-only check after a version is replaced with zero or a recorded migration name is modified. | Fail closed before running migrations; validate every ordered version/name entry against the recognized migration history. Reject gaps, malformed IDs, and unknown names before touching user data. |
| Medium | Retried jobs retained prior-run progress, so a new queued attempt could display nearly completed status. | Clear progress only on explicit requeue; retain last failure details until a successful completion. |
| Medium | Foreign keys did not enforce that creator, analysis, job and transcript links stayed within one project. | Add insert/update triggers for relevant project ownership relationships; add negative transaction tests. |
| Feature gap | Project rename, archive and restore were unavailable as database operations. | Add project rename and reversible `archived_at` migration, plus active/archived project listing methods. No destructive deletion added. |

## Compatibility and data preservation

- **Production data:** No live user databases were opened or mutated; testing uses disposable fixtures.
- **Existing schema:** Core migration v4 only adds a nullable `projects.archived_at` column, an index and ownership triggers. Integration migration v4 only adds lineage triggers. No tables or rows are dropped.
- **Legacy snapshots:** A new backup is created in the legacy file's directory using a unique temporary filename and a content-derived immutable backup name. An atomic same-filesystem hard link publishes without overwriting an existing path; this requires filesystem hard-link support. Source database is opened read-only. Failed imports may retain the safely captured backup to support manual recovery.
- **Project lifecycle:** IDs, creation timestamps, settings' active project ID, jobs, media and transcripts remain unchanged on rename/archive/restore. Existing `list_projects` behavior remains backward-compatible, including archived projects; new list methods allow the UI to filter them intentionally.
- **Collision policy:** Updated legacy files that reuse an imported ID are **rejected rather than merged**. A manual reconciliation/import policy is required for incremental source updates; user data is never overwritten to make an import pass.
- **Transaction policy:** Legacy inserts and import provenance are in one immediate SQLite transaction; failure rolls them back. Migrations are transactional.
- **Source compatibility:** The historical Classic source uses `file` and `url` inputs, `Done`/`Failed`/`Canceled` and active `Starting`/`Running`/`Finishing` job states, and `Copy prompt only`/`api` AI modes, all explicitly handled.
- **Fingerprint limitations:** The streaming 64-bit non-cryptographic FNV fingerprint detects ordinary content changes but is not a security guarantee against deliberate collision attacks; future cryptographic strengthening may be warranted.
- **Existing invalid links:** New triggers are forward-looking: the migration does **not** rewrite, relink or delete possibly inconsistent historical rows.

## Regressions added

- Core/integration migration-ledger gap detection, unexpected version-zero and renamed-ledger rejection, pre-existing record preservation and forced failed-migration rollback.
- Explicit retry progress reset, historical error retention, and success clearing.
- Consistent WAL snapshot, repeat import, changed-content rejection, immutable backup and dangling-symlink protection.
- Distinct legacy databases reusing IDs; unknown input-type rollback; native project-name collision.
- Project rename/archive/restore including reopen and settings preservation.
- Database-enforced creator/research/analysis and job-transcript project isolation.
- Simultaneous job-claim and crash-recovery contention tests, validating one successful claim and idempotent restart recovery.

## Open integration / architecture issues for Agent 00

1. **Agent 01:** If project lifecycle is exposed at the service layer, coordinate use of `rename_project`, `archive_project`, `restore_project`, `list_active_projects` and `list_archived_projects` without changing stable IDs.
2. **Agent 04:** Job crash recovery currently marks all active states interrupted. Multi-process/parallel orchestration needs an explicit ownership or lease design before recovering jobs while another worker may be alive. The current PR does not change the orchestrator.
3. **Agent 05:** Connect archive/restore and rename UI without implicitly deleting or silently switching the active project. Existing `list_projects` still lists all rows.
4. **MCP:** Database schema and `JobRepository` APIs remain backward compatible; ensure full MCP smoke tests succeed on the final PR head.
5. **Safe deletion:** No hard project delete implemented. Existing `ON DELETE CASCADE` relationships make hard deletes risky without an explicit backup/confirmation and retention policy.
6. **Incremental legacy reimport:** Cross-snapshot ID collisions intentionally block the whole import; later work should design source provenance and user-guided reconciliation if update imports are a feature requirement.

## Validation

GitHub Actions are the test authority for this branch. Required evidence before `READY FOR AGENT 00 REVIEW: YES`: `cargo fmt --all -- --check`, `cargo check --workspace --all-targets --locked`, `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`, `cargo test --workspace --locked`, and applicable PR workflows on the exact final commit. Any failures block promotion.
