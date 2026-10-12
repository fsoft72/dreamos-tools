#!/usr/bin/env bash
# Installed when the deckcraft deb package is installed.
dpkg-query -W -f='${Status}' deckcraft 2>/dev/null | grep -q "install ok installed"
