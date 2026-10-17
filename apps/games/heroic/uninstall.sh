#!/usr/bin/env bash
# Remove the Heroic Games Launcher deb package.
set -euo pipefail

apt-get remove -y heroic
echo "Heroic Games Launcher removed."
