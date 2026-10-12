# DreamOS Install - user documentation

`dreamos-install` is a small, local "app store" for dreamos: it browses
a folder of curated apps, shows their description and screenshot,
tells whether each one is already installed and installs (or
uninstalls) it while you watch the output in an embedded terminal.

Source: `dreamos-install/` in this repo (`dreamos-tools`). Design
rationale: [`docs/superpowers/specs/2026-10-12-dreamos-install-design.md`](superpowers/specs/2026-10-12-dreamos-install-design.md).
Developer-facing build/test notes: [`dreamos-install/README.md`](../dreamos-install/README.md).

## Using it

1. Start `dreamos-install` (it asks for the administrator password via
   polkit, because install scripts run as root).
2. Pick a category on the left; the apps of that category appear in the
   middle column with their install status (`Checking...`,
   `Installed`, `Not installed`, `Check failed`).
3. Click an app to read its description on the right.
4. Optionally click **View script** to read, in a modal window with
   syntax highlighting (Esc closes it), the script the next button
   would run.
   Then click **Install** (shown when the app is not installed) or
   **Uninstall** (shown when it is installed and provides an uninstall
   script) and confirm. The script runs in the terminal that replaces
   the description; navigation is locked until it ends.
5. When it ends, the bottom bar shows the outcome and the new status
   (the app is checked again). Click **Back** to return to the
   description.

Closing the window while a script runs asks for confirmation, because
it interrupts the script.

## Catalog folder

The catalog is read from, in order: the first command line argument,
the `DREAMOS_INSTALL_DIR` environment variable, or
`/usr/share/dreamos-install/apps`.

```
apps/
  graphics/              -> category "Graphics"
    gimp/                -> one app
      README.md          (required)
      install.sh         (required)
      check.sh           (required)
      uninstall.sh       (optional)
      icon.png           (optional, shown next to the app in the list)
      screenshot.png     (any file the README references)
```

- Category name = folder name with a capital first letter. Categories
  with no valid app are hidden. Folders starting with `.` are ignored.
- An app folder missing `README.md`, `install.sh` or `check.sh` is
  skipped (a warning is printed on stderr).

### icon.png

Optional square PNG (128x128 is plenty), shown at 40 px at the left of
the app in the list. Apps without it show no icon.

### README.md

Plain Markdown. By convention:

- the first `# Heading` is the app name (fallback: folder name);
- the first paragraph is the short description shown in the list;
- images use paths relative to the app folder, e.g.
  `![Main window](screenshot.png)`. Remote images (`https://...`) are
  not downloaded: their alt text is shown instead.

Supported: headings, paragraphs, bold, italic, inline code, code
blocks, bulleted/numbered lists (nested too), links, images, horizontal
rules. Tables and raw HTML are not rendered.

### Scripts

- All scripts are run as `bash <script>` (no executable bit needed),
  as root, with the app folder as working directory and
  `DREAMOS_APP_DIR` set to its absolute path.
- `DREAMOS_USER` holds the login name of the user who started the tool
  (resolved from `PKEXEC_UID`, then `SUDO_UID`, then the current
  user). Use it for per-user installs, e.g. under `~/.local`: look up
  the home with `getent passwd "$DREAMOS_USER"` and drop privileges
  with `runuser -u "$DREAMOS_USER" -- ...`. See `apps/development/godot`.
- `check.sh` must exit `0` when the app is installed and non-zero
  otherwise. It should be quick and silent: it runs for every app of a
  category when the category is opened, and is stopped after 10
  seconds (status `Check failed`).
- `install.sh` / `uninstall.sh` run in a real terminal, so their output
  (colors included) is shown live and they can ask questions; prefer
  non-interactive commands (`apt-get install -y ...`). A non-zero exit
  code is reported as a failure.

Example `check.sh` for a Debian package:

```
#!/usr/bin/env bash
dpkg-query -W -f='${Status}' gimp 2>/dev/null | grep -q "install ok installed"
```

## Try it

`./scripts/run-dreamos-install.sh` builds the tool and opens it on the
example catalog in `dreamos-install/examples/apps` without asking for
root. The example apps only create marker files under
`/tmp/dreamos-install-demo`.
