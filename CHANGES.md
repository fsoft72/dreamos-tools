# Changes

## 2026-10-13 - nvidia-installer becomes a dreamos-install app

- New app `apps/drivers/nvidia-driver`: `install.sh` detects the GPU
  (`lspci`), picks the package with `nvidia-detect` (override with
  `NVIDIA_PACKAGE`), sets up the DKMS signing key and `mokutil --import`
  when Secure Boot is on, then runs `apt-get install`. `check.sh` and
  `uninstall.sh` complete it. Parsing functions are covered by
  `tests/test.sh` (fixtures moved from the old crate).
- Removed the `nvidia-installer/` GTK wizard, its run script, user doc,
  design spec and plan.

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

- `apps/design/figcraft`: FigCraft interface design app, same deb based
  install flow as PhotoCraft (latest GitHub release).

## 2026-10-12 - per-user installs, Godot

- dreamos-install: scripts (check, install, uninstall) now get
  `DREAMOS_USER`, the login name of the invoking user (from
  `PKEXEC_UID` / `SUDO_UID`), so they can install into the user's
  home while running as root. Docs updated.
- `apps/development/godot`: Godot Engine editor (single file in a
  zip, latest stable GitHub release, SHA512 verified) installed in the
  user's `~/.local` (`opt/godot`, `bin/godot`, menu entry, icon), new
  category "Development".

## 2026-10-12 - View script

- dreamos-install: new **View script** button next to Install /
  Uninstall. Opens a modal read-only window (GtkSourceView 5, shell
  highlighting, line numbers, light/dark scheme) with the script that
  will run. New dependency `sourceview5` (build: `libgtksourceview-5-dev`).

- `apps/graphics/blender`: Blender, latest stable Linux x64 tarball
  from download.blender.org (SHA256 verified), installed in the user's
  `~/.local` like Godot.

## 2026-10-12 - more apps

- apt based (distribution repositories): `graphics/krita`,
  `graphics/inkscape`, `development/tiled`, `video/obs-studio`,
  `video/handbrake`.
- Latest deb from the vendor: `graphics/blockbench` (GitHub release),
  `development/dbeaver` (dbeaver.io, package `dbeaver-ce`). No
  checksums are published, HTTPS only.
- `graphics/libresprite`: no deb exists, latest AppImage from GitHub
  installed in the user's `~/.local` (installs `libfuse2` when missing).

## 2026-10-12 - search

- dreamos-install: search box in the header (Ctrl+F) that filters apps
  of all categories by name, case insensitive substring. Matching
  logic is `catalog::search`, with a unit test. Esc, an empty box or
  picking a category leaves the search mode.

- New category "Internet": `apps/internet/chrome` (latest stable deb
  from Google), `brave-origin` (official Brave apt repo), `opera`
  (official Opera apt repo) and `firefox` (official Mozilla apt repo,
  pinned over Ubuntu's snap stub; its `check.sh` requires the real
  `/usr/lib/firefox/firefox`).

- `apps/internet/firefox`: `install.sh` now removes the Firefox snap
  (after the deb is installed; snap keeps a data snapshot).

- `apps/office/libreoffice`: LibreOffice from The Document Foundation:
  latest stable deb bundle (SHA256 verified) plus the language pack of
  the system language when it exists, installed with `apt-get`.
  Uninstall removes every official versioned release.

## 2026-10-12 - pkexec display fix

- dreamos-install: the pkexec re-exec now goes through `env` and
  forwards `DISPLAY`, `XAUTHORITY`, `WAYLAND_DISPLAY`,
  `XDG_RUNTIME_DIR`, `XDG_SESSION_TYPE`, `GDK_BACKEND` and
  `DBUS_SESSION_BUS_ADDRESS` (`elevate::pkexec_argv`, unit tested).
  Fixes "Failed to open display" after the password prompt, because
  pkexec drops the environment.

- dreamos-install: before the pkexec re-exec the user's GTK look
  (theme, dark preference, icon theme, font, cursor) is read and
  forwarded as `DREAMOS_GTK_*` variables; the elevated process applies
  it to its GtkSettings. Fixes dark / light mode being ignored when
  running as root.
