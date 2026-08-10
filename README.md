# Scriptotar 1.2

Scriptotar is an Apache-2.0-licensed, local-first Linux desktop app for short-form video research, downloading, transcription, and AI-assisted content development.

It accepts Instagram Reels, TikTok, YouTube/Shorts, creator/profile URLs supported by `yt-dlp`, and local video files. The media/transcription engine stays local. AI is optional.

## What 1.2 adds

### Research / “viral finder” workflow

- Scan public creator/profile URLs through the same private `yt-dlp` engine.
- Collect available title/description, views, likes, comments, publish date, duration, and source URL.
- Sort research results by useful metrics.
- Queue selected discoveries directly for download + Whisper transcription.
- Export research results to CSV.
- Save creator watchlists and optionally refresh them while Scriptotar is running.
- No bundled scraped database and no claim of access to private metrics.

### Local content library

Scriptotar now stores a unified SQLite library containing:

- completed transcripts;
- public creator research metadata;
- AI prompts and AI results;
- per-project organization.

This is the open-source equivalent of a hosted “viral library”: it is **your** library, built from sources you choose.

### AI Studio: two modes

Scriptotar’s AI layer is deliberately optional.

**1. Copy prompt only**

- Works without an API key.
- Scriptotar builds a complete prompt from the transcript/research, topic, audience, duration, CTA, and voice instructions.
- Copy it into ChatGPT, Claude, Gemini, a local model, or any other tool.
- Nothing is sent to an AI provider by Scriptotar.

**2. Use API key**

Bring your own key for:

- OpenAI Responses API;
- Anthropic Messages API;
- Gemini `generateContent`;
- custom OpenAI-compatible `/chat/completions` endpoints.

API keys are never written into `settings.json`. When Linux Secret Service is available, **Remember in keyring** uses `secret-tool`; otherwise the token remains in memory for that app session only.

### AI tasks

- Viral breakdown
- Hook ideas
- New short-form script
- Structure remix
- Content ideas
- Caption + CTA
- Voice profile
- B-roll shot list

The Structure Remix prompt explicitly asks the model to reuse only abstract structure and **not** reproduce distinctive wording, catchphrases, jokes, or long phrases from a source creator.

### Projects

Create separate local projects for brands, clients, niches, or channels. New transcripts, research, watchlists, and AI runs are attached to the active project.

### Script timer

AI Studio estimates spoken duration from the source text so a creator can target short-form runtime before recording.

## Existing transcription features preserved

- Queue multiple URLs and local videos.
- Persistent `faster-whisper` worker reuses the loaded model between jobs.
- Whole-process-group cancellation, including FFmpeg children.
- Interrupted-job recovery using `.partial` folders.
- SQLite history.
- Built-in editable transcript viewer with Arabic RTL alignment.
- Word-level timestamps.
- TXT, cleaned TXT, timestamped TXT, SRT, VTT, and JSON outputs.
- 720p / 1080p / Best / Audio-only download choices.
- Browser-cookie selection for sites that require an authenticated browser session.
- `small`, `medium`, `turbo`, and `large-v3` Whisper model choices.
- CPU/CUDA selection and optional batched inference.

## Install

### Latest automatic builds

Every successful push or merge to `main` refreshes the rolling **Scriptotar Latest** GitHub release once the Debian package lane succeeds.

The Debian artifact is the required rolling-release package:

- `scriptotar-latest_all.deb` for Debian, Ubuntu, Pop!_OS, and derivatives.

The portable packaging lane is independent. When it succeeds, the same rolling release also includes:

- `Scriptotar-latest-x86_64.AppImage` as the portable x86_64 build;
- `Scriptotar-latest-x86_64.flatpak` as a single-file Flatpak bundle.

A temporary AppImage or Flatpak packaging failure therefore does not block publication of a valid Debian rolling build. Permanent version-tag releases such as `v1.2.0` require the Debian and portable packaging jobs to succeed and publish all three formats.

### Debian / Ubuntu / Pop!_OS

```bash
sudo apt install ./scriptotar-latest_all.deb
```

### AppImage

```bash
chmod +x Scriptotar-latest-x86_64.AppImage
./Scriptotar-latest-x86_64.AppImage
```

The AppImage bundles the GUI Python/Tk runtime and FFmpeg-facing runtime libraries. On first launch it keeps a small persistent Python base under Scriptotar's data directory so the private Whisper virtualenv remains valid across AppImage remounts.

### Flatpak bundle

```bash
flatpak install --user ./Scriptotar-latest-x86_64.flatpak
flatpak run io.github.mousaxd.scriptotar
```

The Flatpak bundle references the Freedesktop 24.08 runtime. Flatpak may download that runtime during installation if it is not already installed. The Flatpak uses its own XDG data directory for settings, history, and the private Whisper engine.

Launch the Debian install from your app menu or:

```bash
scriptotar
```

On first use click **Install / Repair Engine**. Scriptotar creates a private Python environment under:

```text
~/.local/share/scriptotar/venv
```

No system Python packages are overwritten.

## Output

A successful transcription creates a result directory containing:

```text
video.* or audio.*
transcript.txt
transcript_clean.txt
transcript_timestamps.txt
transcript.srt
transcript.vtt
transcript.json
```

The application database lives at:

```text
~/.local/share/scriptotar/history.sqlite3
```

## Privacy model

- Local video transcription happens on-device after model/media files are available.
- URL downloads and creator scans contact the source website through `yt-dlp`.
- Prompt-only AI mode sends nothing to an AI provider.
- API mode sends the generated prompt to the provider selected by the user.
- Scriptotar has no telemetry, hosted account, or mandatory cloud service.
- Remembered AI tokens use the Linux desktop Secret Service, not Scriptotar’s settings file.

## Responsible research

Scriptotar is a research and transformation tool, not a license to republish other people’s work. Platform terms and copyright still apply. Metadata availability varies by platform, login state, region, extractor support, and source changes.

The app intentionally does not ship a copy of another service’s private “viral database”, paid stock catalog, branding, or copyrighted clip collection.

## Build from source

On Debian/Ubuntu/Pop!_OS, build the Debian package with:

```bash
sudo apt install python3 python3-venv python3-tk ffmpeg libsecret-tools dpkg-dev
./build-deb.sh
```

Build the AppImage with:

```bash
sudo apt install python3 python3-venv python3-tk ffmpeg libsecret-tools curl
./packaging/build-appimage.sh
```

Build the Flatpak bundle with `flatpak-builder` using `io.github.mousaxd.scriptotar.yml`. GitHub Actions performs all three packaging builds automatically on `main` and on version tags.

Run tests directly:

```bash
python3 -m unittest discover -s tests -v
```

## Engine versions

The 1.2 application keeps the proven 1.1 transcription engine version, so upgrading the UI/research layer does not force existing users to reinstall Whisper unnecessarily.

Pinned engine packages:

```text
faster-whisper==1.2.1
yt-dlp[default,curl-cffi]==2026.7.4
```

## License

Apache License 2.0. See `LICENSE`, `NOTICE`, and `THIRD_PARTY_NOTICES.md`.
