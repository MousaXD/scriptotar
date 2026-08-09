# Third-party notices

Scriptotar is distributed under the Apache License 2.0. Its optional/private transcription engine installs third-party packages into the user's local Scriptotar virtual environment.

Important runtime components include:

- **faster-whisper**: Whisper inference using CTranslate2. Review the upstream project and its transitive dependencies for their respective licenses.
- **yt-dlp**: media metadata extraction/downloading. Review the upstream project and extractor/platform terms.
- **FFmpeg**: system-provided media processing. The exact FFmpeg license depends on the distributor build configuration.
- **Tk / Tkinter**: Python GUI toolkit bindings supplied by the operating system.
- **libsecret / secret-tool**: Linux Secret Service integration used only when the user chooses to remember an AI key.

Scriptotar does not bundle Meedro code, branding, datasets, stock-media catalogs, or proprietary assets.
