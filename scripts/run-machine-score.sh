#!/usr/bin/env bash
#
# Build and run machine-score. No elevated privileges needed - it only
# reads hardware info (CPU/GPU/RAM/storage), never writes to disk or
# system state.
#
# Usage:
#   ./scripts/run-machine-score.sh
set -euo pipefail

PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CRATE_DIR="${PROJECT_DIR}/machine-score"

test -f "${CRATE_DIR}/Cargo.toml" || {
    echo "ERROR: ${CRATE_DIR}/Cargo.toml not found" >&2
    exit 1
}
test -n "${DISPLAY:-}${WAYLAND_DISPLAY:-}" || {
    echo "ERROR: no graphical session (DISPLAY/WAYLAND_DISPLAY unset)" >&2
    exit 1
}

cd "${CRATE_DIR}"
cargo build
exec ./target/debug/machine-score
