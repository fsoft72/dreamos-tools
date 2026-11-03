#!/usr/bin/env bash
# Install the latest Craft Anything amd64 deb from GitHub Releases (SHA256
# verified) with apt. Re-run it to update; apt pulls in the libraries the
# package needs.
set -euo pipefail

readonly REPO="storytold/craft-anything"
readonly API_URL="https://api.github.com/repos/${REPO}/releases/latest"

# Fail with a message and exit status 1.
die() {
	echo "ERROR: $*" >&2
	exit 1
}

command -v apt-get >/dev/null || die "apt-get is required (Debian or Ubuntu)"
[ "$(uname -m)" = "x86_64" ] || die "this script installs the x86_64 build only"

if ! command -v curl >/dev/null || ! command -v jq >/dev/null; then
	apt-get update -y || echo "WARNING: apt-get update failed, continuing" >&2
	apt-get install -y curl jq
fi

TMP_DIR="$(mktemp -d)"
chmod 755 "$TMP_DIR"
trap 'rm -rf "$TMP_DIR"' EXIT

echo "Looking for the latest release of ${REPO}..."
curl -fsSL "$API_URL" -o "$TMP_DIR/release.json" || die "cannot query $API_URL"

DEB_URL="$(jq -r '[.assets[] | select(.name | test("-linux-x86_64\\.deb$"))] | first | .browser_download_url // empty' "$TMP_DIR/release.json")"
SUMS_URL="$(jq -r '[.assets[] | select(.name == "SHA256SUMS.txt")] | first | .browser_download_url // empty' "$TMP_DIR/release.json")"
[ -n "$DEB_URL" ] || die "no linux x86_64 deb found in the latest release"
[ -n "$SUMS_URL" ] || die "the release publishes no SHA256 checksums"
FILE="$(basename "$DEB_URL")"

echo "Downloading $FILE ..."
curl -fL --progress-bar "$DEB_URL" -o "$TMP_DIR/$FILE" || die "download failed: $DEB_URL"
curl -fsSL "$SUMS_URL" -o "$TMP_DIR/sums.txt" || die "download failed: $SUMS_URL"

echo "Verifying the SHA256 checksum..."
SHA256="$(awk -v f="$FILE" '$2 == f { print $1 }' "$TMP_DIR/sums.txt")"
[ -n "$SHA256" ] || die "no checksum listed for $FILE"
echo "$SHA256  $TMP_DIR/$FILE" | sha256sum -c - || die "checksum mismatch for $FILE"

echo "Installing $FILE ..."
apt-get install -y "$TMP_DIR/$FILE"
echo "Craft Anything installed."
