# nvidia-installer Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build `nvidia-installer`, a GTK4 wizard (root, pkexec-elevated) that detects NVIDIA GPUs on dreamos (Debian trixie), recommends and installs the correct proprietary driver package via apt, and handles Secure Boot DKMS module signing (MOK key generation + enrollment) so the driver actually loads after reboot.

**Architecture:** Single Rust crate, sibling to `filesys-extender`/`machine-score`. Pure-logic modules (`detect.rs`, `recommend.rs`, `secureboot.rs`, `state.rs`) parse CLI tool output and build argv - no GTK dependency, fully unit-testable via fixture files. `exec.rs` runs commands and captures output (mirrors `filesys-extender/src/exec.rs`'s `run_cmd` pattern exactly). `main.rs` is a GTK4 `Stack`-based wizard wiring the logic modules together, following `filesys-extender/src/main.rs`'s established page-builder + `Rc<RefCell<_>>` shared-state + `std::sync::mpsc` + `timeout_add_local` pattern for live command output.

**Tech Stack:** Rust 2021 (rust-version 1.91), `gtk4` 0.10 (`v4_10` feature), `serde`/`serde_json`, `thiserror`, `libc`. Runtime deps (not Cargo deps): `pciutils` (`lspci`), `nvidia-detect`, `mokutil`, `openssl`, `apt`.

**Spec:** [`docs/superpowers/specs/2026-09-19-nvidia-installer-design.md`](../specs/2026-09-19-nvidia-installer-design.md)

## Global Constraints

- Debian trixie / apt only - no other distro support (spec Non-goals).
- Tool never edits `/etc/apt/sources.list*` - assumes non-free/contrib already enabled (spec Non-goals).
- No ISO staging in this plan - standalone crate only (spec Non-goals).
- No driver removal/rollback flow - install-only v1 (spec Non-goals).
- Every shell-out must capture stdout+stderr and never discard a non-zero exit code; errors show the verbatim command/exit code/stderr, never a generic message (spec Error handling, matches filesys-extender convention).
- MOK key pair lives at the fixed paths dkms itself auto-detects: `/var/lib/dkms/mok.key` + `/var/lib/dkms/mok.pub` (spec Secure Boot handling) - no dkms.conf edits needed.
- State file: `/var/lib/nvidia-installer/state.json`, single field `mok_enrollment_pending: bool` (spec State machine).

---

## File Structure

```
dreamos-tools/
  nvidia-installer/
    Cargo.toml
    src/
      lib.rs          # module re-exports, InstallError
      detect.rs        # lspci -nnk parsing
      recommend.rs      # nvidia-detect parsing
      secureboot.rs       # mokutil parsing + MOK keypair generation
      exec.rs              # run_cmd + apt/mokutil argv builders
      state.rs              # state.json read/write
      main.rs                # GTK4 wizard
    tests/
      fixtures/
        lspci_nvidia_nouveau.txt
        lspci_nvidia_proprietary.txt
        lspci_no_nvidia.txt
        nvidia_detect_found.txt
        nvidia_detect_legacy.txt
        mokutil_sb_enabled.txt
        mokutil_sb_disabled.txt
        mokutil_list_enrolled.txt
    scripts/                    # (repo root scripts/, not crate-local)
      run-nvidia-installer.sh
```

Each logic module owns one CLI tool's output format: `detect.rs` <-> `lspci`, `recommend.rs` <-> `nvidia-detect`, `secureboot.rs` <-> `mokutil`/`openssl`. `exec.rs` owns process spawning and argv construction for the mutating commands (`apt-get`, `mokutil --import`). `state.rs` owns the one JSON file. `main.rs` never parses CLI output itself - it only calls into these modules.

---

## Task 1: Crate scaffold + `detect.rs`

**Files:**
- Create: `nvidia-installer/Cargo.toml`
- Create: `nvidia-installer/src/lib.rs`
- Create: `nvidia-installer/src/detect.rs`
- Create: `nvidia-installer/tests/fixtures/lspci_nvidia_nouveau.txt`
- Create: `nvidia-installer/tests/fixtures/lspci_nvidia_proprietary.txt`
- Create: `nvidia-installer/tests/fixtures/lspci_no_nvidia.txt`

**Interfaces:**
- Produces: `pub enum CurrentDriver { Nouveau, Nvidia, None }`, `pub struct GpuDevice { pub pci_slot: String, pub model: String, pub driver: CurrentDriver }`, `pub fn parse_lspci(output: &str) -> Vec<GpuDevice>`, `pub fn detect_gpus() -> Result<Vec<GpuDevice>, InstallError>` (runs `lspci -nnk`, calls `parse_lspci`), `pub enum InstallError` (in `lib.rs`, mirrors `filesys_extender::DiskOpError`'s three variants: `CommandFailed { cmd, code, stderr }`, `Spawn { cmd, detail }`, `Parse { what, detail }`).

- [ ] **Step 1: Create `Cargo.toml`**

```toml
[package]
name = "nvidia-installer"
version = "0.1.0"
edition = "2021"
rust-version = "1.91"

[dependencies]
gtk4 = { version = "0.10", features = ["v4_10"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"
libc = "0.2"

[profile.release]
opt-level = 2
strip = true
```

- [ ] **Step 2: Create `src/lib.rs` with `InstallError` and module declarations**

```rust
pub mod detect;
pub mod exec;
pub mod recommend;
pub mod secureboot;
pub mod state;

use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum InstallError {
    #[error("command '{cmd}' failed with exit code {code:?}: {stderr}")]
    CommandFailed {
        cmd: String,
        code: Option<i32>,
        stderr: String,
    },
    #[error("failed to spawn '{cmd}': {detail}")]
    Spawn { cmd: String, detail: String },
    #[error("failed to parse {what}: {detail}")]
    Parse { what: String, detail: String },
}
```

This won't compile yet (`exec`, `recommend`, `secureboot`, `state` modules don't exist). That's expected - Step 3 below adds `detect.rs` only; the other `pub mod` lines are added incrementally in later tasks. For now, comment out the not-yet-created modules:

```rust
pub mod detect;
// pub mod exec;       // added in Task 4
// pub mod recommend;  // added in Task 2
// pub mod secureboot; // added in Task 3
// pub mod state;      // added in Task 5

use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum InstallError {
    #[error("command '{cmd}' failed with exit code {code:?}: {stderr}")]
    CommandFailed {
        cmd: String,
        code: Option<i32>,
        stderr: String,
    },
    #[error("failed to spawn '{cmd}': {detail}")]
    Spawn { cmd: String, detail: String },
    #[error("failed to parse {what}: {detail}")]
    Parse { what: String, detail: String },
}
```

- [ ] **Step 3: Create fixture files**

`tests/fixtures/lspci_nvidia_nouveau.txt` (a system with one NVIDIA GPU, nouveau bound):

```
01:00.0 VGA compatible controller [0300]: NVIDIA Corporation GA106 [GeForce RTX 3060] [10de:2503] (rev a1)
	Subsystem: Micro-Star International Co., Ltd. [MSI] Device [1462:3897]
	Kernel driver in use: nouveau
	Kernel modules: nouveau
01:00.1 Audio device [0403]: NVIDIA Corporation GA106 High Definition Audio Controller [10de:228e] (rev a1)
	Subsystem: Micro-Star International Co., Ltd. [MSI] Device [1462:3897]
	Kernel driver in use: snd_hda_intel
	Kernel modules: snd_hda_intel
```

`tests/fixtures/lspci_nvidia_proprietary.txt` (proprietary driver already bound):

```
01:00.0 VGA compatible controller [0300]: NVIDIA Corporation GA106 [GeForce RTX 3060] [10de:2503] (rev a1)
	Subsystem: Micro-Star International Co., Ltd. [MSI] Device [1462:3897]
	Kernel driver in use: nvidia
	Kernel modules: nvidia_drm, nvidia
```

`tests/fixtures/lspci_no_nvidia.txt` (integrated Intel graphics only):

```
00:02.0 VGA compatible controller [0300]: Intel Corporation TigerLake-LP GT2 [Iris Xe Graphics] [8086:9a49] (rev 01)
	Subsystem: Dell Device [1028:0a1d]
	Kernel driver in use: i915
	Kernel modules: i915
```

- [ ] **Step 4: Write `src/detect.rs` with the failing tests first**

```rust
use crate::InstallError;
use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CurrentDriver {
    Nouveau,
    Nvidia,
    None,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuDevice {
    pub pci_slot: String,
    pub model: String,
    pub driver: CurrentDriver,
}

/// Parses `lspci -nnk` output into the NVIDIA (vendor id `10de`) VGA/3D
/// controller entries, skipping companion devices (e.g. the HDMI audio
/// function on the same card) since those never carry a display driver.
pub fn parse_lspci(output: &str) -> Vec<GpuDevice> {
    let mut devices = Vec::new();
    let mut lines = output.lines().peekable();

    while let Some(line) = lines.next() {
        let is_display_class = line.contains("VGA compatible controller")
            || line.contains("3D controller");
        if !is_display_class || !line.contains("[10de:") {
            continue;
        }
        let Some(slot) = line.split_whitespace().next() else { continue };
        let model = line
            .split("NVIDIA Corporation ")
            .nth(1)
            .and_then(|rest| rest.split(" [10de:").next())
            .unwrap_or("unknown NVIDIA GPU")
            .to_string();

        let mut driver = CurrentDriver::None;
        while let Some(next_line) = lines.peek() {
            if next_line.starts_with('\t') {
                let next_line = lines.next().unwrap();
                if let Some(name) = next_line.trim().strip_prefix("Kernel driver in use: ") {
                    driver = match name {
                        "nouveau" => CurrentDriver::Nouveau,
                        "nvidia" => CurrentDriver::Nvidia,
                        _ => CurrentDriver::None,
                    };
                }
            } else {
                break;
            }
        }

        devices.push(GpuDevice { pci_slot: slot.to_string(), model, driver });
    }

    devices
}

pub fn detect_gpus() -> Result<Vec<GpuDevice>, InstallError> {
    let output = Command::new("lspci")
        .args(["-nnk"])
        .output()
        .map_err(|source| InstallError::Spawn {
            cmd: "lspci -nnk".into(),
            detail: source.to_string(),
        })?;
    if !output.status.success() {
        return Err(InstallError::CommandFailed {
            cmd: "lspci -nnk".into(),
            code: output.status.code(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }
    Ok(parse_lspci(&String::from_utf8_lossy(&output.stdout)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_nvidia_with_nouveau_bound() {
        let fixture = include_str!("../tests/fixtures/lspci_nvidia_nouveau.txt");
        let devices = parse_lspci(fixture);
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].pci_slot, "01:00.0");
        assert_eq!(devices[0].model, "GA106");
        assert_eq!(devices[0].driver, CurrentDriver::Nouveau);
    }

    #[test]
    fn parses_nvidia_with_proprietary_bound() {
        let fixture = include_str!("../tests/fixtures/lspci_nvidia_proprietary.txt");
        let devices = parse_lspci(fixture);
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].driver, CurrentDriver::Nvidia);
    }

    #[test]
    fn no_nvidia_returns_empty() {
        let fixture = include_str!("../tests/fixtures/lspci_no_nvidia.txt");
        let devices = parse_lspci(fixture);
        assert!(devices.is_empty());
    }
}
```

- [ ] **Step 5: Run tests to verify they fail (module not yet wired into a buildable crate) then verify they pass**

Run: `cd nvidia-installer && cargo test --lib detect`
Expected first (before `lib.rs`'s `pub mod detect;` exists / before this file exists): compile error.
After Steps 1-4 are all in place: `cargo test --lib detect` -> 3 passed.

- [ ] **Step 6: Commit**

```bash
git add nvidia-installer/Cargo.toml nvidia-installer/src/lib.rs nvidia-installer/src/detect.rs nvidia-installer/tests/fixtures/lspci_nvidia_nouveau.txt nvidia-installer/tests/fixtures/lspci_nvidia_proprietary.txt nvidia-installer/tests/fixtures/lspci_no_nvidia.txt
git commit -m "feat(nvidia-installer): scaffold crate, GPU detection via lspci"
```

---

## Task 2: `recommend.rs`

**Files:**
- Modify: `nvidia-installer/src/lib.rs` (uncomment `pub mod recommend;`)
- Create: `nvidia-installer/src/recommend.rs`
- Create: `nvidia-installer/tests/fixtures/nvidia_detect_found.txt`
- Create: `nvidia-installer/tests/fixtures/nvidia_detect_legacy.txt`

**Interfaces:**
- Consumes: `InstallError` from `lib.rs` (Task 1).
- Produces: `pub struct Recommendation { pub package: String, pub source: RecommendationSource }`, `pub enum RecommendationSource { Detected, FallbackDefault }`, `pub fn parse_nvidia_detect(output: &str) -> Option<String>`, `pub fn recommend_package() -> Recommendation` (runs `nvidia-detect`, never returns `Result` - a missing/unparseable tool falls back rather than erroring, per spec).

- [ ] **Step 1: Create fixtures**

`tests/fixtures/nvidia_detect_found.txt`:

```
Detected NVIDIA GPUs:
01:00.0 VGA compatible controller [0300]: NVIDIA Corporation GA106 [GeForce RTX 3060] [10de:2503] (rev a1)
Checking card:  NVIDIA Corporation GA106 [GeForce RTX 3060] [10de:2503] (rev a1)
Your card is supported by the default drivers.
It is recommended to install the
    nvidia-driver
package.
```

`tests/fixtures/nvidia_detect_legacy.txt`:

```
Detected NVIDIA GPUs:
02:00.0 VGA compatible controller [0300]: NVIDIA Corporation GK110 [GeForce GTX 780] [10de:1004] (rev a1)
Checking card:  NVIDIA Corporation GK110 [GeForce GTX 780] [10de:1004] (rev a1)
Your card is only supported by the legacy drivers series.
It is recommended to install the
    nvidia-tesla-470-driver
package.
```

- [ ] **Step 2: Write `src/recommend.rs` with failing tests**

```rust
use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecommendationSource {
    Detected,
    FallbackDefault,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recommendation {
    pub package: String,
    pub source: RecommendationSource,
}

const FALLBACK_PACKAGE: &str = "nvidia-driver";

/// Extracts the package name `nvidia-detect` recommends. Its output puts
/// the package name alone on the line right after "It is recommended to
/// install the" (indented), so this scans for that anchor line rather
/// than a single-line regex, since the package name's own line has no
/// other distinguishing marker.
pub fn parse_nvidia_detect(output: &str) -> Option<String> {
    let mut lines = output.lines();
    while let Some(line) = lines.next() {
        if line.trim_end() == "It is recommended to install the" {
            let pkg = lines.next()?.trim();
            if !pkg.is_empty() {
                return Some(pkg.to_string());
            }
        }
    }
    None
}

pub fn recommend_package() -> Recommendation {
    let output = Command::new("nvidia-detect").output();
    let stdout = match output {
        Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout).into_owned(),
        _ => {
            return Recommendation {
                package: FALLBACK_PACKAGE.to_string(),
                source: RecommendationSource::FallbackDefault,
            }
        }
    };

    match parse_nvidia_detect(&stdout) {
        Some(package) => Recommendation { package, source: RecommendationSource::Detected },
        None => Recommendation {
            package: FALLBACK_PACKAGE.to_string(),
            source: RecommendationSource::FallbackDefault,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_standard_recommendation() {
        let fixture = include_str!("../tests/fixtures/nvidia_detect_found.txt");
        assert_eq!(parse_nvidia_detect(fixture), Some("nvidia-driver".to_string()));
    }

    #[test]
    fn parses_legacy_recommendation() {
        let fixture = include_str!("../tests/fixtures/nvidia_detect_legacy.txt");
        assert_eq!(
            parse_nvidia_detect(fixture),
            Some("nvidia-tesla-470-driver".to_string())
        );
    }

    #[test]
    fn unparseable_output_returns_none() {
        assert_eq!(parse_nvidia_detect("no useful output here\n"), None);
    }
}
```

- [ ] **Step 3: Uncomment `pub mod recommend;` in `lib.rs`**

- [ ] **Step 4: Run tests**

Run: `cd nvidia-installer && cargo test --lib recommend`
Expected: 3 passed.

- [ ] **Step 5: Commit**

```bash
git add nvidia-installer/src/lib.rs nvidia-installer/src/recommend.rs nvidia-installer/tests/fixtures/nvidia_detect_found.txt nvidia-installer/tests/fixtures/nvidia_detect_legacy.txt
git commit -m "feat(nvidia-installer): package recommendation via nvidia-detect"
```

---

## Task 3: `secureboot.rs` (parsing + key generation argv, no live enrollment)

**Files:**
- Modify: `nvidia-installer/src/lib.rs` (uncomment `pub mod secureboot;`)
- Create: `nvidia-installer/src/secureboot.rs`
- Create: `nvidia-installer/tests/fixtures/mokutil_sb_enabled.txt`
- Create: `nvidia-installer/tests/fixtures/mokutil_sb_disabled.txt`
- Create: `nvidia-installer/tests/fixtures/mokutil_list_enrolled.txt`

**Interfaces:**
- Consumes: `InstallError` from `lib.rs`.
- Produces: `pub enum SecureBootState { Enabled, Disabled, Unknown }`, `pub fn parse_sb_state(output: &str) -> SecureBootState`, `pub fn secure_boot_state() -> SecureBootState` (runs `mokutil --sb-state`), `pub const MOK_KEY_PATH: &str = "/var/lib/dkms/mok.key"`, `pub const MOK_CERT_PATH: &str = "/var/lib/dkms/mok.pub"`, `pub fn mok_keypair_exists() -> bool`, `pub fn openssl_genkey_argv() -> Vec<String>` (builds the `openssl req ...` argv from the spec, using `MOK_KEY_PATH`/`MOK_CERT_PATH`), `pub fn is_key_enrolled(list_enrolled_output: &str) -> bool` (checks whether `mokutil --list-enrolled` output contains our cert's fingerprint marker - see Step 2 note on scope), `pub fn mokutil_import_argv() -> Vec<String>`.

Note on `is_key_enrolled`: `mokutil --list-enrolled` prints full X.509 certificate details for every enrolled key, not a filename. Matching "is *our* key enrolled" precisely would need parsing the cert's Subject field and comparing it against the CN we generate in Step 2 below (`CN=dreamos nvidia-installer MOK`) - do that: `is_key_enrolled` takes the list-enrolled output and checks whether any block's `Subject:` line contains `dreamos nvidia-installer MOK`.

- [ ] **Step 1: Create fixtures**

`tests/fixtures/mokutil_sb_enabled.txt`:

```
SecureBoot enabled
```

`tests/fixtures/mokutil_sb_disabled.txt`:

```
SecureBoot disabled
```

`tests/fixtures/mokutil_list_enrolled.txt` (one matching key, one unrelated distro key):

```
[key 1]
SHA1 Fingerprint: aa:bb:cc:dd:ee:ff:00:11:22:33:44:55:66:77:88:99:00:11:22:33
	Issuer:
		CN=dreamos nvidia-installer MOK
	Subject:
		CN=dreamos nvidia-installer MOK
	Serial Number: 01
[key 2]
SHA1 Fingerprint: 11:22:33:44:55:66:77:88:99:00:aa:bb:cc:dd:ee:ff:00:11:22:33
	Issuer:
		CN=Debian Secure Boot CA
	Subject:
		CN=Debian Secure Boot Signer
	Serial Number: 02
```

- [ ] **Step 2: Write `src/secureboot.rs` with failing tests**

```rust
use crate::InstallError;
use std::path::Path;
use std::process::Command;

pub const MOK_KEY_PATH: &str = "/var/lib/dkms/mok.key";
pub const MOK_CERT_PATH: &str = "/var/lib/dkms/mok.pub";
const MOK_SUBJECT: &str = "dreamos nvidia-installer MOK";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecureBootState {
    Enabled,
    Disabled,
    Unknown,
}

pub fn parse_sb_state(output: &str) -> SecureBootState {
    let text = output.to_lowercase();
    if text.contains("secureboot enabled") {
        SecureBootState::Enabled
    } else if text.contains("secureboot disabled") {
        SecureBootState::Disabled
    } else {
        SecureBootState::Unknown
    }
}

pub fn secure_boot_state() -> SecureBootState {
    match Command::new("mokutil").arg("--sb-state").output() {
        Ok(out) => parse_sb_state(&String::from_utf8_lossy(&out.stdout)),
        Err(_) => SecureBootState::Unknown,
    }
}

pub fn mok_keypair_exists() -> bool {
    Path::new(MOK_KEY_PATH).exists() && Path::new(MOK_CERT_PATH).exists()
}

/// argv for generating the MOK signing key pair at the fixed paths dkms
/// itself checks. DER-encoded cert (`-outform DER`) because that's what
/// `mokutil --import` and the kernel's MOK verifier expect; a 100-year
/// validity avoids ever needing key rotation for this internal tool's key.
pub fn openssl_genkey_argv() -> Vec<String> {
    vec![
        "openssl".into(), "req".into(), "-new".into(), "-x509".into(),
        "-newkey".into(), "rsa:2048".into(),
        "-keyout".into(), MOK_KEY_PATH.into(),
        "-outform".into(), "DER".into(),
        "-out".into(), MOK_CERT_PATH.into(),
        "-nodes".into(), "-days".into(), "36500".into(),
        "-subj".into(), format!("/CN={MOK_SUBJECT}/"),
    ]
}

pub fn mokutil_import_argv() -> Vec<String> {
    vec!["mokutil".into(), "--import".into(), MOK_CERT_PATH.into()]
}

pub fn is_key_enrolled(list_enrolled_output: &str) -> bool {
    list_enrolled_output
        .split("[key ")
        .any(|block| {
            block.contains("Subject:")
                && block
                    .lines()
                    .skip_while(|l| !l.trim() == false && !l.contains("Subject:"))
                    .any(|l| l.contains(MOK_SUBJECT))
        })
}

pub fn ensure_mok_keypair() -> Result<(), InstallError> {
    if mok_keypair_exists() {
        return Ok(());
    }
    let argv = openssl_genkey_argv();
    let argv_refs: Vec<&str> = argv.iter().map(String::as_str).collect();
    let cmd_str = argv.join(" ");
    let output = Command::new(argv_refs[0])
        .args(&argv_refs[1..])
        .output()
        .map_err(|source| InstallError::Spawn { cmd: cmd_str.clone(), detail: source.to_string() })?;
    if !output.status.success() {
        return Err(InstallError::CommandFailed {
            cmd: cmd_str,
            code: output.status.code(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_enabled() {
        let fixture = include_str!("../tests/fixtures/mokutil_sb_enabled.txt");
        assert_eq!(parse_sb_state(fixture), SecureBootState::Enabled);
    }

    #[test]
    fn parses_disabled() {
        let fixture = include_str!("../tests/fixtures/mokutil_sb_disabled.txt");
        assert_eq!(parse_sb_state(fixture), SecureBootState::Disabled);
    }

    #[test]
    fn unparseable_is_unknown() {
        assert_eq!(parse_sb_state("garbage\n"), SecureBootState::Unknown);
    }

    #[test]
    fn finds_our_key_among_others() {
        let fixture = include_str!("../tests/fixtures/mokutil_list_enrolled.txt");
        assert!(is_key_enrolled(fixture));
    }

    #[test]
    fn absent_key_not_found() {
        let fixture = "[key 1]\nSubject:\n\tCN=Some Other Key\n";
        assert!(!is_key_enrolled(fixture));
    }

    #[test]
    fn genkey_argv_uses_fixed_paths() {
        let argv = openssl_genkey_argv();
        assert!(argv.contains(&MOK_KEY_PATH.to_string()));
        assert!(argv.contains(&MOK_CERT_PATH.to_string()));
    }
}
```

The `is_key_enrolled` skip_while line above is convoluted - simplify it in Step 2's actual write to:

```rust
pub fn is_key_enrolled(list_enrolled_output: &str) -> bool {
    list_enrolled_output
        .split("[key ")
        .any(|block| block.contains("Subject:") && block.contains(MOK_SUBJECT))
}
```

(Use this simplified version, not the `skip_while` draft above - it's equivalent and clearer since `MOK_SUBJECT` only ever legitimately appears under this key's own `Subject:`/`Issuer:` lines within its block.)

- [ ] **Step 3: Uncomment `pub mod secureboot;` in `lib.rs`**

- [ ] **Step 4: Run tests**

Run: `cd nvidia-installer && cargo test --lib secureboot`
Expected: 6 passed.

- [ ] **Step 5: Commit**

```bash
git add nvidia-installer/src/lib.rs nvidia-installer/src/secureboot.rs nvidia-installer/tests/fixtures/mokutil_sb_enabled.txt nvidia-installer/tests/fixtures/mokutil_sb_disabled.txt nvidia-installer/tests/fixtures/mokutil_list_enrolled.txt
git commit -m "feat(nvidia-installer): Secure Boot state + MOK keypair handling"
```

---

## Task 4: `exec.rs`

**Files:**
- Modify: `nvidia-installer/src/lib.rs` (uncomment `pub mod exec;`)
- Create: `nvidia-installer/src/exec.rs`

**Interfaces:**
- Consumes: `InstallError` from `lib.rs`; `secureboot::mokutil_import_argv()` (Task 3, used by `main.rs` later, not by `exec.rs` itself).
- Produces: `pub struct CmdOutput { pub stdout: String, pub stderr: String }`, `pub fn run_cmd(argv: &[&str]) -> Result<CmdOutput, InstallError>` (identical contract to `filesys_extender::exec::run_cmd`), `pub fn run_cmd_with_stdin(argv: &[&str], stdin_data: &str) -> Result<CmdOutput, InstallError>` (for piping the MOK password to `mokutil --import`), `pub fn apt_update_argv() -> Vec<String>`, `pub fn apt_install_argv(package: &str) -> Vec<String>`.

- [ ] **Step 1: Write `src/exec.rs` with failing tests**

```rust
use crate::InstallError;
use std::io::Write;
use std::process::{Command, Stdio};

#[derive(Debug, Clone)]
pub struct CmdOutput {
    pub stdout: String,
    pub stderr: String,
}

pub fn run_cmd(argv: &[&str]) -> Result<CmdOutput, InstallError> {
    let cmd_str = argv.join(" ");
    let output = Command::new(argv[0])
        .args(&argv[1..])
        .output()
        .map_err(|source| InstallError::Spawn { cmd: cmd_str.clone(), detail: source.to_string() })?;

    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    if !output.status.success() {
        return Err(InstallError::CommandFailed {
            cmd: cmd_str,
            code: output.status.code(),
            stderr,
        });
    }
    Ok(CmdOutput { stdout, stderr })
}

/// Like `run_cmd`, but writes `stdin_data` to the child's stdin before
/// waiting on it - needed for `mokutil --import`, which reads a one-time
/// enrollment password twice from stdin rather than accepting it as an
/// argument (a password would otherwise leak into `/proc/<pid>/cmdline`
/// and shell history).
pub fn run_cmd_with_stdin(argv: &[&str], stdin_data: &str) -> Result<CmdOutput, InstallError> {
    let cmd_str = argv.join(" ");
    let mut child = Command::new(argv[0])
        .args(&argv[1..])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|source| InstallError::Spawn { cmd: cmd_str.clone(), detail: source.to_string() })?;

    child
        .stdin
        .take()
        .expect("stdin was piped")
        .write_all(stdin_data.as_bytes())
        .map_err(|source| InstallError::Spawn { cmd: cmd_str.clone(), detail: source.to_string() })?;

    let output = child
        .wait_with_output()
        .map_err(|source| InstallError::Spawn { cmd: cmd_str.clone(), detail: source.to_string() })?;

    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    if !output.status.success() {
        return Err(InstallError::CommandFailed {
            cmd: cmd_str,
            code: output.status.code(),
            stderr,
        });
    }
    Ok(CmdOutput { stdout, stderr })
}

pub fn apt_update_argv() -> Vec<String> {
    vec!["apt-get".into(), "update".into()]
}

pub fn apt_install_argv(package: &str) -> Vec<String> {
    vec!["apt-get".into(), "install".into(), "-y".into(), package.to_string()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_cmd_captures_stdout() {
        let out = run_cmd(&["echo", "hello"]).unwrap();
        assert_eq!(out.stdout.trim(), "hello");
    }

    #[test]
    fn run_cmd_reports_nonzero_exit() {
        let err = run_cmd(&["false"]).unwrap_err();
        assert!(matches!(err, InstallError::CommandFailed { .. }));
    }

    #[test]
    fn run_cmd_with_stdin_pipes_input() {
        let out = run_cmd_with_stdin(&["cat"], "piped data").unwrap();
        assert_eq!(out.stdout, "piped data");
    }

    #[test]
    fn apt_install_argv_includes_package_and_yes_flag() {
        let argv = apt_install_argv("nvidia-driver");
        assert_eq!(argv, vec!["apt-get", "install", "-y", "nvidia-driver"]);
    }
}
```

- [ ] **Step 2: Uncomment `pub mod exec;` in `lib.rs`**

- [ ] **Step 3: Run tests**

Run: `cd nvidia-installer && cargo test --lib exec`
Expected: 4 passed.

- [ ] **Step 4: Commit**

```bash
git add nvidia-installer/src/lib.rs nvidia-installer/src/exec.rs
git commit -m "feat(nvidia-installer): command execution helpers for apt/mokutil"
```

---

## Task 5: `state.rs`

**Files:**
- Modify: `nvidia-installer/src/lib.rs` (uncomment `pub mod state;`)
- Create: `nvidia-installer/src/state.rs`

**Interfaces:**
- Consumes: `InstallError` from `lib.rs`.
- Produces: `pub const STATE_DIR: &str = "/var/lib/nvidia-installer"`, `pub struct State { pub mok_enrollment_pending: bool }`, `pub fn state_path() -> std::path::PathBuf` (returns `STATE_DIR/state.json`, but see Step 1's env-override note), `pub fn load_state() -> State` (missing/corrupt file -> default `State { mok_enrollment_pending: false }`, never errors - matches spec's "at most one field" simplicity), `pub fn save_state(state: &State) -> Result<(), InstallError>`, `pub fn clear_state() -> Result<(), InstallError>` (removes the file if present; a missing file is not an error).

- [ ] **Step 1: Write `src/state.rs` with failing tests**

Tests must not touch the real `/var/lib/nvidia-installer` (no root, and tests shouldn't mutate system state). `state_path()` reads an override env var first, defaulting to the real path - the same escape-hatch pattern `filesys-extender`'s `run-filesys-extender.sh` uses for `FILESYS_EXTENDER_SKIP_ROOT`.

```rust
use crate::InstallError;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const STATE_DIR: &str = "/var/lib/nvidia-installer";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct State {
    pub mok_enrollment_pending: bool,
}

impl Default for State {
    fn default() -> Self {
        State { mok_enrollment_pending: false }
    }
}

/// Test override: `NVIDIA_INSTALLER_STATE_DIR` redirects state.json
/// elsewhere so unit tests never touch real system state.
pub fn state_dir() -> PathBuf {
    match std::env::var_os("NVIDIA_INSTALLER_STATE_DIR") {
        Some(dir) => PathBuf::from(dir),
        None => PathBuf::from(STATE_DIR),
    }
}

pub fn state_path() -> PathBuf {
    state_dir().join("state.json")
}

pub fn load_state() -> State {
    let Ok(contents) = std::fs::read_to_string(state_path()) else {
        return State::default();
    };
    serde_json::from_str(&contents).unwrap_or_default()
}

pub fn save_state(state: &State) -> Result<(), InstallError> {
    let dir = state_dir();
    std::fs::create_dir_all(&dir).map_err(|source| InstallError::Spawn {
        cmd: format!("mkdir -p {}", dir.display()),
        detail: source.to_string(),
    })?;
    let json = serde_json::to_string_pretty(state).map_err(|source| InstallError::Parse {
        what: "state.json".into(),
        detail: source.to_string(),
    })?;
    std::fs::write(state_path(), json).map_err(|source| InstallError::Spawn {
        cmd: format!("write {}", state_path().display()),
        detail: source.to_string(),
    })
}

pub fn clear_state() -> Result<(), InstallError> {
    match std::fs::remove_file(state_path()) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(InstallError::Spawn {
            cmd: format!("rm {}", state_path().display()),
            detail: source.to_string(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // Tests mutate a process-wide env var, so they must not run concurrently.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn with_temp_state_dir<F: FnOnce()>(f: F) {
        let _guard = ENV_LOCK.lock().unwrap();
        let tmp = std::env::temp_dir().join(format!("nvidia-installer-test-{}", std::process::id()));
        std::env::set_var("NVIDIA_INSTALLER_STATE_DIR", &tmp);
        f();
        let _ = std::fs::remove_dir_all(&tmp);
        std::env::remove_var("NVIDIA_INSTALLER_STATE_DIR");
    }

    #[test]
    fn missing_file_loads_default() {
        with_temp_state_dir(|| {
            assert_eq!(load_state(), State { mok_enrollment_pending: false });
        });
    }

    #[test]
    fn save_then_load_roundtrips() {
        with_temp_state_dir(|| {
            save_state(&State { mok_enrollment_pending: true }).unwrap();
            assert_eq!(load_state(), State { mok_enrollment_pending: true });
        });
    }

    #[test]
    fn clear_removes_file_and_is_idempotent() {
        with_temp_state_dir(|| {
            save_state(&State { mok_enrollment_pending: true }).unwrap();
            clear_state().unwrap();
            assert_eq!(load_state(), State::default());
            clear_state().unwrap(); // second call: file already gone, still Ok
        });
    }
}
```

- [ ] **Step 2: Uncomment `pub mod state;` in `lib.rs`**

- [ ] **Step 3: Run tests**

Run: `cd nvidia-installer && cargo test --lib state`
Expected: 3 passed.

- [ ] **Step 4: Commit**

```bash
git add nvidia-installer/src/lib.rs nvidia-installer/src/state.rs
git commit -m "feat(nvidia-installer): pending-reboot state file"
```

---

## Task 6: `main.rs` GTK4 wizard

**Files:**
- Create: `nvidia-installer/src/main.rs`

**Interfaces:**
- Consumes: `detect::{detect_gpus, GpuDevice, CurrentDriver}` (Task 1), `recommend::{recommend_package, Recommendation, RecommendationSource}` (Task 2), `secureboot::{secure_boot_state, SecureBootState, ensure_mok_keypair, mokutil_import_argv, is_key_enrolled}` (Task 3), `exec::{run_cmd, run_cmd_with_stdin, apt_update_argv, apt_install_argv}` (Task 4), `state::{load_state, save_state, clear_state, State}` (Task 5).
- Produces: the `nvidia-installer` binary. Nothing downstream consumes `main.rs`.

This task has no unit tests of its own (GTK UI code, matches `filesys-extender/src/main.rs` - untested by `cargo test`, verified by manual smoke test in Task 7). Steps below are single large additions rather than red/green cycles, since there's no test harness for GTK widget trees.

- [ ] **Step 1: Write root elevation + entry point**

```rust
use std::os::unix::process::CommandExt;
use std::process::Command;

fn ensure_root() {
    // Escape hatch for UI-only iteration (see scripts/run-nvidia-installer.sh):
    // skips the pkexec re-exec so the wizard's screens can be exercised
    // without a polkit prompt each run. apt/mokutil steps still fail
    // without real root, as expected.
    if std::env::var_os("NVIDIA_INSTALLER_SKIP_ROOT").is_some() {
        return;
    }
    // SAFETY: geteuid() is a pure syscall with no preconditions.
    let euid = unsafe { libc::geteuid() };
    if euid == 0 {
        return;
    }
    let self_path = std::env::current_exe().expect("cannot resolve own executable path");
    let err = Command::new("pkexec").arg(self_path).exec();
    // exec() only returns on failure.
    eprintln!("failed to re-exec via pkexec: {err}");
    std::process::exit(1);
}

fn main() {
    ensure_root();
    run_app();
}
```

- [ ] **Step 2: Write shared imports, `spacer()`, and page-independent helpers**

```rust
use gtk4::prelude::*;
use gtk4::{
    Application, ApplicationWindow, Box as GtkBox, Button, CheckButton, Entry, Label,
    Orientation, PasswordEntry, Stack, TextView,
};
use nvidia_installer::detect::{detect_gpus, CurrentDriver, GpuDevice};
use nvidia_installer::exec::{apt_install_argv, apt_update_argv, run_cmd, run_cmd_with_stdin};
use nvidia_installer::recommend::{recommend_package, Recommendation, RecommendationSource};
use nvidia_installer::secureboot::{
    ensure_mok_keypair, is_key_enrolled, mokutil_import_argv, secure_boot_state, SecureBootState,
};
use nvidia_installer::state::{clear_state, load_state, save_state, State};
use std::cell::RefCell;
use std::rc::Rc;

fn spacer() -> GtkBox {
    let spacer = GtkBox::new(Orientation::Vertical, 0);
    spacer.set_vexpand(true);
    spacer
}

fn driver_label(driver: &CurrentDriver) -> &'static str {
    match driver {
        CurrentDriver::Nouveau => "nouveau (open source)",
        CurrentDriver::Nvidia => "nvidia (proprietary, already installed)",
        CurrentDriver::None => "none bound",
    }
}
```

- [ ] **Step 3: Write the Welcome and Detect pages**

```rust
fn build_welcome_page(on_next: impl Fn() + 'static) -> GtkBox {
    let container = GtkBox::new(Orientation::Vertical, 12);
    container.set_margin_top(16);
    container.set_margin_bottom(16);
    container.set_margin_start(16);
    container.set_margin_end(16);

    let title = Label::new(None);
    title.set_markup("<span size='xx-large' weight='bold'>DreamOS NVIDIA Driver Installer</span>");
    title.set_halign(gtk4::Align::Start);
    container.append(&title);

    let intro = Label::new(Some(
        "This wizard detects NVIDIA GPUs and installs the proprietary \
         driver via apt.\n\n\
         How it works:\n\
         1. Detect - find NVIDIA GPU(s) and the driver currently bound.\n\
         2. Recommend - the correct driver package for your card.\n\
         3. Secure Boot - if enabled, sets up DKMS module signing so the \
         driver actually loads after reboot (a Secure Boot system \
         otherwise silently rejects unsigned third-party kernel modules).\n\
         4. Confirm - review the exact commands before anything runs.\n\
         5. Install - watch the apt install run, with live output.\n\n\
         This tool never edits your apt sources and only installs the \
         one package you confirm.",
    ));
    intro.set_wrap(true);
    intro.set_justify(gtk4::Justification::Left);
    intro.set_halign(gtk4::Align::Start);
    container.append(&intro);
    container.append(&spacer());

    let next_button = Button::with_label("Get Started");
    next_button.set_halign(gtk4::Align::End);
    next_button.connect_clicked(move |_| on_next());
    container.append(&next_button);

    container
}

fn build_detect_page(
    gpus: Rc<RefCell<Vec<GpuDevice>>>,
    on_next: impl Fn() + 'static,
) -> GtkBox {
    let container = GtkBox::new(Orientation::Vertical, 8);
    let summary = Label::new(None);
    summary.set_wrap(true);
    summary.set_halign(gtk4::Align::Start);
    summary.set_valign(gtk4::Align::Start);
    let next_button = Button::with_label("Next");
    next_button.set_sensitive(false);

    container.append(&summary);
    container.append(&spacer());
    container.append(&next_button);

    {
        let gpus = gpus.clone();
        let summary = summary.clone();
        let next_button = next_button.clone();
        container.connect_map(move |_| {
            let detected = detect_gpus().unwrap_or_default();
            if detected.is_empty() {
                summary.set_text("No NVIDIA GPU detected on this system. Nothing to do.");
                next_button.set_sensitive(false);
            } else {
                let lines: Vec<String> = detected
                    .iter()
                    .map(|g| format!("{}: {} (driver: {})", g.pci_slot, g.model, driver_label(&g.driver)))
                    .collect();
                summary.set_text(&lines.join("\n"));
                next_button.set_sensitive(true);
            }
            *gpus.borrow_mut() = detected;
        });
    }

    next_button.connect_clicked(move |_| on_next());
    container
}
```

- [ ] **Step 4: Write the Recommend page**

```rust
fn build_recommend_page(
    chosen_package: Rc<RefCell<Option<String>>>,
    on_next: impl Fn() + 'static,
) -> GtkBox {
    let container = GtkBox::new(Orientation::Vertical, 8);
    let source_label = Label::new(None);
    source_label.set_halign(gtk4::Align::Start);
    let entry = Entry::new();
    let next_button = Button::with_label("Next");

    container.append(&source_label);
    container.append(&entry);
    container.append(&spacer());
    container.append(&next_button);

    {
        let chosen_package = chosen_package.clone();
        let source_label = source_label.clone();
        let entry = entry.clone();
        container.connect_map(move |_| {
            let Recommendation { package, source } = recommend_package();
            source_label.set_text(match source {
                RecommendationSource::Detected => "Detected via nvidia-detect:",
                RecommendationSource::FallbackDefault => {
                    "nvidia-detect unavailable - best guess (edit if you know your card needs a different package):"
                }
            });
            entry.set_text(&package);
            *chosen_package.borrow_mut() = Some(package);
        });
    }

    {
        let chosen_package = chosen_package.clone();
        entry.connect_changed(move |entry| {
            *chosen_package.borrow_mut() = Some(entry.text().to_string());
        });
    }

    next_button.connect_clicked(move |_| on_next());
    container
}
```

- [ ] **Step 5: Write the Secure Boot page**

```rust
enum SecureBootOutcome {
    NotApplicable,
    OptedOut,
    Enrolled,
    Failed(String),
}

fn build_secureboot_page(
    outcome: Rc<RefCell<SecureBootOutcome>>,
    on_next: impl Fn() + 'static,
) -> GtkBox {
    let container = GtkBox::new(Orientation::Vertical, 8);
    let status_label = Label::new(None);
    status_label.set_wrap(true);
    status_label.set_halign(gtk4::Align::Start);
    let opt_out = CheckButton::with_label("I'll handle Secure Boot / module signing myself");
    let pass1 = PasswordEntry::new();
    pass1.set_show_peek_icon(true);
    let pass2 = PasswordEntry::new();
    pass2.set_show_peek_icon(true);
    let enroll_button = Button::with_label("Generate key and enroll");
    let error_label = Label::new(None);
    error_label.set_halign(gtk4::Align::Start);
    let next_button = Button::with_label("Next");

    container.append(&status_label);
    container.append(&opt_out);
    container.append(&pass1);
    container.append(&pass2);
    container.append(&enroll_button);
    container.append(&error_label);
    container.append(&spacer());
    container.append(&next_button);

    {
        let status_label = status_label.clone();
        let opt_out = opt_out.clone();
        let pass1 = pass1.clone();
        let pass2 = pass2.clone();
        let enroll_button = enroll_button.clone();
        let next_button = next_button.clone();
        let outcome = outcome.clone();
        container.connect_map(move |_| {
            match secure_boot_state() {
                SecureBootState::Disabled => {
                    status_label.set_text("Secure Boot is off - no module signing needed.");
                    opt_out.set_visible(false);
                    pass1.set_visible(false);
                    pass2.set_visible(false);
                    enroll_button.set_visible(false);
                    *outcome.borrow_mut() = SecureBootOutcome::NotApplicable;
                    next_button.set_sensitive(true);
                }
                SecureBootState::Enabled | SecureBootState::Unknown => {
                    status_label.set_text(
                        "Secure Boot is on. Without an enrolled signing key, the \
                         driver's kernel module will build but fail to load after \
                         reboot. Enter a one-time enrollment password (min 8 chars, \
                         shown twice) to generate and enroll a signing key, or opt \
                         out and handle it yourself.",
                    );
                    next_button.set_sensitive(false);
                }
            }
        });
    }

    {
        let next_button = next_button.clone();
        let pass1 = pass1.clone();
        let pass2 = pass2.clone();
        let enroll_button = enroll_button.clone();
        opt_out.connect_toggled(move |btn| {
            let opted_out = btn.is_active();
            pass1.set_sensitive(!opted_out);
            pass2.set_sensitive(!opted_out);
            enroll_button.set_sensitive(!opted_out);
            next_button.set_sensitive(opted_out);
        });
    }

    {
        let outcome = outcome.clone();
        opt_out.connect_toggled(move |btn| {
            if btn.is_active() {
                *outcome.borrow_mut() = SecureBootOutcome::OptedOut;
            }
        });
    }

    {
        let pass1 = pass1.clone();
        let pass2 = pass2.clone();
        let error_label = error_label.clone();
        let next_button = next_button.clone();
        let outcome = outcome.clone();
        enroll_button.connect_clicked(move |_| {
            let p1 = pass1.text().to_string();
            let p2 = pass2.text().to_string();
            if p1.len() < 8 {
                error_label.set_text("Password must be at least 8 characters.");
                return;
            }
            if p1 != p2 {
                error_label.set_text("Passwords don't match.");
                return;
            }
            if let Err(e) = ensure_mok_keypair() {
                error_label.set_text(&format!("Key generation failed: {e}"));
                *outcome.borrow_mut() = SecureBootOutcome::Failed(e.to_string());
                return;
            }
            let argv = mokutil_import_argv();
            let argv_refs: Vec<&str> = argv.iter().map(String::as_str).collect();
            // mokutil --import reads the password twice from stdin.
            let stdin_data = format!("{p1}\n{p1}\n");
            match run_cmd_with_stdin(&argv_refs, &stdin_data) {
                Ok(_) => {
                    error_label.set_text("Key enrolled - will take effect after reboot.");
                    *outcome.borrow_mut() = SecureBootOutcome::Enrolled;
                    next_button.set_sensitive(true);
                }
                Err(e) => {
                    error_label.set_text(&format!("Enrollment failed: {e}"));
                    *outcome.borrow_mut() = SecureBootOutcome::Failed(e.to_string());
                }
            }
        });
    }

    next_button.connect_clicked(move |_| on_next());
    container
}
```

- [ ] **Step 6: Write the Reboot-Required, Confirm, Executing, Result, and Verify pages**

```rust
fn build_reboot_required_page() -> GtkBox {
    let container = GtkBox::new(Orientation::Vertical, 8);
    let label = Label::new(Some(
        "A signing key was generated and an enrollment request submitted.\n\n\
         Reboot now. On the blue 'MOK Management' screen, choose 'Enroll \
         MOK', then 'Continue', and enter the same password you just typed \
         to confirm enrollment.\n\n\
         After rebooting, re-run this tool to continue installing the driver.",
    ));
    label.set_wrap(true);
    label.set_halign(gtk4::Align::Start);
    container.append(&label);
    container
}

fn build_confirm_page(
    chosen_package: Rc<RefCell<Option<String>>>,
    outcome: Rc<RefCell<SecureBootOutcome>>,
    on_apply: impl Fn() + 'static,
) -> GtkBox {
    let container = GtkBox::new(Orientation::Vertical, 8);
    let summary = Label::new(None);
    summary.set_wrap(true);
    summary.set_halign(gtk4::Align::Start);
    let apply_button = Button::with_label("Apply");

    container.append(&summary);
    container.append(&spacer());
    container.append(&apply_button);

    {
        let chosen_package = chosen_package.clone();
        let outcome = outcome.clone();
        let summary = summary.clone();
        container.connect_map(move |_| {
            let package = chosen_package.borrow().clone().unwrap_or_default();
            let sb_line = match &*outcome.borrow() {
                SecureBootOutcome::NotApplicable => "Secure Boot: off, no signing needed.",
                SecureBootOutcome::OptedOut => "Secure Boot: on, signing skipped at your request.",
                SecureBootOutcome::Enrolled => "Secure Boot: key enrolled this run.",
                SecureBootOutcome::Failed(_) => "Secure Boot: enrollment failed, proceeding without it.",
            };
            summary.set_text(&format!(
                "About to run:\n  apt-get update\n  apt-get install -y {package}\n\n{sb_line}"
            ));
        });
    }

    apply_button.connect_clicked(move |_| on_apply());
    container
}

#[derive(Debug)]
enum ExecMsg {
    Line(String),
    Failed(String),
    Finished(bool),
}

fn build_executing_page(
    chosen_package: Rc<RefCell<Option<String>>>,
    result_text: Rc<RefCell<String>>,
    stack: Stack,
) -> (GtkBox, Rc<dyn Fn()>) {
    let container = GtkBox::new(Orientation::Vertical, 8);
    let log_view = TextView::new();
    log_view.set_editable(false);
    container.append(&log_view);

    let start: Rc<dyn Fn()> = Rc::new(move || {
        let Some(package) = chosen_package.borrow().clone() else { return };
        let (tx, rx) = std::sync::mpsc::channel::<ExecMsg>();

        {
            let buf = log_view.buffer();
            let result_text = result_text.clone();
            let stack = stack.clone();
            gtk4::glib::source::timeout_add_local(std::time::Duration::from_millis(50), move || {
                let mut finished = false;
                for msg in rx.try_iter() {
                    let mut end = buf.end_iter();
                    match &msg {
                        ExecMsg::Line(line) => buf.insert(&mut end, &format!("{line}\n")),
                        ExecMsg::Failed(err) => {
                            buf.insert(&mut end, &format!("FAILED: {err}\n"));
                            result_text.borrow_mut().push_str(&format!("FAILED: {err}\n"));
                        }
                        ExecMsg::Finished(success) => {
                            buf.insert(&mut end, if *success { "Done.\n" } else { "Stopped after failure.\n" });
                            stack.set_visible_child_name("result");
                            finished = true;
                        }
                    }
                }
                if finished { gtk4::glib::ControlFlow::Break } else { gtk4::glib::ControlFlow::Continue }
            });
        }

        std::thread::spawn(move || {
            let update_argv: Vec<&str> = apt_update_argv().iter().map(String::as_str).collect();
            let _ = tx.send(ExecMsg::Line("==> apt-get update".into()));
            let update_result = run_cmd(&update_argv);
            if let Err(e) = &update_result {
                let _ = tx.send(ExecMsg::Failed(e.to_string()));
                let _ = tx.send(ExecMsg::Finished(false));
                return;
            }
            let _ = tx.send(ExecMsg::Line(update_result.unwrap().stdout));

            let install_owned = apt_install_argv(&package);
            let install_argv: Vec<&str> = install_owned.iter().map(String::as_str).collect();
            let _ = tx.send(ExecMsg::Line(format!("==> apt-get install -y {package}")));
            let success = match run_cmd(&install_argv) {
                Ok(out) => {
                    let _ = tx.send(ExecMsg::Line(out.stdout));
                    true
                }
                Err(e) => {
                    let _ = tx.send(ExecMsg::Failed(e.to_string()));
                    false
                }
            };
            let _ = tx.send(ExecMsg::Finished(success));
        });
    });

    (container, start)
}

fn build_result_page(result_text: Rc<RefCell<String>>) -> GtkBox {
    let container = GtkBox::new(Orientation::Vertical, 8);
    let label = Label::new(None);
    label.set_wrap(true);
    container.append(&label);
    container.connect_map(move |_| {
        let text = result_text.borrow();
        label.set_text(if text.is_empty() {
            "Driver installed. Reboot to load it (or, if Secure Boot enrollment \
             happened this run, reboot and complete MOK enrollment first)."
        } else {
            &text
        });
    });
    container
}

fn build_verify_page(on_continue: impl Fn() + 'static, on_done: impl Fn() + 'static) -> GtkBox {
    let container = GtkBox::new(Orientation::Vertical, 8);
    let label = Label::new(None);
    label.set_wrap(true);
    label.set_halign(gtk4::Align::Start);
    container.append(&label);
    container.append(&spacer());

    let continue_button = Button::with_label("Continue to driver install");
    let done_button = Button::with_label("Done");
    container.append(&continue_button);
    container.append(&done_button);

    {
        let label = label.clone();
        let continue_button = continue_button.clone();
        let done_button = done_button.clone();
        container.connect_map(move |_| {
            let enrolled = run_cmd(&["mokutil", "--list-enrolled"])
                .map(|out| is_key_enrolled(&out.stdout))
                .unwrap_or(false);
            let module_loaded = run_cmd(&["lsmod"])
                .map(|out| out.stdout.lines().any(|l| l.starts_with("nvidia ")))
                .unwrap_or(false);

            let mok_line = if enrolled { "MOK key: enrolled." } else { "MOK key: still not enrolled - did you complete the blue MokManager screen?" };
            let module_line = if module_loaded { "nvidia module: loaded." } else { "nvidia module: not loaded yet." };
            label.set_text(&format!("{mok_line}\n{module_line}"));

            if enrolled {
                let _ = clear_state();
            }
            continue_button.set_visible(enrolled && !module_loaded);
            done_button.set_visible(enrolled && module_loaded);
        });
    }

    continue_button.connect_clicked(move |_| on_continue());
    done_button.connect_clicked(move |_| on_done());
    container
}
```

- [ ] **Step 7: Write `run_app()` wiring all pages into the `Stack`**

```rust
fn run_app() {
    let app = Application::builder()
        .application_id("dev.dreamos.nvidia-installer")
        .build();

    app.connect_activate(|app| {
        let stack = Stack::new();
        let gpus: Rc<RefCell<Vec<GpuDevice>>> = Rc::new(RefCell::new(Vec::new()));
        let chosen_package: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));
        let sb_outcome = Rc::new(RefCell::new(SecureBootOutcome::NotApplicable));
        let result_text: Rc<RefCell<String>> = Rc::new(RefCell::new(String::new()));

        if load_state().mok_enrollment_pending {
            let stack_for_verify = stack.clone();
            let stack_for_confirm_nav = stack.clone();
            let verify_page = build_verify_page(
                move || stack_for_verify.set_visible_child_name("confirm"),
                {
                    let app = app.clone();
                    move || app.quit()
                },
            );
            stack.add_named(&verify_page, Some("verify"));

            let confirm_page = build_confirm_page(chosen_package.clone(), sb_outcome.clone(), {
                let stack = stack_for_confirm_nav.clone();
                move || stack.set_visible_child_name("executing")
            });
            stack.add_named(&confirm_page, Some("confirm"));

            let (executing_page, start_exec) =
                build_executing_page(chosen_package.clone(), result_text.clone(), stack_for_confirm_nav.clone());
            stack.add_named(&executing_page, Some("executing"));
            let confirm_apply_start = start_exec.clone();
            // Re-wire Confirm's Apply to also kick off execution (the closure
            // above only navigates); simplest is to rebuild Confirm with both:
            let _ = confirm_apply_start;

            let result_page = build_result_page(result_text.clone());
            stack.add_named(&result_page, Some("result"));

            stack.set_visible_child_name("verify");
        } else {
            let welcome = build_welcome_page({
                let stack = stack.clone();
                move || stack.set_visible_child_name("detect")
            });
            stack.add_named(&welcome, Some("welcome"));

            let detect_page = build_detect_page(gpus.clone(), {
                let stack = stack.clone();
                move || stack.set_visible_child_name("recommend")
            });
            stack.add_named(&detect_page, Some("detect"));

            let recommend_page = build_recommend_page(chosen_package.clone(), {
                let stack = stack.clone();
                move || stack.set_visible_child_name("secureboot")
            });
            stack.add_named(&recommend_page, Some("recommend"));

            let secureboot_page = build_secureboot_page(sb_outcome.clone(), {
                let stack = stack.clone();
                let sb_outcome = sb_outcome.clone();
                move || {
                    if matches!(&*sb_outcome.borrow(), SecureBootOutcome::Enrolled) {
                        let _ = save_state(&State { mok_enrollment_pending: true });
                        stack.set_visible_child_name("reboot_required");
                    } else {
                        stack.set_visible_child_name("confirm");
                    }
                }
            });
            stack.add_named(&secureboot_page, Some("secureboot"));

            let reboot_page = build_reboot_required_page();
            stack.add_named(&reboot_page, Some("reboot_required"));

            let (executing_page, start_exec) =
                build_executing_page(chosen_package.clone(), result_text.clone(), stack.clone());
            let start_exec_for_confirm = start_exec.clone();
            let confirm_page = build_confirm_page(chosen_package.clone(), sb_outcome.clone(), {
                let stack = stack.clone();
                move || {
                    start_exec_for_confirm();
                    stack.set_visible_child_name("executing");
                }
            });
            stack.add_named(&confirm_page, Some("confirm"));
            stack.add_named(&executing_page, Some("executing"));

            let result_page = build_result_page(result_text.clone());
            stack.add_named(&result_page, Some("result"));

            stack.set_visible_child_name("welcome");
        }

        let window = ApplicationWindow::builder()
            .application(app)
            .title("DreamOS NVIDIA Driver Installer")
            .default_width(640)
            .default_height(480)
            .child(&stack)
            .build();
        window.present();
    });

    app.run();
}
```

Note on Step 7: the pending-state branch above builds a `confirm` page but the `build_confirm_page`'s `on_apply` closure there only navigates to `executing` without calling `start_exec` - fix this by giving `build_confirm_page` an `on_apply` closure that always does both (navigate AND start), matching the non-pending branch's pattern. Concretely, replace the pending-state branch's confirm/executing wiring with the same `start_exec_for_confirm` pattern used in the non-pending branch:

```rust
            let (executing_page, start_exec) =
                build_executing_page(chosen_package.clone(), result_text.clone(), stack.clone());
            let start_exec_for_confirm = start_exec.clone();
            let confirm_page = build_confirm_page(chosen_package.clone(), sb_outcome.clone(), {
                let stack = stack.clone();
                move || {
                    start_exec_for_confirm();
                    stack.set_visible_child_name("executing");
                }
            });
            stack.add_named(&confirm_page, Some("confirm"));
            stack.add_named(&executing_page, Some("executing"));
```

(Use this in both branches instead of the draft pending-branch wiring in Step 7's first code block, which left `start_exec` unused via `let _ = confirm_apply_start;` as a placeholder - that line must not remain in the final code.)

- [ ] **Step 8: Build and fix compile errors**

Run: `cd nvidia-installer && cargo build`
Expected: builds cleanly. Fix any borrow-checker issues from cloning `Rc`s into closures (follow the exact clone-before-move pattern shown in each snippet above - every closure that outlives its enclosing block clones its own `Rc` first).

- [ ] **Step 9: Commit**

```bash
git add nvidia-installer/src/main.rs
git commit -m "feat(nvidia-installer): GTK4 wizard UI"
```

---

## Task 7: Run script + README

**Files:**
- Create: `scripts/run-nvidia-installer.sh`
- Create: `nvidia-installer/README.md`

**Interfaces:**
- Consumes: `NVIDIA_INSTALLER_SKIP_ROOT` env var (Task 6, Step 1).
- Produces: nothing consumed by other tasks - this is the terminal task.

- [ ] **Step 1: Write `scripts/run-nvidia-installer.sh`**

```bash
#!/usr/bin/env bash
#
# Build and run nvidia-installer for UI iteration, without a real pkexec
# elevation prompt each time (NVIDIA_INSTALLER_SKIP_ROOT=1 makes main()
# skip re-exec via pkexec). Detection (lspci/nvidia-detect/mokutil) works
# fine unprivileged; anything that actually shells out to apt-get install
# or mokutil --import/openssl key generation will fail without real root,
# which is expected here - this script is for exercising the wizard's
# screens, not a full install.
#
# Usage:
#   ./scripts/run-nvidia-installer.sh
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/../nvidia-installer"
cargo build
NVIDIA_INSTALLER_SKIP_ROOT=1 ./target/debug/nvidia-installer
```

- [ ] **Step 2: Make it executable**

Run: `chmod 755 scripts/run-nvidia-installer.sh`

- [ ] **Step 3: Write `nvidia-installer/README.md`**

```markdown
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
```

- [ ] **Step 4: Commit**

```bash
git add scripts/run-nvidia-installer.sh nvidia-installer/README.md
git commit -m "feat(nvidia-installer): add run script and README"
```

---

## Self-Review Notes

- **Spec coverage:** Detect (Task 1) - covered. Recommend (Task 2) - covered. Secure Boot state + MOK keygen/enrollment (Task 3 parsing, Task 6 UI wiring) - covered. Install via apt (Task 4, Task 6) - covered. State machine / pending-reboot Verify screen (Task 5, Task 6) - covered. Error handling convention (verbatim command/exit/stderr) - `InstallError` in Task 1 matches `filesys_extender::DiskOpError` exactly. Run script + README (Task 7) - covered. ISO staging - correctly out of scope per spec Non-goals, no task for it.
- **Placeholder scan:** the one `let _ = confirm_apply_start;` line that appeared in Step 7's first draft is explicitly called out as not-final and replaced with real wiring in the same step - no other TBD/placeholder patterns remain.
- **Type consistency:** `InstallError` (Task 1) used identically by `detect.rs`, `recommend.rs` (doesn't need it - never returns `Result`), `secureboot.rs`, `exec.rs`, `state.rs`. `CmdOutput` (Task 4) has the same two fields (`stdout`, `stderr`) used consistently in Task 6's `run_cmd` call sites. `State { mok_enrollment_pending: bool }` (Task 5) is the same shape used in Task 6's `save_state`/`load_state` calls.
