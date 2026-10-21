#!/usr/bin/env bash
# Installed when the filezilla deb package is installed.
dpkg-query -W -f='${Status}' filezilla 2>/dev/null | grep -q "install ok installed"
