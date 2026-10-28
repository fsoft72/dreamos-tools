#!/usr/bin/env bash
# Install the latest Spacedrive v2 amd64 deb from GitHub Releases with apt.
# The 0.4.x stable build needs ffmpeg 7, which Ubuntu 26.04 does not ship, so the
# newest v2 release (a pre-release for now) is used instead.
# Re-run it to update; apt pulls in the libraries the package needs.
set -euo pipefail

readonly REPO="spacedriveapp/spacedrive"
readonly API_URL="https://api.github.com/repos/${REPO}/releases?per_page=30"

# Fail with a message and exit status 1.
die() {
	echo "ERROR: $*" >&2
	exit 1
}

command -v apt-get >/dev/null || die "apt-get is required (Debian or Ubuntu)"
[ "$(uname -m)" = "x86_64" ] || die "this script installs the x86_64 build only"

command -v curl >/dev/null || apt-get install -y curl

TMP_DIR="$(mktemp -d)"
chmod 755 "$TMP_DIR"
trap 'rm -rf "$TMP_DIR"' EXIT

echo "Looking for the latest release of ${REPO}..."
curl -fsSL "$API_URL" -o "$TMP_DIR/release.json" || die "cannot query $API_URL"

# The first v2 desktop deb in the list (newest first): not nightly, 0.4.x, the frontend tarball, dmg or exe
URL="$(grep -o '"browser_download_url": *"[^"]*/download/v2[^"/]*/Spacedrive-linux-x86_64\.deb"' "$TMP_DIR/release.json" | head -n1 | sed 's/.*: *"//; s/"$//')"
[ -n "$URL" ] || die "no v2 amd64 deb found in the recent releases"
FILE="$(basename "$URL")"

echo "Downloading $FILE ..."
curl -fL --progress-bar "$URL" -o "$TMP_DIR/$FILE" || die "download failed: $URL"

echo "Installing $FILE ..."
apt-get install -y "$TMP_DIR/$FILE"
echo "Spacedrive installed."
