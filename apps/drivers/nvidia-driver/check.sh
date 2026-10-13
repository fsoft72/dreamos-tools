#!/usr/bin/env bash
# Installed when the NVIDIA proprietary driver metapackage (nvidia-driver or
# a legacy nvidia-tesla-*-driver) is installed.
dpkg-query -W -f='${Package} ${Status}\n' 'nvidia-driver' 'nvidia-tesla-*-driver' 2>/dev/null | grep -q "install ok installed"
