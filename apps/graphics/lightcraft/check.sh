#!/usr/bin/env bash
# Installed when the lightcraft deb package is installed.
dpkg-query -W -f='${Status}' lightcraft 2>/dev/null | grep -q "install ok installed"
