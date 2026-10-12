#!/usr/bin/env bash
# Install Tiled from the distribution repositories with apt.
set -euo pipefail

apt-get update
apt-get install -y tiled
echo "Tiled installed."
