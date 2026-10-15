#!/usr/bin/env bash
#
# Build and run dreamos-install on the example catalog for UI iteration,
# without a real pkexec elevation prompt each time
# (DREAMOS_INSTALL_SKIP_ROOT=1 makes the scripts run without pkexec). The
# example apps only touch /tmp/dreamos-install-demo, so they install fine
# unprivileged; real catalog scripts that need root will fail here.
#
# Usage:
#   ./scripts/run-dreamos-install.sh [catalog-dir]
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/../dreamos-install"
cargo build
DREAMOS_INSTALL_SKIP_ROOT=1 ./target/debug/dreamos-install "${1:-examples/apps}"
