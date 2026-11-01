#!/usr/bin/env bash
# Install the latest git-exp from GitHub Releases (SHA256 verified) into
# /opt/git-exp, with a command in /usr/local/bin and an application menu
# entry. Re-run it to update.
set -euo pipefail

readonly REPO="fsoft72/git-exp"
readonly API_URL="https://api.github.com/repos/${REPO}/releases/latest"
readonly APP_DIR="/opt/git-exp"
readonly BINARY="${APP_DIR}/git-exp"
readonly ICON_FILE="${APP_DIR}/icon.png"
readonly LAUNCHER="/usr/local/bin/git-exp"
readonly DESKTOP_FILE="/usr/share/applications/git-exp.desktop"
SELF_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly SELF_DIR

# Fail with a message and exit status 1.
die() {
	echo "ERROR: $*" >&2
	exit 1
}

command -v apt-get >/dev/null || die "apt-get is required (Debian or Ubuntu)"
[ "$(uname -m)" = "x86_64" ] || die "only x86_64 builds are installed by this script"

TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

echo "Installing git-exp dependencies..."
apt-get update -y || echo "WARNING: apt-get update failed, continuing" >&2
apt-get install -y curl jq git desktop-file-utils libgl1 libxkbcommon0 libxkbcommon-x11-0 \
	libwayland-client0 libx11-6 libxcursor1 libxi6 libxrandr2

echo "Looking for the latest release of ${REPO}..."
curl -fsSL "$API_URL" -o "$TMP_DIR/release.json" || die "cannot query $API_URL"

# Assets are raw binaries named git-exp-<version>-linux-x86_64, plus a sha256sums.txt
BIN_URL="$(jq -r '[.assets[] | select(.name | test("-linux-x86_64$"))] | first | .browser_download_url // empty' "$TMP_DIR/release.json")"
SUMS_URL="$(jq -r '[.assets[] | select(.name | test("-sha256sums\\.txt$"))] | first | .browser_download_url // empty' "$TMP_DIR/release.json")"
[ -n "$BIN_URL" ] || die "no linux x86_64 binary found in the latest release"
[ -n "$SUMS_URL" ] || die "the release publishes no SHA256 checksums"
FILE="$(basename "$BIN_URL")"

echo "Downloading $FILE ..."
curl -fL --progress-bar "$BIN_URL" -o "$TMP_DIR/$FILE" || die "download failed: $BIN_URL"
curl -fsSL "$SUMS_URL" -o "$TMP_DIR/sums.txt" || die "download failed: $SUMS_URL"

echo "Verifying the SHA256 checksum..."
SHA256="$(awk -v f="$FILE" '$2 == f { print $1 }' "$TMP_DIR/sums.txt")"
[ -n "$SHA256" ] || die "no checksum listed for $FILE"
echo "$SHA256  $TMP_DIR/$FILE" | sha256sum -c - || die "checksum mismatch for $FILE"

echo "Installing into $APP_DIR ..."
rm -rf "$APP_DIR"
install -d -m 755 "$APP_DIR"
install -m 755 "$TMP_DIR/$FILE" "$BINARY"
install -m 644 "$SELF_DIR/icon.png" "$ICON_FILE"
ln -sf "$BINARY" "$LAUNCHER"

cat >"$DESKTOP_FILE" <<DESKTOP
[Desktop Entry]
Type=Application
Name=git-exp
Comment=Graphical Git interface inspired by VS Code Source Control
Exec=${LAUNCHER} %F
Icon=${ICON_FILE}
Terminal=false
Categories=Development;RevisionControl;
StartupWMClass=git-exp
DESKTOP
update-desktop-database /usr/share/applications 2>/dev/null || true

echo "git-exp installed."
