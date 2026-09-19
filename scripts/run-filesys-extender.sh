#!/usr/bin/env bash
#
# Build and run filesys-extender for UI iteration, without a real pkexec
# elevation prompt each time (FILESYS_EXTENDER_SKIP_ROOT=1 makes main()
# skip re-exec via pkexec). Disk-listing screens (lsblk/findmnt) work fine
# unprivileged; anything that actually shells out to parted/mkfs.ext4/
# resize2fs will fail without real root, which is expected here - this
# script is for exercising the wizard's screens, not full disk operations.
#
# Usage:
#   ./scripts/run-filesys-extender.sh
set -euo pipefail

PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CRATE_DIR="${PROJECT_DIR}/filesys-extender"

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
FILESYS_EXTENDER_SKIP_ROOT=1 exec ./target/debug/filesys-extender
