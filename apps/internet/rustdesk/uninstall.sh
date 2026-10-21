#!/usr/bin/env bash
# Remove the RustDesk deb package. The settings in the home folders are left untouched.
set -euo pipefail

apt-get remove -y rustdesk
echo "RustDesk removed."
