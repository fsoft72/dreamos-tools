#!/usr/bin/env bash
# Remove the NVIDIA proprietary driver and fall back to nouveau. The
# Secure Boot signing key in /var/lib/dkms is kept, so a later install
# does not need a new enrollment.
set -euo pipefail

# Packages to remove: everything NVIDIA that apt installed, except the
# nvidia-detect helper.
mapfile -t PACKAGES < <(dpkg-query -W -f='${Package} ${Status}\n' 2>/dev/null |
	awk '$2 == "install" && $4 == "installed" && $1 ~ /^(nvidia-.*|libnvidia-.*|libcuda1|xserver-xorg-video-nvidia|firmware-nvidia-gsp)$/ && $1 != "nvidia-detect" {print $1}')

if [ "${#PACKAGES[@]}" -eq 0 ]; then
	echo "No NVIDIA driver packages found."
	exit 0
fi

echo "Removing: ${PACKAGES[*]}"
apt-get remove --purge -y "${PACKAGES[@]}"
update-initramfs -u
echo "NVIDIA driver removed. Reboot to go back to the nouveau driver."
