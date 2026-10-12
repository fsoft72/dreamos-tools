#!/usr/bin/env bash
# Download the latest Blockbench deb from GitHub and install it with apt.
# The project does not publish checksums, the download is over HTTPS only.
set -euo pipefail

readonly REPO="JannisX11/blockbench"
readonly API_URL="https://api.github.com/repos/${REPO}/releases/latest"

# Fail with a message and exit status 1.
die() {
	echo "ERROR: $*" >&2
	exit 1
}

command -v curl >/dev/null || die "curl is required"

TMP_DIR="$(mktemp -d)"
chmod 755 "$TMP_DIR"
trap 'rm -rf "$TMP_DIR"' EXIT

echo "Looking for the latest release of ${REPO}..."
curl -fsSL "$API_URL" -o "$TMP_DIR/release.json" || die "cannot query $API_URL"

URL="$(grep -o '"browser_download_url": *"[^"]*\.deb"' "$TMP_DIR/release.json" | head -n1 | sed 's/.*: *"//; s/"$//' || true)"
[ -n "$URL" ] || die "no deb found in the latest release"
FILE="$(basename "$URL")"

echo "Downloading $FILE ..."
curl -fL --progress-bar "$URL" -o "$TMP_DIR/$FILE" || die "download failed: $URL"

echo "Installing $FILE ..."
apt-get install -y "$TMP_DIR/$FILE"
echo "Blockbench installed."
