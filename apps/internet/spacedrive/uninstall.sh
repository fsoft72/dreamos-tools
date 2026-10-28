#!/usr/bin/env bash
# Remove the Spacedrive deb package. The libraries and settings in the home folders are left untouched.
set -euo pipefail

apt-get remove -y spacedrive
echo "Spacedrive removed."
