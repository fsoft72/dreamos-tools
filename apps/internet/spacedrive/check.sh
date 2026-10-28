#!/usr/bin/env bash
# Installed when the spacedrive deb package is installed.
dpkg-query -W -f='${Status}' spacedrive 2>/dev/null | grep -q "install ok installed"
