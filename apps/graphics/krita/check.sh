#!/usr/bin/env bash
# Installed when the krita deb package is installed.
dpkg-query -W -f='${Status}' krita 2>/dev/null | grep -q "install ok installed"
