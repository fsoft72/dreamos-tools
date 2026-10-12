#!/usr/bin/env bash
# Installed when the editor binary exists in the user's ~/.local/opt/godot.
USER_HOME="$(getent passwd "${DREAMOS_USER:-$(id -un)}" | cut -d: -f6)"
[ -x "$USER_HOME/.local/opt/godot/godot" ]
