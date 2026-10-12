#!/usr/bin/env bash
# Download the latest stable Godot editor (single file in a zip) and install
# it in the home of the user who started dreamos-install.
set -euo pipefail

readonly REPO="godotengine/godot"
readonly API_URL="https://api.github.com/repos/${REPO}/releases/latest"
readonly TARGET_USER="${DREAMOS_USER:-$(id -un)}"
USER_HOME="$(getent passwd "$TARGET_USER" | cut -d: -f6)"
readonly USER_HOME
readonly OPT_DIR="${USER_HOME}/.local/opt/godot"
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
	for tool in curl unzip; do
		command -v "$tool" >/dev/null || apt-get install -y "$tool"
	done
	APP_DIR="${DREAMOS_APP_DIR:-$(dirname "$(readlink -f "$0")")}"
	exec runuser -u "$TARGET_USER" -- env HOME="$USER_HOME" DREAMOS_APP_DIR="$APP_DIR" DREAMOS_USER="$TARGET_USER" bash "$(readlink -f "$0")"
fi

command -v curl >/dev/null || die "curl is required"
command -v unzip >/dev/null || die "unzip is required"
readonly APP_DIR="${DREAMOS_APP_DIR:-$(dirname "$(readlink -f "$0")")}"

TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

echo "Looking for the latest release of ${REPO}..."
curl -fsSL "$API_URL" -o "$TMP_DIR/release.json" || die "cannot query $API_URL"

URL="$(grep -o '"browser_download_url": *"[^"]*-stable_linux\.x86_64\.zip"' "$TMP_DIR/release.json" | head -n1 | sed 's/.*: *"//; s/"$//' || true)"
[ -n "$URL" ] || die "no linux x86_64 build found in the latest release"
SUMS_URL="$(grep -o '"browser_download_url": *"[^"]*SHA512-SUMS\.txt"' "$TMP_DIR/release.json" | head -n1 | sed 's/.*: *"//; s/"$//' || true)"
FILE="$(basename "$URL")"

echo "Downloading $FILE ..."
curl -fL --progress-bar "$URL" -o "$TMP_DIR/$FILE" || die "download failed: $URL"

if [ -n "$SUMS_URL" ]; then
	echo "Verifying SHA512 checksum..."
	curl -fsSL "$SUMS_URL" -o "$TMP_DIR/SHA512-SUMS.txt" || die "cannot download $SUMS_URL"
	EXPECTED="$(awk -v f="$FILE" '$2 == f {print $1}' "$TMP_DIR/SHA512-SUMS.txt" | head -n1)"
	ACTUAL="$(sha512sum "$TMP_DIR/$FILE" | awk '{print $1}')"
	[ -n "$EXPECTED" ] || die "$FILE not listed in SHA512-SUMS.txt"
	[ "$EXPECTED" = "$ACTUAL" ] || die "checksum mismatch (expected $EXPECTED, got $ACTUAL)"
fi

echo "Installing into $OPT_DIR ..."
unzip -oq "$TMP_DIR/$FILE" -d "$TMP_DIR/unpacked"
EDITOR_BIN="$(find "$TMP_DIR/unpacked" -maxdepth 1 -type f -name 'Godot_*' | head -n1)"
[ -n "$EDITOR_BIN" ] || die "editor binary not found in $FILE"

mkdir -p "$OPT_DIR" "$BIN_DIR" "$APPS_DIR" "$ICONS_DIR"
install -m 755 "$EDITOR_BIN" "$OPT_DIR/godot"
install -m 644 "$APP_DIR/icon.png" "$ICONS_DIR/godot.png"
ln -sf "$OPT_DIR/godot" "$BIN_DIR/godot"

cat > "$APPS_DIR/godot.desktop" <<DESKTOP
[Desktop Entry]
Type=Application
Name=Godot Engine
Comment=2D and 3D game engine
Exec=${OPT_DIR}/godot --editor %f
Icon=${ICONS_DIR}/godot.png
Categories=Development;IDE;
Terminal=false
DESKTOP

echo "Godot installed for user $TARGET_USER."
