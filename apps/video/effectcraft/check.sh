#!/usr/bin/env bash
# Installed when the effectcraft deb package is installed.
dpkg-query -W -f='${Status}' effectcraft 2>/dev/null | grep -q "install ok installed"
