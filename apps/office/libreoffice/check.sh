#!/usr/bin/env bash
# Installed when any LibreOffice Writer package is installed (official
# versioned packages or the distribution one).
dpkg-query -W -f='${Status}\n' 'libreoffice[0-9]*-writer' libreoffice-writer 2>/dev/null | grep -q "install ok installed"
