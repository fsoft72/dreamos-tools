#!/usr/bin/env bash
# Slow on purpose, then checks the marker file.
sleep 3
[ -f /tmp/dreamos-install-demo/slow-check ]
