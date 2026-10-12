#!/usr/bin/env bash
# Install Inkscape from the distribution repositories with apt.
set -euo pipefail

apt-get update
apt-get install -y inkscape
echo "Inkscape installed."
