# Scriptotar Next 1.3.0 — Release Readiness

**Snapshot: 2026-11-08.** Version 1.3.0 is the preview candidate integrating `integration/scriptotar-owner-overhaul-20261107` into `main`; source integration and public publication are separate. Installed-package and local real-provider evidence is available below. Hosted YouTube access and the remaining manual acceptance are unresolved; this is not a public-release certification.

## Scope and identity

The candidate combines the seven owner-overhaul workstreams: Rust/Tauri integration, database migrations/recovery, MCP authorization and connection setup, media-engine cleanup, Arabic/RTL UI, AI/creator workflows, and release checks.

Tauri, the Rust workspace and its nine local lockfile packages, the Svelte package/npm lockfile, and the transcription sidecar use `1.3.0`. Sidecar protocol remains `1`. Archived Classic 1.3.0 is a separate historical product line.

Two follow-up fixes are relevant to acceptance:

- Packaging retries transient FFmpeg/tunnel download failures and can fetch the same pinned tunnel asset through GitHub's official API. Binary digests, exact asset identity, size bounds, and permanent-error failures remain enforced.
- `42349aa` removes the canary's arbitrary 8 KiB compressed-media minimum. The exact YouTube input produces valid 5,286-byte Opus audio locally. Empty/oversized files, invalid audio, wrong providers, and failed decoding/inference/exports still fail validation.

## Recorded evidence

Evidence applies to its named commit and environment, not to later edits or another runner.

| Source / environment | Evidence | Outcome |
| --- | --- | --- |
| `427cbcd`, hosted core checks | [Integration](https://github.com/MousaXD/scriptotar/actions/runs/42471864196), [MCP](https://github.com/MousaXD/scriptotar/actions/runs/42471864209), [Linux package](https://github.com/MousaXD/scriptotar/actions/runs/42471864187), [Windows installer/native cleanup](https://github.com/MousaXD/scriptotar/actions/runs/42471864217) | All 17 core/security checks passed. |
| `42349aa`, hosted package/MCP checks | [Linux package](https://github.com/MousaXD/scriptotar/actions/runs/42485460932), [Windows installer/native cleanup](https://github.com/MousaXD/scriptotar/actions/runs/42485460901), [MCP](https://github.com/MousaXD/scriptotar/actions/runs/42485460961) | These installed-package and MCP checks passed; assess the complete final-commit check suite separately. |
| `42349aa`, developer machine | [Retained local result summary](../evidence/2026-11-08-local-live-platforms.json) | All three pinned sources passed actual download, decoding, real Whisper `small` CPU inference, and all six export-type checks with the pinned dependencies and a fresh model cache. |
| `42349aa`, hosted live providers | [Live run](https://github.com/MousaXD/scriptotar/actions/runs/42485461014) | YouTube requested sign-in/bot verification before metadata completed. Instagram passed with three speech segments; TikTok passed with zero. |

In the full local run, YouTube and TikTok returned zero speech segments and Instagram three. The zero-segment inputs establish inference/export execution, not recognition accuracy. The local source-tree run does not certify an installed package or turn the failed hosted canary green.

See [source-access diagnosis](../SOURCE_ACCESS.md), [the E2E inventory](../E2E-COVERAGE-1.3.0.md), and [PR #80](https://github.com/MousaXD/scriptotar/pull/80) for the candidate's complete checks. The [1.2.0 readiness record](NEXT_1.2.0_RELEASE_READINESS.md) preserves earlier evidence.

## Remaining acceptance

Assess the actual final candidate SHA for Rust formatting/check/Clippy/tests, Svelte checks/tests/build, sidecar protocol and real dependency imports, migration/recovery, MCP stdio/HTTP authorization and persistence, Windows/Linux installed-package acceptance, strict dependency audits, repository hygiene, and release ownership/race contracts. An earlier all-green core run does not certify a newer commit.

Keep the hosted YouTube access failure explicit. Do not change the pinned URL, substitute fixture output, or weaken validation to make it pass. Record any release-owner disposition separately from the observed technical result.

CUDA fallback, Arabic/multilingual accuracy, an authorized live ChatGPT tunnel, interactive Windows journeys, historical customer database upgrades, and performance remain unverified. FFmpeg's upstream archive URL is mutable despite enforced digest/size validation. Three previously disclosed low-severity npm advisory paths remain.

## Publication

The sole rolling preview publisher is `tauri-next-release.yml`. It requires successful mandatory push checks on the exact `main` commit, validated matching Windows/Linux package artifacts, and `SCRIPTOTAR_NEXT_DISTRIBUTION_APPROVED=true`. Source edits, a version bump, and local test success do not authorize publication or introduce a stable Next tag. See [the publisher contract](../../sidecars/transcription/RELEASE-PUBLISHING.md).
