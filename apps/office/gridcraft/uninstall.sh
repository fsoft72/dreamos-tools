#!/usr/bin/env bash
# Remove the GridCraft deb package.
set -euo pipefail

apt-get remove -y gridcraft
echo "GridCraft removed."
