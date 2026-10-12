#!/usr/bin/env bash
# Remove the WordCraft deb package.
set -euo pipefail

apt-get remove -y wordcraft
echo "WordCraft removed."
