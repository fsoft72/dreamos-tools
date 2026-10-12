#!/usr/bin/env bash
# Installed when the opera-stable deb package is installed.
dpkg-query -W -f='${Status}' opera-stable 2>/dev/null | grep -q "install ok installed"
