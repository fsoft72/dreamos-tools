#!/usr/bin/env bash
# Download the latest DBeaver Community deb from dbeaver.io and install it with apt.
# The project does not publish checksums, the download is over HTTPS only.
set -euo pipefail

readonly DEB_URL="https://dbeaver.io/files/dbeaver-ce_latest_amd64.deb"

# Fail with a message and exit status 1.
die() {
	echo "ERROR: $*" >&2
	exit 1
}

command -v curl >/dev/null || die "curl is required"

TMP_DIR="$(mktemp -d)"
chmod 755 "$TMP_DIR"
trap 'rm -rf "$TMP_DIR"' EXIT

echo "Downloading DBeaver Community ..."
curl -fL --progress-bar "$DEB_URL" -o "$TMP_DIR/dbeaver-ce.deb" || die "download failed: $DEB_URL"

echo "Installing DBeaver Community ..."
apt-get install -y "$TMP_DIR/dbeaver-ce.deb"
echo "DBeaver Community installed."
