#!/usr/bin/env bash
# Installed when the tiled deb package is installed.
dpkg-query -W -f='${Status}' tiled 2>/dev/null | grep -q "install ok installed"
