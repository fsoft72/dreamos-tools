#!/usr/bin/env bash
# Remove the DesignCraft deb package.
set -euo pipefail

apt-get remove -y designcraft
echo "DesignCraft removed."
