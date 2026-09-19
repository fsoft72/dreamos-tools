# nvidia-installer

GTK4 wizard that detects NVIDIA GPUs and installs the proprietary
driver via apt on a Debian trixie system (dreamos's base). Handles
Secure Boot DKMS module signing (MOK key generation + enrollment) so
the installed driver actually loads after reboot. See
[the design spec](../docs/superpowers/specs/2026-09-19-nvidia-installer-design.md)
for the full rationale.

## Build

    cargo build --release

Output: `target/release/nvidia-installer`.

## Run

Must run as root (it self-elevates via `pkexec` if not already root):

    ./target/release/nvidia-installer

## Runtime dependencies

- `pciutils` (`lspci`) - GPU detection.
- `nvidia-detect` - package recommendation (falls back to the plain
  `nvidia-driver` metapackage if absent).
- `mokutil`, `openssl` - Secure Boot MOK key generation/enrollment.
- `apt-get` - driver install.

## Manual test procedures

- Unit tests (no root needed): `cargo test`
- Secure Boot / MOK enrollment flow: manual only, needs a real machine
  or a Secure-Boot-capable VM (OVMF) - generates a real signing key at
  `/var/lib/dkms/mok.key`/`mok.pub` and submits a real enrollment
  request via `mokutil --import`. Verify by rebooting and completing
  the blue MokManager screen, then re-running the wizard to confirm
  the Verify screen reports the key enrolled and the module loaded.
- Full install: manual only, needs a real or virtual NVIDIA GPU -
  requires actual internet access and non-free/non-free-firmware apt
  components enabled (dreamos ships with these on by default).
- UI iteration without root or a real NVIDIA card:
  `./scripts/run-nvidia-installer.sh` - the Detect page will correctly
  report "no NVIDIA GPU detected" on non-NVIDIA hardware, which still
  exercises the Welcome/Detect dead-end path end to end.
