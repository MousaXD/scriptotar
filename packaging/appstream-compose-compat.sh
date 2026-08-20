#!/bin/sh
# Compatibility shim for flatpak-builder invoking appstream-compose on Freedesktop SDK 24.08+
if command -v appstreamcli >/dev/null 2>&1; then
  ARGS=""
  for arg in "$@"; do
    case "$arg" in
      --basename=*) ;;
      *) ARGS="$ARGS \"$arg\"" ;;
    esac
  done
  eval "appstreamcli compose $ARGS" || true
fi
exit 0

