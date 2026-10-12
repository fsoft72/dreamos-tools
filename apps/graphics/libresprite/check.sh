#!/usr/bin/env bash
# Installed when the AppImage exists in the user's ~/.local/opt/libresprite.
USER_HOME="$(getent passwd "${DREAMOS_USER:-$(id -un)}" | cut -d: -f6)"
[ -x "$USER_HOME/.local/opt/libresprite/libresprite.AppImage" ]
