#!/usr/bin/env bash
# Install the latest rqbit desktop amd64 deb from GitHub Releases with apt.
# Re-run it to update; apt pulls in the libraries the package needs.
set -euo pipefail

readonly REPO="ikatson/rqbit"
readonly API_URL="https://api.github.com/repos/${REPO}/releases/latest"

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

# The desktop app deb only: not the arm64 deb or the command line binaries
URL="$(grep -o '"browser_download_url": *"[^"]*/rqbit-desktop_[0-9][0-9.]*_amd64\.deb"' "$TMP_DIR/release.json" | head -n1 | sed 's/.*: *"//; s/"$//')"
[ -n "$URL" ] || die "no amd64 deb found in the latest release"
FILE="$(basename "$URL")"

echo "Downloading $FILE ..."
curl -fL --progress-bar "$URL" -o "$TMP_DIR/$FILE" || die "download failed: $URL"

echo "Installing $FILE ..."
apt-get install -y "$TMP_DIR/$FILE"
echo "rqbit installed."
