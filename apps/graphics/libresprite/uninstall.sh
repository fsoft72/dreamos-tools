#!/usr/bin/env bash
# Remove the LibreSprite files installed by install.sh from the user's home.
set -euo pipefail

TARGET_USER="${DREAMOS_USER:-$(id -un)}"
USER_HOME="$(getent passwd "$TARGET_USER" | cut -d: -f6)"
[ -n "$USER_HOME" ] || { echo "ERROR: cannot find the home of user '$TARGET_USER'" >&2; exit 1; }

rm -rf "$USER_HOME/.local/opt/libresprite"
rm -f "$USER_HOME/.local/bin/libresprite" \
	"$USER_HOME/.local/share/applications/libresprite.desktop" \
	"$USER_HOME/.local/share/icons/libresprite.png"
echo "LibreSprite removed for user $TARGET_USER."
