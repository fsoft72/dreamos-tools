#!/usr/bin/env bash
# Remove the RetroArch deb package (the libretro PPA and ~/.config/retroarch are kept).
set -euo pipefail

apt-get remove -y retroarch
echo "RetroArch removed."
