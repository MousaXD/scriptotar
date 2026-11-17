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
      cargo test --locked -p scriptotar-ai provider_consent
    )
    ;;
  receipt-v12:128)
    # Independently test Agent 02's NEW v12 recovery BEFORE owner integration.
    # This writes only the disposable 'target' CI checkout, never #128.
    test -f target/crates/scriptotar-db/src/agent_effects_external.rs
    python3 - <<'PY'
from pathlib import Path
target = Path('target/crates/scriptotar-db/src/agent_effects_external.rs')
source = target.read_text()
marker = '#[cfg(test)]\nmod tests {'
assert marker in source, 'cannot find actual v12 DB test fixture'
closing = source.rfind('}')
assert closing > source.index(marker), 'bad real DB fixture boundary'
assert not source[closing + 1:].strip(), 'DB test module is not last'
extra = Path('audit/tests/defragmentation/security/external_receipt_replay_additional_tests.rs').read_text()
target.write_text(source[:closing] + '\n' + extra + '\n' + source[closing:])
PY
    (
      cd target
      # All original five regressions plus new adversarial and positive
      # provider-scoping, retry, ACK, migration and restart tests.
      cargo test --locked -p scriptotar-db
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
  combined-v11:114)
    # This is an integration-source DB/writing gate, never full host approval.
    # It must still run while the guarded paid provider path is disabled.
    for file in \
      target/crates/scriptotar-db/src/writing_schema.rs \
      target/apps/desktop/src-tauri/src/services/study_writing_persistence.rs \
      target/apps/desktop/src-tauri/tests/study_writing.rs; do
      test -f "$file" || { echo "NOT INTEGRATED V11: $file" >&2; exit 4; }
    done
    cat audit/tests/defragmentation/security/canonical_writing_additional_tests.rs >> target/apps/desktop/src-tauri/tests/study_writing.rs
    cat audit/tests/defragmentation/security/canonical_v11_integrity_tests.rs >> target/apps/desktop/src-tauri/tests/study_writing.rs
    (
      cd target
      # Real versioned migrations, historical upgrades, collision rollback.
      cargo test --locked -p scriptotar-db
      # Real canonical SQLite three-draft transaction and independent v11 tests.
      cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --test study_writing --locked -- --nocapture
    )
    ;;
  combined:114)
    # This exact PR #114 head must have all feature owners integrated by Agent 00.
    # A missing module is a release-blocking FAIL, not a skipped test.
    for file in \
      target/crates/scriptotar-ai/src/provider_consent.rs \
      target/crates/scriptotar-ai/src/provider_openai_wire.rs \
      target/apps/desktop/src-tauri/src/services/provider_openai_wire.rs \
      target/crates/scriptotar-ai/src/writing/verification/mod.rs \
      target/apps/desktop/src-tauri/src/services/study_writing_persistence.rs \
      target/apps/desktop/src-tauri/src/services/study_verification.rs \
      target/crates/scriptotar-db/src/agent_effects_external.rs; do
      test -f "$file" || { echo "NOT INTEGRATED: $file" >&2; exit 4; }
    done
    mkdir -p target/tests/defragmentation/security/tests
    cp audit/tests/defragmentation/security/Cargo.toml target/tests/defragmentation/security/Cargo.toml
    cp audit/tests/defragmentation/security/tests/provider_context_race.rs target/tests/defragmentation/security/tests/
    cp audit/tests/defragmentation/security/tests/provider_wire_boundary.rs target/tests/defragmentation/security/tests/
    cp audit/tests/defragmentation/security/test_study_claim_boundary.py target/tests/defragmentation/security/
    cp audit/tests/defragmentation/security/test_combined_host_wiring.py target/tests/defragmentation/security/
    # Assert production wiring, not merely that feature files exist on disk.
    # Collect the structural authorization failure without skipping Rust host,
    # database and canonical-v11 tests. The final gate MUST stay nonzero.
    combined_host_static_status=0
    (cd target && python3 -m unittest discover -s tests/defragmentation/security -p 'test_combined_host_wiring.py' -v) || combined_host_static_status=$?
    cat audit/tests/defragmentation/security/canonical_writing_additional_tests.rs >> target/apps/desktop/src-tauri/tests/study_writing.rs
    cat audit/tests/defragmentation/security/canonical_v11_integrity_tests.rs >> target/apps/desktop/src-tauri/tests/study_writing.rs
    # Add one independent v10 cross-run receipt replay test inside the REAL
    # database's existing Rust test fixture. This modifies only CI's checkout;
    # never persist any changes into PR #114 or the production source tree.
    python3 - <<'PY'
from pathlib import Path
target = Path('target/crates/scriptotar-db/src/agent_effects_external.rs')
source = target.read_text()
marker = '#[cfg(test)]\nmod tests {'
assert marker in source, 'cannot find real v10 test fixture'
closing = source.rfind('}')
assert closing > source.index(marker)
assert not source[closing + 1:].strip(), 'v10 test fixture is not last in file'
extra = Path('audit/tests/defragmentation/security/external_receipt_replay_additional_tests.rs').read_text()
target.write_text(source[:closing] + '\n' + extra + '\n' + source[closing:])
PY
    (
      cd target
      cargo test --manifest-path tests/defragmentation/security/Cargo.toml --test provider_context_race -- --nocapture
      # Network is forbidden: independent public wire API assertions all pass
      # an already-expired deadline and use a placeholder key.
      wire_security_status=0
      cargo test --manifest-path tests/defragmentation/security/Cargo.toml --test provider_wire_boundary -- --nocapture || wire_security_status=$?
      cargo test --locked -p scriptotar-ai provider_consent
      # Exercise the REAL compiled HTTPS wire parser, fixed endpoint, hard output
      # caps, exact serialized body and one-use authenticated response proof.
      # This is local/no-network; it is NOT proof of billable HTTP integration.
      cargo test --locked -p scriptotar-ai provider_openai_wire
      # A newly surfaced cross-run receipt replay may be a security FAIL.
      # Preserve host, verification, and canonical-writing evidence anyway.
      db_security_status=0
      cargo test --locked -p scriptotar-db || db_security_status=$?
      # Compile/test the actual Tauri host, not only path-included feature modules.
      cargo test --locked -p scriptotar-desktop --lib
      cargo test --locked -p scriptotar-ai verification
      python3 -m unittest discover -s tests/defragmentation/security -p 'test_*.py' -v
      cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --test study_writing --locked -- --nocapture
      if [ "$db_security_status" -ne 0 ]; then
        echo "FAIL: independent v10 database/receipt replay gate (exit $db_security_status)" >&2
        exit "$db_security_status"
      fi
      if [ "$wire_security_status" -ne 0 ]; then
        echo "FAIL: independent exact-wire transport authorization gate (exit $wire_security_status)" >&2
        exit "$wire_security_status"
      fi
    )
    if [ "$combined_host_static_status" -ne 0 ]; then
      echo "FAIL: combined host authorization remains a release blocker (exit $combined_host_static_status)" >&2
      exit "$combined_host_static_status"
    fi
    ;;
  *)
    echo "ERROR: unsupported PR/gate mapping: $gate:$pr" >&2
    exit 3
    ;;
esac
