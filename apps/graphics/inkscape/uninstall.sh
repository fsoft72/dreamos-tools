#!/usr/bin/env bash
# Remove the Inkscape deb package.
set -euo pipefail

apt-get remove -y inkscape
echo "Inkscape removed."
