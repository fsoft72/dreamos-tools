#!/usr/bin/env bash
# Installed when the dbeaver-ce deb package is installed.
dpkg-query -W -f='${Status}' dbeaver-ce 2>/dev/null | grep -q "install ok installed"
