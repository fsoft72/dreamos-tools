#!/usr/bin/env bash
# Installed when the obs-studio deb package is installed.
dpkg-query -W -f='${Status}' obs-studio 2>/dev/null | grep -q "install ok installed"
