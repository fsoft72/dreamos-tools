#!/usr/bin/env bash
# Installed when the handbrake deb package is installed.
dpkg-query -W -f='${Status}' handbrake 2>/dev/null | grep -q "install ok installed"
