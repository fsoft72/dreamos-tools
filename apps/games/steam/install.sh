#!/usr/bin/env bash
# Install Steam with apt: enable the 32-bit architecture and the repository
# component that ships Steam (multiverse on Ubuntu, contrib/non-free on Debian).
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

# Add contrib and non-free to every Debian apt source that lacks them
# (deb822 .sources files and the classic sources.list).
enable_debian_components() {
	local file
	for file in /etc/apt/sources.list.d/*.sources; do
		[ -f "$file" ] || continue
		grep -q '^Components:.*non-free' "$file" && continue
		sed -i '/^Components:/ s/$/ contrib non-free/' "$file"
	done
	if [ -f /etc/apt/sources.list ]; then
		sed -i '/^deb .*main/ { /non-free/! s/main/main contrib non-free/ }' /etc/apt/sources.list
	fi
}

readonly FAMILY="$(distro_family)"

echo "Enabling the 32-bit architecture..."
dpkg --add-architecture i386

if [ "$FAMILY" = ubuntu ]; then
	echo "Enabling the multiverse repository..."
	command -v add-apt-repository >/dev/null || apt-get install -y software-properties-common
	add-apt-repository -y multiverse
	PACKAGE="steam"
else
	echo "Enabling the contrib and non-free components..."
	enable_debian_components
	PACKAGE="steam-installer"
fi

apt-get update
apt-get install -y "$PACKAGE"
echo "Steam installed."
