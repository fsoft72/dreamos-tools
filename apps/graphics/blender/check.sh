#!/usr/bin/env bash
# Installed when the Blender binary exists in the user's ~/.local/opt/blender.
USER_HOME="$(getent passwd "${DREAMOS_USER:-$(id -un)}" | cut -d: -f6)"
[ -x "$USER_HOME/.local/opt/blender/blender" ]
