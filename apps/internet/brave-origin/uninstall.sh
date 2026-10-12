#!/usr/bin/env bash
# Remove the Brave Origin deb package (the apt repository is left in place).
set -euo pipefail

apt-get remove -y brave-origin
echo "Brave Origin removed."
