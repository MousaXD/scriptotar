# Agent 07 — read-only real media / revocation provenance verifier

**QA-only commit series. This does not complete installed acceptance.**

The production Study revocation contract intentionally retains the immutable
Study source row and its original \`reference_id\`, locator and fingerprint,
while nulling only the selected \`study_sources.transcript_id\` in one immediate
transaction and incrementing \`content_studies.revision\`. It must not destroy
\`study_evidence\`, \`draft_source_links\`, \`study_source_jobs\`, or existing
draft/version content. None of these properties can be inferred from a screenshot.

\`tests/defragmentation/native-e2e/real_study_lineage.py\` is a **READ-ONLY
evidence verifier**. It requires two *actual* FFmpeg-produced WebM files and
their original manifest from \`real_source_fixtures.py\`. It rehashes those bytes
and reads the application's *existing* SQLite database in a single, read-only
WAL transaction. It refuses to continue if:

- either source is absent, the manifest/origin hashes are duplicated or the
  WebM bytes changed;
- two distinct **completed local-file jobs** have not persisted their real
  \`job_transcript_links\` to different transcripts/media/sources in the
  expected project;
- any transcript lacks nonempty text and serialized transcript segments;
- both transcripts are not independently linked through genuine
  \`study_sources\` rows belonging to the exact Study; or
- a source is reassigned, a fingerprint duplicates another origin, or Study
  revisions or historical evidence/draft links change unexpectedly.

**Run only after native UI ingestion, sidecar execution, Study Select Sources,
save, full application process quit and relaunch:**

\`\`\`bash
python3 tests/defragmentation/native-e2e/real_study_lineage.py \
  --database /path/to/actual-installed-profile/scriptotar.sqlite3 \
  --manifest /path/to/real-media/origin-manifest.json \
  --fixture-dir /path/to/real-media \
  --project-id ACTUAL_PROJECT_UUID \
  --study-id ACTUAL_STUDY_UUID \
  --output /path/to/real-native-evidence/study-lineage-readonly.json
\`\`\`

The IDs **must be read from the existing project's canonical SQLite rows**.
Never insert or patch any rows as acceptance preparation. The helper cannot
start or emulate Queue, sidecars, accessibility controls or IPC.

The \`verify_revocation(before, after, revoked_transcript_id, reopened)\` API
accepts four separately collected read-only states (the pre-revoke state, the
immediate state, the exact selected transcript ID, and the state after an
actual installed application relaunch/reopen). Its comparison ensures only
the chosen origin's live link becomes a tombstone, the second origin and
historical evidence/draft data survive, the Study revision increases once,
and the reopened state does not resurrect access.

**Important:** even a successful SQL comparison remains
\`native_16_case_certification=NOT TESTED\` and
\`source_revocation_native_certification=NOT TESTED\`. The native driver must
separately prove exact real Queue UI actions, bundled real Whisper sidecar
transcripts, Studio Select Sources/reload, actual visible source revocation,
failed stale source access/actions, no resurrection, and authenticated
AT-SPI/UIA evidence. Only a combined original installed run may promote a
native case to PASS.

The \`test_real_study_lineage*.py\` regressions deliberately construct fake
bytes and a disposable mini-SQLite DB for **source-only rejection testing**.
Those examples are not media, app snapshots or accepted native fixtures.
They must never appear in an installed-run evidence artifact as genuine data.

This work changes neither product code nor SQLite. It is independent of
PR143 read-only denial and PR146/PR150 forged mutating IPC testability.
