#!/usr/bin/env bash
# Installed when the designcraft deb package is installed.
dpkg-query -W -f='${Status}' designcraft 2>/dev/null | grep -q "install ok installed"
