#!/usr/bin/env bash
# Demo uninstall: removes the marker file.
set -euo pipefail

echo "Removing Hello Tool..."
rm -f /tmp/dreamos-install-demo/hello-tool
echo "Done."
