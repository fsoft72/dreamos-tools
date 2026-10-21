#!/bin/sh
# Test scripts/build-apt-repo.sh end to end. Run it as root in a throwaway
# ubuntu container (it installs packages and writes /etc/apt):
#   docker run --rm -v "$PWD":/src:ro ubuntu:26.04 sh /src/scripts/tests/test-apt-repo.sh
set -eu
SRC=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
WORK=$(mktemp -d)
REPO="$WORK/repo"
PORT=8089
BASE_URL="http://127.0.0.1:$PORT"

DEBIAN_FRONTEND=noninteractive apt-get update -qq >/dev/null
DEBIAN_FRONTEND=noninteractive apt-get install -y -qq apt-utils gnupg dpkg-dev python3 >/dev/null

# _fail <message>: stop the test
_fail() { echo "FAIL: $1" >&2; exit 1; }

# _make_deb <version>: builds a tiny dreamos-install package in $WORK
_make_deb() {
    d="$WORK/pkg-$1"
    mkdir -p "$d/DEBIAN" "$d/usr/bin"
    printf '#!/bin/sh\necho %s\n' "$1" > "$d/usr/bin/dreamos-install"
    chmod 755 "$d/usr/bin/dreamos-install"
    printf 'Package: dreamos-install\nVersion: %s\nArchitecture: amd64\nMaintainer: t <t@example.com>\nDescription: test\n' "$1" > "$d/DEBIAN/control"
    dpkg-deb --root-owner-group --build "$d" "$WORK/dreamos-install_$1_amd64.deb" >/dev/null
}

# Throwaway signing key, no passphrase
export GNUPGHOME="$WORK/gnupg"
mkdir -m 700 "$GNUPGHOME"
gpg --batch --quiet --passphrase '' --quick-generate-key "test <test@example.com>" default default never 2>/dev/null
GPG_KEY_ID=$(gpg --list-secret-keys --with-colons | awk -F: '/^fpr/{print $10; exit}')
export GPG_KEY_ID BASE_URL

for n in 1200 1201 1202 1203 1204 1205 1206 1207; do
    _make_deb "0.1.20261021.$n"
    "$SRC/scripts/build-apt-repo.sh" "$REPO" "$WORK/dreamos-install_0.1.20261021.${n}_amd64.deb" >/dev/null
done

POOL_COUNT=$(ls "$REPO"/pool/main/d/dreamos-install/*.deb | wc -l)
[ "$POOL_COUNT" -eq 5 ] || _fail "pool has $POOL_COUNT packages, expected 5"
ls "$REPO"/pool/main/d/dreamos-install/ | grep -q '1203' || _fail "newest 5 not kept (1203 missing)"
! ls "$REPO"/pool/main/d/dreamos-install/ | grep -q '1202' || _fail "1202 should have been pruned"
echo "PASS: pool pruned to the newest 5"

gpg --batch --verify "$REPO/dists/stable/InRelease" 2>/dev/null || _fail "InRelease does not verify"
echo "PASS: InRelease signature"
[ -f "$REPO/dreamos-tools.gpg" ] && [ -f "$REPO/index.html" ] && [ -f "$REPO/.nojekyll" ] || _fail "key, index.html or .nojekyll missing"
grep -q "signed-by=/etc/apt/keyrings/dreamos-tools.gpg] $BASE_URL stable main" "$REPO/index.html" || _fail "index.html lacks the source line"
echo "PASS: key, index.html, .nojekyll"

# Serve the repository and use it like a user would
(cd "$REPO" && python3 -m http.server "$PORT" --bind 127.0.0.1 >/dev/null 2>&1) &
SERVER=$!
trap 'kill $SERVER 2>/dev/null || true' EXIT
sleep 1
install -d -m 0755 /etc/apt/keyrings
cp "$REPO/dreamos-tools.gpg" /etc/apt/keyrings/dreamos-tools.gpg
echo "deb [signed-by=/etc/apt/keyrings/dreamos-tools.gpg] $BASE_URL stable main" > /etc/apt/sources.list.d/dreamos-tools.list
apt-get update -qq >/dev/null 2>&1 || _fail "apt-get update failed"
apt-get install -y -qq dreamos-install >/dev/null 2>&1 || _fail "apt-get install failed"
[ "$(/usr/bin/dreamos-install)" = "0.1.20261021.1207" ] || _fail "installed version is not 1207"
echo "PASS: apt update and install of 1207"

_make_deb "0.1.20261021.1208"
"$SRC/scripts/build-apt-repo.sh" "$REPO" "$WORK/dreamos-install_0.1.20261021.1208_amd64.deb" >/dev/null
apt-get update -qq >/dev/null 2>&1 || _fail "second apt-get update failed"
apt-get upgrade -y -qq >/dev/null 2>&1 || _fail "apt-get upgrade failed"
[ "$(/usr/bin/dreamos-install)" = "0.1.20261021.1208" ] || _fail "apt upgrade did not install 1208"
echo "PASS: apt upgrade to 1208"
echo "All checks passed"
