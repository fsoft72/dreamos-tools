#!/usr/bin/env bash
# Download the latest PhotoCraft x86_64 AppImage from GitHub and install it system wide.
set -euo pipefail

readonly REPO="storytold/photocraft"
readonly API_URL="https://api.github.com/repos/${REPO}/releases/latest"
readonly INSTALL_DIR="/opt/photocraft"
readonly APPIMAGE="${INSTALL_DIR}/photocraft.AppImage"
readonly APP_DIR="${DREAMOS_APP_DIR:-$(dirname "$(readlink -f "$0")")}"

# Fail with a message and exit status 1.
die() {
	echo "ERROR: $*" >&2
	exit 1
}

command -v curl >/dev/null || die "curl is required"

TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

echo "Looking for the latest release of ${REPO}..."
curl -fsSL "$API_URL" -o "$TMP_DIR/release.json" || die "cannot query $API_URL"

URL="$(grep -o '"browser_download_url": *"[^"]*linux-x86_64\.AppImage"' "$TMP_DIR/release.json" | head -n1 | sed 's/.*: *"//; s/"$//')"
[ -n "$URL" ] || die "no linux x86_64 AppImage found in the latest release"
SUMS_URL="$(grep -o '"browser_download_url": *"[^"]*SHA256SUMS\.txt"' "$TMP_DIR/release.json" | head -n1 | sed 's/.*: *"//; s/"$//')"
FILE="$(basename "$URL")"

echo "Downloading $FILE ..."
curl -fL --progress-bar "$URL" -o "$TMP_DIR/$FILE" || die "download failed: $URL"

if [ -n "$SUMS_URL" ]; then
	echo "Verifying SHA256 checksum..."
	curl -fsSL "$SUMS_URL" -o "$TMP_DIR/SHA256SUMS.txt" || die "cannot download $SUMS_URL"
	EXPECTED="$(awk -v f="$FILE" '$2 == f {print $1}' "$TMP_DIR/SHA256SUMS.txt" | head -n1)"
	ACTUAL="$(sha256sum "$TMP_DIR/$FILE" | awk '{print $1}')"
	[ -n "$EXPECTED" ] || die "$FILE not listed in SHA256SUMS.txt"
	[ "$EXPECTED" = "$ACTUAL" ] || die "checksum mismatch (expected $EXPECTED, got $ACTUAL)"
fi

# AppImages need FUSE 2 to mount themselves.
if ! ldconfig -p | grep -q 'libfuse\.so\.2'; then
	echo "Installing FUSE 2 runtime..."
	apt-get install -y libfuse2t64 || apt-get install -y libfuse2 || die "cannot install libfuse2"
fi

echo "Installing into $INSTALL_DIR ..."
install -d "$INSTALL_DIR"
install -m 755 "$TMP_DIR/$FILE" "$APPIMAGE"
install -m 644 "$APP_DIR/icon.png" "$INSTALL_DIR/photocraft.png"
ln -sf "$APPIMAGE" /usr/local/bin/photocraft

cat > /usr/share/applications/photocraft.desktop <<DESKTOP
[Desktop Entry]
Type=Application
Name=PhotoCraft
Comment=Image editor written in Rust
Exec=${APPIMAGE} %F
Icon=${INSTALL_DIR}/photocraft.png
Categories=Graphics;RasterGraphics;
Terminal=false
DESKTOP

echo "PhotoCraft ${FILE} installed."
