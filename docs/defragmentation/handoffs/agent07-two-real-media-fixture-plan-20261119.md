# Agent 07 — genuine two-source media fixture preparation (NOT INSTALLED CERTIFIED)

The QA-only `tests/defragmentation/native-e2e/real_source_fixtures.py`
acquires two original public LibriVox audio recordings from Wikimedia Commons,
retaining exact original URL, narrator, separate source ID, on-disk SHA-256,
decoded duration and an actual FFmpeg-produced WebM/Opus SHA-256. It rejects
redirects, unreviewed origins, oversized/truncated files, duplicate content
and previous-run artifact reuse. Media is never embedded in the repository.

Original source pages:
- <https://commons.wikimedia.org/wiki/File:Shortstory025_reflection.ogg>
- <https://commons.wikimedia.org/wiki/File:Charon_dunsany_sc.ogg>

**Opt-in preparation only** (run in a disposable working directory with FFmpeg
and FFprobe available):

```sh
python3 tests/defragmentation/native-e2e/real_source_fixtures.py \
  --prepare --output /tmp/scriptotar-two-real-media-new-run
```

This creates two independently sourced `.ogg` files, two genuinely converted
`.webm` files and `origin-manifest.json`. Every output has separate source
and artifact hashes. **This is a preparation utility, NOT a native acceptance
runner, not a Whisper transcription claim and not a source-permission fixture.**

### Still required before `multi_source_transcript=PASS`

1. Attach both real WebM files through the installed application's supported
   user interaction / Queue route, without direct SQLite insertion, test-only
   frontend backend stubs or privileged Tauri debug commands.
2. Prove the installed sidecar actually generated two transcripts from those
   separately hashed source files, recording the real media and sidecar hashes,
   authenticated Queue job IDs, host/PID identities and timing.
3. Select both source IDs through the genuine installed Study UI; then reopen
   the Study and verify persisted transcript provenance, selected source IDs,
   independent origin and canonical source/evidence/draft/job rows.
4. Protect against duplicate origin, hardcoded synthetic transcripts, missing
   attached permissions, stale sidecar receipts, failed source mutations and
   per-OS UIA/AT-SPI absence. A test which cannot prove these conditions must
   remain `NOT TESTED` or `FAIL`, never `PASS`.

No paid provider egress, production authorization, package scripts, CI workflows,
or current 16-case verdicts are changed by the helper.
