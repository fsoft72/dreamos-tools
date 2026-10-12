#!/usr/bin/env bash
# Installed when the cadcraft deb package is installed.
dpkg-query -W -f='${Status}' cadcraft 2>/dev/null | grep -q "install ok installed"
