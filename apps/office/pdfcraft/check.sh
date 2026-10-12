#!/usr/bin/env bash
# Installed when the pdfcraft deb package is installed.
dpkg-query -W -f='${Status}' pdfcraft 2>/dev/null | grep -q "install ok installed"
