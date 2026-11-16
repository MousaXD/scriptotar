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
| PR #114 — combined | **All four test groups on the exact integration HEAD** | FAIL if a required feature module has not been integrated; no SKIP-as-PASS |

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
