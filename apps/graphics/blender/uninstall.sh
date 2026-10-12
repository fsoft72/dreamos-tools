#!/usr/bin/env bash
# Remove the Blender files installed by install.sh from the user's home.
set -euo pipefail

TARGET_USER="${DREAMOS_USER:-$(id -un)}"
USER_HOME="$(getent passwd "$TARGET_USER" | cut -d: -f6)"
[ -n "$USER_HOME" ] || { echo "ERROR: cannot find the home of user '$TARGET_USER'" >&2; exit 1; }

rm -rf "$USER_HOME/.local/opt/blender"
rm -f "$USER_HOME/.local/bin/blender" \
	"$USER_HOME/.local/share/applications/blender.desktop" \
	"$USER_HOME/.local/share/icons/blender.png"
echo "Blender removed for user $TARGET_USER."
