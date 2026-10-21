# dreamos-install

GTK4 browser for a folder based catalog of installable apps. Shows
categories, apps and their Markdown description, tells whether each app
is installed (via its `check.sh`) and runs `install.sh` / `uninstall.sh`
in an embedded VTE terminal. See
[the design spec](../docs/superpowers/specs/2026-10-12-dreamos-install-design.md)
and the [user documentation](../docs/dreamos-install.md).

## Build

    cargo build --release

Output: `target/release/dreamos-install`.

## Run

Runs as a normal user and asks for the administrator password (via
`pkexec`) only when Install or Uninstall is confirmed:

    ./target/release/dreamos-install [catalog-dir]

Catalog folder: first argument, else `$DREAMOS_INSTALL_DIR`, else
`/usr/share/dreamos-install/apps`.

## Install with apt

On Ubuntu 26.04 or later (amd64), the signed apt repository on GitHub Pages
installs the program together with the catalog, and `apt upgrade` keeps both
up to date:

    sudo install -d -m 0755 /etc/apt/keyrings
    sudo curl -fsSL https://fsoft72.github.io/dreamos-tools/dreamos-tools.gpg -o /etc/apt/keyrings/dreamos-tools.gpg
    echo "deb [signed-by=/etc/apt/keyrings/dreamos-tools.gpg] https://fsoft72.github.io/dreamos-tools stable main" | sudo tee /etc/apt/sources.list.d/dreamos-tools.list
    sudo apt update
    sudo apt install dreamos-install

## Releasing

`scripts/release.sh` tags the current commit `v0.1.<YYYYMMDD>.<HHMM>` and
pushes the branch and the tag. The tag starts
`.github/workflows/release-deb.yml`, which builds the `.deb` in an
`ubuntu:26.04` container (`scripts/build-deb.sh`), attaches it to the GitHub
Release and publishes the apt repository on the `gh-pages` branch
(`scripts/build-apt-repo.sh`, signed with the secret `APT_GPG_PRIVATE_KEY`).
One time setup: set that secret and enable GitHub Pages on `gh-pages`.

To build the package by hand: `scripts/build-deb.sh [--version V]`, output in
`dist/deb/`.

## Runtime dependencies

- GTK 4 and the GTK 4 build of VTE (`libvte-2.91-gtk4-0`, >= 0.78).
- GtkSourceView 5 (`libgtksourceview-5-0`) for the script viewer. Build
  needs `libgtksourceview-5-dev`.
- `bash` - every catalog script is run with it.
- `pkexec` (polkit) - privilege elevation.

## Tests

- Unit tests (no root needed): `cargo test`
- UI on the example catalog, without root:
  `./scripts/run-dreamos-install.sh` - the example apps only touch
  `/tmp/dreamos-install-demo`, so install/uninstall work unprivileged.
  Covered by the examples: installed / not installed apps, uninstall,
  a failing install, a slow check (shows "Checking...").
- Packaging scripts, in a throwaway `ubuntu:26.04` container for the apt one:
  `sh scripts/tests/test-release.sh`,
  `sh scripts/tests/test-deb.sh <deb> apps`,
  `docker run --rm -v "$PWD":/src:ro ubuntu:26.04 sh /src/scripts/tests/test-apt-repo.sh`.
