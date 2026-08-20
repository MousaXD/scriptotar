# Contributing to Scriptotar

Thanks for improving Scriptotar. The repository contains two supported application lines:

- **Scriptotar Next 0.1.x**: Rust + Tauri 2 + Svelte with the Python transcription sidecar.
- **Scriptotar Classic 1.2.x**: the preserved Python/Tkinter application.

Do not remove or silently break Classic while working on Next. Keep changes at the correct architectural boundary and add tests for behavior that crosses persistence, process, network, or IPC boundaries.

## Repository map

```text
Cargo.toml                         Rust workspace
crates/                            Rust domain/service/repository crates
apps/desktop/src-tauri/            Tauri desktop host
apps/desktop-ui/                   Svelte + TypeScript frontend
sidecars/transcription/            Python JSONL transcription runtime
scriptotar.py + *_mixin.py         Scriptotar Classic GUI/application
worker.py / creator.py             Classic worker/research/AI helpers
tests/                             Classic and repository contract tests
docs/                              Architecture, distribution, migration, versioning
.github/workflows/                 CI and packaging
```

## Scriptotar Next development

### Linux prerequisites

On Debian/Ubuntu/Pop!_OS, the integrated Tauri build uses the normal WebKit/GTK development dependencies:

```bash
sudo apt update
sudo apt install -y \
  libwebkit2gtk-4.1-dev \
  libappindicator3-dev \
  librsvg2-dev \
  libgtk-3-dev \
  libssl-dev \
  patchelf \
  zenity \
  python3 \
  python3-venv
```

Use a current Rust toolchain, Node.js 22-compatible tooling, and Python 3.12 when reproducing the packaging environment.

### Rust workspace

From the repository root:

```bash
cargo fmt --all -- --check
cargo check --workspace --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --locked
```

Important ownership rules:

- `scriptotar-core` owns stable domain contracts.
- `scriptotar-db` owns the Next SQLite schema and migrations.
- `scriptotar-jobs` owns job lifecycle operations.
- `scriptotar-media` owns media/sidecar host contracts.
- `scriptotar-orchestrator` owns durable sidecar process orchestration.
- `scriptotar-ai` owns AI provider execution and endpoint policy.
- `scriptotar-research` owns creator-research execution and network policy.
- Tauri command handlers should remain thin adapters over services.

### Svelte frontend

```bash
cd apps/desktop-ui
npm ci
npm run check
npm run test
npm run build
```

A browser-only Vite run uses the mock `ScriptotarApi`. The production Tauri window injects the real Tauri-backed client. Do not bypass that API seam by scattering raw host calls through components.

### Tauri application

The integrated production-equivalent compile used by CI is:

```bash
cd apps/desktop
cargo tauri build --no-bundle -- --locked
```

For local Tauri development, install the Tauri CLI version used by CI and launch from `apps/desktop` after installing frontend dependencies.

Packaged releases use generated platform bundle configuration, so a successful `cargo build` alone is not proof that an installer contains or can discover the packaged transcription runtime.

### Python transcription sidecar

Fast protocol/unit tests do not download Whisper models or contact live social platforms:

```bash
cd sidecars/transcription
PYTHONPATH=. python3 -m compileall -q .
PYTHONPATH=. python3 -m unittest discover -s tests -v
```

To verify the real pinned engine dependency set without downloading a model:

```bash
python3 -m pip install -r sidecars/transcription/requirements-engine.txt
python3 -m pip check
python3 -c 'import faster_whisper, yt_dlp; print(faster_whisper.__version__, yt_dlp.version.__version__)'
```

The public sidecar protocol is documented in `sidecars/transcription/PROTOCOL.md`. Rust owns the application database; the sidecar must not open or mutate it.

## Next packaging

The authoritative package workflows are:

- `.github/workflows/windows-tauri.yml`: builds and smoke-tests the Windows NSIS installer.
- `.github/workflows/linux-tauri.yml`: builds and smoke-tests the Linux Debian package.
- `.github/workflows/tauri-next-release.yml`: assembles the rolling `tauri-next-latest` preview from validated package artifacts.

Package smoke tests validate installed resource discovery. Do not weaken them to source-tree-only checks.

The packaged Next runtime is intentionally self-contained for Python/Faster Whisper/yt-dlp/FFmpeg, while Whisper model weights remain first-use downloads.

## Scriptotar Classic development

Classic remains a working Python/Tkinter application and continues to have its own Linux release line.

On Debian/Ubuntu/Pop!_OS:

```bash
sudo apt install python3 python3-venv python3-tk ffmpeg libsecret-tools xvfb dpkg-dev
python3 -m unittest discover -s tests -v
```

Run Classic from source:

```bash
python3 scriptotar.py
```

Build the Classic Debian package:

```bash
./build-deb.sh
```

Portable Classic packages are built by `packaging/build-appimage.sh` and `io.github.mousaxd.scriptotar.yml`; CI also validates them on non-PR Linux release runs.

## Version identity

Do not infer that the Classic and Next numbers should match.

- Classic currently uses `1.3.0`.
- Next currently uses `0.1.0` across the Rust workspace, Tauri package identity, Svelte package, and transcription sidecar.
- Sidecar protocol version `1` is a wire-protocol compatibility number, not an application version.

Read `docs/VERSIONING.md` before changing any version. A repository test verifies the current cross-file identity contract.

## Pull requests

Keep changes focused. Add tests for parsing, persistence, prompt generation, migrations, security-sensitive input handling, package/runtime boundaries, and process recovery as appropriate.

Never commit API keys, cookies, browser profiles, downloaded creator media, Whisper model caches, generated databases, private certificates/keys, or local environment files.

For source-platform integrations, preserve the local-first model and document when a feature contacts a third party. CI should use deterministic local fixtures rather than live Instagram, TikTok, YouTube, or paid AI requests.
