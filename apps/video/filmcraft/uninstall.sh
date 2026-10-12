#!/usr/bin/env bash
# Remove the FilmCraft deb package.
set -euo pipefail

apt-get remove -y filmcraft
echo "FilmCraft removed."
