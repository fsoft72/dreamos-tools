#!/usr/bin/env bash
# Demo install with coloured output; creates a marker file only.
set -euo pipefail

GREEN='\033[0;32m'
RESET='\033[0m'

echo "Reading package lists..."
sleep 1
for pkg in demo-paint demo-paint-data demo-brushes; do
	printf "${GREEN}Setting up${RESET} %s ...\n" "$pkg"
	sleep 0.5
done
mkdir -p /tmp/dreamos-install-demo
touch /tmp/dreamos-install-demo/demo-paint
echo "Demo Paint installed."
