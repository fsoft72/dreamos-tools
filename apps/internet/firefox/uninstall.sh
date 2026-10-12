#!/usr/bin/env bash
# Remove the Firefox deb package (the apt repository is left in place).
set -euo pipefail

apt-get remove -y firefox
echo "Firefox removed."
