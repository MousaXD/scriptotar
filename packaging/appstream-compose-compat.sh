#!/bin/sh
# Compatibility shim for flatpak-builder invoking appstream-compose on Freedesktop SDK 24.08+
if command -v appstreamcli >/dev/null 2>&1; then
  exec appstreamcli compose "$@" || exit 0
fi
exit 0
