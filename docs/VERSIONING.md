# Scriptotar versioning and release identity

Scriptotar Next **1.4.0** is the current Rust/Tauri/Svelte desktop preview candidate. This is the source/package identity, not a claim that 1.4.0 has been published. See the [candidate readiness record](releases/NEXT_1.4.0_RELEASE_READINESS.md).

## Current product versions

| Product line / component | Current version | Role |
| --- | --- | --- |
| Scriptotar desktop application | `1.4.0` | Primary cross-platform desktop application. |
| Scriptotar Rust workspace | `1.4.0` | Rust crate/application version used by the codebase. |
| Scriptotar Svelte package | `1.4.0` | Frontend package identity. |
| Scriptotar transcription sidecar | `1.4.0` | Packaged sidecar/runtime component. |
| Sidecar protocol | `1` | Wire-protocol compatibility version. |
| Scriptotar Classic (Archived) | `1.3.0` | Historical legacy Python/Tkinter line (archived in `archive/legacy-python/`). |

Toolchain/dependency versions such as Tauri CLI `2.11.2`, Python `3.12`, Faster Whisper `1.2.1`, or yt-dlp release numbers are dependency/runtime versions and do not define the Scriptotar application version.

## Sources of truth

The release-visible application version is declared in:

```text
apps/desktop/src-tauri/tauri.conf.json
```

These identities are kept in lockstep with it:

```text
Cargo.toml                                            [workspace.package].version
apps/desktop-ui/package.json                          version
sidecars/transcription/scriptotar_sidecar/version.py  SIDECAR_VERSION
```

The Next rolling preview uses the prerelease tag:

```text
tauri-next-latest
```

Versioned package jobs stage files such as:

```text
Scriptotar-Next-1.4.0-x64-setup.exe
Scriptotar-Next-1.4.0-amd64.deb
```

The rolling GitHub prerelease renames the public assets to:

```text
Scriptotar-Next-latest-x64-setup.exe
Scriptotar-Next-latest-amd64.deb
```

The `latest` filename is therefore a rolling-channel name, not another application version.

## Release-channel policy

### Next preview

`tauri-next-latest` is the rolling **Scriptotar Next preview** channel. It is a GitHub prerelease and must not be described as the stable channel while Next remains on the preview line.

There is currently no stable Scriptotar Next channel and no published macOS Next artifact.

### Archived Classic channels

Classic's historical `continuous` / **Scriptotar Latest** channel and permanent tags such as `v1.3.0` refer to the Python/Tkinter product preserved under `archive/legacy-python/`. They are separate from Next's rolling prerelease. Current Next CI/publishing does not promise continuing Classic builds or publish a stable Next release under those tags.

## When to bump versions

Do not bump a version merely to make Classic and Next look numerically consistent.

For a Classic change:

1. explicitly decide whether historical Classic maintenance is in scope;
2. work in `archive/legacy-python/`, including its `scriptotar_common.py` and package metadata;
3. preserve original user data and distinguish any historical-package changes from Next;
4. run the affected Classic tests/package checks in its own dependency environment;
5. create or publish a matching tag only when that separate release is authorized.

For a Next change:

1. decide whether the Next application release version is changing;
2. update `tauri.conf.json`;
3. keep the Rust workspace, frontend package, sidecar, npm lockfile root package, and local Cargo lockfile package versions aligned unless a future policy explicitly decouples them;
4. update user-facing documentation that names the current Next version;
5. run Rust/frontend/sidecar tests plus affected package validation;
6. assess exact-commit CI, installed-package checks, and separately identified live/manual evidence; local success does not certify another environment;
7. do not infer permission to push, merge, or publish from a numeric version change, or invent a stable Next tag/channel.

## Classic and Next version identities

Next `1.4.0` and archived Classic `1.3.0` identify separate product lines. Classic is the historical Python/Tkinter application; Next is the current Rust/Tauri preview. Their package filenames, data directories, support statements and release channels remain separate.

The repository's version-identity regression test exists to catch accidental drift across these declared source files and the top-level documentation.
