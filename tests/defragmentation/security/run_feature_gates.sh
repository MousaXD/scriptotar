#!/usr/bin/env bash
# CI-owned independent feature tests. Run from the GitHub Actions workspace.
# Never silently skip missing feature implementations.
set -euo pipefail
gate="${1:?provider|migration|writing|verification}"
pr="${2:?expected PR number}"
echo "Audit SHA: $(git -C audit rev-parse HEAD)"
echo "PR #$pr feature SHA: $(git -C target rev-parse HEAD)"
test -f target/Cargo.toml || { echo "ERROR: feature checkout missing"; exit 3; }

case "$gate:$pr" in
  provider:118)
    test -f target/crates/scriptotar-ai/src/provider_consent.rs
    mkdir -p target/tests/defragmentation/security/tests
    cp audit/tests/defragmentation/security/Cargo.toml target/tests/defragmentation/security/Cargo.toml
    cp audit/tests/defragmentation/security/tests/provider_context_race.rs target/tests/defragmentation/security/tests/
    (
      cd target
      cargo test --manifest-path tests/defragmentation/security/Cargo.toml --test provider_context_race -- --nocapture
    )
    ;;
  migration:119)
    test -f target/crates/scriptotar-db/src/agent_effects_external.rs
    (
      cd target
      # Includes real historical v4/v5 upgrades, corruption and IDOR fixtures.
      cargo test --locked -p scriptotar-db
    )
    ;;
  writing:121)
    test -f target/apps/desktop/src-tauri/src/services/study_writing_persistence.rs
    test -f target/apps/desktop/src-tauri/tests/study_writing.rs
    cat audit/tests/defragmentation/security/canonical_writing_additional_tests.rs >> target/apps/desktop/src-tauri/tests/study_writing.rs
    (
      cd target
      # Compiles the unregistered canonical production persistence module via #[path].
      cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --test study_writing --locked -- --nocapture
    )
    ;;
  verification:117)
    test -f target/apps/desktop/src-tauri/src/services/study_verification.rs
    test -f target/crates/scriptotar-ai/src/writing/verification/mod.rs
    mkdir -p target/tests/defragmentation/security
    cp audit/tests/defragmentation/security/test_study_claim_boundary.py target/tests/defragmentation/security/
    (
      cd target
      python3 -m unittest discover -s tests/defragmentation/security -p 'test_*.py' -v
      cargo test --locked -p scriptotar-ai verification
    )
    ;;
  *)
    echo "ERROR: unsupported PR/gate mapping: $gate:$pr" >&2
    exit 3
    ;;
esac
