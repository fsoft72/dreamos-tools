# dreamos-install .deb and apt repository Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship `dreamos-install` (binary + app catalog) as a `.deb` served by a signed apt repository on GitHub Pages, so Ubuntu 26.04+ machines update it with `apt upgrade`.

**Architecture:** Copy the proven opusdm scheme (`~/dev/projects/opusdm`: `scripts/build-deb.sh`, `scripts/build-apt-repo.sh`, `scripts/release.sh`, `.github/workflows/release-deb.yml`) and adapt it to the `dreamos-install` crate. A `v0.1.YYYYMMDD.HHMM` tag starts the workflow, which builds in an `ubuntu:26.04` container, signs with `APT_GPG_PRIVATE_KEY` and force-pushes one orphan commit to `gh-pages`.

**Tech Stack:** POSIX sh, dpkg-deb, dpkg-shlibdeps, apt-ftparchive, gpg, GitHub Actions, Docker (local verification), cargo (Rust, crate `dreamos-install`).

**Spec:** the design agreed in chat on 2026-10-21 (no written spec); summarized in Global Constraints.

## Global Constraints

- Package name `dreamos-install`; architecture `amd64` only; Ubuntu 26.04 and later.
- One package holds binary and catalog: `/usr/bin/dreamos-install`, `/usr/share/dreamos-install/apps/` (copy of `apps/`, the `DEFAULT_ROOT` of `dreamos-install/src/catalog.rs`), `/usr/share/applications/dreamos-install.desktop`, `/usr/share/icons/hicolor/256x256/apps/dreamos-installer.png`.
- `Depends` = `dpkg-shlibdeps` output plus `polkitd, pkexec`; refuse to build when shlibdeps is empty (as opusdm).
- Version `0.1.YYYYMMDD.HHMM`, computed by `scripts/release.sh`, tag `v<version>`; the workflow takes the version from the tag, `Cargo.toml` is NOT touched.
- Apt repo at `https://fsoft72.github.io/dreamos-tools`, suite `stable`, component `main`, public key file `dreamos-tools.gpg`, keep the last 5 versions in the pool.
- Secret `APT_GPG_PRIVATE_KEY` (armored secret key, no passphrase) already set by the user; never commit a private key.
- dreamos-ubuntu is NOT modified (the ISO keeps its own staging of binary and catalog).
- Per the user's rules: English in code and comments, no em dash, constants UPPERCASE, `_` prefix for internal shell functions, every function commented, exit conditions at the top, files end with an empty line, update `CHANGES.md`, local commit only (no push), never `--no-verify`.
- Shell style: `#!/bin/sh`, `set -eu`, same as the opusdm scripts.

## Review Focus

- Catalog folder names with spaces or odd characters must survive the copy and keep their modes (install.sh and check.sh stay executable).
- A second release the same minute (same tag) must be refused by `release.sh`, not overwrite.
- `build-apt-repo.sh` run over an existing `gh-pages` checkout with an older `.deb` must prune to 5 and keep `apt upgrade` working (old to new version ordering with the 4-part version).
- Installing the `.deb` on top of an ISO system whose files are already in the same paths (unowned files) must not fail.
- Removing the package must remove `/usr/share/dreamos-install` entirely and refresh icon and desktop caches without erroring when the tools are missing.

---

### Task 1: build-deb.sh

**Files:**
- Create: `packaging/dreamos-install.desktop` (copy of `~/dev/projects/dreamos-ubuntu/config/includes.chroot/usr/share/applications/dreamos-install.desktop`)
- Create: `packaging/dreamos-installer.png` (copy of the 256x256 icon in dreamos-ubuntu `config/includes.chroot/usr/share/icons/hicolor/256x256/apps/`)
- Create: `scripts/build-deb.sh`
- Create: `scripts/tests/test-deb.sh`
- Modify: `.gitignore` (add `/dist/`)

**Interfaces:**
- Produces: `scripts/build-deb.sh [--version V] [--skip-build]` writes `dist/deb/dreamos-install_<V>_<arch>.deb`. Without `--version`, V is `0.1.$(date +%Y%m%d.%H%M)`. `--skip-build` reuses `dreamos-install/target/release/dreamos-install`. Prints `Done: <path>` last.
- Produces: `scripts/tests/test-deb.sh <deb>` exits 0 only when every check below passes; it is run inside an `ubuntu:26.04` container.

- [ ] **Step 1: Write the failing test.** `scripts/tests/test-deb.sh <deb>` asserts: `dpkg-deb -f Package` is `dreamos-install`, `Architecture` is `amd64`, `Depends` contains `libgtk-4-1`, `libvte-2.91-gtk4-0`, `pkexec`; `dpkg-deb -c` lists the four paths of the Global Constraints, `usr/share/dreamos-install/apps/` has the same number of `manifest.json` files as `apps/` in the repo (passed as `$2`), and every `install.sh` in the package is executable (`-rwxr-xr-x`).
- [ ] **Step 2: Run it against a missing deb to see it fail.** Run: `sh scripts/tests/test-deb.sh dist/deb/none.deb apps` Expected: FAIL (file not found).
- [ ] **Step 3: Implement `scripts/build-deb.sh`** following opusdm's `build-deb.sh`: option parsing with the `--version` / `--skip-build` flags, tool checks (`dpkg-deb dpkg-shlibdeps strip cargo`), `cargo build --release` in `dreamos-install/` unless `--skip-build`, staging under `dist/deb/<name>/`, `strip --strip-unneeded` on the staged binary only, `cp -a apps/. $ROOT/usr/share/dreamos-install/apps/`, shlibdeps in a scratch `debian/control`, `Depends: $SHLIBS, polkitd, pkexec`, `Homepage: https://github.com/fsoft72/dreamos-tools`, `Section: admin`, postinst/postrm refreshing icon cache and desktop database (`|| true`), `chmod -R u=rwX,go=rX` (this keeps the executable bits already set), `dpkg-deb --root-owner-group --build`.
- [ ] **Step 4: Build and test in a clean container.** Run: `docker run --rm -v "$PWD":/src -w /src ubuntu:26.04 sh -c 'apt-get update -qq && apt-get install -y -qq build-essential pkg-config dpkg-dev file curl ca-certificates libgtk-4-dev libvte-2.91-gtk4-dev libgtksourceview-5-dev && curl -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal && . ~/.cargo/env && scripts/build-deb.sh --version 0.1.20261021.1200 && sh scripts/tests/test-deb.sh dist/deb/dreamos-install_0.1.20261021.1200_amd64.deb apps'` Expected: `Done: ...` then all checks PASS. If a `-dev` package name is wrong for 26.04, fix it in this command and in Task 4.
- [ ] **Step 5: Install and remove check (same container, covers Review Focus 4 and 5).** Pre-create `/usr/share/dreamos-install/apps/x` and `/usr/bin/dreamos-install` as plain files, `apt-get install -y ./dist/deb/*.deb` must succeed, `dreamos-install --help` or `ldd /usr/bin/dreamos-install` shows no missing library, then `apt-get remove -y dreamos-install` leaves no `/usr/share/dreamos-install`.
- [ ] **Step 6: Commit.** `git add packaging scripts/build-deb.sh scripts/tests/test-deb.sh .gitignore && git commit -m "feat(deb): add build-deb.sh for dreamos-install"`

### Task 2: build-apt-repo.sh

**Files:**
- Create: `scripts/build-apt-repo.sh`
- Create: `scripts/tests/test-apt-repo.sh`

**Interfaces:**
- Consumes: the `.deb` from Task 1 (the test builds two fake versions of it with `dpkg-deb` from a tiny staged tree, so it does not need cargo).
- Produces: `GPG_KEY_ID=<id> BASE_URL=<url> scripts/build-apt-repo.sh <repo dir> <deb>...` creates `pool/main/d/dreamos-install/*.deb`, `dists/stable/main/binary-<arch>/Packages{,.gz}`, `dists/stable/{Release,InRelease,Release.gpg}`, `dreamos-tools.gpg`, `index.html`, `.nojekyll`. Same behaviour as opusdm's script with these renames: pool path, `Origin`/`Label` `DreamOS Tools`, key file, index text (package `dreamos-install`, line `deb [signed-by=/etc/apt/keyrings/dreamos-tools.gpg] $BASE_URL stable main`), default `BASE_URL=https://fsoft72.github.io/dreamos-tools`.

- [ ] **Step 1: Write the failing test `scripts/tests/test-apt-repo.sh`.** In a container with `apt-utils gnupg dpkg-dev python3`: generate a throwaway passphrase-less key, build fake packages `dreamos-install` versions `0.1.20261021.1200`, `0.1.20261021.1201` and six more up to `.1207`, run the script once per package into the same dir, then assert: pool holds exactly 5 `.deb` (newest 5), `InRelease` verifies with the exported key, the repo is served by `python3 -m http.server`, `apt-get update` with the `signed-by` source succeeds, `apt-get install dreamos-install` installs `.1207`, and adding a `.1208` and running `apt-get upgrade` upgrades it (covers Review Focus 3).
- [ ] **Step 2: Run it to see it fail.** Run the test in `ubuntu:26.04` (`docker run --rm -v "$PWD":/src -w /src ubuntu:26.04 sh scripts/tests/test-apt-repo.sh`). Expected: FAIL, `scripts/build-apt-repo.sh` missing.
- [ ] **Step 3: Implement `scripts/build-apt-repo.sh`** by copying opusdm's with the renames above.
- [ ] **Step 4: Run the test again.** Expected: all assertions PASS.
- [ ] **Step 5: Commit.** `git add scripts/build-apt-repo.sh scripts/tests/test-apt-repo.sh && git commit -m "feat(deb): add build-apt-repo.sh"`

### Task 3: release.sh

**Files:**
- Create: `scripts/release.sh`
- Create: `scripts/tests/test-release.sh`

**Interfaces:**
- Produces: `scripts/release.sh [-y]`. Computes `VERSION=0.1.$(date +%Y%m%d.%H%M)`, `TAG=v$VERSION`; message `DreamOS Tools dreamos-install $VERSION`. Environment override `RELEASE_DATE` (a string for `date -d`) exists only so the test can force a collision. Same guards and prompt as opusdm's `release.sh` (detached HEAD, dirty tree, tag exists locally or on the remote, remote missing); pushes the branch, then the tag.

- [ ] **Step 1: Write the failing test `scripts/tests/test-release.sh`.** Build a throwaway git repo plus a bare "remote" in `mktemp -d`, copy the script in, and check: clean tree with `-y` creates tag `v0.1.<date>` on the remote; a second run in the same minute (`RELEASE_DATE` fixed) fails with "already exists"; a dirty tree fails; detached HEAD fails; answering `n` aborts without pushing. The real repo is never pushed to.
- [ ] **Step 2: Run to see it fail.** Run: `sh scripts/tests/test-release.sh` Expected: FAIL, script missing.
- [ ] **Step 3: Implement `scripts/release.sh`** from opusdm's, replacing the Cargo.toml version read with the date version.
- [ ] **Step 4: Run the test.** Expected: PASS.
- [ ] **Step 5: Commit.** `git add scripts/release.sh scripts/tests/test-release.sh && git commit -m "feat(deb): add release.sh"`

### Task 4: GitHub workflow

**Files:**
- Create: `.github/workflows/release-deb.yml`

**Interfaces:**
- Consumes: `scripts/build-deb.sh --version`, `scripts/build-apt-repo.sh`, secret `APT_GPG_PRIVATE_KEY`.

- [ ] **Step 1: Write `.github/workflows/release-deb.yml`** from opusdm's: trigger on tag `v*` and `workflow_dispatch`; job `deb` in `ubuntu:26.04` with the build dependencies verified in Task 1 Step 4 (plus `git`, `curl`), Rust via rustup, a step that sets `VERSION="${GITHUB_REF_NAME#v}"` for tags (on `workflow_dispatch` there is no tag: omit `--version` and the script uses the date), runs `scripts/build-deb.sh --version "$VERSION"`, uploads the artifact `dreamos-install-deb` from `dist/deb/*.deb`, attaches it to the Release on tags; job `apt-repo` identical to opusdm's with `BASE_URL` from the repository name and the artifact name above. Remove opusdm's "tag matches Cargo.toml" step.
- [ ] **Step 2: Validate the YAML.** Run: `python3 -c "import yaml,sys; yaml.safe_load(open('.github/workflows/release-deb.yml'))"` Expected: no output, exit 0.
- [ ] **Step 3: Commit.** `git add .github/workflows/release-deb.yml && git commit -m "ci(deb): add release-deb workflow"`

### Task 5: Docs and changelog

**Files:**
- Modify: `dreamos-install/README.md` (new "Install with apt" section: the 4 shell lines from `index.html`, plus "Releasing": run `scripts/release.sh`)
- Modify: `CHANGES.md` (new top entry dated 2026-10-21)

- [ ] **Step 1: Edit both files.** CHANGES entry follows the existing style and lists, honestly, what was checked (Task 1 to 3 container runs) and NOT checked (the workflow on GitHub, the real key and Pages, a real `release.sh` push, `apt upgrade` from the real repo).
- [ ] **Step 2: Commit.** `git add dreamos-install/README.md CHANGES.md && git commit -m "docs(deb): document the apt repository and releases"`
