#!/usr/bin/env bash
# Installed when the google-chrome-stable deb package is installed.
dpkg-query -W -f='${Status}' google-chrome-stable 2>/dev/null | grep -q "install ok installed"
