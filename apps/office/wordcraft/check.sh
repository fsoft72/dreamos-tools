#!/usr/bin/env bash
# Installed when the wordcraft deb package is installed.
dpkg-query -W -f='${Status}' wordcraft 2>/dev/null | grep -q "install ok installed"
