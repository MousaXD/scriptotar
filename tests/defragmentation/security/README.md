# Independent Scriptotar security regressions — Agent 10

**Not a release certification.** The branch is based on integration commit
`3009989a3aa73248b6432ba0cd84821c59110993`, where the five feature PRs
are not integrated. Tests for feature PRs are intentionally staged independently.

## Reproduce and run

1. Check out **PR #118 SHA** `cdcd07d61f219e5be08052eea6ccddcafcc3c7e9` (also present on earlier `bcb3f3a`)
   and copy this directory's `Cargo.toml` and `tests/provider_context_race.rs`
   to `tests/defragmentation/security/` in that checkout. Run:

   ```sh
   cargo test --manifest-path tests/defragmentation/security/Cargo.toml \
       --test provider_context_race -- --nocapture
   ```

   The fence-rejection control should pass. The stale-project/Study-context race
   should fail on PR #118: the mock fence revokes context *after* the prior scope
   validation, yet the mock provider is called. Zero live API calls are made.

2. Check out **PR #117 SHA** `0847a06cf5af0d2432ad15495a6cd6110079f954`.
   Copy `test_study_claim_boundary.py` into the same security directory, then run:

   ```sh
   python3 -m unittest discover -s tests/defragmentation/security \
       -p 'test_*.py' -v
   ```

   This contract now **passes by design** on the newer PR #117: the host
   explicitly sets `draft_content_bound: false` for noncanonical
   caller-provided draft text, and the verification core refuses Verified
   status and requires publication review. The historical `9267066`
   pre-mitigation head is expected to fail this guard.

3. On the integration baseline (before #117 is merged), the Python host guard
   **SKIPS** rather than treating a missing implementation as a pass. The
   independent Rust harness requires PR #118's module and cannot compile on
   the unintegrated baseline. Do not mistake a skip for certification.

## Test evidence categories

- **Code-path finding:** provider-consent race remains present at the new #118 head. The Rust test is expected to fail and has **not been executed** by this Agent against a compiled feature checkout. Latest #117 independently added a fail-closed `draft_content_bound` guard; its Python security contract is expected to pass (not executed locally).
- **CI:** upstream GitHub Actions results are cited in the audit handoff; they
  do not execute this nested independent Rust harness.
- **NOT TESTED:** real provider costs, installed Tauri WebView, real credentials,
  local sandbox escape and platform-specific behaviors.

Agent 00 owns registration of these negative regressions in the integrated
CI; do not drop failing cases or silently mark them approved.
