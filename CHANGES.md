# Changes

## 2026-10-12 - dreamos-install

- New tool `dreamos-install/`: GTK4 browser for a folder based app
  catalog (categories -> apps -> Markdown description), install status
  via each app's `check.sh`, install/uninstall scripts run in an
  embedded VTE terminal.
- Example catalog in `dreamos-install/examples/apps/` and
  `scripts/run-dreamos-install.sh` for UI iteration without root.
- Docs: design spec
  (`docs/superpowers/specs/2026-10-12-dreamos-install-design.md`) and
  user documentation (`docs/dreamos-install.md`).

## 2026-10-12 - apps catalog

- New `apps/` catalog folder for dreamos-install.
- `apps/graphics/photocraft`: PhotoCraft image editor. `install.sh`
  always fetches the latest GitHub release AppImage (x86_64), verifies
  its SHA256, installs it in `/opt/photocraft` with a `photocraft`
  launcher and a desktop entry. `uninstall.sh` removes it.
