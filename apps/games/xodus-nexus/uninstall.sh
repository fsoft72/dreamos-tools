#!/usr/bin/env bash
# Remove the Xodus Nexus AppImage, its launcher and menu entry. User data in
# the home folders (games, settings, sign-in) is left untouched.
set -euo pipefail

rm -rf /opt/xodus-nexus
rm -f /usr/local/bin/xodus-nexus /usr/share/applications/xodus-nexus.desktop
update-desktop-database /usr/share/applications 2>/dev/null || true
echo "Xodus Nexus removed."
