#!/usr/bin/env bash
# Remove the HandBrake deb package.
set -euo pipefail

apt-get remove -y handbrake
echo "HandBrake removed."
