#!/usr/bin/env bash
# Installed when the gridcraft deb package is installed.
dpkg-query -W -f='${Status}' gridcraft 2>/dev/null | grep -q "install ok installed"
