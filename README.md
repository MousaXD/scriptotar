# Scriptotar

Scriptotar is an Apache-2.0-licensed, local-first desktop application for short-form video research, downloading, transcription, and AI-assisted content development.

This repository currently contains **two supported desktop lines** while the product moves from Python/Tkinter to Rust/Tauri.

| Line | Current version | Status | Platforms / packages | Choose it when |
| --- | --- | --- | --- | --- |
| **Scriptotar Next** | **0.1.0** | **Preview** | Windows NSIS `.exe`, Linux Debian `.deb` | You want the current product direction and are comfortable running preview software. |
| **Scriptotar Classic** | **1.2.0** | **Supported legacy/stable line** | Linux Debian `.deb`, AppImage, Flatpak | You want the established Python/Tkinter application or need the Classic portable Linux packages. |

For new Windows users, **Scriptotar Next is the only packaged Windows application**. For Linux users, choose Next for the new Rust/Tauri experience or Classic when you specifically need the established Classic line, AppImage, or Flatpak packaging.

## Scriptotar Next

Scriptotar Next is the current Rust + Tauri 2 + Svelte application. Rust owns application state, SQLite persistence, job orchestration, AI/research services, and the Tauri command boundary. Svelte owns the desktop UI. A packaged Python sidecar owns media execution and Faster Whisper transcription.

### What Next includes

- Project-based local workspace with persistent jobs, transcripts, research items, watchlists, settings, and AI-run history.
- URL and local-media transcription with cancellation, retry, interrupted-job recovery, word/segment timestamps, and transcript export formats.
- A self-contained packaged transcription runtime containing Python, Faster Whisper, yt-dlp, FFmpeg, and ffprobe. A separate Python or FFmpeg installation is not required by the packaged app.
- Creator/profile research using the packaged yt-dlp command and public metadata exposed by supported sources.
- AI Studio with **Copy Prompt** mode and BYOK execution for OpenAI, Anthropic, Gemini, and OpenAI-compatible endpoints.
- Local library/search, transcript workspace, native media/output pickers, and persistent application settings.

Whisper model weights are intentionally **not** bundled. The selected model is downloaded on first uncached use and then reused from Scriptotar Next's model cache.

### Install the Next preview

The rolling GitHub prerelease channel is:

```text
tauri-next-latest
```

Its public package names are:

```text
Scriptotar-Next-latest-x64-setup.exe
Scriptotar-Next-latest-amd64.deb
```

On Debian/Ubuntu/Pop!_OS:

```bash
sudo apt install ./Scriptotar-Next-latest-amd64.deb
```

The Windows installer is currently a preview build and is **not claimed to be Authenticode-signed**. No macOS distributable is published yet.

### Next data location

Scriptotar Next uses Tauri's application-data directory for bundle identifier:

```text
io.github.mousaxd.scriptotar.next
```

The default application-data root is therefore:

```text
Linux:   ${XDG_DATA_HOME:-~/.local/share}/io.github.mousaxd.scriptotar.next
Windows: %APPDATA%\io.github.mousaxd.scriptotar.next
macOS:   ~/Library/Application Support/io.github.mousaxd.scriptotar.next   (development only; no package is published)
```

Important data below that directory includes:

```text
scriptotar.sqlite3        Rust-owned application database
models/                   downloaded Whisper model cache
transcription-output/     fallback transcription output location
```

Tests and diagnostics can override the application-data root with `SCRIPTOTAR_DATA_DIR`.

### Migration from Classic

Next does **not** overwrite or delete the Classic database.

On startup, Next looks for Classic/WesamBoss `history.sqlite3` databases in the supported XDG/home and Classic Flatpak data roots. If exactly one valid legacy database is found, Next creates a safe SQLite snapshot in the Next data directory and imports it idempotently into the Rust-owned database. The original database remains untouched.

If multiple distinct legacy databases are found, Next refuses to guess which one to import. Invalid, truncated, symlinked, or otherwise unsafe candidates are rejected rather than silently followed.

### Current Next limitations

- Next is still a **0.1.x preview**, not the stable release channel.
- Windows preview packages are unsigned unless signing credentials are configured outside the repository.
- Linux Next packaging is Debian-only; Next AppImage/Flatpak packages are not published yet.
- No macOS package/signing/notarization lane is release-ready.
- Whisper model weights require network access on first uncached use.
- CUDA acceleration depends on compatible host NVIDIA hardware/drivers; CPU is the portable path.
- URL transcription and creator research still depend on the availability and behavior of third-party source platforms.
- The active project currently falls back to Inbox when the app restarts; durable active-project selection is a separate follow-up.

See [`docs/NEXT_DISTRIBUTION.md`](docs/NEXT_DISTRIBUTION.md) and [`docs/NEXT_MIGRATION.md`](docs/NEXT_MIGRATION.md) for the detailed runtime and architecture contracts.

## Scriptotar Classic

Scriptotar Classic is the legacy Python/Tkinter application. It remains supported and is intentionally kept in this repository while Next matures.

Classic 1.2 provides:

- URL/local-media queues and Faster Whisper transcription;
- public creator/profile research and watchlists;
- a local SQLite content library and projects;
- Copy Prompt and BYOK AI workflows;
- transcript editing/export, Arabic RTL handling, and word timestamps;
- Debian, AppImage, and Flatpak packaging on Linux.

### Install Classic

The rolling Classic release is **Scriptotar Latest**, backed by the `continuous` tag. Its rolling asset names are:

```text
scriptotar-latest_all.deb
Scriptotar-latest-x86_64.AppImage
Scriptotar-latest-x86_64.flatpak
```

The Debian package is the required Classic rolling artifact; AppImage and Flatpak are included when their portable packaging lane succeeds. Permanent Classic releases use version tags such as `v1.2.0` and publish versioned Debian/AppImage/Flatpak artifacts.

Debian/Ubuntu/Pop!_OS:

```bash
sudo apt install ./scriptotar-latest_all.deb
```

AppImage:

```bash
chmod +x Scriptotar-latest-x86_64.AppImage
./Scriptotar-latest-x86_64.AppImage
```

Flatpak:

```bash
flatpak install --user ./Scriptotar-latest-x86_64.flatpak
flatpak run io.github.mousaxd.scriptotar
```

### Classic data location

Classic stores normal Linux user data under:

```text
${XDG_DATA_HOME:-~/.local/share}/scriptotar/
```

Important files/directories include:

```text
history.sqlite3    Classic application database
settings.json      Classic application settings
venv/              private transcription-engine environment for the Debian/source path
```

The Classic Flatpak uses its sandboxed XDG data root, normally under:

```text
~/.var/app/io.github.mousaxd.scriptotar/data/scriptotar/
```

Classic also preserves migration support from the former `wesamboss` data directory.

### Classic package/runtime model

The Classic Debian package depends on system Python 3, Tk, FFmpeg, and Secret Service tooling, then manages its private Whisper engine environment under the Classic data directory. The AppImage and Flatpak use their own packaging/runtime arrangements. Classic and Next package internals are therefore intentionally different.

## Release channels

| Channel | Product line | Stability | Meaning |
| --- | --- | --- | --- |
| `tauri-next-latest` | Scriptotar Next 0.1.x | **Preview prerelease** | Rolling Windows + Linux Next preview. |
| `continuous` / **Scriptotar Latest** | Scriptotar Classic 1.2.x | **Supported Classic rolling release** | Rolling Classic Linux release. |
| `v1.2.0`-style tags | Scriptotar Classic | **Permanent versioned release** | Immutable Classic release snapshot whose tag must match the Classic app version. |

There is **no stable Scriptotar Next release channel yet**. The Classic and Next version numbers are intentionally independent; see [`docs/VERSIONING.md`](docs/VERSIONING.md).

## Privacy model

- Media/transcription processing is local after required media/model files are available.
- URL downloads and creator scans contact the selected source platform through yt-dlp.
- Copy Prompt mode sends nothing to an AI provider.
- Next BYOK credentials are request-time/session values and are not part of normal persisted application settings or SQLite data.
- Classic can optionally remember an AI token through Linux Secret Service when the user explicitly chooses the keyring option.
- Scriptotar has no mandatory hosted account or telemetry service.

## Responsible research

Scriptotar is a research and transformation tool, not a license to republish other people's work. Platform terms and copyright still apply. Metadata availability varies by platform, login state, region, extractor support, and source changes.

The app intentionally does not ship another service's private database, paid stock catalog, branding, or copyrighted clip collection.

## Development

Start with [`CONTRIBUTING.md`](CONTRIBUTING.md). It documents the Rust workspace, Tauri shell, Svelte frontend, Python sidecar, packaging lanes, and the preserved Classic Python application.

## License

Apache License 2.0. See `LICENSE`, `NOTICE`, and `THIRD_PARTY_NOTICES.md`.
