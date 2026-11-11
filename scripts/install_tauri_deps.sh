#!/usr/bin/env bash
# Install required native Tauri libraries, never bypassing compilation or tests.
# GitHub's Azure Ubuntu mirror occasionally fails DNS; fall back only to the
# official Ubuntu repositories, retaining the image's package-signing key.
set -euo pipefail

packages=(
  libwebkit2gtk-4.1-dev
  libappindicator3-dev
  librsvg2-dev
  libgtk-3-dev
  libssl-dev
  libdbus-1-dev
  pkg-config
  patchelf
)

mirror_switched=false
configure_official_ubuntu_mirrors() {
  local codename source_file keyring tmp
  codename="$(. /etc/os-release; printf '%s' "${VERSION_CODENAME:-}")"
  source_file=/etc/apt/sources.list.d/ubuntu.sources
  keyring=/usr/share/keyrings/ubuntu-archive-keyring.gpg

  # Only rewrite the default ephemeral Ubuntu runner sources; never add
  # unsigned or third-party mirrors and never alter an unknown distribution.
  case "$codename" in jammy|noble) ;; *) return 1 ;; esac
  if [[ ! -f "$source_file" || ! -f "$keyring" ]]; then return 1; fi
  if ! timeout 8 getent ahostsv4 archive.ubuntu.com >/dev/null 2>&1 ||
     ! timeout 8 getent ahostsv4 security.ubuntu.com >/dev/null 2>&1; then
    printf 'Official Ubuntu mirror DNS is unavailable; retaining signed runner sources.\n' >&2
    return 1
  fi

  tmp="$(mktemp)"
  cat > "$tmp" <<SOURCES
Types: deb
URIs: https://archive.ubuntu.com/ubuntu
Suites: $codename $codename-updates $codename-backports
Components: main restricted universe multiverse
Signed-By: $keyring

Types: deb
URIs: https://security.ubuntu.com/ubuntu
Suites: $codename-security
Components: main restricted universe multiverse
Signed-By: $keyring
SOURCES
  sudo install -m 0644 "$tmp" "$source_file"
  rm -f "$tmp"
  mirror_switched=true
  printf 'Using signed official Ubuntu repositories for %s instead of the runner mirror list.\n' "$codename"
}

apt_update() {
  sudo env DEBIAN_FRONTEND=noninteractive timeout 180 apt-get update \
    -o Acquire::Retries=1 \
    -o Acquire::http::Timeout=20 \
    -o Acquire::https::Timeout=20
}

apt_install() {
  sudo env DEBIAN_FRONTEND=noninteractive timeout 480 \
    apt-get install -y --no-install-recommends \
    -o Acquire::Retries=2 \
    -o Acquire::http::Timeout=30 \
    -o Acquire::https::Timeout=30 "${packages[@]}"
}

# Avoid a lengthy failed download against Azure when its DNS is already down.
if ! timeout 8 getent ahostsv4 azure.archive.ubuntu.com >/dev/null 2>&1; then
  if configure_official_ubuntu_mirrors; then apt_update; fi
fi

if ! apt_install; then
  if [[ "$mirror_switched" == false ]]; then
    configure_official_ubuntu_mirrors || true
  fi
  apt_update
  apt_install
fi

# This is the actual native-build prerequisite; missing libs always fail.
pkg-config --exists gtk+-3.0 gdk-3.0 webkit2gtk-4.1
printf 'PASS: native Tauri GTK/WebKit libraries installed\n'
