#!/usr/bin/env bash
# Install FileZilla from the distribution repositories with apt.
set -euo pipefail

apt-get update
apt-get install -y filezilla
echo "FileZilla installed."
