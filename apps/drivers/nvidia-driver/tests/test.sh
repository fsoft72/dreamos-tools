#!/usr/bin/env bash
# Unit tests for the parsing functions of install.sh (no root needed).
# Usage: bash tests/test.sh
set -uo pipefail

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=../install.sh
source "$DIR/../install.sh"
FIX="$DIR/fixtures"
FAILURES=0

# Compare two values and report the result.
assert_eq() {
	local name="$1" expected="$2" actual="$3"
	if [ "$expected" = "$actual" ]; then
		echo "ok   $name"
	else
		echo "FAIL $name: expected '$expected', got '$actual'"
		FAILURES=$((FAILURES + 1))
	fi
}

assert_eq "lspci nouveau" "01:00.0|GA106 [GeForce RTX 3060]|nouveau" "$(parse_lspci <"$FIX/lspci_nvidia_nouveau.txt")"
assert_eq "lspci proprietary" "01:00.0|GA106 [GeForce RTX 3060]|nvidia" "$(parse_lspci <"$FIX/lspci_nvidia_proprietary.txt")"
assert_eq "lspci no nvidia" "" "$(parse_lspci <"$FIX/lspci_no_nvidia.txt")"

assert_eq "nvidia-detect standard" "nvidia-driver" "$(parse_nvidia_detect <"$FIX/nvidia_detect_found.txt")"
assert_eq "nvidia-detect legacy" "nvidia-tesla-470-driver" "$(parse_nvidia_detect <"$FIX/nvidia_detect_legacy.txt")"
assert_eq "nvidia-detect garbage" "" "$(echo "no useful output here" | parse_nvidia_detect)"

assert_eq "sb enabled" "enabled" "$(parse_sb_state <"$FIX/mokutil_sb_enabled.txt")"
assert_eq "sb disabled" "disabled" "$(parse_sb_state <"$FIX/mokutil_sb_disabled.txt")"
assert_eq "sb garbage" "unknown" "$(echo "garbage" | parse_sb_state)"

is_key_enrolled <"$FIX/mokutil_list_enrolled.txt" && found=yes || found=no
assert_eq "key enrolled" "yes" "$found"
printf '[key 1]\nSubject:\n\tCN=Some Other Key\n' | is_key_enrolled && found=yes || found=no
assert_eq "key absent" "no" "$found"

[ "$FAILURES" -eq 0 ] || exit 1
