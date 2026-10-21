#!/usr/bin/env bash
# Installed when the rustdesk deb package is installed.
dpkg-query -W -f='${Status}' rustdesk 2>/dev/null | grep -q "install ok installed"
