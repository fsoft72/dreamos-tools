#!/usr/bin/env bash
# Remove PhotoCraft files installed by install.sh.
set -euo pipefail

rm -rf /opt/photocraft
rm -f /usr/local/bin/photocraft /usr/share/applications/photocraft.desktop
echo "PhotoCraft removed."
