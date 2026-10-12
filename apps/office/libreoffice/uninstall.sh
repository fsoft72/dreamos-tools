#!/usr/bin/env bash
# Remove every LibreOffice release installed from the official versioned
# packages (libreoffice26.2*, libobasis26.2*, ...).
set -euo pipefail

apt-get remove -y 'libreoffice[0-9]*' 'libobasis[0-9]*'
echo "LibreOffice removed."
