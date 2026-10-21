#!/usr/bin/env bash
# Install the latest Xodus Nexus AppImage from GitHub (SHA512 verified) into
# /opt/xodus-nexus, with a command in /usr/local/bin and an application menu entry.
set -euo pipefail

readonly REPO="domsvoboda/xodus-nexus"
readonly API_URL="https://api.github.com/repos/${REPO}/releases/latest"
readonly APP_DIR="/opt/xodus-nexus"
readonly APPIMAGE="${APP_DIR}/Xodus-Nexus.AppImage"
readonly ICON_FILE="${APP_DIR}/icon.png"
readonly LAUNCHER="/usr/local/bin/xodus-nexus"
readonly DESKTOP_FILE="/usr/share/applications/xodus-nexus.desktop"
SELF_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly SELF_DIR

# Fail with a message and exit status 1.
die() {
	echo "ERROR: $*" >&2
	exit 1
}

# Print the first package of the arguments that apt knows (t64 renames differ per release).
first_available() {
	local pkg
	for pkg in "$@"; do
		if apt-cache show "$pkg" >/dev/null 2>&1; then
			echo "$pkg"
			return
		fi
	done
	die "none of these packages is available: $*"
}

command -v apt-get >/dev/null || die "apt-get is required (Debian or Ubuntu)"
[ "$(uname -m)" = "x86_64" ] || die "only x86_64 builds are published"

TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

echo "Installing the runtime libraries..."
apt-get update -y || echo "WARNING: apt-get update failed, continuing" >&2
apt-get install -y curl desktop-file-utils libwebkit2gtk-4.1-0 libnss3 libxtst6 \
	"$(first_available libfuse2t64 libfuse2)" \
	"$(first_available libgtk-3-0t64 libgtk-3-0)" \
	"$(first_available libasound2t64 libasound2)"

echo "Looking for the latest release of ${REPO}..."
curl -fsSL "$API_URL" -o "$TMP_DIR/release.json" || die "cannot query $API_URL"

URL="$(grep -o '"browser_download_url": *"[^"]*x86_64\.AppImage"' "$TMP_DIR/release.json" | head -n1 | sed 's/.*: *"//; s/"$//')"
[ -n "$URL" ] || die "no x86_64 AppImage found in the latest release"
FILE="$(basename "$URL")"

echo "Downloading $FILE ..."
curl -fL --progress-bar "$URL" -o "$TMP_DIR/$FILE" || die "download failed: $URL"
curl -fsSL "${URL}.sha512sum" -o "$TMP_DIR/$FILE.sha512sum" || die "cannot download the checksum file"

echo "Verifying the SHA512 checksum..."
(cd "$TMP_DIR" && sha512sum -c "$FILE.sha512sum") || die "checksum mismatch for $FILE"

echo "Installing into $APP_DIR ..."
install -d -m 755 "$APP_DIR"
install -m 755 "$TMP_DIR/$FILE" "$APPIMAGE"
install -m 644 "$SELF_DIR/icon.png" "$ICON_FILE"
ln -sf "$APPIMAGE" "$LAUNCHER"

cat >"$DESKTOP_FILE" <<DESKTOP
[Desktop Entry]
Type=Application
Name=Xodus Nexus
Comment=Open-source Xbox-inspired game launcher
Exec=${LAUNCHER} %U
Icon=${ICON_FILE}
Terminal=false
Categories=Game;
StartupWMClass=xodus-nexus
DESKTOP
update-desktop-database /usr/share/applications 2>/dev/null || true

echo "Xodus Nexus installed. On first run, install the Nexus backend and runtime from Settings."
