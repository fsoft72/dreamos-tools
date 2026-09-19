# DreamOS NVIDIA Driver Installer - user documentation

`nvidia-installer` is a GTK4 wizard that detects NVIDIA GPUs, installs
the proprietary driver via `apt`, and - if Secure Boot is on - sets up
DKMS module signing so the driver actually loads after reboot.

Source: `nvidia-installer/` in this repo (`dreamos-tools`). Design
rationale: [`docs/superpowers/specs/2026-09-19-nvidia-installer-design.md`](superpowers/specs/2026-09-19-nvidia-installer-design.md).
Developer-facing build/test notes: [`nvidia-installer/README.md`](../nvidia-installer/README.md).

## What it does

It answers one question: *does this PC have an NVIDIA GPU, and if so,
what's the right way to get the proprietary driver installed and
actually working?* It detects your card, picks the correct driver
package for it, installs it via `apt`, and - the part manual
instructions usually skip - handles Secure Boot: on a Secure Boot
system, an unsigned third-party kernel module (like the NVIDIA driver
built by DKMS) silently fails to load after reboot with no obvious
error. This tool generates and enrolls a signing key so that doesn't
happen.

It targets Debian trixie (dreamos's base) only, needs root (it
self-elevates via `pkexec`), and never edits your `apt` sources - it
assumes `contrib`/`non-free`/`non-free-firmware` are already enabled,
which they are by default on dreamos.

## Installing / running

To build and run it directly:

```sh
cd nvidia-installer
cargo build --release
sudo ./target/release/nvidia-installer
```

(It re-elevates itself via `pkexec` if not already run as root, so a
plain `./target/release/nvidia-installer` also works and will prompt.)

For quick iteration from this repo's root, without a real elevation
prompt each time:

```sh
./scripts/run-nvidia-installer.sh
```

## Using the wizard

Every screen shows the DreamOS logo and "DreamOS NVIDIA Driver
Installer" title at the top, with the action button pinned to the
bottom of the window.

### 1. Welcome

A short description of what the tool does and the steps ahead. Click
**Get Started** to continue.

### 2. Detect

Scans PCI devices for NVIDIA hardware and shows each card found, plus
which driver is currently bound to it (`nouveau`, `nvidia`, or none).
If no NVIDIA GPU is found, this screen dead-ends here - there's
nothing else for the tool to do.

### 3. Recommend

Shows the driver package it will install, normally detected via
Debian's own `nvidia-detect` tool (labeled "Detected via
nvidia-detect"). If `nvidia-detect` isn't installed or its output
can't be parsed, it falls back to the plain `nvidia-driver` metapackage
and says so ("best guess"). The package name is an editable text
field - override it if you know your card needs something different
(e.g. a legacy `nvidia-tesla-4XX-driver` for an older card).

### 4. Secure Boot

- **Secure Boot off**: nothing to do here, just click **Next**.
- **Secure Boot on**: explains why signing matters, then either:
  - Type a one-time enrollment password (min 8 characters, entered
    twice) and click **Generate key and enroll**. This creates a
    signing key at `/var/lib/dkms/mok.key`/`mok.pub` (the fixed
    location `dkms` itself already checks, so no extra configuration
    is needed) and submits it to the firmware via `mokutil --import`.
  - Or check **"I'll handle Secure Boot / module signing myself"** to
    skip this and proceed at your own risk - the driver will install
    but its kernel module won't load until you sort out signing
    another way.

### 5. Reboot Required (only if you just enrolled a key)

Enrollment doesn't take effect until you reboot and approve it in the
firmware's blue **MOK Management** screen: choose **Enroll MOK**, then
**Continue**, then type the same password you just entered. After
that, re-run this tool - it picks up where it left off.

### 6. Verify (only when re-run after a pending enrollment)

Checks whether the key is now enrolled and reports it. If it is, you
can continue straight into the install; if it isn't, you likely didn't
complete the blue MOK Management screen - reboot and try again.

### 7. Confirm

Shows exactly what's about to run - `apt-get update` and
`apt-get install -y <package>` - plus a one-line Secure Boot status.
Click **Apply** to proceed.

### 8. Executing

Live output from `apt-get update` and the driver install, streamed as
it happens.

### 9. Result

Success or failure, with the full output. On success: **reboot** to
load the new driver (and, if you enrolled a signing key this run,
complete the MOK enrollment screen on that same reboot if you haven't
already).

## Known limitations (v1)

These are deliberate scope decisions, not bugs:

- **Debian/apt only.** No other distro's package manager is supported.
- **Doesn't edit apt sources.** If `contrib`/`non-free`/
  `non-free-firmware` aren't enabled, the install step will fail with
  apt's own "package not found" error rather than being fixed
  automatically.
- **No hybrid-graphics (Optimus) setup.** Installs the driver only;
  laptop GPU-switching configuration is a separate, later concern.
- **Install-only.** There's no driver removal/rollback wizard yet.
- **Not staged into the dreamos ISO yet.** This is a standalone tool
  in this repo for now; integrating it into the live-build package
  list and Openbox menu is a separate follow-up task in the `dreamos`
  repo.

## Troubleshooting

- **"No NVIDIA GPU detected" on a machine that has one**: the tool
  only recognizes PCI VGA/3D-controller entries with vendor ID `10de`
  (NVIDIA's). Run `lspci -nnk` yourself and check the card shows up
  with that vendor ID.
- **Driver installed but `nvidia-smi` / graphics don't work after
  reboot**: almost always a Secure Boot signing problem. Check
  `mokutil --sb-state` - if it says enabled, re-run this tool: the
  Verify screen will tell you whether your key is actually enrolled
  and whether the `nvidia` kernel module loaded.
- **MOK enrollment password rejected on the blue firmware screen**:
  you must use the *same* password you typed into the wizard's Secure
  Boot screen - it's a one-time password for that enrollment request
  only, not your login or disk-encryption password.
- **`apt-get install` fails with "Unable to locate package"**: the
  recommended package isn't available - most likely
  `contrib`/`non-free`/`non-free-firmware` aren't enabled in
  `/etc/apt/sources.list`. This tool doesn't edit apt sources itself
  (see Known limitations); enable them manually and retry.
