#!/usr/bin/env bash
# Add the official Opera apt repository and install Opera stable from it.
set -euo pipefail

readonly KEY_URL="https://deb.opera.com/archive.key"
readonly KEYRING="/etc/apt/keyrings/opera-browser.asc"
readonly SOURCES="/etc/apt/sources.list.d/opera-stable.sources"

command -v curl >/dev/null || apt-get install -y curl

echo "Adding the Opera apt repository ..."
install -d -m 0755 /etc/apt/keyrings
curl -fsSLo "$KEYRING" "$KEY_URL"
cat > "$SOURCES" <<SOURCES_FILE
Types: deb
URIs: https://deb.opera.com/opera-stable/
Suites: stable
Components: non-free
Signed-By: $KEYRING
SOURCES_FILE

# The package would otherwise ask whether to add its own repository file,
# which is the one written above.
echo "opera-stable opera-stable/add-deb-source boolean false" | debconf-set-selections

apt-get update
DEBIAN_FRONTEND=noninteractive apt-get install -y opera-stable
echo "Opera installed."
