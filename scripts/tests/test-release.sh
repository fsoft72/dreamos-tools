#!/bin/sh
# Test scripts/release.sh against a throwaway repository and a bare remote.
# The real repository is never touched or pushed.
#
# Usage: scripts/tests/test-release.sh
set -eu
SRC=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT
FIXED_DATE="2026-10-21 15:30"
TAG=v0.1.20261021.1530

# _fail <message>: stop the test
_fail() { echo "FAIL: $1" >&2; exit 1; }

git init -q --bare "$WORK/remote.git"
git init -q -b main "$WORK/repo"
mkdir -p "$WORK/repo/scripts"
cp "$SRC/scripts/release.sh" "$WORK/repo/scripts/release.sh"
cd "$WORK/repo"
git config user.name test
git config user.email test@example.com
git remote add origin "$WORK/remote.git"
git add -A
git commit -q -m init

# A dirty tree is refused
echo dirty > dirty.txt
RELEASE_DATE=$FIXED_DATE sh scripts/release.sh -y >/dev/null 2>&1 && _fail "dirty tree accepted"
echo "PASS: dirty tree refused"
rm dirty.txt

# Answering no aborts and pushes nothing
echo n | RELEASE_DATE=$FIXED_DATE sh scripts/release.sh >/dev/null 2>&1 && _fail "answer n accepted"
git ls-remote --tags origin | grep -q "$TAG" && _fail "tag pushed after answer n"
git rev-parse -q --verify "refs/tags/$TAG" >/dev/null && _fail "tag created after answer n"
echo "PASS: answer n aborts"

# A normal release pushes the branch and the tag
RELEASE_DATE=$FIXED_DATE sh scripts/release.sh -y >/dev/null || _fail "release failed"
git ls-remote --tags origin | grep -q "refs/tags/$TAG" || _fail "tag missing on the remote"
git ls-remote --heads origin | grep -q "refs/heads/main" || _fail "branch missing on the remote"
echo "PASS: release pushes branch and tag"

# The same minute again is refused
OUT=$(RELEASE_DATE=$FIXED_DATE sh scripts/release.sh -y 2>&1) && _fail "second release accepted"
echo "$OUT" | grep -q "already exists" || _fail "no 'already exists' message: $OUT"
echo "PASS: same version refused"

# A tag only on the remote is refused too
git tag -d "$TAG" >/dev/null
OUT=$(RELEASE_DATE=$FIXED_DATE sh scripts/release.sh -y 2>&1) && _fail "remote-only tag accepted"
echo "$OUT" | grep -q "already exists on" || _fail "no remote message: $OUT"
echo "PASS: tag existing only on the remote refused"

# Detached HEAD is refused
git checkout -q --detach
RELEASE_DATE="2026-10-21 15:31" sh scripts/release.sh -y >/dev/null 2>&1 && _fail "detached HEAD accepted"
echo "PASS: detached HEAD refused"
echo "All checks passed"
