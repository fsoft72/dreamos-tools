#!/usr/bin/env bash
# Remove the Opera deb package (the apt repository is left in place).
set -euo pipefail

apt-get remove -y opera-stable
echo "Opera removed."
