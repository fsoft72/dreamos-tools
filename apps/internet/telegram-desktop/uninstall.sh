#!/usr/bin/env bash
# Remove Telegram Desktop, its launcher and menu entry. Accounts and data in
# the home folder (~/.local/share/TelegramDesktop) are left untouched.
set -euo pipefail

rm -rf /opt/telegram-desktop
rm -f /usr/local/bin/telegram-desktop /usr/share/applications/telegram-desktop.desktop
update-desktop-database /usr/share/applications 2>/dev/null || true
echo "Telegram Desktop removed."
