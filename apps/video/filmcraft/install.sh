#!/usr/bin/env bash
# Download the latest FilmCraft amd64 deb from GitHub and install it with apt.
set -euo pipefail

readonly REPO="storytold/filmcraft"
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

URL="$(grep -o '"browser_download_url": *"[^"]*linux-x86_64\.deb"' "$TMP_DIR/release.json" | head -n1 | sed 's/.*: *"//; s/"$//')"
[ -n "$URL" ] || die "no linux x86_64 deb found in the latest release"
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

echo "Installing $FILE ..."
apt-get install -y "$TMP_DIR/$FILE"
echo "FilmCraft installed."
