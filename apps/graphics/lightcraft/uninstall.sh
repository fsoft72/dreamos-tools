#!/usr/bin/env bash
# Remove the LightCraft deb package.
set -euo pipefail

apt-get remove -y lightcraft
echo "LightCraft removed."
