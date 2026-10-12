#!/usr/bin/env bash
# Remove the Google Chrome deb package (the apt repository is left in place).
set -euo pipefail

apt-get remove -y google-chrome-stable
echo "Google Chrome removed."
