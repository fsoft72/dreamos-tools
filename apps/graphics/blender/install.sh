#!/usr/bin/env bash
# Download the latest stable Blender (Linux x64 tarball) and install it in
# the home of the user who started dreamos-install.
set -euo pipefail

readonly BASE_URL="https://download.blender.org/release"
readonly TARGET_USER="${DREAMOS_USER:-$(id -un)}"
USER_HOME="$(getent passwd "$TARGET_USER" | cut -d: -f6)"
readonly USER_HOME
readonly OPT_DIR="${USER_HOME}/.local/opt/blender"
readonly BIN_DIR="${USER_HOME}/.local/bin"
readonly APPS_DIR="${USER_HOME}/.local/share/applications"
readonly ICONS_DIR="${USER_HOME}/.local/share/icons"

# Fail with a message and exit status 1.
die() {
	echo "ERROR: $*" >&2
	exit 1
}

[ -n "$USER_HOME" ] || die "cannot find the home of user '$TARGET_USER'"

# Root phase: install tools, then drop privileges so every file is created
# directly by the target user.
if [ "$(id -u)" -eq 0 ] && [ "$TARGET_USER" != root ]; then
	for tool in curl xz; do
		command -v "$tool" >/dev/null || apt-get install -y "$([ "$tool" = xz ] && echo xz-utils || echo "$tool")"
	done
	APP_DIR="${DREAMOS_APP_DIR:-$(dirname "$(readlink -f "$0")")}"
	exec runuser -u "$TARGET_USER" -- env HOME="$USER_HOME" DREAMOS_APP_DIR="$APP_DIR" DREAMOS_USER="$TARGET_USER" bash "$(readlink -f "$0")"
fi

command -v curl >/dev/null || die "curl is required"
command -v xz >/dev/null || die "xz is required"
readonly APP_DIR="${DREAMOS_APP_DIR:-$(dirname "$(readlink -f "$0")")}"

TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

echo "Looking for the latest Blender release..."
curl -fsSL "$BASE_URL/" -o "$TMP_DIR/index.html" || die "cannot list $BASE_URL/"
SERIES="$(grep -o 'href="Blender[0-9.]*/"' "$TMP_DIR/index.html" | sed 's/href="//; s/\/"$//' | sort -V | tail -n1 || true)"
[ -n "$SERIES" ] || die "no Blender release folder found"

curl -fsSL "$BASE_URL/$SERIES/" -o "$TMP_DIR/series.html" || die "cannot list $BASE_URL/$SERIES/"
FILE="$(grep -o 'href="blender-[0-9.]*-linux-x64\.tar\.xz"' "$TMP_DIR/series.html" | sed 's/href="//; s/"$//' | sort -V | tail -n1 || true)"
[ -n "$FILE" ] || die "no linux x64 build found in $SERIES"
VERSION="${FILE#blender-}"
VERSION="${VERSION%-linux-x64.tar.xz}"

echo "Downloading $FILE ..."
curl -fL --progress-bar "$BASE_URL/$SERIES/$FILE" -o "$TMP_DIR/$FILE" || die "download failed: $BASE_URL/$SERIES/$FILE"

echo "Verifying SHA256 checksum..."
curl -fsSL "$BASE_URL/$SERIES/blender-$VERSION.sha256" -o "$TMP_DIR/sums.txt" || die "cannot download the checksum file"
EXPECTED="$(awk -v f="$FILE" '$2 == f {print $1}' "$TMP_DIR/sums.txt" | head -n1)"
ACTUAL="$(sha256sum "$TMP_DIR/$FILE" | awk '{print $1}')"
[ -n "$EXPECTED" ] || die "$FILE not listed in the checksum file"
[ "$EXPECTED" = "$ACTUAL" ] || die "checksum mismatch (expected $EXPECTED, got $ACTUAL)"

echo "Installing into $OPT_DIR ..."
mkdir -p "$TMP_DIR/unpacked" "$BIN_DIR" "$APPS_DIR" "$ICONS_DIR"
tar -xJf "$TMP_DIR/$FILE" -C "$TMP_DIR/unpacked"
SRC_DIR="$(find "$TMP_DIR/unpacked" -mindepth 1 -maxdepth 1 -type d | head -n1)"
[ -x "$SRC_DIR/blender" ] || die "blender binary not found in $FILE"

rm -rf "$OPT_DIR"
mkdir -p "$(dirname "$OPT_DIR")"
mv "$SRC_DIR" "$OPT_DIR"
install -m 644 "$APP_DIR/icon.png" "$ICONS_DIR/blender.png"
ln -sf "$OPT_DIR/blender" "$BIN_DIR/blender"

cat > "$APPS_DIR/blender.desktop" <<DESKTOP
[Desktop Entry]
Type=Application
Name=Blender
Comment=3D creation suite
Exec=${OPT_DIR}/blender %f
Icon=${ICONS_DIR}/blender.png
Categories=Graphics;3DGraphics;
MimeType=application/x-blender;
Terminal=false
DESKTOP

echo "Blender $VERSION installed for user $TARGET_USER."
