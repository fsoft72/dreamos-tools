#!/usr/bin/env bash
# Remove the DBeaver Community deb package.
set -euo pipefail

apt-get remove -y dbeaver-ce
echo "DBeaver Community removed."
