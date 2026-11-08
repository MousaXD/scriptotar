# Security policy

Current development and CI cover the **Scriptotar Next 1.3.0 preview candidate**, built with Rust/Tauri/Svelte and the transcription sidecar. **Scriptotar Classic 1.3.0** is historical source preserved under `archive/legacy-python/`. Identify the affected product line and version when reporting a problem; matching version numbers do not make their data, packages, or release channels interchangeable.

## Release surfaces and integrity

- `main` is the integration target; a candidate on another branch is not a published release.
- `tauri-next-latest` is the rolling Next prerelease channel.
- Classic's `continuous` channel and tags such as `v1.3.0` identify historical Classic artifacts, not a stable Next release.

The Next publisher requires its configured exact-main-commit gates and distribution approval. A passing installer or candidate version does not authorize skipping a failed check or publishing a stable tag. Packaging verifies pinned runtime/tunnel artifacts and includes provenance and license notices. Transient download retries and GitHub asset-API fallback retain the existing binary digests and size checks.

## Credentials and private content

Never include API keys, bearer tokens, secret tunnel URLs, cookies, browser profiles, private media, databases, model caches, signing keys, or private certificates in public reports or commits.

Next's AI BYOK keys are request/session values. They must not enter SQLite settings, browser persistent storage, ordinary configuration serialization, or logs. Copy Prompt requires no provider key; BYOK execution sends selected source/prompt content to the configured provider. Endpoint validation runs below the UI before credentials are attached.

The MCP wizard has a separate optional **Remember key** choice for the restricted tunnel-runtime key. That opt-in uses the operating-system keyring rather than SQLite or the repository. Connection path URLs are bearer credentials. Revoking/disconnecting and rotating credentials are desktop-owned operations.

Archived Classic's optional remembered AI key uses the historical keyring path; do not generalize that behavior into a claim that Next stores AI keys in settings.

## MCP authorization

MCP capabilities start disabled. Read-data, write-data, public creator research, media jobs, file reads/writes, overwrites, and command execution are independently granted in the desktop app. Authentication does not replace those grants. The MCP catalog has no tool for changing permissions or tokens.

File access is restricted to approved roots, and protected policy/database files are denied. Commands require an approved executable and receive a restricted environment. An approved shell/interpreter still runs with the user's OS privileges and can exceed ordinary file-tool boundaries; this is not an OS sandbox. Recheck the actual requested operation rather than treating any tool grant as blanket authorization.

The local HTTP gateway binds to loopback and enforces its token, Origin, and request-size rules. Policy/token changes affect subsequent and pending calls as implemented by the gateway. See [MCP setup and security](docs/mcp/README.md) for limits, revocation, and the command-process boundary.

## Data, media, and migration

Next uses Tauri's application-data directory for bundle identifier `io.github.mousaxd.scriptotar`, with `SCRIPTOTAR_DATA_DIR` available for diagnostics. Classic's historical XDG/Flatpak paths are separate. Legacy import stages a WAL-aware SQLite backup, does not overwrite the source database, and rejects unsafe or ambiguous source candidates. Preserve these properties when changing migrations or recovery.

Provider access failures must remain visible. Do not import private cookies into CI, substitute a different source, weaken media validation, or treat zero speech segments as an accuracy pass. Small compressed media can be valid: enforce nonempty/max-size, audio/duration, decoding, and inference checks rather than an arbitrary minimum compressed size. See [source-access diagnosis](docs/SOURCE_ACCESS.md).

## Reporting vulnerabilities

Use GitHub's private vulnerability-reporting/Security Advisory flow when available for issues involving secrets, authorization bypass, command execution, filesystem escapes, runtime tampering, or data corruption. Do not post working secrets, personal databases, or private media in a public issue.

Include the product line, version and commit/release channel, operating system, installation format, affected boundary, and a minimal safe reproduction. Use synthetic data where possible, and distinguish a provider/network restriction from a reproducible application defect.
