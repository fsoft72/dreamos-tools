# dreamos - how it works

Source: `~/dev/projects/dreamos` (repo `fsoft72/dreamos`, not a git repo
in this tool's context, checked live).

## What it is

Debian 13 "trixie" live ISO. Desktop = OpusDM (`opusdm-hub` +
`opusdm-lister`, custom Rust apps) on Openbox. Built with Debian
`live-build`, always inside Docker (never touches Ubuntu host directly).

## Build pipeline

1. **`scripts/build-opusdm.sh`** - compiles OpusDM (`opusdm-hub`,
   `opusdm-lister`, discovered via `cargo metadata` on the OpusDM
   workspace, default src `/home/fabio/dev/projects/opusdm`, override
   `OPUSDM_SRC`) in a throwaway `debian:trixie` container. Must run
   first and after every OpusDM change (Ubuntu host glibc is newer than
   trixie's -> host-built bins won't start on the ISO).
   - Stages binaries into `config/includes.chroot/usr/bin/`.
   - Stages OpusDM user config (`~/.config/opusdm`, theme incl.) and
     background into `config/includes.chroot/etc/skel/` so live user
     gets it via `/etc/skel` at first login. Rewrites the background
     `image_path` in `settings.json` to the live user's home.
   - Writes `vendor/opusdm/opusdm-bin.tar.gz` (reproducible: fixed
     order/owner/mtime, `gzip -n`) - tracked fallback for envs without
     OpusDM sources (fresh checkout, CI).

2. **`build.sh`** - builds `dreamos-lb` Docker image (Dockerfile:
   `debian:trixie` pinned by digest + `live-build`, `debootstrap`,
   `squashfs-tools`, `xorriso`, syslinux/grub-efi/isolinux, etc), then
   runs `lb build` inside `--privileged` container (needed for
   debootstrap/chroot mounts). Refuses to start without the two OpusDM
   binaries (unpacks the tarball fallback if present).
   - Default mode: **`--fast`** - reuses existing `chroot/`, force-recopies
     `config/includes.chroot` (`lb chroot_includes_after_packages
     --force`), redoes only the binary/squashfs/iso stage. Skips
     debootstrap + package install entirely.
   - `--full` - full clean + full `lb build` (debootstrap, packages, all).
   - Falls back to full build automatically if no `chroot/` or no
     kernel installed in it yet.
   - Guards: refuses to build if `config/includes.chroot/home` exists
     (would break `/etc/skel` seeding -> live user boots to tty1 shell
     instead of X; happened once, now hard-blocked).
   - Output: `dreamos-amd64.hybrid.iso` (renamed from live-build's
     `live-image-amd64.hybrid.iso`). Package cache persisted in `cache/`.

3. **`auto/config`** - single source of truth for `lb config` (regenerate
   `config/` by running `lb config`, which re-invokes this script).
   Key settings: distro trixie, amd64, archive areas main+contrib+non
   -free+non-free-firmware, bootloaders syslinux+grub-efi (BIOS+UEFI
   hybrid ISO), no debian-installer, no apt-recommends. Boot cmdline:
   `boot=live components username=user hostname=dreamos
   locales=it_IT.UTF-8 keyboard-layouts=it timezone=Europe/Rome
   persistence systemd.unit=multi-user.target`.

## Config layout (`config/`)

- **`package-lists/*.list.chroot`**:
  - `system.list.chroot`: sudo, locales, firmware-linux/iwlwifi,
    live-boot/config(-systemd), user-setup, live-tools
  - `desktop.list.chroot`: xorg (legacy + libinput), xinit, openbox,
    lightdm+gtk-greeter, lxterminal, dbus-user-session, gtk4/vte4 libs,
    adwaita + breeze icon themes, arandr
  - `network.list.chroot`: network-manager(-gnome)
  - `installer.list.chroot`: calamares + calamares-settings-debian
  - `live.list.chroot`: live-boot, live-config(-systemd), systemd-sysv
- **`hooks/normal/`** (run during chroot build):
  - `0100-locale-keyboard-timezone.hook.chroot`: bakes it_IT.UTF-8 +
    en_US.UTF-8 locales, it/us keyboard (Alt+Shift toggle), Europe/Rome
    tz onto the chroot - so an on-disk Calamares install matches live
    boot defaults without depending on live-config's kernel-cmdline setup.
  - `0200-calamares-launcher.hook.chroot`: writes
    `/usr/share/applications/install-dreamos.desktop` (`pkexec
    calamares`), ensures `user` is in group `sudo`.
  - `0300-cleanup.hook.chroot`: `apt-get clean`, strips
    `/usr/share/doc`, `/var/log`, cached debs, locale data except
    it/en/C - keeps ISO small.
- **`includes.chroot/`** - files copied verbatim into rootfs:
  - `etc/skel/` - seeds new home dirs (`.bash_profile`, `.xinitrc`,
    `.config/gtk-{3,4}.0`, `.config/opusdm/` incl. breeze-dark icon
    theme, `opusdm/backgrounds/dream01.jpg`)
  - `etc/xdg/openbox/` - `autostart` (just `opusdm-hub &`), `rc.xml`,
    `menu.xml` (right-click desktop menu: Terminal, OpusDM, Install
    dreamos, session actions)
  - `etc/lightdm/`, `etc/X11/xorg.conf.d/20-resolution.conf`
  - `etc/systemd/system/getty@tty1.service.d/autologin.conf` - autologin
  - `usr/bin/opusdm-hub`, `opusdm-lister`, `opusdm-theme`, `xtask` -
    staged by `build-opusdm.sh`
  - `usr/share/backgrounds/dreamos.png`

Boot chain: autologin `user` on tty1 -> `.bash_profile` runs `startx`
(only if `$DISPLAY` unset and tty1) -> `.xinitrc` runs `dbus-run-session
-- openbox-session` -> Openbox `autostart` launches `opusdm-hub`.
`opusdm-hub` IS the desktop shell: draws background + desktop icons,
own toolbar, opens first Lister window on `$HOME`. No separate
panel/wallpaper setter.

## Local dev / test scripts (`scripts/`, `tests/`, `start.sh`)

- **`scripts/test-chroot.sh`** (needs root, `systemd-nspawn`) - shell or
  `--boot` (systemd) or one-off command in built `chroot/`, no ISO
  repack. Fastest way to check packages/locale/keyboard/OpusDM bins
  (`ldd`). Runs as root (live-config user doesn't exist outside real boot).
- **`scripts/test-desktop.sh`** (needs Xephyr, systemd-nspawn, sudo,
  host X session) - runs actual Openbox+opusdm-hub session from
  `chroot/` in a nested Xephyr window, creating throwaway `user`
  (uid 1000, sudo) inside the tree if missing. For iterating on
  autostart/rc.xml/menu.xml/OpusDM without full ISO rebuild.
- **`start.sh`** - boots the built ISO in QEMU. `--uefi`/`--bios`
  (default), `--res WxH` (default 1920x1080 via virtio-vga EDID),
  `--fit`/`--fullscreen`, KVM auto-enabled if `/dev/kvm` writable.
  Persists UEFI NVRAM per-project (`.ovmf-vars.fd`, gitignored).
- **`tests/verify-iso.sh`** - sanity check: `file(1)` reports bootable,
  `xorriso -report_el_torito` shows both BIOS and UEFI/EFI entries.
- **`clean.sh`** - `lb clean --purge` in container; `--all` also drops
  `cache/` and the ISO.

## Persistence & install

- USB persistence: second partition labelled `persistence`, containing
  `persistence.conf` with `/ union` -> `/home` and `/etc` survive reboot.
- Disk install: "Install dreamos" launcher (Openbox menu or
  `.desktop` file) runs Calamares via `pkexec`.

## Release / CI

- **`scripts/release-to-live.sh`** - rebuilds OpusDM binaries+tarball,
  commits tarball on `master` (`SOURCE_BRANCH` override) if changed,
  fast-forward-merges into `live` branch, pushes. Refuses if working
  tree dirty or not on source branch.
- **`.github/workflows/build-iso.yml`** - triggered by push to `live`
  (or manual dispatch): frees runner disk, runs `./build.sh`, publishes
  ISO as rolling `live-latest` prerelease. `concurrency` group
  serializes builds; `cache/` cached between runs.

## Key facts to remember for future work

- Never build OpusDM on the host - glibc mismatch breaks the live ISO.
- Never let anything land at `config/includes.chroot/home` - breaks
  `/etc/skel` seeding, silently kills the graphical boot (already bit
  them once, see CHANGES.md "Unreleased").
- `--fast` build path only touches `includes.chroot` + repack; use it
  for OpusDM/config iteration; use `--full` (or let it auto-fallback)
  after `package-lists` or `auto/config` changes.
- Default locale it_IT.UTF-8 (en_US.UTF-8 also generated), keyboard
  it,us (Alt+Shift), timezone Europe/Rome - defined in three places
  that must stay consistent: `auto/config` bootappend-live, the locale
  hook (bakes into chroot), and live-config at boot.
