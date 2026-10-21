#!/usr/bin/env bash
# Remove the FileZilla deb package. The saved sites in the home folder are left untouched.
set -euo pipefail

apt-get remove -y filezilla
echo "FileZilla removed."
