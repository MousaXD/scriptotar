# Scriptotar

Scriptotar is an Apache-2.0-licensed, local-first cross-platform desktop application for short-form video research, downloading, transcription, and AI-assisted content development.

Built with **Rust (Tauri v2)** and **Svelte 5**, Scriptotar provides high-performance local AI workflows with self-contained runtime packaging for Windows and Linux.

## Overview

- **Version**: `1.0.0`
- **Supported Platforms**: Windows (`.exe` NSIS installer) and Linux (`.deb` package)
- **Zero Configuration**: Bundles Faster Whisper, FFmpeg, and yt-dlp. No separate Python or FFmpeg installation required.
- **Bilingual & RTL**: Native Arabic and English localization with instant language switching.
- **Smart Queue**: Drag-and-drop local video files and web links, one-click clipboard link ingestion, and robust job recovery.
- **Creator Research**: Scan creator profiles across YouTube, TikTok, and Instagram with metrics and watchlists.
- **AI Studio**: Interactive Speaking Calculator, prompt crafting with anti-plagiarism guardrails, and BYOK integration (OpenAI, Anthropic, Gemini, OpenAI-compatible).
- **Search**: Fast SQLite Full-Text Search (FTS5) with Arabic morphology support.

---

## Installation

Download the latest release for your platform from the [GitHub Releases](https://github.com/MousaXD/scriptotar/releases) page:

### Windows
Download and run the installer:
- **`Scriptotar-1.0.0-x64-setup.exe`** (or `Scriptotar-latest-x64-setup.exe`)

### Linux (Debian / Ubuntu / Pop!_OS)
Download and install the Debian package:
```bash
sudo apt install ./Scriptotar-1.0.0-amd64.deb
```

---

## Architecture & Data Location

Scriptotar uses Tauri's application-data directory:
- **Linux**: `${XDG_DATA_HOME:-~/.local/share}/io.github.mousaxd.scriptotar`
- **Windows**: `%APPDATA%\io.github.mousaxd.scriptotar`

Important data within that directory:
- `scriptotar.sqlite3`: Rust-owned SQLite database with FTS5 index.
- `models/`: Faster Whisper model cache.

---

## Historical Archive

The original Python/Tkinter implementation (v1.0.0 – v1.3.0) has been archived in [`archive/legacy-python/`](archive/legacy-python/) for historical reference.

---

## Privacy & Local Execution

- **Local Processing**: Media transcription runs 100% offline and on-device via Faster Whisper.
- **Source Downloads**: URL downloads and creator profile research contact the requested source platforms (YouTube, TikTok, Instagram) through `yt-dlp`.
- **AI Privacy**:
  - **Copy Prompt**: 100% local. Nothing is sent to any external provider.
  - **BYOK Direct Execution**: Uses your own API key for the current session only. Keys are not saved in unencrypted databases.
- **Zero Telemetry**: No user tracking or analytics.

---

## Development & Building

### Prerequisites
- Node.js 20+ and npm
- Rust 1.80+ (via rustup)
- Python 3.12 (for transcription engine bundle)

### Quick Start
```bash
# Install frontend dependencies
cd apps/desktop-ui
npm install

# Run frontend test suite
npm test

# Run Rust workspace test suite
cargo test --workspace
```

---

## License

Licensed under the [Apache-2.0 License](LICENSE). See `LICENSE`, `NOTICE`, and `THIRD_PARTY_NOTICES.md`.
