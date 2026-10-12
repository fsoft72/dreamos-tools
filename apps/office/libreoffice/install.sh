#!/usr/bin/env bash
# Download the latest stable LibreOffice deb bundle from The Document
# Foundation, verify it and install every package with apt.
set -euo pipefail

readonly BASE_URL="https://download.documentfoundation.org/libreoffice/stable"

# Fail with a message and exit status 1.
die() {
	echo "ERROR: $*" >&2
	exit 1
}

command -v curl >/dev/null || apt-get install -y curl

TMP_DIR="$(mktemp -d)"
chmod 755 "$TMP_DIR"
trap 'rm -rf "$TMP_DIR"' EXIT

echo "Looking for the latest LibreOffice release..."
curl -fsSL "$BASE_URL/" -o "$TMP_DIR/index.html" || die "cannot list $BASE_URL/"
VERSION="$(grep -o 'href="[0-9][0-9.]*/"' "$TMP_DIR/index.html" | sed 's/href="//; s/\/"$//' | sort -V | tail -n1 || true)"
[ -n "$VERSION" ] || die "no LibreOffice release found"
readonly DEB_URL="$BASE_URL/$VERSION/deb/x86_64"

# Download one bundle and verify it against its .sha256 file.
fetch_bundle() {
	local file="$1" expected actual
	curl -fL --progress-bar "$DEB_URL/$file" -o "$TMP_DIR/$file" || return 1
	expected="$(curl -fsSL "$DEB_URL/$file.sha256" | awk '{print $1}')" || die "cannot download the checksum of $file"
	actual="$(sha256sum "$TMP_DIR/$file" | awk '{print $1}')"
	[ -n "$expected" ] && [ "$expected" = "$actual" ] || die "checksum mismatch for $file"
	tar -xzf "$TMP_DIR/$file" -C "$TMP_DIR"
	rm -f "$TMP_DIR/$file"
}

echo "Downloading LibreOffice $VERSION ..."
fetch_bundle "LibreOffice_${VERSION}_Linux_x86-64_deb.tar.gz" || die "download failed: $DEB_URL"

# Language pack for the system language (e.g. it_IT -> it-IT, then it). Not
# every language has one, a missing pack is not an error.
# Run as root by pkexec the environment has no LANG: read the system default.
SYSTEM_LANG="${LANG:-}"
[ -n "$SYSTEM_LANG" ] || SYSTEM_LANG="$(. /etc/default/locale 2>/dev/null && echo "${LANG:-}")" || true
LANG_CODE="${SYSTEM_LANG%%.*}"
if [ -n "$LANG_CODE" ] && [ "${LANG_CODE%%_*}" != "en" ] && [ "$LANG_CODE" != "C" ] && [ "$LANG_CODE" != "POSIX" ]; then
	for code in "${LANG_CODE//_/-}" "${LANG_CODE%%_*}"; do
		if curl -fsI "$DEB_URL/LibreOffice_${VERSION}_Linux_x86-64_deb_langpack_${code}.tar.gz" >/dev/null 2>&1; then
			echo "Downloading the language pack '$code' ..."
			fetch_bundle "LibreOffice_${VERSION}_Linux_x86-64_deb_langpack_${code}.tar.gz" || echo "Language pack not downloaded, continuing."
			break
		fi
	done
fi

echo "Installing LibreOffice $VERSION ..."
# Every package of the bundles, desktop menu entries included.
mapfile -t DEBS < <(find "$TMP_DIR" -name '*.deb' | sort)
[ "${#DEBS[@]}" -gt 0 ] || die "no deb packages found in the download"
apt-get install -y "${DEBS[@]}"
echo "LibreOffice $VERSION installed."
