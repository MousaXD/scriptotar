# Security policy

Scriptotar currently has two supported application lines: the **Scriptotar Next 0.1.x preview** and the **Scriptotar Classic 1.2.x** legacy/stable line. Security fixes should preserve both unless a change explicitly documents a narrower affected surface.

## Supported release surfaces

- `main`: current development and integration target.
- `tauri-next-latest`: rolling Scriptotar Next preview channel.
- `continuous` / **Scriptotar Latest**: rolling Scriptotar Classic channel.
- Current permanent Classic release tags such as `v1.2.0`.

Scriptotar Next is still a prerelease. A preview label is not permission to weaken secret handling, input validation, package integrity checks, or local-data protections.

## Secrets

Never include API keys, cookies, browser profiles, access tokens, downloaded private media, database files, model caches, private certificates, or signing keys in bug reports or commits.

### Scriptotar Next

Next treats BYOK credentials as request-time/session values. API keys are not part of `ApplicationSettings` and must not be written to the Rust-owned SQLite database, browser storage, logs, crash output, or normal settings serialization.

Custom AI endpoints are validated below the UI boundary before credentials are attached. Remote plaintext HTTP endpoints are not accepted; loopback development endpoints are handled by the backend's explicit endpoint policy.

### Scriptotar Classic

Classic keeps the AI key in memory unless the user explicitly chooses **Remember in keyring**. On Linux, that option uses Secret Service through `secret-tool`; the key is not written into `settings.json`.

## Local data and migration

Next stores its database in Tauri's application-data directory under bundle identifier `io.github.mousaxd.scriptotar.next`. Classic uses the `scriptotar` XDG data directory, with a separate Flatpak sandbox path when installed as Flatpak.

The Next migration bridge must not overwrite the source Classic/WesamBoss database. Legacy discovery rejects unsafe candidates and refuses to guess when multiple distinct databases are present.

Do not attach real user databases to public issues. Create a minimal synthetic reproduction instead.

## Reporting vulnerabilities

Prefer GitHub's private vulnerability reporting / Security Advisory flow for vulnerabilities that could expose secrets, execute commands, bypass URL or endpoint policy, escape intended filesystem boundaries, tamper with packaged runtimes, or corrupt local data.

Avoid posting proof-of-concept secrets, private media, authentication cookies, or personal database contents in a public issue.

When reporting, include the affected line (**Next** or **Classic**), version/release channel, operating system, installation format, and the smallest safe reproduction steps you can provide.
