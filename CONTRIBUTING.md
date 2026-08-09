# Contributing to Scriptotar

Thanks for improving Scriptotar.

## Development setup

On Debian/Ubuntu/Pop!_OS:

```bash
sudo apt install python3 python3-venv python3-tk ffmpeg libsecret-tools xvfb dpkg-dev
python3 -m unittest discover -s tests -v
```

Run the app from source:

```bash
python3 scriptotar.py
```

Build the package:

```bash
./build-deb.sh
```

## Pull requests

Keep changes focused. Add tests for parsing, persistence, prompt generation, migrations, and security-sensitive input handling. Never commit API keys, cookies, browser profiles, downloaded creator media, or Whisper model caches.

For source-platform integrations, preserve the local-first model and document when a feature contacts a third party.
