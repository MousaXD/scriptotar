# Security policy

## Secrets

Never include API keys, cookies, browser profiles, access tokens, downloaded private media, or database files in bug reports or commits.

Scriptotar's AI key field is session-only unless the user explicitly chooses **Remember in keyring**, which uses Linux Secret Service via `secret-tool`.

## Reporting vulnerabilities

Prefer GitHub's private vulnerability reporting / Security Advisory flow for vulnerabilities that could expose secrets, execute commands, bypass URL handling, or corrupt local data. Avoid posting proof-of-concept secrets or private media in a public issue.

## Supported release

Security fixes target the latest release and `main`.
