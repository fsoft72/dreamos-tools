#!/usr/bin/env bash
# Installed when the inkscape deb package is installed.
dpkg-query -W -f='${Status}' inkscape 2>/dev/null | grep -q "install ok installed"
