#!/bin/sh
# Build the dreamos-install .deb: the binary and the app catalog in one package.
#
# Usage: scripts/build-deb.sh [--version V] [--skip-build]
#   --version V    package version (default: 0.1.<YYYYMMDD>.<HHMM> of now)
#   --skip-build   reuse dreamos-install/target/release/dreamos-install
#
# Output: dist/deb/dreamos-install_<version>_<arch>.deb
#
# Layout:
#   /usr/bin/dreamos-install
#   /usr/share/dreamos-install/apps/            the catalog (apps/ of the repo)
#   /usr/share/applications/dreamos-install.desktop
#   /usr/share/icons/hicolor/256x256/apps/dreamos-installer.png
# Shared library dependencies are computed with dpkg-shlibdeps, so they match
# the Ubuntu release the package is built on.
set -eu
SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
REPO_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)
CRATE_DIR="$REPO_ROOT/dreamos-install"
BIN="$CRATE_DIR/target/release/dreamos-install"
VERSION=""
SKIP_BUILD=0

while [ $# -gt 0 ]; do
    case "$1" in
        --version) [ $# -ge 2 ] || { echo "build-deb: --version needs a value" >&2; exit 2; }; VERSION=$2; shift ;;
        --skip-build) SKIP_BUILD=1 ;;
        -h|--help) sed -n '2,8p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
        *) echo "build-deb: unknown option $1" >&2; exit 2 ;;
    esac
    shift
done

for tool in dpkg-deb dpkg-shlibdeps dpkg strip; do
    command -v "$tool" >/dev/null 2>&1 || { echo "build-deb: $tool not found (package: dpkg-dev, binutils)" >&2; exit 1; }
done
[ "$SKIP_BUILD" -eq 1 ] || command -v cargo >/dev/null 2>&1 || { echo "build-deb: cargo not found" >&2; exit 1; }

[ -n "$VERSION" ] || VERSION="0.1.$(date +%Y%m%d.%H%M)"
# Debian versions start with a digit and allow only these characters
case "$VERSION" in
    [0-9]*) ;;
    *) echo "build-deb: invalid version $VERSION" >&2; exit 2 ;;
esac
printf '%s' "$VERSION" | grep -Eq '^[0-9A-Za-z.+~-]+$' || { echo "build-deb: invalid version $VERSION" >&2; exit 2; }

cd "$REPO_ROOT"

[ "$SKIP_BUILD" -eq 1 ] || (cd "$CRATE_DIR" && cargo build --release)
[ -x "$BIN" ] || { echo "build-deb: $BIN not found, build it first" >&2; exit 1; }
[ -d "$REPO_ROOT/apps" ] || { echo "build-deb: $REPO_ROOT/apps not found" >&2; exit 1; }

ARCH=$(dpkg --print-architecture)
NAME="dreamos-install_${VERSION}_${ARCH}"
OUT_DIR="$REPO_ROOT/dist/deb"
ROOT="$OUT_DIR/$NAME"

echo "Staging $NAME"
rm -rf "$ROOT"
mkdir -p "$ROOT/DEBIAN" "$ROOT/usr/bin" "$ROOT/usr/share/applications" \
    "$ROOT/usr/share/dreamos-install/apps" "$ROOT/usr/share/icons/hicolor/256x256/apps"

# The binary is stripped in the staging copy only: target/ keeps its symbols
install -m 755 "$BIN" "$ROOT/usr/bin/dreamos-install"
strip --strip-unneeded "$ROOT/usr/bin/dreamos-install"
cp -a "$REPO_ROOT/apps/." "$ROOT/usr/share/dreamos-install/apps/"
install -m 644 "$REPO_ROOT/packaging/dreamos-install.desktop" "$ROOT/usr/share/applications/dreamos-install.desktop"
install -m 644 "$REPO_ROOT/packaging/dreamos-installer.png" "$ROOT/usr/share/icons/hicolor/256x256/apps/dreamos-installer.png"

# Shared library dependencies. dpkg-shlibdeps wants a debian/control in the
# current folder, so run it in a scratch one.
SHLIBS_DIR=$(mktemp -d)
trap 'rm -rf "$SHLIBS_DIR"' EXIT
mkdir -p "$SHLIBS_DIR/debian"
printf 'Source: dreamos-install\n\nPackage: dreamos-install\nArchitecture: any\n' > "$SHLIBS_DIR/debian/control"
SHLIBS=$(cd "$SHLIBS_DIR" && dpkg-shlibdeps -O -e"$ROOT/usr/bin/dreamos-install" 2>/dev/null | sed -n 's/^shlibs:Depends=//p')
[ -n "$SHLIBS" ] || { echo "build-deb: dpkg-shlibdeps found no dependencies, refusing to build" >&2; exit 1; }

# Do not inherit the umask of the build machine: 755 folders, 644 data files
# (executables keep their 755 and the maintainer scripts are set below)
chmod -R u=rwX,go=rX "$ROOT"

INSTALLED_SIZE=$(du -sk "$ROOT" | cut -f1)
cat > "$ROOT/DEBIAN/control" <<CONTROL
Package: dreamos-install
Version: $VERSION
Section: admin
Priority: optional
Architecture: $ARCH
Maintainer: Fabio Rotondo <fabio.rotondo@gmail.com>
Installed-Size: $INSTALLED_SIZE
Depends: $SHLIBS, polkitd, pkexec
Homepage: https://github.com/fsoft72/dreamos-tools
Description: DreamOS application installer and catalog
 DreamOS Installer lists the applications of the DreamOS catalog and runs
 their install, check and uninstall scripts with administrator rights.
 The catalog is shipped with the program.
CONTROL

cat > "$ROOT/DEBIAN/postinst" <<'POSTINST'
#!/bin/sh
set -e
gtk-update-icon-cache -f -t /usr/share/icons/hicolor 2>/dev/null || true
update-desktop-database -q /usr/share/applications 2>/dev/null || true
exit 0
POSTINST
cat > "$ROOT/DEBIAN/postrm" <<'POSTRM'
#!/bin/sh
set -e
case "$1" in
    remove|purge)
        gtk-update-icon-cache -f -t /usr/share/icons/hicolor 2>/dev/null || true
        update-desktop-database -q /usr/share/applications 2>/dev/null || true
        ;;
esac
exit 0
POSTRM
chmod 755 "$ROOT/DEBIAN/postinst" "$ROOT/DEBIAN/postrm"

dpkg-deb --root-owner-group --build "$ROOT" "$OUT_DIR/$NAME.deb"
echo "Done: $OUT_DIR/$NAME.deb"
