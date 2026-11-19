# Agent 07 — HTTP 429 remediation and authenticated offline original media

**Evidence:** exact installed Linux workflow #44568353251 reached media acquisition, then Wikimedia Commons returned HTTP 429 Too Many Requests. Agent00 and Agent08 reviewed the original two-source-attempt.json. It did not enqueue audio, transcribe, create a transcript, attach Study sources or revoke access. The 16-case verdict remains 11 PASS / 0 FAIL / 5 NOT TESTED on both operating systems.

## Legitimate acquisition without repeating a throttled request

The driver now requires explicit --allow-origin-fetch before it will access the two approved Commons originals. Do not set that flag on a rate-limited runner; do not rotate endpoints, impersonate clients, use proxies or otherwise bypass an origin service's rate limit.

An operator with legitimate access to both original LibriVox recordings can run the committed preparation tool on a permitted machine/network. Use a fresh, empty directory:

    python3 tests/defragmentation/native-e2e/real_source_fixtures.py \
      --prepare --output /path/to/new/real-origins
    sha256sum /path/to/new/real-origins/origin-manifest.json

The independent reviewer must verify both exact public-domain Commons file pages, distinct approved origin URLs, original Ogg hashes, WebM Opus hashes/durations and listen to the actual audio. Copy both real Ogg files, both real WebM files, and origin-manifest.json to the installed QA machine by a trusted, integrity-preserving channel. Pin the approved manifest SHA-256 **outside** the copied bundle. A self-supplied manifest checksum does not prove media origin.

With exact installed package and real X11 + window manager + AT-SPI session, execute:

    env NO_AT_BRIDGE=0 GTK_A11Y=1 GTK_MODULES=atk-bridge \
      /usr/bin/python3 tests/defragmentation/native-e2e/linux_real_media_attempt.py \
        --executable /usr/bin/scriptotar-desktop \
        --package /path/to/exact-installed.deb \
        --source-sha FULL_EXACT_SOURCE_COMMIT_SHA \
        --run-url https://github.com/MousaXD/scriptotar/actions/runs/ACTUAL_PACKAGE_RUN_ID \
        --evidence-dir /path/to/new/native-evidence \
        --prepared-fixture-dir /path/to/new/real-origins \
        --prepared-manifest-sha256 INDEPENDENTLY_REVIEWED_SHA256 \
        --ffprobe /path/to/packaged/ffprobe

If executed outside GitHub Actions, the run URL identifies *package provenance*, not proof that manual UI interaction happened in the hosted workflow. Independently record manual operator, host OS, run timestamp, exact package/executable SHA-256, AT-SPI interactions and original screenshots. Never claim manual results as hosted-CI success.

The addendum checks a pinned manifest SHA, both approved source identities/pages, original Ogg content SHA, WebM Opus SHA and FFprobe duration/codec, then attempts real installed Queue and bundled sidecar transcription, canonical Study relationships, graphical relaunch and source revocation. Do not insert synthetic SQLite rows or transcript data.

## Evidence behavior and anti-spoof checks

A missing offline bundle, HTTP 429, unsupported native UI control, missing process identity or sidecar transcripts must remain incomplete/NOT TESTED; a demonstrated installed product failure remains FAIL. The addendum now writes a structured stage, blocker code, blocker and attempt outcome to its original run log and two-source-attempt.json. It returns a nonzero status if incomplete. The workflow marks this separate experiment continue-on-error only so it does not prevent the independent native 16-case runner or original-artifact uploads. **The main 16-case gate remains fail-closed.**

The process sampler now rejects an unrelated CI process whose command line merely mentions the sidecar name: it requires the actual executable's filename and SHA-256 and a PID ancestry chain below the launched installed Tauri process.

## Still NOT TESTED

Windows real installed two-source path, stale revoked-source access/action denial, authenticated forged mutating save_study from actual installed WebView IPC (Agent00/Agent10 review), Arabic mixed-direction/RTL clipping and focus, and native Light/Dark/System appearance/contrast on both platforms. No integration/main merge or release certification from this work.
