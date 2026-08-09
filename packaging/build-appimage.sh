#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
VERSION="$(sed -n 's/^VERSION = "\([^" ]*\)"/\1/p' "$ROOT/scriptotar_common.py" | head -n1)"
OUT="${1:-$ROOT/Scriptotar-${VERSION}-x86_64.AppImage}"
WORK="$(mktemp -d "${TMPDIR:-/tmp}/scriptotar-appimage.XXXXXX")"
trap 'rm -rf "$WORK"' EXIT
APPDIR="$WORK/Scriptotar.AppDir"
TOOLS="$WORK/tools"
mkdir -p "$APPDIR/usr/bin" "$APPDIR/usr/lib/scriptotar" "$APPDIR/usr/share/applications" \
  "$APPDIR/usr/share/icons/hicolor/scalable/apps" "$TOOLS"

PYTHON_BIN="$(command -v python3)"
PYVER="$(python3 -c 'import sys; print(f"{sys.version_info.major}.{sys.version_info.minor}")')"
PY_STDLIB="$(python3 -c 'import sysconfig; print(sysconfig.get_path("stdlib"))')"

install -m 0755 "$ROOT/scriptotar.py" "$APPDIR/usr/lib/scriptotar/scriptotar.py"
install -m 0755 "$ROOT/worker.py" "$APPDIR/usr/lib/scriptotar/worker.py"
for file in core.py creator.py scriptotar_common.py ui_mixin.py persistence_mixin.py jobs_mixin.py research_mixin.py ai_mixin.py library_mixin.py requirements-engine.txt; do
  install -m 0644 "$ROOT/$file" "$APPDIR/usr/lib/scriptotar/$file"
done
install -m 0644 "$ROOT/scriptotar.desktop" "$APPDIR/usr/share/applications/scriptotar.desktop"
install -m 0644 "$ROOT/scriptotar.svg" "$APPDIR/usr/share/icons/hicolor/scalable/apps/scriptotar.svg"

cp -L "$PYTHON_BIN" "$APPDIR/usr/bin/python3"
cp -a "$PY_STDLIB" "$APPDIR/usr/lib/"
for bin in ffmpeg ffprobe secret-tool; do
  if command -v "$bin" >/dev/null 2>&1; then
    cp -L "$(command -v "$bin")" "$APPDIR/usr/bin/$bin"
  fi
done
for path in /usr/share/tcltk /usr/lib/tcltk /usr/share/python-wheels; do
  if [[ -e "$path" ]]; then
    mkdir -p "$APPDIR$(dirname "$path")"
    cp -a "$path" "$APPDIR$(dirname "$path")/"
  fi
done

# Copy non-glibc ELF dependencies for Python, its extension modules, FFmpeg and secret-tool.
declare -A SEEN=()
copy_deps() {
  local file="$1" dep base dest
  [[ -e "$file" ]] || return 0
  while IFS= read -r dep; do
    [[ -n "$dep" && -e "$dep" ]] || continue
    base="$(basename "$dep")"
    case "$base" in
      libc.so.*|libpthread.so.*|libdl.so.*|librt.so.*|libm.so.*|libresolv.so.*|ld-linux*|linux-vdso.so.*) continue ;;
    esac
    [[ -z "${SEEN[$dep]:-}" ]] || continue
    SEEN[$dep]=1
    dest="$APPDIR$dep"
    mkdir -p "$(dirname "$dest")"
    cp -L "$dep" "$dest"
    copy_deps "$dep"
  done < <(ldd "$file" 2>/dev/null | awk '/=> \/[^ ]+/ {print $3} /^\// {print $1}')
}

copy_deps "$PYTHON_BIN"
for bin in ffmpeg ffprobe secret-tool; do
  command -v "$bin" >/dev/null 2>&1 && copy_deps "$(command -v "$bin")"
done
while IFS= read -r module; do copy_deps "$module"; done < <(find "$PY_STDLIB" -type f -name '*.so' -print)

cat > "$APPDIR/AppRun" <<EOF2
#!/bin/sh
set -eu
APPDIR="\${APPDIR:?AppImage runtime did not set APPDIR}"
PYVER="$PYVER"
RUNTIME_VERSION="scriptotar-$VERSION-python-$PYVER"
DATA_BASE="\${XDG_DATA_HOME:-\$HOME/.local/share}/scriptotar"
RUNTIME_ROOT="\$DATA_BASE/appimage-runtime-\$PYVER"
MARKER="\$RUNTIME_ROOT/.scriptotar-runtime-version"

if [ ! -x "\$RUNTIME_ROOT/usr/bin/python3" ] || [ ! -f "\$MARKER" ] || [ "\$(cat "\$MARKER" 2>/dev/null || true)" != "\$RUNTIME_VERSION" ]; then
  TMP="\$RUNTIME_ROOT.tmp.\$\$"
  rm -rf "\$TMP"
  mkdir -p "\$TMP/usr/bin" "\$TMP/usr/lib" "\$TMP/usr/share"
  cp -a "\$APPDIR/usr/bin/python3" "\$TMP/usr/bin/python3"
  cp -a "\$APPDIR/usr/lib/python$PYVER" "\$TMP/usr/lib/"
  [ ! -d "\$APPDIR/usr/lib/x86_64-linux-gnu" ] || cp -a "\$APPDIR/usr/lib/x86_64-linux-gnu" "\$TMP/usr/lib/"
  [ ! -d "\$APPDIR/usr/share/python-wheels" ] || cp -a "\$APPDIR/usr/share/python-wheels" "\$TMP/usr/share/"
  printf '%s\n' "\$RUNTIME_VERSION" > "\$TMP/.scriptotar-runtime-version"
  rm -rf "\$RUNTIME_ROOT"
  mv "\$TMP" "\$RUNTIME_ROOT"
fi

export SCRIPTOTAR_INSTALL="\$APPDIR/usr/lib/scriptotar"
export SCRIPTOTAR_ENGINE_PYTHON="\$RUNTIME_ROOT/usr/bin/python3"
export PATH="\$APPDIR/usr/bin:\$PATH"
export PYTHONNOUSERSITE=1
export PYTHONDONTWRITEBYTECODE=1
export LD_LIBRARY_PATH="\$APPDIR/usr/lib/x86_64-linux-gnu:\$APPDIR/usr/lib:\${LD_LIBRARY_PATH:-}"
[ ! -d "\$APPDIR/usr/share/tcltk/tcl8.6" ] || export TCL_LIBRARY="\$APPDIR/usr/share/tcltk/tcl8.6"
[ ! -d "\$APPDIR/usr/share/tcltk/tk8.6" ] || export TK_LIBRARY="\$APPDIR/usr/share/tcltk/tk8.6"
exec "\$APPDIR/usr/bin/python3" "\$SCRIPTOTAR_INSTALL/scriptotar.py" "\$@"
EOF2
chmod 0755 "$APPDIR/AppRun"

ln -s usr/share/applications/scriptotar.desktop "$APPDIR/scriptotar.desktop"
ln -s usr/share/icons/hicolor/scalable/apps/scriptotar.svg "$APPDIR/scriptotar.svg"
ln -s scriptotar.svg "$APPDIR/.DirIcon"

# Validate the bundled Python/Tk runtime before creating the image.
PYTHONHOME="$APPDIR/usr" LD_LIBRARY_PATH="$APPDIR/usr/lib/x86_64-linux-gnu:$APPDIR/usr/lib" \
  "$APPDIR/usr/bin/python3" -c 'import sqlite3, ssl, tkinter; print("AppImage Python/Tk runtime OK")'

APPIMAGETOOL="$TOOLS/appimagetool-x86_64.AppImage"
curl -fsSL -o "$APPIMAGETOOL" \
  https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-x86_64.AppImage
chmod +x "$APPIMAGETOOL"
mkdir -p "$(dirname "$OUT")"
ARCH=x86_64 APPIMAGE_EXTRACT_AND_RUN=1 "$APPIMAGETOOL" "$APPDIR" "$OUT"
chmod +x "$OUT"
echo "Built: $OUT"
