#!/usr/bin/env bash
# Download the latest Google Chrome stable deb from Google and install it with apt.
# The deb adds Google's apt repository, so later updates come with the system ones.
set -euo pipefail

readonly DEB_URL="https://dl.google.com/linux/direct/google-chrome-stable_current_amd64.deb"

# Fail with a message and exit status 1.
die() {
	echo "ERROR: $*" >&2
	exit 1
}

command -v curl >/dev/null || apt-get install -y curl

TMP_DIR="$(mktemp -d)"
chmod 755 "$TMP_DIR"
trap 'rm -rf "$TMP_DIR"' EXIT

echo "Downloading Google Chrome ..."
curl -fL --progress-bar "$DEB_URL" -o "$TMP_DIR/google-chrome-stable.deb" || die "download failed: $DEB_URL"

echo "Installing Google Chrome ..."
apt-get install -y "$TMP_DIR/google-chrome-stable.deb"
echo "Google Chrome installed."
