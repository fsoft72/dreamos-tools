#!/usr/bin/env bash
# Add the official Mozilla apt repository, install the Firefox deb from it and
# remove the Firefox snap. Ubuntu's own "firefox" package only installs the
# snap, so the repository is pinned with a higher priority.
set -euo pipefail

readonly KEY_URL="https://packages.mozilla.org/apt/repo-signing-key.gpg"
readonly KEYRING="/etc/apt/keyrings/packages.mozilla.org.asc"
readonly SOURCES="/etc/apt/sources.list.d/mozilla.sources"
readonly PREFERENCES="/etc/apt/preferences.d/mozilla"

command -v curl >/dev/null || apt-get install -y curl

echo "Adding the Mozilla apt repository ..."
install -d -m 0755 /etc/apt/keyrings
curl -fsSLo "$KEYRING" "$KEY_URL"
cat > "$SOURCES" <<SOURCES_FILE
Types: deb
URIs: https://packages.mozilla.org/apt
Suites: mozilla
Components: main
Signed-By: $KEYRING
SOURCES_FILE
cat > "$PREFERENCES" <<'PREFERENCES_FILE'
Package: *
Pin: origin packages.mozilla.org
Pin-Priority: 1000
PREFERENCES_FILE

apt-get update
apt-get install -y firefox

# Only after the deb is in place: drop the snap build. snap takes an automatic
# snapshot of its user data first (kept for a while, see `snap saved`).
if command -v snap >/dev/null && snap list firefox >/dev/null 2>&1; then
	echo "Removing the Firefox snap ..."
	snap remove firefox
fi
echo "Firefox installed."
