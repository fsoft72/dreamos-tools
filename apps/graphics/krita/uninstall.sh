#!/usr/bin/env bash
# Remove the Krita deb package.
set -euo pipefail

apt-get remove -y krita
echo "Krita removed."
