#!/usr/bin/env bash
# Install Krita from the distribution repositories with apt.
set -euo pipefail

apt-get update
apt-get install -y krita
echo "Krita installed."
