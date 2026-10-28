#!/usr/bin/env bash
# Remove the rqbit deb package. The settings and the downloaded files are left untouched.
set -euo pipefail

apt-get remove -y rqbit-desktop
echo "rqbit removed."
