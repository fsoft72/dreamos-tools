#!/usr/bin/env bash
# Installed when the vectorcraft deb package is installed.
dpkg-query -W -f='${Status}' vectorcraft 2>/dev/null | grep -q "install ok installed"
