#!/usr/bin/env bash
# Install the latest Telegram Desktop from GitHub Releases (SHA256 verified)
# into /opt/telegram-desktop, with a command in /usr/local/bin and an
# application menu entry. Re-run it to update.
set -euo pipefail

readonly REPO="telegramdesktop/tdesktop"
readonly API_URL="https://api.github.com/repos/${REPO}/releases/latest"
readonly APP_DIR="/opt/telegram-desktop"
readonly BINARY="${APP_DIR}/Telegram"
readonly ICON_FILE="${APP_DIR}/icon.png"
readonly LAUNCHER="/usr/local/bin/telegram-desktop"
readonly DESKTOP_FILE="/usr/share/applications/telegram-desktop.desktop"
SELF_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly SELF_DIR

# Fail with a message and exit status 1.
die() {
	echo "ERROR: $*" >&2
	exit 1
}

command -v apt-get >/dev/null || die "apt-get is required (Debian or Ubuntu)"
[ "$(uname -m)" = "x86_64" ] || die "only x86_64 builds are published"

TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

echo "Installing the tools and the libraries Telegram needs..."
apt-get update -y || echo "WARNING: apt-get update failed, continuing" >&2
apt-get install -y curl jq desktop-file-utils xz-utils libpango-1.0-0 libpangocairo-1.0-0 \
	libpangoft2-1.0-0 libcairo2 libfreetype6 libfontconfig1

echo "Looking for the latest release of ${REPO}..."
curl -fsSL "$API_URL" -o "$TMP_DIR/release.json" || die "cannot query $API_URL"

# The asset name is td-setup-linux-x64-<version>.tar.xz; GitHub publishes its SHA256 as "digest"
ASSET="$(jq -c '[.assets[] | select(.name | test("^td-setup-linux-x64-.*\\.tar\\.xz$"))] | first // empty' "$TMP_DIR/release.json")"
[ -n "$ASSET" ] || die "no linux x64 archive found in the latest release"
URL="$(jq -r '.browser_download_url' <<<"$ASSET")"
FILE="$(jq -r '.name' <<<"$ASSET")"
SHA256="$(jq -r '.digest // empty' <<<"$ASSET")"
SHA256="${SHA256#sha256:}"
[ -n "$SHA256" ] || die "the release publishes no SHA256 checksum for $FILE"

echo "Downloading $FILE ..."
curl -fL --progress-bar "$URL" -o "$TMP_DIR/$FILE" || die "download failed: $URL"

echo "Verifying the SHA256 checksum..."
echo "$SHA256  $TMP_DIR/$FILE" | sha256sum -c - || die "checksum mismatch for $FILE"

echo "Installing into $APP_DIR ..."
tar -xJf "$TMP_DIR/$FILE" -C "$TMP_DIR"
[ -x "$TMP_DIR/Telegram/Telegram" ] || die "the archive does not contain Telegram/Telegram"
rm -rf "$APP_DIR"
install -d -m 755 "$APP_DIR"
install -m 755 "$TMP_DIR/Telegram/Telegram" "$BINARY"
install -m 755 "$TMP_DIR/Telegram/Updater" "${APP_DIR}/Updater"
install -m 644 "$SELF_DIR/icon.png" "$ICON_FILE"
ln -sf "$BINARY" "$LAUNCHER"

cat >"$DESKTOP_FILE" <<DESKTOP
[Desktop Entry]
Type=Application
Name=Telegram Desktop
Comment=Official desktop version of Telegram messaging app
Exec=${LAUNCHER} -- %U
Icon=${ICON_FILE}
Terminal=false
Categories=Chat;Network;InstantMessaging;Qt;
MimeType=x-scheme-handler/tg;x-scheme-handler/tonsite;
StartupWMClass=TelegramDesktop
DESKTOP
update-desktop-database /usr/share/applications 2>/dev/null || true

echo "Telegram Desktop installed."
