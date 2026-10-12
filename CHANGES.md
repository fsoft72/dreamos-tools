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

- `apps/graphics/designcraft` (page layout editor) and
  `apps/office/wordcraft` (word processor, reads/writes .docx): same
  deb based install flow as PhotoCraft (latest GitHub release).

- `apps/audio/soundcraft`: SoundCraft digital audio workstation, same
  deb based install flow as PhotoCraft (latest GitHub release), new
  category "Audio".

- `apps/office/deckcraft`: DeckCraft presentation app, same deb based
  install flow as PhotoCraft (latest GitHub release).

- `apps/design/cadcraft`: CadCraft CAD and drafting app, same deb
  based install flow as PhotoCraft (latest GitHub release), new
  category "Design".

- `apps/office/gridcraft`: GridCraft spreadsheet app (.xlsx), same deb
  based install flow as PhotoCraft (latest GitHub release).

## 2026-10-12 - per-user installs, Godot

- dreamos-install: scripts (check, install, uninstall) now get
  `DREAMOS_USER`, the login name of the invoking user (from
  `PKEXEC_UID` / `SUDO_UID`), so they can install into the user's
  home while running as root. Docs updated.
- `apps/development/godot`: Godot Engine editor (single file in a
  zip, latest stable GitHub release, SHA512 verified) installed in the
  user's `~/.local` (`opt/godot`, `bin/godot`, menu entry, icon), new
  category "Development".
