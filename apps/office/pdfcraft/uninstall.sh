#!/usr/bin/env bash
# Remove the PdfCraft deb package.
set -euo pipefail

apt-get remove -y pdfcraft
echo "PdfCraft removed."
