#!/usr/bin/env bash
# Detect the NVIDIA GPU, set up Secure Boot module signing when needed and
# install the proprietary driver with apt (Debian trixie / dreamos).
#
# Optional environment: NVIDIA_PACKAGE overrides the detected driver package
# (e.g. a legacy nvidia-tesla-470-driver for an old card).
set -euo pipefail

readonly MOK_KEY_PATH="/var/lib/dkms/mok.key"
readonly MOK_CERT_PATH="/var/lib/dkms/mok.pub"
readonly MOK_SUBJECT="dreamos nvidia-installer MOK"
readonly FALLBACK_PACKAGE="nvidia-driver"

# Fail with a message and exit status 1.
die() {
	echo "ERROR: $*" >&2
	exit 1
}

# Print one line per NVIDIA GPU found in `lspci -nnk` output (stdin):
# "<pci slot>|<model>|<driver in use or none>".
parse_lspci() {
	awk '
		function flush() { if (slot != "") print slot "|" model "|" driver; slot = "" }
		/^[^\t]/ {
			flush()
			if ((/VGA compatible controller/ || /3D controller/) && /\[10de:/) {
				slot = $1
				model = $0
				sub(/.*NVIDIA Corporation /, "", model)
				sub(/ \[10de:.*/, "", model)
				driver = "none"
			}
			next
		}
		slot != "" && /Kernel driver in use: / {
			driver = $0
			sub(/.*Kernel driver in use: /, "", driver)
		}
		END { flush() }
	'
}

# Print the package recommended by `nvidia-detect` output (stdin), nothing
# when the output cannot be parsed.
parse_nvidia_detect() {
	awk '
		found && NF { gsub(/^[ \t]+|[ \t]+$/, ""); print; exit }
		$0 == "It is recommended to install the" { found = 1 }
	'
}

# Print "enabled", "disabled" or "unknown" from `mokutil --sb-state` output
# (stdin).
parse_sb_state() {
	local text
	text="$(tr '[:upper:]' '[:lower:]')"
	case "$text" in
		*"secureboot enabled"*) echo "enabled" ;;
		*"secureboot disabled"*) echo "disabled" ;;
		*) echo "unknown" ;;
	esac
}

# Succeed when `mokutil --list-enrolled` output (stdin) contains our key.
is_key_enrolled() {
	awk -v subject="$MOK_SUBJECT" '
		/^\[key / { in_key = 1; has_subject = 0 }
		/Subject:/ { has_subject = 1; next }
		has_subject && index($0, subject) { found = 1 }
		END { exit !found }
	'
}

# Create the DKMS signing key pair when it does not exist yet.
ensure_mok_keypair() {
	[ -f "$MOK_KEY_PATH" ] && [ -f "$MOK_CERT_PATH" ] && return 0
	echo "Generating the module signing key..."
	mkdir -p "$(dirname "$MOK_KEY_PATH")"
	openssl req -new -x509 -newkey rsa:2048 -keyout "$MOK_KEY_PATH" -outform DER \
		-out "$MOK_CERT_PATH" -nodes -days 36500 -subj "/CN=$MOK_SUBJECT/"
	chmod 600 "$MOK_KEY_PATH"
}

# Make sure the signing key is (or will be, after a reboot) enrolled when
# Secure Boot is on. Sets MOK_PENDING=1 when the user must complete the
# enrollment at the next boot.
setup_secure_boot() {
	local state
	state="$(mokutil --sb-state 2>/dev/null | parse_sb_state || true)"
	echo "Secure Boot: $state"
	[ "$state" = "enabled" ] || return 0

	ensure_mok_keypair
	if mokutil --list-enrolled 2>/dev/null | is_key_enrolled; then
		echo "The signing key is already enrolled."
		return 0
	fi

	echo
	echo "Secure Boot is on: the driver module must be signed, so its key has to be enrolled."
	echo "Choose a one-time password (min 8 characters). You will type it again at the next boot."
	mokutil --import "$MOK_CERT_PATH" || die "mokutil --import failed"
	MOK_PENDING=1
}

# Install the helper tools the steps below rely on, when missing.
ensure_tools() {
	local missing=()
	command -v lspci >/dev/null || missing+=(pciutils)
	command -v mokutil >/dev/null || missing+=(mokutil)
	command -v openssl >/dev/null || missing+=(openssl)
	[ "${#missing[@]}" -eq 0 ] || apt-get install -y "${missing[@]}"
}

# Print the driver package to install: NVIDIA_PACKAGE, else nvidia-detect's
# recommendation, else the plain nvidia-driver metapackage.
choose_package() {
	local pkg=""
	if [ -n "${NVIDIA_PACKAGE:-}" ]; then
		echo "$NVIDIA_PACKAGE"
		return 0
	fi
	command -v nvidia-detect >/dev/null || apt-get install -y nvidia-detect >&2 || true
	if command -v nvidia-detect >/dev/null; then
		pkg="$(nvidia-detect 2>/dev/null | parse_nvidia_detect || true)"
	fi
	echo "${pkg:-$FALLBACK_PACKAGE}"
}

# Run the whole install.
main() {
	[ "$(id -u)" -eq 0 ] || die "this script must run as root"
	MOK_PENDING=0

	echo "==> apt-get update"
	apt-get update
	ensure_tools

	echo "==> Looking for NVIDIA GPUs"
	local gpus
	gpus="$(lspci -nnk | parse_lspci)"
	[ -n "$gpus" ] || die "no NVIDIA GPU detected"
	while IFS='|' read -r slot model driver; do
		echo "  $slot  $model  (driver in use: $driver)"
	done <<<"$gpus"

	local package
	package="$(choose_package)"
	echo "==> Driver package: $package"

	echo "==> Secure Boot"
	setup_secure_boot

	echo "==> apt-get install -y $package"
	apt-get install -y "$package"

	echo
	echo "NVIDIA driver installed. Reboot to load it."
	if [ "$MOK_PENDING" -eq 1 ]; then
		echo "At the reboot a blue MOK Management screen appears: choose Enroll MOK,"
		echo "Continue, then type the one-time password you just chose."
	fi
}

# Run only when executed, so the tests can source the parsing functions.
if [ "${BASH_SOURCE[0]}" = "$0" ]; then
	main "$@"
fi
