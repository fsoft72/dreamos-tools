#!/usr/bin/env bash
# Remove the Tiled deb package.
set -euo pipefail

apt-get remove -y tiled
echo "Tiled removed."
