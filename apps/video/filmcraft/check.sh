#!/usr/bin/env bash
# Installed when the filmcraft deb package is installed.
dpkg-query -W -f='${Status}' filmcraft 2>/dev/null | grep -q "install ok installed"
