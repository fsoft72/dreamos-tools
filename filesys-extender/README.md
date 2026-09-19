# filesys-extender

GTK4 wizard that creates or grows an ext4 `persistence` partition in the
unused space of a dreamos live USB stick, and writes its
`persistence.conf`. See
[the design spec](../docs/superpowers/specs/2026-09-19-filesys-extender-design.md)
for the full rationale and safety guards.

## Build

    cargo build --release

Output: `target/release/filesys-extender`.

## Run

Must run as root (it self-elevates via `pkexec` if not already root):

    ./target/release/filesys-extender

## Manual test procedures

- Unit tests (no root, no real disk needed): `cargo test`
- Loopback device test of the exact `parted`/`mkfs.ext4`/`resize2fs`
  argv sequences: create a throwaway file, `losetup` it, run the same
  partition/format/inspect commands `exec.rs`'s `steps_for_plan` builds,
  and check the `parted -m ... print free` output shape matches what
  `disk.rs`'s `parse_parted_free` expects (see
  `tests/fixtures/parted_free_real_loopback.txt` for a real captured
  example, and the fix in `disk.rs` this uncovered: free-space lines
  carry a numeric first field too, which must never be read as a real
  partition number).
- Full end-to-end test: build a dreamos ISO with this binary staged at
  `usr/bin/filesys-extender` (see the "Integration with dreamos" section
  of the design spec - that staging step lives in the `dreamos` repo, not
  here), boot it in QEMU via dreamos's `./start.sh`, attach a second
  virtual USB disk larger than the ISO, and run the wizard from the
  Openbox menu end to end.

## Dependencies (for staging into the dreamos ISO)

`parted`, `e2fsprogs` (`resize2fs`), `util-linux` (`lsblk`, `findmnt`,
`partprobe`, `blkid` - already present), `policykit-1` (already required
for the Calamares installer launcher).

## Verified on

- 2026-09-19: unit test suite (23 tests) passing; loopback verification
  against real `parted`/`mkfs.ext4` confirmed and fixed a parsing bug
  (see `disk.rs` free-space handling). GTK wizard smoke-tested on a
  dev machine with no removable disk attached (empty disk list, no
  crash) - full click-through with a real disk and the QEMU end-to-end
  test are still pending.
