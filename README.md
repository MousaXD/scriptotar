# Scriptotar

Research creators, transcribe videos on your device, and turn source material into original content—all in one project-based desktop workspace.

Built with **Rust · Tauri v2 · Svelte 5 · Faster Whisper**. Available as Windows and Linux desktop packages, with an Arabic/English interface and optional MCP integration for assistants.

**Current source version: Scriptotar Next 1.4.0, an unpublished preview candidate.** Published downloads may have a different version. See [Releases](https://github.com/MousaXD/scriptotar/releases), [Changelog](CHANGELOG.md), and [release readiness](docs/releases/NEXT_1.4.0_RELEASE_READINESS.md).

[Features](#features) · [Install](#installation) · [First transcription](#your-first-transcription) · [AI Studio](#ai-studio) · [MCP setup](#mcp-setup) · [Troubleshooting](#troubleshooting) · [Development](#development)

## Features

### Projects and workspace

- Keep creator research, media jobs, transcripts, and AI runs organized by project.
- Create projects from Home or Settings and switch between them from the top bar.
- Resume the last valid selected project when the app restarts.
- Use Home to see current work, recent transcripts, creators, and AI activity.
- Search projects, transcripts, creators, research, and AI runs from the workspace search/command palette.
- Browse saved material in the Library, with text search, type filters, and sorting.

### Creator research and watchlists

- Scan public **YouTube, Instagram, and TikTok** creator/profile URLs.
- Compare available views, likes, comments, publication dates, and video durations.
- Filter by title, creator, or platform; sort by views, likes, comments, date, or observed interactions/views.
- Select individual research items and send them to the transcription queue.
- Save creator watchlists and optionally refresh them while Scriptotar is running.
- Inspect watchlist health, last scan/success, retry timing, and source errors.

Metrics depend on what the source returns. Interactions/views is a derived ratio of observed likes and comments to views, not a measure of unique viewers.

### Media queue and recovery

- Queue supported video URLs, read links from the clipboard, or drop links into the queue.
- Choose a local video through the native picker, drop a file, or enter its path.
- Import local `.mp4`, `.mkv`, `.mov`, `.webm`, `.m4v`, and `.avi` files.
- Choose **720p**, **1080p**, **Best**, or **Audio only** for URL acquisition.
- Set a maximum source duration and an output folder.
- Follow persisted job progress through preparation, download, transcription, and processing.
- Cancel active work, retry failed/interrupted/cancelled jobs, and open a completed job's linked transcript.
- Recover interrupted work after a restart. Valid prepared media can be reused; interrupted Whisper inference starts again from the beginning.

### On-device transcription

- Run **Faster Whisper** locally with `small`, `medium`, `turbo`, or `large-v3`.
- Choose **Auto**, **CPU**, or **CUDA** execution. Auto uses supported GPU acceleration when available and can fall back to CPU.
- Use automatic language detection or explicitly select Arabic or English.
- Optionally translate speech into English and use batched inference.
- Choose whether to copy local source files or retain failed-job artifacts.
- Reuse cached models and the engine between successful jobs.

Python, Faster Whisper dependencies, yt-dlp, FFmpeg, and ffprobe are included in packaged builds. **Model weights download on first uncached use.** After caching the selected model, local-file transcription can run offline. CUDA requires compatible NVIDIA hardware, drivers, and runtime libraries.

### Transcript reader and exports

- Read saved transcripts with timestamped segments and source information.
- Search within a transcript and move between matching segments.
- Copy transcript text or export **TXT**, **timestamp TXT**, **SRT**, **VTT**, and **JSON**.
- Use the transcript list and details panels independently; Arabic transcripts use right-to-left presentation.
- Search transcript contents across the workspace through the SQLite-backed search index.

The transcription runtime also writes cleaned-text output. Subtitle and timestamp exports need segments; a valid silent input can finish with no recognized speech.

### Interface and navigation

- Switch between **English and Arabic**, including right-to-left interface layouts.
- Use dark or system appearance and collapse the sidebar when you want more reading space.
- Open workspace search and commands with **Ctrl+K** / **Cmd+K**.
- Focus transcript search with **Ctrl+F** / **Cmd+F** while reading a transcript.
- Use keyboard navigation and focus controls throughout the main workflows.

## Installation

Download the **Next preview** assets actually listed on [GitHub Releases](https://github.com/MousaXD/scriptotar/releases). The rolling preview tag is `tauri-next-latest`.

| Platform | Rolling preview download |
| --- | --- |
| Windows x64 | `Scriptotar-Next-latest-x64-setup.exe` |
| Debian/Ubuntu Linux x64 | `Scriptotar-Next-latest-amd64.deb` |

### Windows

Run the downloaded NSIS installer, then launch Scriptotar. Preview installers are unsigned unless external signing is configured.

### Linux

Install the downloaded Debian package from its directory:

```bash
sudo apt install ./Scriptotar-Next-latest-amd64.deb
```

Candidate build artifacts use versioned names such as `Scriptotar-Next-1.4.0-x64-setup.exe` and `Scriptotar-Next-1.4.0-amd64.deb`. A successful CI build does not mean those candidate assets have been publicly released.

Next currently has no macOS, AppImage, or Flatpak distribution. See [distribution and runtime details](docs/NEXT_DISTRIBUTION.md).

## Your first transcription

1. Open Scriptotar and create a project, or use **Inbox**.
2. Open **Settings** and choose a model, device, language, and output folder. CPU works without an NVIDIA setup; model size affects speed and resource use.
3. Open **Queue**. Add a supported video URL or choose/drop a local video.
4. Queue the job and follow its actual progress. The first transcription with an uncached model needs internet access to retrieve its weights.
5. Once complete, choose **Open transcript** to read, search, copy, or export it.
6. Open **AI Studio** to use that transcript as an explicit source, or find it later in **Library**.

For creator research, open **Research**, enter a public profile URL, choose a scan limit, and select **Scan profile**. Review the returned items before choosing **Queue selected**. Save a watchlist if you want to revisit the creator.

## AI Studio

AI Studio has two modes:

| Mode | How it works |
| --- | --- |
| **Copy Prompt** | Builds a prompt locally. Copy it into your preferred AI tool; no API key or provider request is needed to build the prompt. |
| **Bring your own key** | Sends the selected prompt/source content to the provider only when you choose **Run with API**. Supports OpenAI, Anthropic, Gemini, and OpenAI-compatible endpoints. |

Choose a saved transcript or enter source text manually, then add a topic, audience, target duration, call to action, and voice direction. The selected source is explicit so an unrelated transcript is not silently used.

Available tasks:

- Viral breakdown and hook ideas.
- New short-form script and structure remix.
- Content ideas and caption + CTA.
- Voice profile and B-roll shot list.

The speaking-duration calculator shows word/character counts and estimated read time at slow, normal, or fast pacing. Completed API runs and their prompts remain associated with their project.

BYOK model names and credentials belong to the chosen provider. AI keys are session/request values, not ordinary persisted settings. The **Local (coming later)** provider option is not an available local LLM integration. See [security and privacy](SECURITY.md).

## MCP setup

Scriptotar exposes **14 MCP tools** for project data, research, queued transcription, and explicitly approved file/command operations. Permissions are controlled in the desktop app and start disabled.

Keep Scriptotar open while using its app-managed gateway or tunnel.

### 1. Enable the actions you need

1. Open **MCP Setup** in the sidebar, or **Settings → MCP setup wizard**.
2. Enable MCP and select the relevant permissions. For an initial read-only connection, enable **Read Scriptotar data**.
3. Add approved directories if you need local-file access, or approved executable paths if you need command execution.
4. Select **Save permissions & continue**. Powerful write/execute grants require the wizard's local acknowledgement.

Reading saved data, creating projects, scanning creators, queuing media, reading files, writing files, overwriting files, and executing commands have separate grants. You can connect successfully while a particular tool remains denied.

### 2. Choose a connection method

| Method | What you provide | What Scriptotar does |
| --- | --- | --- |
| **Cloudflare Quick Tunnel** | No Cloudflare account | Starts the bundled tunnel client and displays a temporary public HTTPS MCP URL. Suitable for development/testing. |
| **OpenAI Secure MCP Tunnel** | A tunnel ID and restricted tunnel-runtime API key | Starts the bundled OpenAI tunnel client and reports its readiness. |
| **My own HTTPS hostname** | A public HTTPS origin you manage | Displays the authenticated MCP URL for your hostname; you manage the external tunnel/reverse proxy. |

**Cloudflare Quick Tunnel:** select it, click **Connect**, and copy the complete URL shown by the wizard. It includes a secret path such as `https://<hostname>/mcp/p/<secret>`; keep that path intact. The hostname and secret can change on reconnection.

**OpenAI Secure MCP Tunnel:** create a tunnel through the [OpenAI tunnel settings](https://platform.openai.com/settings/organization/tunnels), then enter its ID and a restricted runtime key in Scriptotar. Creating/editing a tunnel requires Tunnels **Read + Manage**; running the client requires **Read + Use**. Remembering the runtime key is optional and uses the OS keyring. See the [official Secure MCP Tunnel guide](https://developers.openai.com/api/docs/guides/secure-mcp-tunnels).

**Your own hostname:** forward your HTTPS endpoint to `http://127.0.0.1:39761`. Enter the origin only, such as `https://mcp.example.com`, in Scriptotar; use the complete URL it generates in your MCP client.

### 3. Connect from ChatGPT

1. Open [ChatGPT Plugins](https://chatgpt.com/plugins), select the plus button, and choose **Add custom MCP server**.
2. Give the connection a name and description.
3. For Quick Tunnel or your own hostname, choose the public/server-URL connection and paste Scriptotar's full HTTPS URL. The wizard's secret-path URL authenticates requests through that URL; it does not require a separate OAuth flow.
4. For OpenAI Secure MCP Tunnel, choose **Tunnel** and select or enter the same tunnel ID.
5. Complete the client's authentication/risk review, create the plugin, and inspect the discovered tools.
6. Test with `list_projects` before requesting broader actions.

Account and workspace policies apply. Follow the [official ChatGPT connection instructions](https://developers.openai.com/plugins/deploy/connect-chatgpt#add-the-mcp-server) if your workspace restricts custom servers or its setup differs.

The wizard's **Ready** state establishes local/tunnel readiness; a successful tool call from your client verifies the complete connection.

### Local stdio clients

Clients that can spawn a local MCP process can run the installed executable with `--mcp-stdio`, without a public tunnel. For clients using an `mcpServers` configuration, a Linux example is:

```json
{
  "mcpServers": {
    "scriptotar": {
      "command": "/usr/bin/scriptotar-desktop",
      "args": ["--mcp-stdio"]
    }
  }
}
```

Use your actual installed executable path on other systems. The process uses the normal application-data location; if your desktop uses `SCRIPTOTAR_DATA_DIR`, configure the client with the same directory. App-owned permissions still apply.

### Available tools and permissions

| Tools | Capability | Required local grant |
| --- | --- | --- |
| `list_projects`, `list_research`, `list_transcripts`, `get_transcript`, `search_transcripts`, `list_jobs` | Read saved project data and job states | Read Scriptotar data |
| `create_project` | Create a project | Write Scriptotar data |
| `search_creator` | Scan a public creator and save research | Search creators |
| `download_video` | Queue URL download and transcription | Download videos |
| `enqueue_local_media` | Queue a local video | Download videos + read files + approved path |
| `list_directory`, `read_file` | Read approved local files | Read files + approved directories |
| `write_file` | Write UTF-8 text | Write files + approved directories; separate overwrite grant for replacements |
| `execute_command` | Run an allowlisted executable with arguments | Execute commands + approved executable path |

Queue tools return a job ID, not a finished transcript. Follow `list_jobs` and retrieve the completed transcript before analyzing it. Creator research returns metadata and does not itself transcribe videos.

Example requests after granting the relevant permissions:

- “List my projects and show the latest jobs in this project.”
- “Find transcripts about this topic, then summarize the selected transcript.”
- “Scan this public creator profile and save its research in my project.”
- “Queue this video and report its actual job state.”

### Disconnect and revoke

Choose **Disconnect & revoke URL** in Scriptotar to stop the managed connection/gateway and rotate its URL secret. Keep tunnel URLs, HTTP tokens, and runtime keys private. An approved shell/interpreter runs with your OS privileges; file/command permissions are not an operating-system sandbox.

For authentication details, limits, and advanced setup, see the [MCP guide](docs/mcp/README.md).

## Settings reference

| Area | Options |
| --- | --- |
| Transcription | Model, Auto/CPU/CUDA, automatic/Arabic/English language selection |
| Media | Quality, optional supported browser-cookie source, source-duration limit |
| Storage | Output directory or application default |
| Local processing | Copy source, translate to English, batched inference, retain failed artifacts |
| Watchlists | In-app automatic refresh and interval |
| Interface | English/Arabic, dark/system appearance |
| Migration | Legacy discovery, candidate selection, import status/retry |
| MCP | Permissions, approved paths/programs, connection, disconnect/revoke |

Browser-cookie use is an explicit local setting. The MCP queue tools do not accept cookie arguments, and CI does not import private browser sessions.

## Privacy, data, and migration

Transcription runs locally. Network access is used for source downloads/research, first-use model retrieval, selected MCP tunnels, and explicit BYOK provider requests. Once a model is cached, local-file inference can run offline. Copy Prompt does not contact an AI provider.

The Tauri bundle identifier is `io.github.mousaxd.scriptotar`:

| Platform | Default application-data directory |
| --- | --- |
| Linux | `${XDG_DATA_HOME:-~/.local/share}/io.github.mousaxd.scriptotar` |
| Windows | `%APPDATA%\io.github.mousaxd.scriptotar` |

The directory holds `scriptotar.sqlite3`, the model cache, and runtime/output state. `SCRIPTOTAR_DATA_DIR` overrides it for development and diagnostics. Legacy import stages a SQLite snapshot and preserves the source database; ambiguous candidates require a local choice. See [migration and recovery](docs/NEXT_MIGRATION.md).

The historical Python/Tkinter application, **Scriptotar Classic 1.3.0**, is preserved in [archive/legacy-python](archive/legacy-python/README.md). Its packages, history, and data layout are separate from the current Next preview.

## Troubleshooting

| Problem | What to check |
| --- | --- |
| YouTube/Instagram/TikTok rejects a URL | Confirm source access, availability, and any required account access. Public URLs can be challenged or region-restricted. See [source-access diagnostics](docs/SOURCE_ACCESS.md). |
| The first transcription cannot load a model | Check internet access and the model cache; package installation does not include model weights. |
| CUDA cannot start | Check compatible NVIDIA support, or select CPU/Auto. Explicit CUDA errors are surfaced rather than silently ignored. |
| A job was interrupted | Use its retry action. Valid prepared media may be reused, but interrupted inference restarts. |
| No speech appears | Inspect the source audio and language choice. Silent media can return zero segments; an execution pass is not an accuracy guarantee. |
| MCP connects but a tool is denied | Enable that specific permission in Scriptotar; approve the relevant file root or executable when needed. |
| A remote MCP client cannot connect | Keep the app open, use the full generated HTTPS URL or matching tunnel ID, and check your workspace's custom-server permissions. |
| An old Quick Tunnel URL stops working | Reconnect in Scriptotar and update the client's URL; its hostname/secret can change. |
| The UI is running in a browser with sample data | A standalone Vite session uses the mock API. Launch the Tauri desktop host for actual persistence and native workflows. |

## Development

CI uses **Node.js 22**, **Rust 1.97.1**, **Python 3.12**, and **Tauri CLI 2.11.2**. Linux builds need GTK/WebKit and related native libraries. See [CONTRIBUTING.md](CONTRIBUTING.md) for the environment and complete checks.

From the repository root:

```bash
npm ci --prefix apps/desktop-ui
npm run check --prefix apps/desktop-ui
npm test --prefix apps/desktop-ui
npm run build --prefix apps/desktop-ui
python3 -m unittest discover -s tests -p 'test_*.py' -v
PYTHONPATH=sidecars/transcription python3 -m unittest discover -s sidecars/transcription/tests -v
cargo test --workspace --locked
```

For a development desktop session after installing/configuring the sidecar dependencies:

```bash
cargo install tauri-cli --version 2.11.2 --locked
cd apps/desktop
cargo tauri dev
```

Rust owns the database, jobs, research, AI requests, and application services. Svelte is the frontend; the Python JSONL sidecar executes media/transcription. See [backend architecture](docs/architecture/rust-foundation.md) and the [sidecar protocol](sidecars/transcription/PROTOCOL.md).

## Verification and further reading

Unit/fixture tests, compiled-app flows, installed-package checks, live-provider runs, and speech-accuracy acceptance establish different things. The full local canary at source commit `42349aa` passed real YouTube/Instagram/TikTok download, decoding, Whisper inference, and exports; YouTube/TikTok returned zero speech segments. Hosted CI separately encountered YouTube bot/sign-in verification. See the exact environment and outcomes in the linked evidence.

- [E2E coverage and boundaries](docs/E2E-COVERAGE-1.3.0.md)
- [Local real-provider evidence](docs/evidence/2026-11-08-local-live-platforms.json)
- [Release readiness](docs/releases/NEXT_1.4.0_RELEASE_READINESS.md) and [publisher contract](sidecars/transcription/RELEASE-PUBLISHING.md)
- [Version policy](docs/VERSIONING.md) and [changelog](CHANGELOG.md)
- [Security and private reporting](SECURITY.md)

## License

[Apache-2.0](LICENSE). See [NOTICE](NOTICE) and [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) for attribution and bundled dependency information.
