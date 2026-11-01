#!/usr/bin/env bash
# Remove git-exp, its launcher and menu entry. Settings in the home folder
# are left untouched.
set -euo pipefail

rm -rf /opt/git-exp
rm -f /usr/local/bin/git-exp /usr/share/applications/git-exp.desktop
update-desktop-database /usr/share/applications 2>/dev/null || true
echo "git-exp removed."
