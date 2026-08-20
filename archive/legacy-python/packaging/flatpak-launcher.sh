#!/bin/sh
set -eu
export SCRIPTOTAR_INSTALL=/app/lib/scriptotar
exec python3 /app/lib/scriptotar/scriptotar.py "$@"
