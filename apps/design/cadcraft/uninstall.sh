#!/usr/bin/env bash
# Remove the CadCraft deb package.
set -euo pipefail

apt-get remove -y cadcraft
echo "CadCraft removed."
