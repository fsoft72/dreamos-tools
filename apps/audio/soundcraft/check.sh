#!/usr/bin/env bash
# Installed when the soundcraft deb package is installed.
dpkg-query -W -f='${Status}' soundcraft 2>/dev/null | grep -q "install ok installed"
