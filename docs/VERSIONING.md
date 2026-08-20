# Scriptotar versioning and release identity

Scriptotar **1.0.0** is the primary cross-platform desktop application built with Rust (Tauri v2) and Svelte 5.

## Current product versions

| Product line / component | Current version | Role |
| --- | --- | --- |
| Scriptotar desktop application | `1.0.0` | Primary cross-platform desktop application. |
| Scriptotar Rust workspace | `1.0.0` | Rust crate/application version used by the codebase. |
| Scriptotar Svelte package | `1.0.0` | Frontend package identity. |
| Scriptotar transcription sidecar | `1.0.0` | Packaged sidecar/runtime component. |
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
Scriptotar-Next-0.1.0-x64-setup.exe
Scriptotar-Next-0.1.0-amd64.deb
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

### Classic rolling release

`continuous` / **Scriptotar Latest** is the rolling supported Classic channel. The Classic Debian artifact is required for that release; Classic AppImage/Flatpak assets are added when the portable packaging lane succeeds.

### Classic permanent versions

Classic permanent version tags use `v<Classic version>`, for example:

```text
v1.3.0
```

The release workflow rejects a Classic version tag that does not match the application version.

## When to bump versions

Do not bump a version merely to make Classic and Next look numerically consistent.

For a Classic change:

1. decide whether a Classic application release is actually being made;
2. update `scriptotar_common.py`;
3. update Classic package metadata to exactly match;
4. update user-facing documentation that names the current Classic version;
5. run Classic tests and package validation;
6. create a matching permanent tag only when intentionally publishing that release.

For a Next change:

1. decide whether the Next application release version is changing;
2. update `tauri.conf.json`;
3. keep the Rust workspace, frontend package, and sidecar version aligned unless a future policy explicitly decouples one component;
4. update user-facing documentation that names the current Next version;
5. run Rust/frontend/sidecar tests plus affected package validation;
6. do not invent a stable Next tag/channel merely because the numeric version changed.

## Why Classic and Next differ

`1.3.0` describes the mature Python/Tkinter line. `0.1.0` describes the newer Tauri replacement while it is still being released as a preview. Treating one as the version of the other would make package filenames, support statements, migration documentation, and release channels ambiguous.

The repository's version-identity regression test exists to catch accidental drift across these declared source files and the top-level documentation.
