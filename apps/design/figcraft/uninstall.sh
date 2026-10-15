#!/usr/bin/env bash
# Remove the FigCraft deb package.
set -euo pipefail

apt-get remove -y figcraft
echo "FigCraft removed."
