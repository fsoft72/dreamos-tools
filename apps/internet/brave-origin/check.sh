#!/usr/bin/env bash
# Installed when the brave-origin deb package is installed.
dpkg-query -W -f='${Status}' brave-origin 2>/dev/null | grep -q "install ok installed"
