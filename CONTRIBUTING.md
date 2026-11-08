# Contributing to Scriptotar

Current development targets **Scriptotar Next 1.3.0**, the Rust/Tauri/Svelte preview. The Python/Tkinter **Classic 1.3.0** sources are historical and live under `archive/legacy-python/`. Preserve that archive and the safety of existing user data; Next's publication workflow does not publish a new Classic release.

## Repository map

```text
Cargo.toml                         Rust workspace and shared application version
crates/                            Domain, database, jobs, media, AI, research, MCP
apps/desktop/src-tauri/             Tauri desktop host
apps/desktop-ui/                    Svelte + TypeScript frontend
sidecars/transcription/             Python JSONL media/transcription runtime
archive/legacy-python/              Historical Classic source and packaging
scripts/ and tests/                 Repository contracts and integration helpers
docs/                              Architecture, distribution, migration, test evidence
.github/workflows/                  CI, installed-package checks, preview publisher
```

## Development environment

Match the CI toolchain when reproducing a build: Node.js 22, Rust 1.97.1 with rustfmt/Clippy, Python 3.12, and Tauri CLI 2.11.2. Package/runtime dependency pins are in `sidecars/transcription/requirements-engine.txt` and `requirements-bundle.txt`; JavaScript and Rust dependencies use their checked-in lockfiles.

On Debian/Ubuntu, the repository helper installs the GTK/WebKit, OpenSSL, DBus, and packaging libraries used by CI:

```bash
bash scripts/install_tauri_deps.sh
```

The helper uses sudo/apt. The desktop's Linux native picker also uses `zenity`. Installed-GUI acceptance uses Xvfb. See the package workflow for its complete environment rather than assuming a bare `cargo build` produces an installer.

## Frontend and desktop

Run frontend checks from the repository root:

```bash
npm ci --prefix apps/desktop-ui
npm run check --prefix apps/desktop-ui
npm test --prefix apps/desktop-ui
npm run build --prefix apps/desktop-ui
```

A browser-only development session is useful for UI work:

```bash
npm run dev --prefix apps/desktop-ui
```

It uses the mock `ScriptotarApi`. The Tauri window selects the real client. Keep host calls behind `apps/desktop-ui/src/api/`; do not open SQLite or spawn processes from frontend components.

For the integrated desktop shell:

```bash
cargo install tauri-cli --version 2.11.2 --locked
cd apps/desktop
cargo tauri dev
```

The development host needs a configured Python engine environment for real transcription. For example, bootstrap an isolated environment from the repository root:

```bash
python3 sidecars/transcription/bootstrap.py --venv .venv
```

Set `SCRIPTOTAR_SIDECAR_PYTHON` to that environment's absolute Python executable before launching the desktop. Packaged applications use their bundled runtime instead. See [sidecar development](sidecars/transcription/README.md).

## Rust and repository contracts

Build frontend assets before checking the integrated host, then run the checks appropriate to the change:

```bash
cargo fmt --all -- --check
cargo check --workspace --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --locked
python3 -m unittest discover -s tests -p 'test_*.py' -v
python3 -m unittest discover -s scripts/tests -v
python3 -m unittest discover -s .github/tests -v
python3 scripts/check_github_action_pins.py
python3 scripts/check_repo_hygiene.py
```

Rust tests include real localhost servers. An environment that forbids local socket binding can fail those tests before product behavior is exercised; diagnose that restriction rather than removing the test.

The integrated production-equivalent compile is run from `apps/desktop`:

```bash
cargo tauri build --no-bundle -- --locked
```

Ownership follows the existing boundaries: `scriptotar-core` defines domain/repository contracts; `scriptotar-db` owns SQLite and migrations; jobs/media/orchestrator own durable execution and the sidecar host; AI/research own provider execution and network policy; `scriptotar-mcp` owns tool authorization and transports. Tauri commands adapt these services to the UI.

## Python sidecar

Fast tests do not need engine dependencies, live platforms, or Whisper models:

```bash
PYTHONPATH=sidecars/transcription python3 -m compileall -q sidecars/transcription
PYTHONPATH=sidecars/transcription python3 -m unittest discover -s sidecars/transcription/tests -v
```

For real engine imports, use an isolated environment:

```bash
python3 -m venv .venv
.venv/bin/python -m pip install -r sidecars/transcription/requirements-engine.txt
.venv/bin/python -m pip check
.venv/bin/python -c 'import faster_whisper, yt_dlp; print(faster_whisper.__version__, yt_dlp.version.__version__)'
```

These paths use POSIX shell conventions; Windows environments use their `Scripts` directory. The sidecar owns media execution and protocol v1, and must not open the Rust application database. See [PROTOCOL.md](sidecars/transcription/PROTOCOL.md).

## Packaging and live checks

- `integration.yml` checks Rust/frontend/sidecar integration and dependency audits.
- `mcp-e2e.yml` exercises real MCP processes and the compiled desktop executable.
- `windows-tauri.yml` and `linux-tauri.yml` build/install actual packages and validate their bundled resources.
- `tauri-next-release.yml` is the sole rolling preview publisher, with exact-main-commit gates and the existing distribution approval variable.

Normal unit and installed-app flows use clearly identified deterministic providers. Separate live canaries contact the pinned YouTube/Instagram/TikTok URLs and download a real Whisper model. They fail on source-access or inference errors. Keep those outcomes distinct from fixture tests and retain exact source URLs. See [source-access diagnosis and local commands](docs/SOURCE_ACCESS.md) and [the E2E inventory](docs/E2E-COVERAGE-1.3.0.md).

## Versioning and pull requests

The workspace, Tauri, frontend, sidecar, and local lockfile package identities are aligned at `1.3.0`; protocol v1 is independent. Follow [VERSIONING.md](docs/VERSIONING.md) when changing them. A candidate version is not a publication approval.

Run meaningful checks for changed parsing, persistence, migrations, process cleanup, IPC, or authorization behavior. Respect the requested scope for local changes, commits, pushes, merges, and publication. Do not change release gates to obtain a green check.

Never commit API keys, tunnel URLs/tokens, cookies, browser profiles, downloaded media, model caches, generated databases, private keys, or local environment files. Use synthetic fixtures for public bug reports. Historical Classic reproduction belongs inside `archive/legacy-python/` with its own dependencies; its old entry points are not root-level Next commands.
