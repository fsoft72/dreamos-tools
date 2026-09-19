#!/usr/bin/env bash
#
# Build and run nvidia-installer for UI iteration, without a real pkexec
# elevation prompt each time (NVIDIA_INSTALLER_SKIP_ROOT=1 makes main()
# skip re-exec via pkexec). Detection (lspci/nvidia-detect/mokutil) works
# fine unprivileged; anything that actually shells out to apt-get install
# or mokutil --import/openssl key generation will fail without real root,
# which is expected here - this script is for exercising the wizard's
# screens, not a full install.
#
# Usage:
#   ./scripts/run-nvidia-installer.sh
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/../nvidia-installer"
cargo build
NVIDIA_INSTALLER_SKIP_ROOT=1 ./target/debug/nvidia-installer
