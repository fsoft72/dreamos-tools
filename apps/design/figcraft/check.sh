#!/usr/bin/env bash
# Installed when the figcraft deb package is installed.
dpkg-query -W -f='${Status}' figcraft 2>/dev/null | grep -q "install ok installed"
