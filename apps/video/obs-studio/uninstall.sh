#!/usr/bin/env bash
# Remove the OBS Studio deb package.
set -euo pipefail

apt-get remove -y obs-studio
echo "OBS Studio removed."
