#!/usr/bin/env bash
# Install RetroArch with apt. On Ubuntu the official libretro stable PPA is
# added first; on Debian the distribution package is used.
set -euo pipefail

# Fail with a message and exit status 1.
die() {
	echo "ERROR: $*" >&2
	exit 1
}

# Print the distribution family: "ubuntu" or "debian".
distro_family() {
	local id id_like
	id="$(. /etc/os-release && echo "${ID:-}")"
	id_like="$(. /etc/os-release && echo "${ID_LIKE:-}")"
	case " $id $id_like " in
		*" ubuntu "*) echo ubuntu ;;
		*" debian "*) echo debian ;;
		*) die "unsupported distribution: ${id:-unknown}" ;;
	esac
}

if [ "$(distro_family)" = ubuntu ]; then
	echo "Adding the libretro stable PPA..."
	command -v add-apt-repository >/dev/null || apt-get install -y software-properties-common
	add-apt-repository -y ppa:libretro/stable
fi

apt-get update
apt-get install -y retroarch
echo "RetroArch installed."
