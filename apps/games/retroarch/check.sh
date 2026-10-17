#!/usr/bin/env bash
# Installed when the retroarch deb package is installed.
dpkg-query -W -f='${Status}' retroarch 2>/dev/null | grep -q "install ok installed"
