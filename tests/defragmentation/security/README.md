# Agent 10 — independent security regression execution

**Release status: NO-GO until the combined production integration is tested.**
Feature PR CI success is not a combined application security certificate.

## Required GitHub Actions gate

[Independent security regressions](https://github.com/MousaXD/scriptotar/actions/workflows/independent-security-regressions.yml)
runs against the **head of each named PR at checkout time**, never a pinned
historical success. The log prints the exact audit and feature commit SHAs.

| Gate | Real module / native test | Scope |
| --- | --- | --- |
| PR #118 — provider | Nested Cargo integration test `provider_context_race.rs` against the *actual* `scriptotar-ai` code | Revocation after reservation, deliberately concurrent scope change before permit, live read-lock/egress ordering, fail-closed missing lock and no provider replay |
| PR #119 — migration | `cargo test --locked -p scriptotar-db` on real SQLite databases | Historical v4/v5 upgrade preservation, v10 schema/ledger, cross-project and crash recovery |
| PR #121 — writing | Production-path `study_writing.rs` integration tests plus independent appended tests that call `CanonicalSqliteCandidateStore` | Three-draft atomicity, duplicate receipts, tampered payload, stale source fingerprint, forged provenance, model-supported claim quarantine |
| PR #117 — verification | `scriptotar-ai` verification unit tests + independent host contract | Source-content hashes, independent origin, high-stakes review, model-claim distrust, project/Study isolation, fail-closed unbound canonical draft |
| PR #114 — combined-v11 | **Real integration SHA, native v11 migration and canonical writing SQLite tests** | Runs even if the paid provider remains disabled; includes receipt/version immutability, historical upgrade/collision rollback, lineage, duplicate and three-draft atomicity. This PASS is NOT full-host certification. |
| PR #114 — combined | **All four feature groups plus native v11 writing integrity and real host registration checks on exact integration SHA** | FAIL if a guard/paid-effect adapter is absent or any native test fails; no SKIP-as-PASS |

This audit workflow is separate from `ci.yml`; its status must be configured
as a **required release/integration check** by Agent 00 before final signoff.
The PR #114 combined gate explicitly refuses to pass with only isolated feature
branches. Re-run after every final owner integration SHA.

No real paid request, API key, external network egress or public content
publishing is used by these tests. A successful mock provider alone is not
evidence that live HTTP adapter hard bounds and billing receipts are correct.

## Local execution (against the appropriate feature or combined checkout)

On a checkout that contains `crates/scriptotar-ai/src/provider_consent.rs`
and this independent test directory:

```sh
cargo test --manifest-path tests/defragmentation/security/Cargo.toml \
  --test provider_context_race -- --nocapture
```

For the rest, run against feature PRs or their **registered** combined
integration:

```sh
cargo test --locked -p scriptotar-db
cargo test --locked -p scriptotar-ai verification
python3 -m unittest discover -s tests/defragmentation/security -p 'test_*.py' -v
cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml \
  --test study_writing --locked -- --nocapture
```

The four independent Rust writing tests in
`canonical_writing_additional_tests.rs` are deliberately appended to Agent
05's canonical integration fixture by `run_feature_gates.sh`, not independently
compiled as a free-standing test crate. Calling only `cargo test --test
study_writing` without that appended file omits these extra tests.

The **three additional native SQLite v11 tests** in
`canonical_v11_integrity_tests.rs` run only on the combined PR #114 checkout
(after v11 is registered). They verify immutable batch receipts, immutable
structured first versions, cross-project forged receipt rejection and reopen
after process restart. The isolated #121 writing branch deliberately does not
need to carry Agent 00's v11 migration.

The separate **combined-v11** gate uses the same actual integration checkout but does not require a live paid adapter. The full **combined** gate still fails closed if paid-host authorization is not implemented. Do not equate a v11-only PASS with a release certificate.

The combined host gate copies `test_combined_host_wiring.py` and runs it via
Python unittest against the real `target/apps/desktop/src-tauri/src` tree;
then it runs `cargo test --locked -p scriptotar-desktop --lib` along with the
nested provider race suite and full native SQLite and writing tests. This
structural check fails closed for missing real host authorization/paid-effect
implementations, but cannot by itself prove an atomic lock is shared with
every Study/source/project mutator. The final exact-SHA native integration,
production transport and installed-app evidence remain mandatory.

The nested Cargo manifest uses its own test workspace. The normal root
`cargo test --workspace` **does not include** the provider race regression.

## Limitations / evidence labels

- **PASS** means the GitHub log records a completed passing test on an exact
  checkout and the tested path is part of the stated module.
- **FAIL** means a test assertion or compilation gate failed. A missing module
  in the combined integration fails deliberately rather than counting as a
  skip.
- **NOT TESTED** means there is no executed evidence for the required behavior,
  including native installed WebView, live remote credentials, provider receipt
  authenticity, sidecar sandbox escape, or release package-specific IPC security.
- Source code review, historical CI, queued jobs, and tests authored but not
  executed cannot be represented as PASS.
- An isolated unit test passing while a paid adapter is disabled is evidence
  of fail-closed behavior, **not proof of secure production paid egress**.

Owner production code remains outside Agent 10's scope; communicate failures
to Agents 00, 01, 02, 04 or 05 and keep the PR in NO-GO state.
