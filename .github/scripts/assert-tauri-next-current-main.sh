#!/usr/bin/env bash
set -euo pipefail

: "${SOURCE_SHA:?SOURCE_SHA must be set to the commit being published}"

REMOTE="${RELEASE_REMOTE:-origin}"
BRANCH="${RELEASE_MAIN_BRANCH:-main}"
REMOTE_REF="refs/remotes/${REMOTE}/${BRANCH}"

git fetch --no-tags "$REMOTE" "+refs/heads/${BRANCH}:${REMOTE_REF}"
CURRENT_MAIN_SHA="$(git rev-parse "$REMOTE_REF")"

if [[ "$CURRENT_MAIN_SHA" != "$SOURCE_SHA" ]]; then
  echo "::error title=Stale Scriptotar Next release source::Refusing to publish ${SOURCE_SHA}; current ${BRANCH} is ${CURRENT_MAIN_SHA}."
  exit 42
fi

if [[ -n "${GITHUB_OUTPUT:-}" ]]; then
  printf 'current_main_sha=%s\n' "$CURRENT_MAIN_SHA" >> "$GITHUB_OUTPUT"
fi

printf 'Verified release source %s is current %s.\n' "$SOURCE_SHA" "$BRANCH"
