# dreamos-install - design spec

Date: 2026-10-12
Repo: `dreamos-tools` (this repo), folder `dreamos-install/`
Related: [`docs/dreamos.md`](../../dreamos.md) - dreamos is a Debian 13
"trixie" live ISO; this tool targets that base only.

## Problem

dreamos users need a simple, curated way to install extra software
(development tools, graphics apps, system utilities...). Each piece of
software may need a custom install procedure (apt, extra repos, a
downloaded .deb, flatpak...). `dreamos-install` is a local, folder
driven "app store": every app is a folder with a Markdown description,
an install script and a check script. The tool browses that catalog,
shows the description, tells whether the app is already installed and
runs the install script inside an embedded terminal so the user sees
live output.

## Goals

- Scan a root folder of categories, each containing app folders.
- Show categories, the apps of the selected category, and the Markdown
  description (with images) of the selected app.
- Know whether each app is installed by running its `check.sh`.
- Run `install.sh` (and the optional `uninstall.sh`) in an embedded VTE
  terminal, then re-check the app status.

## Non-goals (v1)

- No search, no remote catalog download, no install queue (one script
  at a time).
- No sandboxing of the scripts: the catalog is trusted content shipped
  with the OS (the scripts run as root).
- No full Markdown/HTML rendering: tables, raw HTML and remote images
  are not rendered (remote images show their alt text).

## Catalog format

```
<root>/
  graphics/                -> category "Graphics"
    gimp/
      README.md            -> required
      install.sh           -> required
      check.sh             -> required
      uninstall.sh         -> optional
      screenshot.png       -> referenced from README.md
```

- Category display name = folder name with the first letter
  capitalized. Categories are sorted alphabetically; apps are sorted by
  display name (case-insensitive). Hidden entries (starting with `.`)
  are ignored.
- App metadata comes from `README.md` by convention:
  - name = text of the first level-1 heading (`# Name`); fallback: the
    folder name.
  - summary = plain text of the first paragraph; fallback: empty.
  - screenshot = first image with a local path; fallback: none.
- An app folder missing any required file is skipped with a warning on
  stderr. A category without valid apps is not shown.
- Scripts are run with `bash <script>` (no executable bit needed), with
  the app folder as working directory and `DREAMOS_APP_DIR` set to it.
- `check.sh`: exit code 0 = installed, any other exit code = not
  installed. Timeout 10 seconds; a timeout or a spawn error is reported
  as "check failed".

## Root folder resolution

1. First command line argument.
2. `DREAMOS_INSTALL_DIR` environment variable.
3. Default `/usr/share/dreamos-install/apps`.

The resolved path is made absolute and forwarded as an argument to the
`pkexec` re-exec, because `pkexec` drops the environment.

## Privileges

The binary runs as a normal user. Only `install.sh` / `uninstall.sh` are
elevated: when the user confirms the action they are run through
`pkexec` (polkit asks for the password at that moment). `check.sh` runs
unprivileged. `DREAMOS_INSTALL_SKIP_ROOT=1` skips `pkexec` for UI
iteration (`scripts/run-dreamos-install.sh`).

## Architecture (crate `dreamos-install/`)

- `catalog.rs` - scans the root and builds
  `Vec<Category { name, dir, apps: Vec<App> }>`; an `App` holds `dir`,
  `name`, `summary`, `screenshot`, `has_uninstall`. No GTK, unit tested.
- `markdown.rs` - parses Markdown with `pulldown-cmark` into
  `Vec<Block>` (`Heading(level, markup)`, `Paragraph(markup)`,
  `ListItem(depth, markup)`, `Code(text)`, `Image { src, alt }`,
  `Rule`), where `markup` is escaped Pango markup (bold, italic,
  inline code, links). Also extracts the metadata above. No GTK, unit
  tested.
- `status.rs` - runs `check.sh` with a timeout and returns
  `Installed` / `NotInstalled` / `CheckFailed(reason)`. No GTK, unit
  tested with fake scripts.
- `render.rs` - turns `Vec<Block>` into GTK widgets: `Label`s with
  markup, `gtk::Picture` for local images (resolved relative to the app
  folder), alt text for remote or missing images.
- `ui.rs` - main window, lists, detail pane, terminal pane.
- `main.rs` - root resolution, pkexec re-exec, app start.

## UI

Single window, header with the `ball.png` logo and the title
"DreamOS Install" (no Welcome page: this is a browser, not a wizard).

```
+ (logo) DreamOS Install ---------------------------------------+
| Categories  | Apps                    | Detail (Markdown)      |
| Development | GIMP        Installed   |  # GIMP ...            |
| Graphics  < | Inkscape    Not inst.   |  [screenshot]          |
| System      | Krita       Checking... |                        |
+-------------+-------------------------+------------------------+
|                                    [Uninstall]  [Install]      |
+---------------------------------------------------------------+
```

- Selecting a category lists its apps and starts the check of every app
  not checked yet, in background threads; results are cached per app
  and shown as a status label in the list and in the detail pane.
- Action bar at the bottom:
  - **Install**: visible when the app is not installed (or the check
    failed).
  - **Uninstall**: visible when the app is installed and has
    `uninstall.sh`.
  - Both ask for confirmation first.
- Running a script swaps the detail pane for a VTE terminal running
  `bash install.sh` (or `uninstall.sh`). While it runs, category list,
  app list and actions are disabled. When the child exits, the terminal
  and the action bar show the outcome ("Completed successfully" /
  "Failed (exit code N)") plus a **Back** button; the app is re-checked.
- Closing the window while a script runs asks for confirmation.

## Dependencies

`gtk4 0.10` (`v4_10`), `vte4 0.9` (VTE >= 0.78, dreamos ships the gtk4
VTE libs), `pulldown-cmark`, `libc`, `thiserror`.

## Testing

- Unit tests: catalog scan on a temp dir (valid/invalid apps, sorting,
  hidden entries, empty categories), Markdown metadata extraction and
  block conversion (escaping, inline styles, images), status with fake
  check scripts (exit 0, exit 1, timeout).
- Example catalog in `dreamos-install/examples/apps/` with harmless
  apps (e.g. an app whose install script only touches a file under
  `/tmp`) to exercise the UI end to end.
- Manual: `./scripts/run-dreamos-install.sh` runs the UI on the example
  catalog without root.
