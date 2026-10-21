#!/bin/sh
# Check a dreamos-install .deb.
#
# Usage: scripts/tests/test-deb.sh <file.deb> <apps dir of the repo>
#
# Exits 0 only when every check passes.
set -eu
DEB=${1:-}
APPS=${2:-}
FAILS=0

# _check <description> <command...>: runs the command, counts a failure
_check() {
    desc=$1
    shift
    if "$@"; then echo "PASS: $desc"; else echo "FAIL: $desc"; FAILS=$((FAILS + 1)); fi
}

[ -f "$DEB" ] || { echo "FAIL: $DEB not found" >&2; exit 1; }
[ -d "$APPS" ] || { echo "FAIL: apps dir $APPS not found" >&2; exit 1; }

LIST=$(dpkg-deb -c "$DEB")
DEPENDS=$(dpkg-deb -f "$DEB" Depends)

# _has_path <path>: the package lists this path
_has_path() { echo "$LIST" | grep -q " \./$1\$"; }
# _depends_on <name>: the Depends field mentions this package
_depends_on() { echo "$DEPENDS" | grep -q "$1"; }

_check "package name" test "$(dpkg-deb -f "$DEB" Package)" = dreamos-install
_check "architecture" test "$(dpkg-deb -f "$DEB" Architecture)" = amd64
_check "depends on gtk4" _depends_on libgtk-4-1
_check "depends on vte" _depends_on libvte-2.91-gtk4-0
_check "depends on pkexec" _depends_on pkexec
_check "binary" _has_path usr/bin/dreamos-install
_check "desktop entry" _has_path usr/share/applications/dreamos-install.desktop
_check "icon" _has_path usr/share/icons/hicolor/256x256/apps/dreamos-installer.png

EXPECTED=$(find "$APPS" -name manifest.json | wc -l)
GOT=$(echo "$LIST" | grep -c '/usr/share/dreamos-install/apps/.*manifest.json$' || true)
_check "catalog manifests ($GOT of $EXPECTED)" test "$GOT" -eq "$EXPECTED"

EXPECTED=$(find "$APPS" -name install.sh | wc -l)
GOT=$(echo "$LIST" | grep -c '/usr/share/dreamos-install/apps/.*/install.sh$' || true)
_check "catalog install.sh ($GOT of $EXPECTED)" test "$GOT" -eq "$EXPECTED"

# _catalog_readable: every catalog entry is readable by group and others
_catalog_readable() {
    ! echo "$LIST" | grep ' \./usr/share/dreamos-install/apps/' | grep -qEv '^.r..r..r'
}
_check "catalog files readable by everyone" _catalog_readable

[ "$FAILS" -eq 0 ] || { echo "$FAILS check(s) failed" >&2; exit 1; }
echo "All checks passed"
