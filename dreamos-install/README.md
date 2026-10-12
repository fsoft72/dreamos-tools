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

Must run as root (it self-elevates via `pkexec` if not already root):

    ./target/release/dreamos-install [catalog-dir]

Catalog folder: first argument, else `$DREAMOS_INSTALL_DIR`, else
`/usr/share/dreamos-install/apps`.

## Runtime dependencies

- GTK 4 and the GTK 4 build of VTE (`libvte-2.91-gtk4-0`, >= 0.78).
- `bash` - every catalog script is run with it.
- `pkexec` (polkit) - privilege elevation.

## Tests

- Unit tests (no root needed): `cargo test`
- UI on the example catalog, without root:
  `./scripts/run-dreamos-install.sh` - the example apps only touch
  `/tmp/dreamos-install-demo`, so install/uninstall work unprivileged.
  Covered by the examples: installed / not installed apps, uninstall,
  a failing install, a slow check (shows "Checking...").
