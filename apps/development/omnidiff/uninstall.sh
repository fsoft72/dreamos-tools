#!/usr/bin/env bash
# Remove OmniDiff, its launcher and menu entry.
set -euo pipefail

rm -rf /opt/omnidiff
rm -f /usr/local/bin/omnidiff /usr/share/applications/omnidiff.desktop
update-desktop-database /usr/share/applications 2>/dev/null || true
echo "omnidiff removed."
