#!/usr/bin/env bash
# Remove the SoundCraft deb package.
set -euo pipefail

apt-get remove -y soundcraft
echo "SoundCraft removed."
