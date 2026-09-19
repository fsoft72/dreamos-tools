# filesys-extender - design spec

Date: 2026-09-19
Repo: `dreamos-tools` (this repo), folder `filesys-extender/`
Related: [`docs/dreamos.md`](../../dreamos.md) - how the dreamos live ISO
is built and how persistence currently works (manual steps).

## Problem

dreamos live ISOs are dd'd onto USB sticks much larger than the ISO
itself. Debian live-boot/live-config already support a persistence
overlay (a `persistence`-labelled partition + `persistence.conf`), but
Debian ships no tool to create/grow that partition into the unused
space - it's a fully manual `fdisk`/`mkfs`/echo-a-file procedure
(documented in dreamos's own README). `filesys-extender` automates
that procedure with a GUI, as the first tool in this repo.

## Goals

- Detect the live USB stick and any unallocated space on it.
- Create a `persistence` partition (ext4) in that space if none
  exists, or grow an existing one if more space has since become
  available (e.g. after a re-`dd`, or a partition was shrunk
  elsewhere).
- Write `persistence.conf` (`/ union`) into the new/grown partition.
- Be safe by construction: destructive operations require explicit,
  hard-to-misclick confirmation, and only ever target the disk the
  live system actually booted from by default.

## Non-goals (v1)

- Not a generic partition manager (no arbitrary resize/move/delete of
  other partitions).
- No filesystem choice - ext4 only.
- No support for reclaiming space that isn't immediately contiguous
  after the persistence partition.
- No automatic rollback of partially-completed operations.
- Not responsible for staging the built binary into the dreamos ISO
  build (`config/includes.chroot/usr/bin/`) or its package-list entries
  - that's a `dreamos` repo change, out of scope for this spec, which
  only defines the interface: a single binary, `filesys-extender`,
  installed to `/usr/bin`, launched via `pkexec`.

## Repo layout

```
dreamos-tools/
  filesys-extender/
    Cargo.toml
    src/
      lib.rs       # disk detection, inspection, planning - no GTK dep
      main.rs       # GTK4 UI, wizard state machine, calls lib.rs
      disk.rs        # lsblk/parted parsing, plan generation
      exec.rs         # shell-out command runner + step execution
    tests/
      fixtures/        # sample lsblk -J / parted print free output
```

Single crate: `lib.rs` (disk logic) + `main.rs` (GTK4 UI) split so the
planning logic is unit-testable without a display. Future tools in
this repo get their own sibling top-level folders; no shared Cargo
workspace until a second tool actually needs to share code with this
one (YAGNI).

Dependencies: `gtk4` (version `0.10`, feature `v4_10` - matches
OpusDM's pin, so the ISO's already-installed GTK4 libs cover it), plus
whatever small crates are needed for JSON parsing of `lsblk -J` output
(e.g. `serde`/`serde_json`, also already an OpusDM workspace dep).

## Architecture

### Privilege model

Whole app runs as root via `pkexec filesys-extender`, mirroring the
existing dreamos pattern (`install-dreamos.desktop` runs `Exec=pkexec
calamares`). `main()`:

```
if geteuid() != 0 {
    exec pkexec filesys-extender  // replace process, or spawn+exit
}
```

No privilege-separated helper binary, no custom polkit action file -
the GUI itself needs root for `parted`/`mkfs.ext4`/`resize2fs`, same
as Calamares/gparted. Simpler to build and audit for a single-purpose
internal tool; revisit only if this tool's surface grows significantly.

### Disk operations (`disk.rs`, `exec.rs`)

All destructive operations shell out to standard, already-available
or trivially-added CLI tools rather than reimplementing partition
table math:

- `lsblk -J -o NAME,SIZE,RM,TRAN,MOUNTPOINT,MODEL,PATH` - enumerate
  block devices, JSON-parsed.
- `findmnt /run/live/medium` (fallback: parse `live-media=` from
  `/proc/cmdline`) - identify the device the live system actually
  booted from, to pre-select it in the UI.
- `parted <dev> unit MiB print free` (machine/script-friendly output)
  - inspect current partition table and free space.
- `parted <dev> mkpart primary ext4 <start> <end>` - create the
  persistence partition.
- `parted <dev> resizepart <N> <end>` - grow an existing persistence
  partition into adjacent free space.
- `mkfs.ext4 -L persistence <partition>` - format.
- `resize2fs <partition>` - grow filesystem after `resizepart`.
- `partprobe <dev>` / `udevadm settle` - make the kernel/udev see the
  new partition table before touching the new device node.
- `blkid` - confirm the new partition's label/UUID after formatting.

Every shell-out goes through one `run_cmd(argv) -> Result<Output,
DiskOpError>` helper in `exec.rs` that always captures stdout+stderr
and never discards a non-zero exit code. `DiskOpError` carries the
command, exit code, and stderr, so the UI can show the exact failure
verbatim - no generic "something went wrong."

### Data flow / state machine

```
DiskList
  -> SelectedDisk (inspect: partition table, free space, existing
     'persistence' partition if any)
  -> Plan (Create | Grow), computed from SelectedDisk
  -> ConfirmTyped (device path must be typed exactly to enable Apply)
  -> Executing (steps run sequentially, live command log shown)
  -> Result (Ok | Err, with full log)
```

`Plan` is `Create` when no `persistence`-labelled partition exists and
there is unallocated space; `Grow` when one exists AND unallocated
space immediately follows it in the partition table; otherwise the UI
shows "no action possible" with the reason (no free space, or
existing partition isn't at the end of the free region) instead of a
plan.

### Safety guards

- Disk list is filtered to `RM=1` (removable) devices from `lsblk`;
  the live-boot device (from `findmnt`/`cmdline`) is pre-selected but
  never auto-confirmed - user still goes through the full wizard.
- A partition currently mounted is never touched; if the detected
  `persistence` partition (or the target free region's disk) has a
  mounted partition, the tool reports it and requires the user to
  unmount first rather than force-unmounting itself.
- The confirm screen requires typing the exact device path (e.g.
  `/dev/sdb`) before "Apply" is enabled - no click-through confirm
  dialog for a destructive disk operation.
- Grow only proceeds when free space is contiguous and immediately
  after the existing persistence partition (no shuffling of other
  partitions in v1).
- Every command run is logged verbatim (argv + exit code + stderr) to
  a session log file under the invoking (pre-elevation) user's runtime
  dir; the path is shown on the Result screen for post-mortem.

### Error handling

- Any failed step stops the sequence immediately; already-completed
  steps are not rolled back (rollback of partition-table operations is
  itself risky and out of scope). The Result screen states plainly
  which steps succeeded and which failed, with the verbatim error.
- No retries are automatic - user re-runs the tool, which re-inspects
  the disk from scratch and computes a fresh plan.

## UI flow (GTK4 wizard, one window, five screens)

1. **Disk list** - table of removable disks (path, size, model), live-
   boot device pre-selected/highlighted.
2. **Inspect** - selected disk's current partition table + free space,
   and the computed `Plan` (Create/Grow/"no action possible" + why).
3. **Confirm** - plan summary (exact commands about to run) + a text
   entry that must match the device path exactly to enable "Apply".
4. **Executing** - steps run one by one, each with its command and
   live stdout/stderr in a scrollable log view.
5. **Result** - success or failure summary, full log, and the on-disk
   log file path.

## Integration with dreamos (interface only, not this spec's scope)

For the `dreamos` repo to consume this tool, it will eventually need:

- New/extended package-list entries: `parted`, `e2fsprogs` (provides
  `resize2fs`), `policykit-1` (verify already present via Calamares's
  `pkexec` use). `lsblk`/`blkid` are `util-linux`, already installed.
- The built `filesys-extender` binary staged into
  `config/includes.chroot/usr/bin/filesys-extender`, analogous to how
  `scripts/build-opusdm.sh` stages OpusDM binaries today.
- A menu entry in `config/includes.chroot/etc/xdg/openbox/menu.xml`
  next to "Install dreamos", `Exec=pkexec filesys-extender`.

These are noted here for context only; actual implementation of the
dreamos-side staging is a separate, later task in the `dreamos` repo.

## Testing

- `disk.rs`/planning logic: unit tests driven by fixture text files
  under `tests/fixtures/` (`lsblk -J` and `parted ... print free`
  sample output for: empty disk, disk with existing persistence
  partition + free space, disk with persistence partition and no free
  space, disk with no free space at all). No real block device needed.
- `exec.rs` command execution: manually tested against a loopback
  device (`truncate` a sparse file + `losetup`), documented as a
  manual test procedure (not run in CI - destructive disk ops don't
  belong in automated CI).
- End-to-end GUI: manual smoke test once staged into a dreamos ISO,
  via dreamos's existing `start.sh` (QEMU, real virtual disk) or
  `scripts/test-desktop.sh` (Xephyr, faster iteration on UI only, no
  real disk-op testing there since it runs against the build chroot's
  own filesystem, not a removable disk).

## Open questions / follow-ups for the implementation plan

None outstanding - all decisions above were confirmed during
brainstorming. The implementation plan (next step, via writing-plans)
should sequence: `lib.rs`/`disk.rs` + unit tests first (testable
without GTK), then `exec.rs` + loopback manual test, then `main.rs`
GTK wizard wired to the tested backend.
