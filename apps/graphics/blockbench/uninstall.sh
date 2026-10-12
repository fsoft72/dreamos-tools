#!/usr/bin/env bash
# Remove the Blockbench deb package.
set -euo pipefail

apt-get remove -y blockbench
echo "Blockbench removed."
