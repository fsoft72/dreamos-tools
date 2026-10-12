#!/usr/bin/env bash
# Install HandBrake from the distribution repositories with apt.
set -euo pipefail

apt-get update
apt-get install -y handbrake
echo "HandBrake installed."
