#!/usr/bin/env bash
# Download the latest LibreSprite AppImage from GitHub and install it in the
# home of the user who started dreamos-install.
# The project does not publish checksums, the download is over HTTPS only.
set -euo pipefail

readonly REPO="LibreSprite/LibreSprite"
readonly API_URL="https://api.github.com/repos/${REPO}/releases/latest"
readonly TARGET_USER="${DREAMOS_USER:-$(id -un)}"
USER_HOME="$(getent passwd "$TARGET_USER" | cut -d: -f6)"
readonly USER_HOME
readonly OPT_DIR="${USER_HOME}/.local/opt/libresprite"
readonly APPIMAGE="${OPT_DIR}/libresprite.AppImage"
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
	command -v curl >/dev/null || apt-get install -y curl
	# AppImages need FUSE 2 to mount themselves.
	if ! ldconfig -p | grep -q 'libfuse\.so\.2'; then
		echo "Installing FUSE 2 runtime..."
		apt-get install -y libfuse2t64 || apt-get install -y libfuse2 || die "cannot install libfuse2"
	fi
	APP_DIR="${DREAMOS_APP_DIR:-$(dirname "$(readlink -f "$0")")}"
	exec runuser -u "$TARGET_USER" -- env HOME="$USER_HOME" DREAMOS_APP_DIR="$APP_DIR" DREAMOS_USER="$TARGET_USER" bash "$(readlink -f "$0")"
fi

command -v curl >/dev/null || die "curl is required"
readonly APP_DIR="${DREAMOS_APP_DIR:-$(dirname "$(readlink -f "$0")")}"

TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

echo "Looking for the latest release of ${REPO}..."
curl -fsSL "$API_URL" -o "$TMP_DIR/release.json" || die "cannot query $API_URL"

URL="$(grep -o '"browser_download_url": *"[^"]*linux-x86_64\.AppImage"' "$TMP_DIR/release.json" | head -n1 | sed 's/.*: *"//; s/"$//' || true)"
[ -n "$URL" ] || die "no linux x86_64 AppImage found in the latest release"
FILE="$(basename "$URL")"

echo "Downloading $FILE ..."
curl -fL --progress-bar "$URL" -o "$TMP_DIR/$FILE" || die "download failed: $URL"

echo "Installing into $OPT_DIR ..."
mkdir -p "$OPT_DIR" "$BIN_DIR" "$APPS_DIR" "$ICONS_DIR"
install -m 755 "$TMP_DIR/$FILE" "$APPIMAGE"
install -m 644 "$APP_DIR/icon.png" "$ICONS_DIR/libresprite.png"
ln -sf "$APPIMAGE" "$BIN_DIR/libresprite"

cat > "$APPS_DIR/libresprite.desktop" <<DESKTOP
[Desktop Entry]
Type=Application
Name=LibreSprite
Comment=Animated sprite editor and pixel art tool
Exec=${APPIMAGE} %F
Icon=${ICONS_DIR}/libresprite.png
Categories=Graphics;2DGraphics;RasterGraphics;
Terminal=false
DESKTOP

echo "LibreSprite installed for user $TARGET_USER."
