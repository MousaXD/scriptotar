#!/usr/bin/env bash
set -euo pipefail

VERSION="1.2.0"
ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
BUILD="$(mktemp -d "${TMPDIR:-/tmp}/scriptotar-build.XXXXXX")"
trap 'rm -rf "$BUILD"' EXIT
PKG="$BUILD/pkg"
OUT="${1:-$ROOT/scriptotar_${VERSION}_all.deb}"

mkdir -p \
  "$PKG/DEBIAN" \
  "$PKG/opt/scriptotar" \
  "$PKG/usr/bin" \
  "$PKG/usr/share/applications" \
  "$PKG/usr/share/icons/hicolor/scalable/apps" \
  "$PKG/usr/share/doc/scriptotar"

chmod 0755 "$PKG/DEBIAN"

install -m 0755 "$ROOT/scriptotar.py" "$PKG/opt/scriptotar/scriptotar.py"
install -m 0755 "$ROOT/worker.py" "$PKG/opt/scriptotar/worker.py"
install -m 0644 "$ROOT/core.py" "$PKG/opt/scriptotar/core.py"
install -m 0644 "$ROOT/creator.py" "$PKG/opt/scriptotar/creator.py"
for module in scriptotar_common.py ui_mixin.py persistence_mixin.py jobs_mixin.py research_mixin.py ai_mixin.py library_mixin.py; do
  install -m 0644 "$ROOT/$module" "$PKG/opt/scriptotar/$module"
done
install -m 0644 "$ROOT/requirements-engine.txt" "$PKG/opt/scriptotar/requirements-engine.txt"
install -m 0644 "$ROOT/scriptotar.desktop" "$PKG/usr/share/applications/scriptotar.desktop"
install -m 0644 "$ROOT/scriptotar.svg" "$PKG/usr/share/icons/hicolor/scalable/apps/scriptotar.svg"
install -m 0644 "$ROOT/README.md" "$PKG/usr/share/doc/scriptotar/README.md"
install -m 0644 "$ROOT/LICENSE" "$PKG/usr/share/doc/scriptotar/LICENSE"
install -m 0644 "$ROOT/NOTICE" "$PKG/usr/share/doc/scriptotar/NOTICE"
install -m 0644 "$ROOT/THIRD_PARTY_NOTICES.md" "$PKG/usr/share/doc/scriptotar/THIRD_PARTY_NOTICES.md"

cat > "$PKG/usr/bin/scriptotar" <<'EOF'
#!/bin/sh
exec python3 /opt/scriptotar/scriptotar.py "$@"
EOF
chmod 0755 "$PKG/usr/bin/scriptotar"

cat > "$PKG/DEBIAN/control" <<'EOF'
Package: scriptotar
Version: 1.2.0
Section: utils
Priority: optional
Architecture: all
Conflicts: wesamboss
Replaces: wesamboss
Depends: python3 (>= 3.10), python3-venv, python3-tk, ffmpeg, libsecret-tools
Maintainer: Scriptotar Local Build <local@scriptotar.invalid>
Description: Local-first short-form video research and transcription desktop app
 Scriptotar downloads and transcribes supported videos, researches public creator
 feeds, stores a local content library, and builds original AI-ready prompts.
 Optional bring-your-own-key AI integrations never require a Scriptotar account.
EOF

cat > "$PKG/DEBIAN/postinst" <<'EOF'
#!/bin/sh
set -e
if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database -q /usr/share/applications || true
fi
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
  gtk-update-icon-cache -q -t -f /usr/share/icons/hicolor || true
fi
exit 0
EOF
chmod 0755 "$PKG/DEBIAN/postinst"

cat > "$PKG/DEBIAN/postrm" <<'EOF'
#!/bin/sh
set -e
if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database -q /usr/share/applications || true
fi
exit 0
EOF
chmod 0755 "$PKG/DEBIAN/postrm"

python3 -m py_compile "$ROOT/scriptotar.py" "$ROOT/worker.py" "$ROOT/core.py" "$ROOT/creator.py" "$ROOT"/*_mixin.py "$ROOT/scriptotar_common.py"
PYTHONPATH="$ROOT" python3 -m unittest discover -s "$ROOT/tests" -v

dpkg-deb --build --root-owner-group "$PKG" "$OUT"
dpkg-deb --info "$OUT"
echo "Built: $OUT"
