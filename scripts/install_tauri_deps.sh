#!/usr/bin/env bash
# Install the native Tauri libraries required by the existing workspace tests.
# Prefer the hosted runner's apt indices and bound network fallback time.
# Do not skip compilation, Clippy or any test when dependency installation fails.
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
if ! sudo env DEBIAN_FRONTEND=noninteractive timeout 180 \
    apt-get install -y --no-install-recommends "${packages[@]}"; then
  sudo timeout 180 apt-get update \
    -o Acquire::Retries=1 \
    -o Acquire::http::Timeout=20 \
    -o Acquire::https::Timeout=20
  sudo env DEBIAN_FRONTEND=noninteractive timeout 180 \
    apt-get install -y --no-install-recommends "${packages[@]}"
fi
pkg-config --exists gtk+-3.0 gdk-3.0 webkit2gtk-4.1
printf 'PASS: native Tauri GTK/WebKit libraries installed\n'
