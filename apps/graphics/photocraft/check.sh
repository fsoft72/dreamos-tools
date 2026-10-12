#!/usr/bin/env bash
# Installed when the photocraft deb package is installed.
dpkg-query -W -f='${Status}' photocraft 2>/dev/null | grep -q "install ok installed"
