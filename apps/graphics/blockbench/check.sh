#!/usr/bin/env bash
# Installed when the blockbench deb package is installed.
dpkg-query -W -f='${Status}' blockbench 2>/dev/null | grep -q "install ok installed"
