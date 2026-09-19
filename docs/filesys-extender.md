# filesys-extender - user documentation

`filesys-extender` is a GTK4 wizard for dreamos live USB sticks. It
creates or grows an ext4 **persistence partition** in the unused space
left over after dd'ing a dreamos ISO onto a USB stick larger than the
ISO, so files you keep in `$HOME` and `/etc` survive a reboot instead
of being wiped on every boot.

Source: `filesys-extender/` in this repo (`dreamos-tools`). Design
rationale: [`docs/superpowers/specs/2026-09-19-filesys-extender-design.md`](superpowers/specs/2026-09-19-filesys-extender-design.md).
Developer-facing build/test notes: [`filesys-extender/README.md`](../filesys-extender/README.md).

## What problem this solves

A dreamos ISO is a few gigabytes; USB sticks are usually much bigger.
Debian's live-boot/live-config already know how to use a `persistence`
overlay if one exists (a partition labelled `persistence` containing a
`persistence.conf` file with `/ union` in it) - but nothing in Debian
creates that partition for you. Without it, `/home` and `/etc` reset to
their pristine state on every boot.

`filesys-extender` automates the partition-table surgery that would
otherwise be a manual `parted`/`mkfs.ext4`/`echo` procedure.

## What it does, precisely

Depending on the state of the disk you pick, it does one of three things:

- **Create**: if there is no `persistence`-labelled partition on the
  disk yet, and there is free space at the end of the disk, it creates
  a new primary partition there, formats it ext4, labels it
  `persistence`, mounts it, and writes `persistence.conf` (`/ union`)
  into it.
- **Grow**: if a `persistence` partition already exists and there is
  free space immediately after it (for example because you re-flashed
  the ISO onto the same, now-larger-relative-to-ISO stick), it grows
  that partition into the free space and grows the ext4 filesystem to
  match, then re-mounts it and re-writes `persistence.conf`.
- **Nothing**: if the persistence partition already uses all available
  space, or there is no free space at all, it says so and does not let
  you proceed.

It never touches a mounted partition, never resizes or moves any other
partition, and only ever lists disks the kernel reports as removable.

## Installing / running

`filesys-extender` is meant to be launched from dreamos's Openbox menu
once staged into the live ISO (see the design spec's "Integration with
dreamos" section - staging it into the ISO is a separate `dreamos`-repo
task, not covered here).

To build and run it directly:

```sh
cd filesys-extender
cargo build --release
sudo ./target/release/filesys-extender
```

It needs root to actually change partitions, and self-elevates via
`pkexec` if not already running as root - so running it unprivileged
from a terminal will pop a polkit authentication prompt and re-launch
itself as root.

For UI-only iteration without a real polkit prompt each time (disk
listing works fine unprivileged; anything that would actually run
`parted`/`mkfs.ext4` will simply fail without real root, which is
expected):

```sh
./scripts/run-filesys-extender.sh
```

### System requirements

`parted`, `mkfs.ext4` (from `e2fsprogs`), `resize2fs` (from
`e2fsprogs`), `lsblk`/`findmnt`/`blkid` (from `util-linux`),
`partprobe`, `udevadm`, `mount`/`umount`, and `pkexec` (from
`policykit-1`). On dreamos these are already installed or trivially
added to a package list; see the README for the exact package names.

## Using the wizard

The window has six screens, shown one at a time. A logo and the title
"DreamOS File System Extender" appear at the top of the first screen.
Every screen keeps its action button pinned to the bottom of the
window regardless of how much content is above it.

### 1. Welcome

A short explanation of what the tool does and how the five steps that
follow work (this is the same walkthrough as this document, condensed
for the screen). Click **Get Started** to continue.

### 2. Disk list

Lists every disk the kernel reports as removable - each row shows the
device path, size, and model (for example `/dev/sdb - 14.9 GiB -
Cruzer`). The disk you actually booted the live system from is
pre-selected automatically (if it can be detected), but nothing
happens until you explicitly click a row and press **Next**.

If no removable disk is plugged in, the list is empty and there is
nothing to select - plug in the USB stick and reopen the wizard.

### 3. Inspect

As soon as this screen opens, the tool re-reads the selected disk's
current partition table and free space (fresh every time - going back
and picking a different disk always re-inspects from scratch) and
shows one of:

- *"Will create a new ext4 'persistence' partition on `/dev/sdX`,
  using N.N GiB of free space."*
- *"Will grow the existing 'persistence' partition on `/dev/sdX` up to
  N.N GiB."*
- *"Nothing to do: `<reason>`"* - for example the persistence partition
  already uses all available space, or there is no free space at all.
  **Next** stays disabled in this case; there is nothing to confirm.

Click **Next** to proceed (only enabled when there is an actual plan).

### 4. Confirm

The single safety gate before anything destructive happens. You must
type the exact device path shown (e.g. `/dev/sdb`) into the text field
- **Apply** stays disabled until the typed text matches exactly. This
is deliberate: there is no click-through "Are you sure?" dialog for a
destructive disk operation, because those are too easy to dismiss on
autopilot.

Double-check you are targeting the right stick before typing the path
- especially if you have more than one USB drive attached.

### 5. Executing

Once you click **Apply**, each underlying command runs in sequence and
its description and output appear live in the log view: partition
creation/resize, waiting for the kernel to notice the new partition
table, formatting/resizing the filesystem, mounting, writing
`persistence.conf`, and unmounting. If a step fails, the log shows
`FAILED: <exact error>` and execution stops immediately - steps that
already ran are **not** rolled back (partition-table rollback is
itself risky, so the tool reports state honestly instead of guessing).

### 6. Result

Shows "Completed successfully." on success, or the failure detail
carried over from the Executing screen if something went wrong. To
retry after a failure, close and reopen the wizard - it always
re-inspects the disk from scratch.

## Persistence, after the wizard finishes

Once the wizard reports success, the persistence overlay is active for
the *next* boot onward - nothing further to configure. Files you
create or change under `$HOME` and `/etc` will now survive a reboot of
that USB stick.

## Safety notes

- Only disks flagged removable (`RM=1` in `lsblk`) are ever shown as
  candidates - internal/fixed disks never appear in the list.
- A mounted partition is never touched by the tool; if something on
  the target disk is mounted in a way that blocks the operation, the
  relevant step fails with the exact error rather than force-unmounting.
- Growing only ever happens into free space immediately following the
  existing persistence partition - the tool never shuffles other
  partitions around to make room.
- There is no filesystem choice: the persistence partition is always
  ext4. This matches what live-boot's persistence overlay expects and
  keeps the tool's surface small.
- No automatic retry or rollback after a failed step - the log tells
  you exactly what ran and what didn't, and you decide what to do next
  (including fixing things manually with `parted`/`fdisk` if needed).

## Troubleshooting

- **Disk list is empty**: no removable disk detected. Make sure the
  USB stick is actually plugged in and the kernel sees it (`lsblk`
  from a terminal should list it).
- **"Nothing to do" on the Inspect screen**: either the persistence
  partition already uses all available space (nothing left to grow
  into), or there is no free space on the disk at all (the ISO/other
  partitions already fill it). Use a larger stick, or free space
  manually with `parted`/`fdisk` first.
- **Apply stays disabled on the Confirm screen**: the typed text must
  match the device path exactly, including `/dev/` and no trailing
  characters.
- **A step fails during Executing**: read the exact error in the log.
  Common causes: the target partition or disk is mounted elsewhere, or
  a required tool (`parted`, `mkfs.ext4`, `resize2fs`) is missing from
  the running system.
- **The window never opens / nothing happens when launching**: check
  whether a polkit authentication dialog appeared (it may be behind
  another window) - the app waits for that before doing anything.
