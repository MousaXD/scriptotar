# Scriptotar 1.1.0

Native Linux desktop app for downloading and transcribing Instagram Reels, TikTok, YouTube/Shorts, and local video files.

## Highlights in 1.1

- Queue multiple URLs and local videos.
- Persistent faster-whisper worker reuses the loaded model between jobs.
- Cancellation terminates the whole worker process group, including FFmpeg children.
- Temporary `.partial` job folders are atomically renamed only after success.
- SQLite history for completed/failed jobs.
- Built-in transcript viewer/editor with Arabic RTL alignment.
- Word-level timestamps.
- Cleaner subtitle generation.
- TXT, cleaned TXT, timestamped TXT, SRT, VTT, and JSON outputs.
- 720p/1080p/best/audio-only download modes.
- 30m/60m/2h/unlimited duration safety limits.
- Browser-cookie options for Instagram.
- `small`, `medium`, `turbo`, and `large-v3` model choices.
- Optional batched inference.
- Pinned engine versions with `pip check` and import verification.

## Install

```bash
sudo apt install ./scriptotar_1.1.0_all.deb
```

Launch from your app menu or:

```bash
scriptotar
```

On first use click **Install / Repair Engine**. The private Python engine is stored in:

```text
~/.local/share/scriptotar/venv
```

No system Python packages are overwritten.

## Output

Each successful job creates a folder containing:

```text
video.* or audio.*
transcript.txt
transcript_clean.txt
transcript_timestamps.txt
transcript.srt
transcript.vtt
transcript.json
```

## Notes

- Transcription is local after media/model files are present.
- URL downloads still contact the source site.
- The first use of each Whisper model downloads its model files.
- Instagram can require an authenticated browser session. Select the browser in Settings.
- CUDA mode requires a compatible NVIDIA CUDA/cuDNN setup for CTranslate2/faster-whisper.

