#!/usr/bin/env bash
# Install OBS Studio from the distribution repositories with apt.
set -euo pipefail

apt-get update
apt-get install -y obs-studio
echo "OBS Studio installed."
