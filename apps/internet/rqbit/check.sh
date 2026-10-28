#!/usr/bin/env bash
# Installed when the rqbit-desktop deb package is installed.
dpkg-query -W -f='${Status}' rqbit-desktop 2>/dev/null | grep -q "install ok installed"
