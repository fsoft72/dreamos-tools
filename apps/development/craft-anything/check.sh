#!/usr/bin/env bash
# Installed when the craft-anything deb package is installed.
dpkg-query -W -f='${Status}' craft-anything 2>/dev/null | grep -q "install ok installed"
