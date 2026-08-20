# Scriptotar Classic (Legacy Python / Tkinter)

This directory contains the historical Python/Tkinter implementation of Scriptotar (v1.0.0 – v1.3.0).

## Historical Context

Scriptotar began as a standalone Python utility using Tkinter and system FFmpeg. Starting with version **1.0.0**, Scriptotar was completely rewritten and modernized into a cross-platform desktop application built with:

- **Backend**: Rust + Tauri v2
- **Frontend**: Svelte 5 + CSS Tokens + Lucide Icons
- **Database**: SQLite with FTS5 Full-Text Search
- **Runtime**: Self-contained Faster Whisper + FFmpeg + yt-dlp sidecar

The primary codebase now resides in:
- `apps/desktop/` (Tauri & Rust backend)
- `apps/desktop-ui/` (Svelte 5 UI)
- `crates/` (Modular Rust domain crates)
- `sidecars/` (Self-contained transcription engine)

These files are preserved here for historical reference.
