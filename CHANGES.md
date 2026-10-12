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
  always fetches the latest GitHub release deb (amd64), verifies its
  SHA256 and installs it with `apt-get install`. `uninstall.sh` removes
  the package.
- `apps/graphics/vectorcraft`: VectorCraft vector art tool, same
  deb based install flow as PhotoCraft (latest GitHub release).

## 2026-10-12 - app icons

- dreamos-install: optional `icon.png` in an app folder is shown at
  the left of the app in the list (`App.icon`, `ICON_FILE`).
- Icons added for PhotoCraft and VectorCraft (from the ArtCraft site)
  and for the `demo-paint` example. Docs updated.

- `apps/video/filmcraft`: FilmCraft nonlinear video editor, same deb
  based install flow as PhotoCraft (latest GitHub release), new
  category "Video".

- `apps/graphics/lightcraft`: LightCraft photo library and raw
  developer, same deb based install flow as PhotoCraft (latest GitHub
  release).

- `apps/office/pdfcraft`: PdfCraft PDF reader and editor, same deb
  based install flow as PhotoCraft (latest GitHub release), new
  category "Office".

- `apps/video/effectcraft`: EffectCraft motion graphics and
  compositing tool, same deb based install flow as PhotoCraft (latest
  GitHub release).
