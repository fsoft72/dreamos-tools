#!/usr/bin/env bash
# Remove the EffectCraft deb package.
set -euo pipefail

apt-get remove -y effectcraft
echo "EffectCraft removed."
