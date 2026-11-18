#!/usr/bin/env bash
# CI-owned independent feature tests. Run from the GitHub Actions workspace.
# Never silently skip missing feature implementations.
set -euo pipefail
gate="${1:?provider|migration|receipt-v12|writing|verification|combined-v11|combined|native-evidence}"
pr="${2:?expected PR number}"
echo "Audit SHA: $(git -C audit rev-parse HEAD)"
echo "PR #$pr feature SHA: $(git -C target rev-parse HEAD)"
test -f target/Cargo.toml || { echo "ERROR: feature checkout missing"; exit 3; }

case "$gate:$pr" in
  native-evidence:131)
    # Independently probe Agent 07's actual QA runner from PR #131.
    # This test MUST fail if unreviewed screenshot-only validation is
    # auto-certified as a successful installed-app negative path.
    test -f target/tests/defragmentation/native-e2e/linux_webkit.py
    python3 audit/tests/defragmentation/security/test_native_visual_adjudication.py \
      target/tests/defragmentation/native-e2e/linux_webkit.py
    ;;
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
extras = [
    Path('audit/tests/defragmentation/security/external_receipt_replay_additional_tests.rs').read_text(),
    Path('audit/tests/defragmentation/security/external_v12_upgrade_additional_tests.rs').read_text(),
]
target.write_text(source[:closing] + '\n' + '\n'.join(extras) + '\n' + source[closing:])
PY
    (
      cd target
      # All original five regressions plus new adversarial and positive
      # provider-scoping, retry, ACK, migration and restart tests.
      cargo test --locked -p scriptotar-db
    )
    ;;
  revocation:137)
    # Independent source-permission withdrawal security gate on Agent 00's
    # unmerged production PR #137. Augment only a disposable checkout's real
    # versioned SQLite fixture, not PR #137's source code or migrations.
    test -f target/crates/scriptotar-db/tests/content_studies.rs
    test -f target/crates/scriptotar-db/src/studies.rs
    test -f audit/tests/defragmentation/security/source_revocation_additional_tests.rs
    cat audit/tests/defragmentation/security/source_revocation_additional_tests.rs >> target/crates/scriptotar-db/tests/content_studies.rs
    (
      cd target
      cargo test --locked -p scriptotar-db --test content_studies -- --nocapture
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
    # PR #140 is a test-only correction to PR #124's already-merged audit
    # module. The target checkout is the *unmodified pinned integration commit*
    # and still contains the old discovery bug until Agent 00 merges #140.
    # Simulate that exact owned test correction in this disposable checkout;
    # never patch a production module and fail if *any other* source differs.
    python3 - <<'PY'
from pathlib import Path
import hashlib
audit_path = Path('audit/tests/defragmentation/security/test_native_visual_adjudication.py')
target_path = Path('target/tests/defragmentation/security/test_native_visual_adjudication.py')
audited = audit_path.read_text()
original = target_path.read_text()
old = (
    'TARGET = Path(sys.argv[1] if len(sys.argv) > 1 else\\n'
    '              "target/tests/defragmentation/native-e2e/linux_webkit.py").resolve()'
)
new = (
    '# The dedicated SHA-pinned gate invokes this file directly with the Agent 07\\n'
    '# checkout as argv[1]. Ordinary unittest discovery *imports* this module;\\n'
    '# its argv[1] is "discover", not a file path. In that case always exercise\\n'
    '# the actual in-repository native evidence helper rather than opening "discover"\\n'
    '# or silently skipping these adversarial assertions.\\n'
    'LOCAL_NATIVE_QA = Path(__file__).resolve().parents[1] / "native-e2e" / "linux_webkit.py"\\n'
    'TARGET = (Path(sys.argv[1]).resolve()\\n'
    '          if __name__ == "__main__" and len(sys.argv) > 1\\n'
    '          else LOCAL_NATIVE_QA)'
)
required = [
    'test_image_only_angle_rejection_cannot_be_autocertified',
    'test_unbound_semantic_flag_is_not_request_correlated_proof',
    'test_unrelated_fresh_angle_invalid_alert_is_not_rejection_proof',
    'test_preexisting_angle_error_cannot_certify_retry',
    'test_visual_fallback_does_not_hide_human_review_obligation',
]
for name in required:
    assert original.count('def ' + name + '(') == 1, 'missing original independent test: ' + name
    assert audited.count('def ' + name + '(') == 1, 'audit dropped independent test: ' + name
if original != audited:
    assert original.count(old) == 1, 'unexpected integration audit module, refuse to overlay'
    assert original.replace(old, new) == audited, 'unauthorized modification in candidate audit tests'
    target_path.write_text(audited)
    print('Overlaying EXACT Agent 10 PR #140 test-only discovery correction on pinned integration checkout')
else:
    print('Exact Agent 10 test-only discovery correction already present in integration')
print('Audit test source SHA256:', hashlib.sha256(audited.encode()).hexdigest())
print('Production source and integration Git commit remain unchanged')
PY
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
    # Independently append a host regression into CI's disposable checkout.
    # This directly exercises AppServices and compares every canonical SQLite
    # table's *contents* across denied schemes and forged project IDs.
    # Never change production files, disable the provider fuse, or mock storage.
    test -f audit/tests/defragmentation/security/host_url_nonmutation_additional_tests.rs
    python3 - <<'PY'
from pathlib import Path
source = Path('target/apps/desktop/src-tauri/src/services.rs')
body = source.read_text()
marker = '#[cfg(test)]\nmod tests {'
assert marker in body, 'missing actual AppServices Rust host fixture'
closing = body.rfind('}')
assert closing > body.index(marker), 'missing host fixture close'
assert not body[closing + 1:].strip(), 'host test module not last in services.rs'
extra = Path('audit/tests/defragmentation/security/host_url_nonmutation_additional_tests.rs').read_text()
assert 'independent_host_denied_url_and_foreign_project_preserve_all_canonical_contents' in extra
source.write_text(body[:closing] + '\n' + extra + '\n' + body[closing:])
PY
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
extras = [
    Path('audit/tests/defragmentation/security/external_receipt_replay_additional_tests.rs').read_text(),
]
schema = Path('target/crates/scriptotar-db/src/lib.rs').read_text()
if 'pub const LATEST_SCHEMA_VERSION: u32 = 12;' in schema:
    extras.append(
        Path('audit/tests/defragmentation/security/external_v12_upgrade_additional_tests.rs').read_text()
    )
target.write_text(source[:closing] + '\n' + '\n'.join(extras) + '\n' + source[closing:])
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
