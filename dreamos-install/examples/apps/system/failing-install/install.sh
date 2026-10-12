#!/usr/bin/env bash
# Fails on purpose.
echo "Downloading..."
sleep 1
echo "error: package not found" >&2
exit 2
