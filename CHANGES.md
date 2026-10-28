# Changes

## 2026-10-28 - Spacedrive added to the dreamos-install catalog

- New app `apps/internet/spacedrive`: installs the newest `v2` release of
  `Spacedrive-linux-x86_64.deb` from GitHub Releases with `apt`
  (v2.0.0-alpha.2, a pre-release), `"updatable": true`. The latest stable
  0.4.3 was not used: its deb needs ffmpeg 7 (`libavcodec.so.61`) without
  declaring it, and Ubuntu 26.04 only ships ffmpeg 8, so it cannot start. The
  script reads `releases?per_page=30` (`releases/latest` ignores pre-releases)
  and picks the first `v2*` deb, skipping nightly. The check is the dpkg status
  of `spacedrive`, uninstall is `apt-get remove`. Icon: the 128x128 icon of the
  package; screenshot supplied by the user (webp converted to a 1600px PNG).
  Checked: install, check, uninstall in a clean `ubuntu:26.04` container, no
  missing libraries. NOT checked: the app launching, on a real desktop, in the
  catalog UI.

## 2026-10-28 - rqbit added to the dreamos-install catalog

- New app `apps/internet/rqbit`: installs the latest `rqbit-desktop_*_amd64.deb`
  from GitHub Releases with `apt` (9.0.1), `"updatable": true` so re-running
  install updates it. The check is the dpkg status of `rqbit-desktop`,
  uninstall is `apt-get remove`. Icon: the 128x128 icon of the package;
  screenshot supplied by the user. Checked: install, check, uninstall in a
  clean `ubuntu:26.04` container, no missing libraries, the app starts under
  Xvfb. NOT checked: the app on a real desktop, in the catalog UI.

## 2026-10-21 - Update button in dreamos-install

- dreamos-install: new optional manifest key `updatable` (boolean, default
  `false`). For an installed app with `"updatable": true` the bottom bar
  shows a blue **Update** button next to **Uninstall**; it asks for
  confirmation and runs the install script again as root (`Action::Update`),
  and **View script** shows that script. Invalid values (not a boolean) skip
  the app with a warning, like any invalid manifest.
- Set `"updatable": true` in 35 catalog apps and in the `hello-tool` example:
  their `install.sh` downloads the latest release or runs `apt-get install`,
  so running it again updates the app. NOT set on `nvidia-driver` (Secure Boot
  and module signing steps, driver updates come with apt).
- Tests: `updatable_is_false_unless_the_manifest_says_true` (catalog),
  `update_reruns_the_install_script` (ui); `cargo test` 25 + 4 pass, clippy
  clean. Checked on screen (Xvfb, `ubuntu:26.04` container, example catalog):
  the blue Update button next to Uninstall for the installed Hello Tool.
  NOT checked: the click on Update (confirmation, run), the button on the
  real catalog apps, that every flagged install script really updates.

## 2026-10-21 - FileZilla added to the dreamos-install catalog

- New app `apps/internet/filezilla`: installs `filezilla` from the Ubuntu
  repositories with `apt` (3.69.6 on 26.04); the check is the dpkg status,
  uninstall is `apt-get remove`. FileZilla has no GitHub repository, so the
  manifest only has `homepage`. Icon: the 480x480 icon of the package scaled to
  128x128; screenshot taken from the package running in Xvfb. Checked: install,
  check, uninstall in a clean `ubuntu:26.04` container, no missing libraries.
  NOT checked: the app on a real desktop, in the catalog UI.

## 2026-10-21 - Telegram Desktop added to the dreamos-install catalog

- New app `apps/internet/telegram-desktop`: `telegram-desktop` is not in the
  Ubuntu 26.04 archive, so it installs the latest `td-setup-linux-x64`
  archive from GitHub Releases (SHA256 verified against the `digest` GitHub
  publishes, read with `jq`) into `/opt/telegram-desktop`, with
  `/usr/local/bin/telegram-desktop`, a menu entry (handles `tg://` links) and
  the pango, cairo, freetype and fontconfig libraries via `apt`. The app
  cannot update itself in `/opt`: re-run Install. Icon and screenshot from
  the project repository. Checked: install, reinstall, check, uninstall in a
  clean `ubuntu:26.04` container, no missing libraries (`ldd`). NOT checked:
  the app running (libraries it loads at run time, Wayland), in the catalog UI.

## 2026-10-21 - RustDesk added to the dreamos-install catalog

- New app `apps/internet/rustdesk`: installs the latest standard `x86_64` deb
  from GitHub Releases with `apt` (not the sciter, unattended-wayland or
  aarch64 builds); the check is the `rustdesk` dpkg status, uninstall is
  `apt-get remove`. Icon from the project `res/`, screenshot from its README.
  Checked: install, check, uninstall in a clean `ubuntu:26.04` container (no
  missing libraries). The deb has no published checksum, so only HTTPS
  protects the download. NOT checked: the app running, in the catalog UI.

## 2026-10-21 - dreamos-install .deb and apt repository

- New `scripts/build-deb.sh [--version V] [--skip-build]`: builds
  `dist/deb/dreamos-install_<version>_amd64.deb` with the binary
  (`/usr/bin`), the whole `apps/` catalog (`/usr/share/dreamos-install/apps`),
  the desktop entry and the icon (copies in `packaging/`, taken from
  dreamos-ubuntu). `Depends` comes from `dpkg-shlibdeps` plus `polkitd,
  pkexec`. Default version `0.1.<YYYYMMDD>.<HHMM>`.
- New `scripts/build-apt-repo.sh`: signed apt repository (`stable main`,
  last 5 versions kept) for GitHub Pages, key file `dreamos-tools.gpg`.
- New `scripts/release.sh [-y]`: tags `v0.1.<YYYYMMDD>.<HHMM>` and pushes
  branch and tag; refuses a dirty tree, a detached HEAD and an existing tag.
- New `.github/workflows/release-deb.yml`: on a `v*` tag builds the `.deb` in
  `ubuntu:26.04`, attaches it to the Release and publishes `gh-pages` with the
  secret `APT_GPG_PRIVATE_KEY`.
- Tests in `scripts/tests/`. Checked: `.deb` built and tested in a clean
  `ubuntu:26.04` container (33 manifests and 33 `install.sh`, no missing
  libraries, install over existing unowned files, removal); apt repository
  served over HTTP with a throwaway key (pruning to 5, `apt update`, install,
  `apt upgrade` to a newer version); `release.sh` against a temporary
  repository and remote; the workflow parses as YAML. NOT checked: the
  workflow on GitHub (never run), the real key and Pages, a real
  `release.sh` push, `apt upgrade` from the real repository, the program
  running from the installed package. dreamos-ubuntu is unchanged (its ISO
  keeps its own copy of binary and catalog).

## 2026-10-21 - Xodus Nexus added to the dreamos-install catalog

- New app `apps/games/xodus-nexus`: installs the latest `x86_64` AppImage
  from GitHub Releases (SHA512 verified) into `/opt/xodus-nexus`, with
  `/usr/local/bin/xodus-nexus` and a menu entry, plus the runtime libraries
  via `apt`. Icon from the project repository, screenshot from its docs.

## 2026-10-21 - Copy and Download in the script viewer

- dreamos-install: the **View script** window has a bottom bar with
  **Copy** (script text to the clipboard) and **Download** (save dialog
  proposing the script file name).

## 2026-10-21 - manifest.json for every dreamos-install app

- dreamos-install: each app folder now needs a `manifest.json` (name,
  icon, readme, check, install, uninstall, screenshot, github, homepage;
  paths relative to the manifest, defaults for the conventional file
  names). The app name comes from the manifest, not from the README.
  Invalid or missing manifests skip the app with a warning. New
  dependencies: `serde`, `serde_json`.
- dreamos-install: new **Github Page** and **Home page** buttons next to
  **View script**, shown when the manifest defines `github` / `homepage`
  (opened in the default browser).
- Added `manifest.json` to the 32 catalog apps and to the example apps.
  Github / home links come from the app READMEs and upstream projects;
  apps without a GitHub repository only have `homepage`.

## 2026-10-17 - RetroArch added to the dreamos-install catalog

- New app `apps/games/retroarch`: `apt-get install retroarch`, with the
  official `ppa:libretro/stable` added first on Ubuntu (no Snap, no
  Flatpak). Icon and screenshot from the Flathub listing.

## 2026-10-17 - Screenshots for Steam and Heroic

- Added `screenshot.png` (from the Flathub listings) to `apps/games/steam`
  and `apps/games/heroic` and referenced it in their READMEs.

## 2026-10-17 - Heroic Games Launcher added to the dreamos-install catalog

- New app `apps/games/heroic`: downloads the latest `linux-amd64.deb`
  from the GitHub releases and installs it with `apt-get install`
  (the release publishes no checksum file, so none is verified).
  Icon from the project repository.

## 2026-10-17 - Steam added to the dreamos-install catalog

- New app `apps/games/steam` (new `games` category): enables the `i386`
  architecture, then `multiverse` + `steam` on Ubuntu or
  `contrib`/`non-free` + `steam-installer` on Debian. Steam logo icon.
  `check.sh` and `uninstall.sh` handle both package names.

## 2026-10-15 - dreamos-install asks for root only on Install

- The app now starts as a normal user. The administrator password is
  asked by `pkexec` only when an install or uninstall script runs
  (`elevate::script_argv`, unit tested), not at startup. `check.sh`
  runs unprivileged. Removed the startup re-exec and the GTK look
  forwarding, no longer needed.

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
