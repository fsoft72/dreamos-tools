#!/usr/bin/env bash
# Remove the VectorCraft deb package.
set -euo pipefail

apt-get remove -y vectorcraft
echo "VectorCraft removed."
