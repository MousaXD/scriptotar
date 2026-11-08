# Source access and real-media checks

Scriptotar uses yt-dlp to obtain source metadata and media. A public video URL
does not guarantee anonymous access from every machine or network. Source
access, file validation, Whisper inference, and recognition accuracy are
separate outcomes.

## YouTube diagnosis recorded on 2026-11-08

The pinned test URL is:

```text
https://youtu.be/2PuFyjAs7JA?si=yqHYehEncAJH67oB
```

| Environment / check | Observed result | What it establishes |
| --- | --- | --- |
| Hosted CI, commits `427cbcd` and `42349aa` | YouTube requested sign-in/bot verification before metadata extraction completed. | That runner could not access this source anonymously. |
| Developer machine, yt-dlp `2026.10.21` and `2026.10.26` | The same URL's metadata loaded without cookies or sign-in. | The source was accessible in that local environment. |
| Developer machine, unchanged Scriptotar downloader and pinned `2026.10.21` | Downloaded 5,286 bytes of Opus audio; FFprobe measured 11.021 seconds; FFmpeg decoded it successfully. | A real valid audio download succeeded locally. This does not prove Whisper inference. |
| Developer machine, corrected canary at `42349aa` | All three exact sources passed real download, decoding, Faster Whisper `small` CPU inference, and six export-type checks. YouTube/TikTok returned zero segments, Instagram three. | Local source-tree inference/export execution passed; this is not hosted-CI or installed-package evidence. |
| Old canary size rule | Rejected that valid file because it was smaller than 8 KiB. | A separate test-validation bug existed after the download. |

The canary correction in `42349aa` accepts nonempty compressed files up to the
existing 300 MiB limit. Expected provider, audio-track/duration checks, FFmpeg
decoding, real CPU inference, and export checks remain in the full canary.
Regression coverage rejects empty, oversized, and invalid-audio downloads.
This correction does not authenticate a request or resolve the hosted runner's
sign-in challenge. A later run must establish its own result.

The full local rerun used the pinned engine, no cookies, and an initially empty
temporary model cache. The [retained result summary](evidence/2026-11-08-local-live-platforms.json)
records the exact source commit, environment, model, and per-platform outcomes.
YouTube and TikTok produced zero speech segments; that does not prove recognition
accuracy. Instagram produced three segments. The hosted CI challenge remains a
separate result; local success does not turn its failed check green.

## Diagnose the stage that failed

- **Metadata/source access:** look for `SOURCE_ACCESS_FAILED`, sign-in/bot
  messages, unavailable videos, network errors, or regional restrictions. The
  job has not reached Whisper at this point.
- **Downloaded media:** check the actual file, expected provider, nonzero size,
  audio track, finite positive duration, and decoding. Compressed size alone
  cannot establish validity; short or silent audio can be very small.
- **Model retrieval:** the selected Whisper model must be available in the
  configured cache. A first-use model-host failure is separate from a source
  platform rejecting a video.
- **Inference/export:** only report success after the requested model/device
  runs and the resulting transcript/output files are validated. Report empty
  text or zero segments explicitly.

Preserve the user's exact URL. Do not replace it with another video, substitute
fixture output, ignore a failed provider, or convert access failure into a green
check. Use browser/account access only when the user explicitly chooses that
supported workflow; CI does not import private browser cookies.

The current engine pins are in
[`requirements-engine.txt`](../sidecars/transcription/requirements-engine.txt).
Both tested downloader versions could read the source locally, so those tests
do not establish that upgrading yt-dlp fixes the CI challenge. Downloader
updates also need the existing dependency and packaged-license review.

yt-dlp's supported JavaScript-runtime requirements are documented in its
[EJS guide](https://github.com/yt-dlp/yt-dlp/wiki/EJS). Standalone diagnostic
flags/configuration do not automatically configure Scriptotar's Python API
calls or add a runtime to an installer. Do not describe a separately installed
Node/Deno test as proof of packaged application support.

## Reproduce through the application downloader

From the repository root, with Python 3.12 and FFmpeg/ffprobe available:

```bash
python3 -m venv .venv
.venv/bin/python -m pip install -r sidecars/transcription/requirements-engine.txt
.venv/bin/python -m pip check
PYTHONPATH=sidecars/transcription .venv/bin/python -m unittest discover -s tests -p test_live_media_contract.py -v
PYTHONPATH=sidecars/transcription .venv/bin/python scripts/live_media_e2e.py --platform youtube --download-only --summary-json /tmp/scriptotar-youtube-download.json
```

That last command checks a real download and media probe. It deliberately does
not certify transcription. For real download, decoding, Faster Whisper `small`
CPU inference, and all transcript exports, omit `--download-only`:

```bash
HF_HOME=/tmp/scriptotar-live-model-cache PYTHONPATH=sidecars/transcription .venv/bin/python scripts/live_media_e2e.py --platform youtube --summary-json /tmp/scriptotar-youtube-full.json
```

The full run downloads the model if uncached. Omit `--platform youtube` to run
all three exact sources. Each provider gets a separate result; any provider
failure makes the run fail. Use an isolated cache for a first-use retrieval
test and retain the JSON/log as evidence, without committing downloaded media,
models, credentials, or user databases.

For known spoken-content accuracy and a fresh-process offline cache test, use
the separate [packaged Whisper canary](../sidecars/transcription/WHISPER-CANARY.md).
