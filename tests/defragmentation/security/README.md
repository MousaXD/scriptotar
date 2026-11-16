# Independent Scriptotar security regressions — Agent 10

**Not a release certification.** The branch is based on integration commit
`3009989a3aa73248b6432ba0cd84821c59110993`, where the five feature PRs
are not integrated. Tests for feature PRs are intentionally staged independently.

## Reproduce and run

1. Check out **exact PR #118 SHA** `bcb3f3a7d08426ee013530236ea2252012fa4da3`
   and copy this directory's `Cargo.toml` and `tests/provider_context_race.rs`
   to `tests/defragmentation/security/` in that checkout. Run:

   ```sh
   cargo test --manifest-path tests/defragmentation/security/Cargo.toml \
       --test provider_context_race -- --nocapture
   ```

   The fence-rejection control should pass. The stale-project/Study-context race
   should fail on PR #118: the mock fence revokes context *after* the prior scope
   validation, yet the mock provider is called. Zero live API calls are made.

2. Check out **exact PR #117 SHA** `9267066ce3576431b666c6c8cb0f20ba8eaad228`.
   Copy `test_study_claim_boundary.py` into the same security directory, then run:

   ```sh
   python3 -m unittest discover -s tests/defragmentation/security \
       -p 'test_*.py' -v
   ```

   It should fail because canonical metadata are checked while the supplied
   `ScriptDraft` body is used for claim extraction. The existing successful
   Rust host test builds an unsaved `ScriptDraft` with the correct draft UUID
   and revision, illustrating the problematic trust boundary.

3. On the integration baseline (before #117 is merged), the Python host guard
   **SKIPS** rather than treating a missing implementation as a pass. The
   independent Rust harness requires PR #118's module and cannot compile on
   the unintegrated baseline. Do not mistake a skip for certification.

## Test evidence categories

- **Code-path finding:** exact source inspection demonstrates a reachable
  trust-boundary gap. The new test documents the expected failure but has **not
  been executed by this Agent** against a compiled feature checkout.
- **CI:** upstream GitHub Actions results are cited in the audit handoff; they
  do not execute this nested independent Rust harness.
- **NOT TESTED:** real provider costs, installed Tauri WebView, real credentials,
  local sandbox escape and platform-specific behaviors.

Agent 00 owns registration of these negative regressions in the integrated
CI; do not drop failing cases or silently mark them approved.
