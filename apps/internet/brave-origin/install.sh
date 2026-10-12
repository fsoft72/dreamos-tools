#!/usr/bin/env bash
# Add the official Brave apt repository and install Brave Origin from it.
set -euo pipefail

readonly BASE_URL="https://brave-browser-apt-release.s3.brave.com"
readonly KEYRING="/usr/share/keyrings/brave-browser-archive-keyring.gpg"
readonly SOURCES="/etc/apt/sources.list.d/brave-browser-release.sources"

command -v curl >/dev/null || apt-get install -y curl

echo "Adding the Brave apt repository ..."
curl -fsSLo "$KEYRING" "$BASE_URL/brave-browser-archive-keyring.gpg"
curl -fsSLo "$SOURCES" "$BASE_URL/brave-browser.sources"

apt-get update
apt-get install -y brave-origin
echo "Brave Origin installed."
