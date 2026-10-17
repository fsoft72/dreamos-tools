#!/usr/bin/env bash
# Remove the Steam deb package (games in ~/.local/share/Steam are kept).
set -euo pipefail

for pkg in steam steam-installer; do
	if dpkg-query -W -f='${Status}' "$pkg" 2>/dev/null | grep -q "install ok installed"; then
		apt-get remove -y "$pkg"
	fi
done
echo "Steam removed."
