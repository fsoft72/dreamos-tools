#!/usr/bin/env bash
# Demo install: creates a marker file, nothing else.
set -euo pipefail

echo "Installing Hello Tool..."
mkdir -p /tmp/dreamos-install-demo
for step in 1 2 3; do
	echo "  step $step/3"
	sleep 1
done
echo "hello" > /tmp/dreamos-install-demo/hello-tool
echo "Done."
