#!/usr/bin/env bash
# Remove the Craft Anything deb package. Projects and settings in the home folder are left untouched.
set -euo pipefail

apt-get remove -y craft-anything
echo "Craft Anything removed."
