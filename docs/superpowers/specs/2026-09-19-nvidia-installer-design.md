# nvidia-installer - design spec

Date: 2026-09-19
Repo: `dreamos-tools` (this repo), folder `nvidia-installer/`
Related: [`docs/dreamos.md`](../../dreamos.md) - dreamos is a Debian 13
"trixie" live ISO; this tool targets that base only.

## Problem

dreamos ships the open `nouveau` driver by default. Users with an
NVIDIA card who want the proprietary driver (better performance,
CUDA/NVENC support) currently have to do it manually: enable
non-free/non-free-firmware, find the right `nvidia-driver` package for
their card, `apt install`, and - if Secure Boot is on - separately
figure out DKMS module signing (MOK enrollment) or the module silently
fails to load after install with no obvious error. `nvidia-installer`
automates detection and installation with a GUI, following the same
pattern as `filesys-extender` and `machine-score` in this repo.

## Goals

- Detect NVIDIA GPU(s) present via PCI enumeration, and which driver
  (nouveau / already-proprietary / none) is currently bound.
- Recommend the correct Debian driver package for the detected
  hardware, using Debian's own `nvidia-detect` tool rather than
  reimplementing a GPU-generation table.
- Detect Secure Boot state and, if enabled, walk the user through DKMS
  MOK key generation + enrollment so the built kernel module actually
  loads after reboot instead of failing silently.
- Install via `apt-get`, with a live log, and a clear final state
  ("installed, reboot to load" / "installed and verified working" /
  "failed, here's why").
- Be safe by construction: no destructive disk/data operations here
  (this tool never touches partitions or user files), but package
  installs and MOK enrollment are still system-state changes requiring
  explicit confirmation.

## Non-goals (v1)

- No multi-distro support - Debian trixie / apt only, matches dreamos.
- No support for NVIDIA Optimus / hybrid graphics switching (`prime`
  config) - installs the driver only; laptop MUX/offload setup is a
  separate, later concern.
- Assumes `contrib`/`non-free`/`non-free-firmware` apt components are
  already enabled in `/etc/apt/sources.list` (true for dreamos by
  default); the tool does not edit apt sources. If the recommended
  package can't be found, it reports that plainly instead of editing
  sources itself.
- Not responsible for staging the built binary into the dreamos ISO
  build - out of scope for this spec, same reasoning as
  filesys-extender's spec. Standalone tool only for now.
- No driver *removal*/rollback wizard - v1 is install-only.

## Repo layout

```
dreamos-tools/
  nvidia-installer/
    Cargo.toml
    src/
      lib.rs          # re-exports below - no GTK dep
      detect.rs        # lspci parsing: card presence + current driver
      recommend.rs      # runs/parses `nvidia-detect`, package choice
      secureboot.rs       # mokutil --sb-state, MOK keypair generation
      exec.rs              # argv builders + command runner (shared pattern with filesys-extender's exec.rs)
      state.rs              # /var/lib/nvidia-installer/state.json - pending-reboot marker
      main.rs                # GTK4 UI, wizard state machine
    tests/
      fixtures/                # sample lspci/nvidia-detect/mokutil output
```

Sibling top-level folder, own `Cargo.toml`, no shared workspace (same
YAGNI reasoning as filesys-extender - revisit if a second tool actually
needs shared code).

Dependencies: `gtk4` (`0.10`, `v4_10` feature, same pin as
filesys-extender), `serde`/`serde_json` (state file), `thiserror`.
Runtime (not Rust deps, but required on the system): `pciutils`
(`lspci`), `nvidia-detect`, `mokutil`, `openssl` - noted for the future
ISO package-list change, not installed by this tool itself.

## Architecture

### Privilege model

Same as filesys-extender: whole app runs as root via `pkexec
nvidia-installer`, self-elevating in `main()` if not already root. No
helper binary - installing packages and enrolling MOK keys both
inherently need root.

### Detection (`detect.rs`)

- `lspci -nnk` - find PCI VGA/3D-controller entries whose vendor ID is
  `10de` (NVIDIA). For each, also read the `Kernel driver in use:`
  line from the same stanza to report current state: `nouveau`,
  `nvidia` (already proprietary - tool then just verifies/offers
  reinstall), or none (no driver bound at all).
- No card found -> wizard shows a dead-end "no NVIDIA GPU detected"
  screen and stops; nothing else runs.

### Recommendation (`recommend.rs`)

- Shells `nvidia-detect` (Debian's own detection tool, ships in
  `nvidia-detect` package) and parses its stdout for the suggested
  package name (it prints a line like `It is recommended to install
  the nvidia-driver package`). Regex-extract the package token from
  that line.
- If `nvidia-detect` is missing (`ENOENT`) or its output doesn't match
  the expected pattern, falls back to the plain `nvidia-driver`
  metapackage and flags the recommendation as "best guess" in the UI
  rather than "detected" - user can still override it.
- The Recommend screen shows the suggested package in an editable
  combo box (suggested value pre-filled) so a user who knows better
  (e.g. a legacy card needing `nvidia-tesla-470-driver`) can type a
  different package name before confirming.

### Secure Boot handling (`secureboot.rs`)

- `mokutil --sb-state` - parses for "SecureBoot enabled" vs "disabled".
  If `mokutil` itself is missing, treat as "unknown", show a warning,
  and let the user choose to proceed anyway or abort.
- If enabled:
  - Check for an existing MOK key pair at the fixed paths dkms itself
    already looks for: `/var/lib/dkms/mok.key` +
    `/var/lib/dkms/mok.pub` (this is the same location Debian's `dkms`
    package auto-detects and uses to sign modules at build time with
    zero extra config - no dkms.conf changes needed).
  - If absent, generate them: `openssl req -new -x509 -newkey rsa:2048
    -keyout mok.key -outform DER -out mok.pub -nodes -days 36500
    -subj "/CN=dreamos nvidia-installer MOK/"`.
  - Enroll the public key: `mokutil --import /var/lib/dkms/mok.pub`.
    This is interactive - `mokutil` reads a one-time enrollment
    password twice from stdin. The wizard collects the password via a
    GTK password-entry field (shown twice, must match, min 8 chars per
    `mokutil`'s own requirement) and pipes it to the child process's
    stdin instead of relying on a real TTY prompt.
  - On success, write `state.json` marking `mok_enrollment_pending:
    true` and show the "reboot required" screen (see State machine
    below) - the actual key enrollment only completes when the user
    reboots and approves it in the blue MokManager firmware screen,
    typing the same password.
- If a MOK key pair already exists and is already enrolled (`mokutil
  --list-enrolled` contains it), skip straight to install - dkms will
  auto-sign with the existing key.

### Package install (`exec.rs`)

Same `run_cmd(argv) -> Result<Output, InstallError>` pattern as
filesys-extender's `exec.rs`: every shell-out captures stdout+stderr,
non-zero exit is never silently discarded, `InstallError` carries
command/exit code/stderr verbatim for the UI.

- `apt-get update`
- `apt-get install -y <package>` (package name from the Recommend
  screen, possibly user-overridden)

Both run with output streamed line-by-line into the Executing screen's
log view (matches filesys-extender's live log UX).

### State machine (`state.rs` + wizard flow)

`/var/lib/nvidia-installer/state.json` records at most one field:
`mok_enrollment_pending: bool`. On launch:

```
Welcome
  -> Detect (no card -> dead end)
  -> Recommend (package choice, editable)
  -> SecureBoot (skip if SB off; "I'll handle SB myself" opt-out
     checkbox always available)
     -> [if opted in and key needs enrolling: password entry -> enroll
        -> writes state.json, shows RebootRequired, wizard exits here]
  -> Confirm (plan summary: exact apt commands, + MOK status line)
  -> Executing (apt update/install, live log)
  -> Result (success/failure + full log path)
```

If the wizard is launched and `state.json` says
`mok_enrollment_pending: true`, it skips straight to a **Verify**
screen instead of Welcome: checks `mokutil --list-enrolled` (key now
actually enrolled post-reboot?) and `lsmod | grep -q ^nvidia` /
`nvidia-smi` exit code (module loaded?), reports pass/fail for each,
and on full success clears `state.json` and offers to continue into
the normal Recommend->Confirm->Executing flow if the driver isn't
installed yet, or just reports "all good" if it already is.

### Error handling

- Any failed step (detect, enroll, apt) stops the sequence immediately
  and shows the verbatim command + exit code + stderr - no generic
  "something went wrong", matching filesys-extender's convention.
- MOK enrollment failure (bad password confirmation, `mokutil`
  non-zero exit) does not write `state.json` - wizard just returns to
  the SecureBoot screen to retry.
- No automatic retries anywhere; re-running the tool always
  re-detects/re-plans from scratch (except the one Verify short-circuit
  above).

## UI flow (GTK4 wizard, one window)

1. **Welcome** - what the tool does, in plain language.
2. **Detect** - card(s) found + current driver state, or dead-end
   screen if none found.
3. **Recommend** - suggested package (editable), source noted
   ("detected via nvidia-detect" or "best guess - nvidia-detect
   unavailable").
4. **Secure Boot** - SB state, explanation of why signing matters,
   opt-in checkbox to let the tool generate+enroll a MOK key, password
   entry (shown twice) if opting in.
5. **Reboot Required** (only reached after a fresh MOK enrollment) -
   the one-time password shown once more for reference, instructions
   to approve enrollment in the blue MokManager screen on next boot,
   and to re-run this tool afterward.
6. **Verify** (only reached when relaunched with a pending state) -
   pass/fail for MOK enrollment + module load.
7. **Confirm** - exact `apt-get` commands about to run + MOK status
   line.
8. **Executing** - live stdout/stderr log.
9. **Result** - success/failure summary + full log path.

## Testing

- `detect.rs`/`recommend.rs`/`secureboot.rs` parsing logic: unit tests
  against fixture text files under `tests/fixtures/` (`lspci -nnk`
  samples for NVIDIA-present/absent/already-proprietary, sample
  `nvidia-detect` stdout, sample `mokutil --sb-state` /
  `--list-enrolled` output). No real hardware or root needed.
- `exec.rs` argv builders: unit tests checking exact argv construction
  (no real `apt-get`/`mokutil` invocation in CI).
- MOK enrollment + apt install: manual test procedure only, documented
  in the crate's README - needs a real (or VM with OVMF/Secure-Boot-
  capable) machine, since it changes real firmware-trusted-key state
  and installs real packages. Not run in CI.
- End-to-end GUI: manual smoke test, run the built binary via
  `scripts/run-nvidia-installer.sh` (mirrors the existing
  `run-machine-score.sh`/`run-filesys-extender.sh` pattern) on a VM
  with a passed-through or emulated NVIDIA GPU, or at minimum exercise
  the Detect dead-end path on any non-NVIDIA machine.

## Open questions / follow-ups for the implementation plan

None outstanding - all decisions above were confirmed during
brainstorming. Suggested implementation sequence for writing-plans:
`detect.rs` + fixtures first (no GTK, no root needed to test), then
`recommend.rs`, then `secureboot.rs` (parsing only - key generation
tested manually), then `exec.rs`, then `state.rs`, then `main.rs` GTK
wizard wiring everything together, then the `run-nvidia-installer.sh`
script.
