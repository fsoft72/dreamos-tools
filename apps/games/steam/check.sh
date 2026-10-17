#!/usr/bin/env bash
# Installed when the steam (Ubuntu) or steam-installer (Debian) package is installed.
for pkg in steam steam-installer; do
	dpkg-query -W -f='${Status}' "$pkg" 2>/dev/null | grep -q "install ok installed" && exit 0
done
exit 1
