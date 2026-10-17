#!/usr/bin/env bash
# Installed when the heroic deb package is installed.
dpkg-query -W -f='${Status}' heroic 2>/dev/null | grep -q "install ok installed"
