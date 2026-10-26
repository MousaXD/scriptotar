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
# WebKit download sizes can exceed GitHub's default Ubuntu mirror throughput;
# keep a strict bound, but allow enough time for a real first-time download.
if ! sudo env DEBIAN_FRONTEND=noninteractive timeout 480 \
    apt-get install -y --no-install-recommends \
    -o Acquire::Retries=2 -o Acquire::http::Timeout=30 \
    -o Acquire::https::Timeout=30 "${packages[@]}"; then
  sudo timeout 180 apt-get update \
    -o Acquire::Retries=1 \
    -o Acquire::http::Timeout=20 \
    -o Acquire::https::Timeout=20
  sudo env DEBIAN_FRONTEND=noninteractive timeout 480 \
    apt-get install -y --no-install-recommends \
    -o Acquire::Retries=2 -o Acquire::http::Timeout=30 \
    -o Acquire::https::Timeout=30 "${packages[@]}"
fi
pkg-config --exists gtk+-3.0 gdk-3.0 webkit2gtk-4.1
printf 'PASS: native Tauri GTK/WebKit libraries installed\n'
