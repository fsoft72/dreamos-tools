#!/usr/bin/env bash
# Remove the DeckCraft deb package.
set -euo pipefail

apt-get remove -y deckcraft
echo "DeckCraft removed."
