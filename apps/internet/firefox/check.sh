#!/usr/bin/env bash
# Installed when the Mozilla deb is installed. The Ubuntu "firefox" package
# is only a stub for the snap and does not ship /usr/lib/firefox/firefox.
dpkg-query -W -f='${Status}' firefox 2>/dev/null | grep -q "install ok installed" && [ -x /usr/lib/firefox/firefox ]
