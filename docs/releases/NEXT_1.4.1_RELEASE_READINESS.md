# Scriptotar Next 1.4.1 hotfix

Version 1.4.1 fixes downloads aborted with `ENGINE_PROTOCOL` when yt-dlp writes
console progress into the JSON event channel. JSON now uses a dedicated handle;
Python, native-library and child-process diagnostics go to stderr. yt-dlp console
progress is disabled while the application progress hook remains active.

The owner explicitly requested pushing, merging and publishing this patch on
2026-11-12. Publication uses the existing `tauri-next-latest` prerelease channel
and the sole release writer, with all mandatory exact-main-SHA gates retained.
The exact owner-approval commit message is
`owner-approve-scriptotar-next-1.4.1-hotfix-publish-20261112`.

## Evidence before the version bump

- The installed 1.4.0 application failed twice on the exact post
  `https://www.instagram.com/p/DfiW-6ssCCU/`. Its journal contained `[download]`
  text concatenated with a JSON progress event, followed by `ENGINE_PROTOCOL`.
- The noisy-worker and real yt-dlp transport regressions fail on the original
  code and pass with the fix. The local sidecar suite ran 99 tests: 98 passed
  and one actual-Windows-only check was skipped on Linux.
- The installed supervisor with the locally rebuilt engine downloaded 2,564,329
  bytes, decoded the audio, ran real Whisper `small` inference and wrote all six
  transcript exports. The source duration was 23.208333 seconds.
- The installed desktop binary's MCP entrypoint then retried that same URL
  against the original application database and existing `medium` settings.
  The job completed, saved two speech segments and persisted its transcript link.

These runs used the pre-bump 1.4.0 version identity with the repaired local engine.
They are local execution evidence, not hosted CI or rebuilt 1.4.1 package evidence.
The fixed version's CI and package outcomes are recorded in the pull request and
the [rolling release](https://github.com/MousaXD/scriptotar/releases/tag/tauri-next-latest).

## Required publication checks

The merged source SHA must pass Tauri migration integration, MCP E2E, security
hygiene, Windows installer and Linux package gates. The integration gate now
also runs the pinned yt-dlp HTTP transport regression after dependency installation.
The separate live-provider workflow checks the exact Instagram post through the
real supervisor and worker; platform access failures remain failures.

Existing preview limitations remain documented in the
[1.4.0 readiness record](NEXT_1.4.0_RELEASE_READINESS.md): native performance and
independent visual certification, hardware-specific GPU behavior, and hosted
YouTube sign-in challenges are not certified by this hotfix.
