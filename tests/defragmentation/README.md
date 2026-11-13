# Defragmentation QA-only harness

These fixtures are owned by Agent 07, separate from all implementation code. They intentionally fail closed on unsupported evidence labels.

Run source/fixture contracts from repo root:

```sh
python3 -m unittest discover -s tests/defragmentation -t tests -p 'test_*.py' -v
```

Run the broader repo Python surface tests, if prerequisites are installed:

```sh
PYTHONPATH=sidecars/transcription python3 -m unittest discover -s tests -p 'test_*.py' -v
```

The existing Chromium QA workflow also launches the first contract command inside its Node runner. Running the fixture validator says the acceptance **catalog is well formed, not that any P0–P3 application behavior passed**.

Performance diagnostics (no network, no simulated evidence):

```sh
python3 tests/defragmentation/benchmarks/native_session.py --executable /usr/bin/scriptotar --source-sha YOUR_FULL_SHA --trial cold --seconds 20 --output /tmp/scriptotar-process-cold.json
python3 tests/defragmentation/benchmarks/sqlite_readonly.py --db /tmp/isolated-migrated-copy.sqlite3 --source-sha YOUR_FULL_SHA --output /tmp/scriptotar-sqlite.json
```

Replace executable with a verified installed path and provide real SHA. Native session samples only the launched process; it is **not** a WebView-ready signal or installer verification. A `--ready-file` requires a human/external probe and is explicitly labeled operator acknowledgement. Do not silently relabel as automated readiness. The SQL reader uses a read-only connection and samples table ID queries, not the real Tauri bootstrap or IPC. Generate synthetic 100/1k/10k datasets in isolated, migrated copies using the current schema under the data owner's reviewed fixture tooling. Never seed or benchmark a user's live database.

The `verify_evidence.verify(record)` helper distinguishes `browser-mock`, `native-installed`, `rust-ipc`, `mcp-e2e`, `sidecar-fixture`, `provider-live`, and `manual-native` labels. It rejects a purported native PASS without a package SHA, executable SHA, machine information and a non-mock real native UI probe. The verifier checks structure, **not the authenticity** of an uploaded trace. Only independent review of raw exact-head evidence certifies.

Catalog statuses are deliberately `NOT_WIRED` and `NOT_RUN`. Agents who implement a scenario should add separate production integration tests in their own areas; Agent 07 can then link observed test references in a versioned evidence report. Do not change catalog status to PASS merely to make the metadata unit tests green.

Produce three isolated, synthetic SQLite load levels using a freshly initialized scratch template (never a personal DB):

```sh
python3 tests/defragmentation/benchmarks/seed_workspace.py --template /tmp/blank-migrated.db --output /tmp/synthetic-100.db --rows 100
python3 tests/defragmentation/benchmarks/seed_workspace.py --template /tmp/blank-migrated.db --output /tmp/synthetic-1000.db --rows 1000
python3 tests/defragmentation/benchmarks/seed_workspace.py --template /tmp/blank-migrated.db --output /tmp/synthetic-10000.db --rows 10000
```

The seeder refuses occupied research/jobs/AI/transcript collections and non-Inbox personal projects, and verifies integrity/counts before reporting a file SHA. It does not create real playable media, real inference, or an installed-app startup metric.
