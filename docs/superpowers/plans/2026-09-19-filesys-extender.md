# filesys-extender Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build `filesys-extender`, a GTK4 GUI tool that creates or
grows an ext4 `persistence` partition in the unused space of a dreamos
live USB stick, and writes its `persistence.conf`.

**Architecture:** Single Rust crate under `dreamos-tools/filesys-extender/`.
`disk.rs` + `exec.rs` hold all disk-detection, planning and command-execution
logic (no GTK dependency, fully unit-testable against fixture text).
`main.rs` is a GTK4 wizard (5 screens: disk list -> inspect -> confirm ->
executing -> result) that calls into that logic. The whole binary
self-elevates via `pkexec` at startup, mirroring the existing
`install-dreamos.desktop` -> `pkexec calamares` pattern in the `dreamos` repo.

**Tech Stack:** Rust (edition 2021), `gtk4` 0.10 (`v4_10` feature),
`serde`/`serde_json`, `thiserror`, `libc`.

**Spec:** [`docs/superpowers/specs/2026-09-19-filesys-extender-design.md`](../specs/2026-09-19-filesys-extender-design.md)

## Global Constraints

- `gtk4 = { version = "0.10", features = ["v4_10"] }` - matches OpusDM's pin exactly.
- ext4 only, no filesystem choice in the UI.
- Whole app runs as root via `pkexec filesys-extender` (self-re-exec if not root) - no separate privileged helper binary.
- No automatic rollback of partially-completed disk operations - report exact state and stop.
- Confirm screen must require the device path typed exactly before "Apply" is enabled - no click-through confirm on a destructive op.
- Crate layout: `src/lib.rs` (re-exports), `src/disk.rs`, `src/exec.rs` (no GTK deps), `src/main.rs` (GTK4 UI). No shared Cargo workspace across tools yet.
- Only disks with `rm=true` (removable) from `lsblk` are ever listed as targets.

---

## File Structure

```
dreamos-tools/
  filesys-extender/
    Cargo.toml
    src/
      lib.rs        # re-exports disk:: and exec:: for main.rs and tests
      disk.rs        # lsblk/parted parsing, Disk/PartitionInfo/PartedEntry/Plan, compute_plan, live-boot detection
      exec.rs         # run_cmd, Step, steps_for_plan, execute_steps, write_persistence_conf
      main.rs          # GTK4 wizard, self-elevation
    tests/
      fixtures/
        lsblk_with_persistence.json
        lsblk_mixed.json
        parted_free_with_space.txt
    README.md
```

---

### Task 1: Crate scaffold, error type, command runner

**Files:**
- Create: `filesys-extender/Cargo.toml`
- Create: `filesys-extender/src/lib.rs`
- Create: `filesys-extender/src/exec.rs`

**Interfaces:**
- Produces: `pub enum DiskOpError` (variants `CommandFailed { cmd, code: Option<i32>, stderr }`, `Spawn { cmd, detail }`, `Parse { what, detail }`), `pub struct CmdOutput { pub stdout: String, pub stderr: String }`, `pub fn run_cmd(argv: &[&str]) -> Result<CmdOutput, DiskOpError>`.

- [ ] **Step 1: Create the crate**

```bash
cd /home/fabio/dev/projects/dreamos-tools
cargo new --bin filesys-extender
```

- [ ] **Step 2: Write `Cargo.toml`**

```toml
[package]
name = "filesys-extender"
version = "0.1.0"
edition = "2021"
rust-version = "1.91"

[dependencies]
gtk4 = { version = "0.10", features = ["v4_10"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"
libc = "0.2"
```

- [ ] **Step 3: Write the failing test for `run_cmd`**

Create `filesys-extender/src/exec.rs`:

```rust
use crate::DiskOpError;
use std::process::Command;

#[derive(Debug, Clone)]
pub struct CmdOutput {
    pub stdout: String,
    pub stderr: String,
}

pub fn run_cmd(argv: &[&str]) -> Result<CmdOutput, DiskOpError> {
    todo!("implemented in step 4")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_cmd_captures_stdout_on_success() {
        let out = run_cmd(&["echo", "hello"]).unwrap();
        assert_eq!(out.stdout.trim(), "hello");
    }

    #[test]
    fn run_cmd_returns_command_failed_on_nonzero_exit() {
        let err = run_cmd(&["false"]).unwrap_err();
        match err {
            DiskOpError::CommandFailed { cmd, code, .. } => {
                assert_eq!(cmd, "false");
                assert_eq!(code, Some(1));
            }
            other => panic!("expected CommandFailed, got {other:?}"),
        }
    }

    #[test]
    fn run_cmd_returns_spawn_error_for_missing_binary() {
        let err = run_cmd(&["definitely-not-a-real-binary-xyz"]).unwrap_err();
        assert!(matches!(err, DiskOpError::Spawn { .. }));
    }
}
```

- [ ] **Step 4: Write `lib.rs` with the error type, then implement `run_cmd`**

`filesys-extender/src/lib.rs`:

```rust
pub mod disk;
pub mod exec;

use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum DiskOpError {
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

(`disk.rs` doesn't exist yet - create an empty `filesys-extender/src/disk.rs` with just `// placeholder, filled in Task 2` so the crate compiles.)

Replace the `todo!()` in `exec.rs`:

```rust
pub fn run_cmd(argv: &[&str]) -> Result<CmdOutput, DiskOpError> {
    let cmd_str = argv.join(" ");
    let output = Command::new(argv[0])
        .args(&argv[1..])
        .output()
        .map_err(|source| DiskOpError::Spawn {
            cmd: cmd_str.clone(),
            detail: source.to_string(),
        })?;

    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    if !output.status.success() {
        return Err(DiskOpError::CommandFailed {
            cmd: cmd_str,
            code: output.status.code(),
            stderr,
        });
    }

    Ok(CmdOutput { stdout, stderr })
}
```

Delete the auto-generated `src/main.rs` placeholder content and replace with a one-liner so `cargo test` (lib-only tests) still builds a valid binary target:

```rust
fn main() {
    println!("filesys-extender: UI not implemented yet (see Task 7+)");
}
```

- [ ] **Step 5: Run the tests**

```bash
cd filesys-extender && cargo test exec::
```
Expected: 3 tests pass (`run_cmd_captures_stdout_on_success`, `run_cmd_returns_command_failed_on_nonzero_exit`, `run_cmd_returns_spawn_error_for_missing_binary`).

- [ ] **Step 6: Commit**

```bash
cd /home/fabio/dev/projects/dreamos-tools
git add filesys-extender/Cargo.toml filesys-extender/src/lib.rs filesys-extender/src/exec.rs filesys-extender/src/disk.rs filesys-extender/src/main.rs filesys-extender/Cargo.lock
git commit -m "feat(filesys-extender): scaffold crate, error type, run_cmd"
```

---

### Task 2: `lsblk` JSON parsing - list removable disks

**Files:**
- Modify: `filesys-extender/src/disk.rs`
- Create: `filesys-extender/tests/fixtures/lsblk_with_persistence.json`
- Create: `filesys-extender/tests/fixtures/lsblk_mixed.json`

**Interfaces:**
- Consumes: `crate::exec::run_cmd` (from Task 1).
- Produces: `pub struct Disk { pub path: String, pub size_bytes: u64, pub model: String, pub partitions: Vec<PartitionInfo> }`, `pub struct PartitionInfo { pub path: String, pub size_bytes: u64, pub mountpoint: Option<String>, pub label: Option<String> }`, `pub fn parse_lsblk_json(json: &str) -> Result<Vec<Disk>, DiskOpError>`, `pub fn list_candidates() -> Result<Vec<Disk>, DiskOpError>`.

- [ ] **Step 1: Write fixture files**

`filesys-extender/tests/fixtures/lsblk_with_persistence.json`:

```json
{
   "blockdevices": [
      {
         "name": "sda", "path": "/dev/sda", "size": 256060514304,
         "rm": false, "tran": null, "mountpoint": null, "model": "Samsung SSD", "label": null,
         "children": []
      },
      {
         "name": "sdb", "path": "/dev/sdb", "size": 16008609792,
         "rm": true, "tran": "usb", "mountpoint": null, "model": "Cruzer", "label": null,
         "children": [
            { "name": "sdb1", "path": "/dev/sdb1", "size": 1257242624, "rm": true, "tran": null, "mountpoint": null, "model": null, "label": null },
            { "name": "sdb2", "path": "/dev/sdb2", "size": 100662295, "rm": true, "tran": null, "mountpoint": null, "model": null, "label": "persistence" }
         ]
      }
   ]
}
```

`filesys-extender/tests/fixtures/lsblk_mixed.json`:

```json
{
   "blockdevices": [
      {
         "name": "sda", "path": "/dev/sda", "size": 256060514304,
         "rm": false, "tran": null, "mountpoint": "/", "model": "Samsung SSD", "label": null,
         "children": []
      },
      {
         "name": "sdc", "path": "/dev/sdc", "size": 8053063680,
         "rm": true, "tran": "usb", "mountpoint": null, "model": "Ultra Fit", "label": null,
         "children": []
      }
   ]
}
```

- [ ] **Step 2: Write the failing tests**

Append to `filesys-extender/src/disk.rs`:

```rust
#[cfg(test)]
mod lsblk_tests {
    use super::*;

    #[test]
    fn parses_removable_disk_with_persistence_partition() {
        let json = include_str!("../tests/fixtures/lsblk_with_persistence.json");
        let disks = parse_lsblk_json(json).unwrap();
        assert_eq!(disks.len(), 1);
        let disk = &disks[0];
        assert_eq!(disk.path, "/dev/sdb");
        assert_eq!(disk.partitions.len(), 2);
        assert_eq!(disk.partitions[1].label.as_deref(), Some("persistence"));
    }

    #[test]
    fn filters_out_non_removable_disks() {
        let json = include_str!("../tests/fixtures/lsblk_mixed.json");
        let disks = parse_lsblk_json(json).unwrap();
        assert_eq!(disks.len(), 1);
        assert_eq!(disks[0].path, "/dev/sdc");
    }

    #[test]
    fn parse_lsblk_json_rejects_malformed_input() {
        let err = parse_lsblk_json("not json").unwrap_err();
        assert!(matches!(err, DiskOpError::Parse { .. }));
    }
}
```

- [ ] **Step 3: Run tests to verify they fail**

```bash
cd filesys-extender && cargo test disk::lsblk_tests
```
Expected: FAIL to compile (`parse_lsblk_json` not defined).

- [ ] **Step 4: Implement**

Replace the placeholder content of `filesys-extender/src/disk.rs` with:

```rust
use crate::DiskOpError;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct LsblkOutput {
    blockdevices: Vec<LsblkDevice>,
}

#[derive(Debug, Deserialize)]
struct LsblkDevice {
    path: String,
    size: u64,
    rm: bool,
    model: Option<String>,
    mountpoint: Option<String>,
    label: Option<String>,
    #[serde(default)]
    children: Vec<LsblkDevice>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Disk {
    pub path: String,
    pub size_bytes: u64,
    pub model: String,
    pub partitions: Vec<PartitionInfo>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PartitionInfo {
    pub path: String,
    pub size_bytes: u64,
    pub mountpoint: Option<String>,
    pub label: Option<String>,
}

pub fn parse_lsblk_json(json: &str) -> Result<Vec<Disk>, DiskOpError> {
    let parsed: LsblkOutput = serde_json::from_str(json).map_err(|e| DiskOpError::Parse {
        what: "lsblk JSON".into(),
        detail: e.to_string(),
    })?;

    Ok(parsed
        .blockdevices
        .into_iter()
        .filter(|d| d.rm)
        .map(|d| Disk {
            path: d.path,
            size_bytes: d.size,
            model: d.model.unwrap_or_default(),
            partitions: d
                .children
                .into_iter()
                .map(|c| PartitionInfo {
                    path: c.path,
                    size_bytes: c.size,
                    mountpoint: c.mountpoint,
                    label: c.label,
                })
                .collect(),
        })
        .collect())
}

pub fn list_candidates() -> Result<Vec<Disk>, DiskOpError> {
    let out = crate::exec::run_cmd(&[
        "lsblk", "-J", "-b", "-o", "NAME,PATH,SIZE,RM,TRAN,MOUNTPOINT,MODEL,LABEL",
    ])?;
    parse_lsblk_json(&out.stdout)
}
```

- [ ] **Step 5: Run tests to verify they pass**

```bash
cd filesys-extender && cargo test disk::lsblk_tests
```
Expected: 3 tests pass.

- [ ] **Step 6: Commit**

```bash
cd /home/fabio/dev/projects/dreamos-tools
git add filesys-extender/src/disk.rs filesys-extender/tests/fixtures/lsblk_with_persistence.json filesys-extender/tests/fixtures/lsblk_mixed.json
git commit -m "feat(filesys-extender): parse lsblk JSON into Disk/PartitionInfo"
```

---

### Task 3: `parted -m print free` parsing

**Files:**
- Modify: `filesys-extender/src/disk.rs`
- Create: `filesys-extender/tests/fixtures/parted_free_with_space.txt`

**Interfaces:**
- Consumes: `crate::exec::run_cmd` (Task 1).
- Produces: `pub struct PartedEntry { pub number: Option<u32>, pub start_bytes: u64, pub end_bytes: u64, pub size_bytes: u64, pub fs_or_free: String }`, `pub fn parse_parted_free(output: &str) -> Result<Vec<PartedEntry>, DiskOpError>`, `pub fn inspect_free_space(device: &str) -> Result<Vec<PartedEntry>, DiskOpError>`.

- [ ] **Step 1: Write the fixture**

`filesys-extender/tests/fixtures/parted_free_with_space.txt`:

```
BYT;
/dev/sdb:16008609792B:scsi:512:512:msdos:Cruzer:;
1:0B:1048575B:1048576B:free;
2:1048576B:1258291199B:1257242624B:fat32::boot, lba;
3:1258291200B:1358954495B:100662295B:ext4::;
4:1358954496B:16008609791B:14649655296B:free;
```

- [ ] **Step 2: Write the failing tests**

Append to `filesys-extender/src/disk.rs`:

```rust
#[cfg(test)]
mod parted_tests {
    use super::*;

    #[test]
    fn parses_partitions_and_free_space() {
        let out = include_str!("../tests/fixtures/parted_free_with_space.txt");
        let entries = parse_parted_free(out).unwrap();
        assert_eq!(entries.len(), 4);
        assert_eq!(entries[2].number, Some(3));
        assert_eq!(entries[2].fs_or_free, "ext4");
    }

    #[test]
    fn last_entry_is_free_space_at_end_of_disk() {
        let out = include_str!("../tests/fixtures/parted_free_with_space.txt");
        let entries = parse_parted_free(out).unwrap();
        let last = entries.last().unwrap();
        assert_eq!(last.fs_or_free, "free");
        assert_eq!(last.start_bytes, 1358954496);
        assert_eq!(last.end_bytes, 16008609791);
    }

    #[test]
    fn skips_disk_summary_line() {
        let out = include_str!("../tests/fixtures/parted_free_with_space.txt");
        let entries = parse_parted_free(out).unwrap();
        assert!(entries.iter().all(|e| e.start_bytes != 16008609792));
    }
}
```

- [ ] **Step 3: Run tests to verify they fail**

```bash
cd filesys-extender && cargo test disk::parted_tests
```
Expected: FAIL to compile (`parse_parted_free` not defined).

- [ ] **Step 4: Implement**

Append to `filesys-extender/src/disk.rs`:

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct PartedEntry {
    pub number: Option<u32>,
    pub start_bytes: u64,
    pub end_bytes: u64,
    pub size_bytes: u64,
    pub fs_or_free: String,
}

fn parse_bytes_field(field: &str) -> Result<u64, DiskOpError> {
    field
        .trim_end_matches('B')
        .parse::<u64>()
        .map_err(|e| DiskOpError::Parse {
            what: "parted byte field".into(),
            detail: format!("{field:?}: {e}"),
        })
}

pub fn parse_parted_free(output: &str) -> Result<Vec<PartedEntry>, DiskOpError> {
    let mut entries = Vec::new();
    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with("BYT;") {
            continue;
        }
        let fields: Vec<&str> = line.trim_end_matches(';').split(':').collect();
        let Some(first) = fields.first() else { continue };
        if first.starts_with("/dev/") {
            continue; // disk summary line
        }

        let number = first.parse::<u32>().ok();
        let start_bytes = parse_bytes_field(fields.get(1).copied().unwrap_or(""))?;
        let end_bytes = parse_bytes_field(fields.get(2).copied().unwrap_or(""))?;
        let size_bytes = parse_bytes_field(fields.get(3).copied().unwrap_or(""))?;
        let fs_or_free = fields.get(4).copied().unwrap_or("").to_string();

        entries.push(PartedEntry {
            number,
            start_bytes,
            end_bytes,
            size_bytes,
            fs_or_free,
        });
    }
    Ok(entries)
}

pub fn inspect_free_space(device: &str) -> Result<Vec<PartedEntry>, DiskOpError> {
    let out = crate::exec::run_cmd(&["parted", "-m", "-s", device, "unit", "B", "print", "free"])?;
    parse_parted_free(&out.stdout)
}
```

- [ ] **Step 5: Run tests to verify they pass**

```bash
cd filesys-extender && cargo test disk::parted_tests
```
Expected: 3 tests pass.

- [ ] **Step 6: Commit**

```bash
cd /home/fabio/dev/projects/dreamos-tools
git add filesys-extender/src/disk.rs filesys-extender/tests/fixtures/parted_free_with_space.txt
git commit -m "feat(filesys-extender): parse parted print free output"
```

---

### Task 4: Plan computation (Create / Grow / NoAction)

**Files:**
- Modify: `filesys-extender/src/disk.rs`

**Interfaces:**
- Consumes: `PartedEntry` (Task 3), `PartitionInfo` (Task 2).
- Produces: `pub enum Plan { Create { device: String, partition_number: u32, start_bytes: u64, end_bytes: u64 }, Grow { device: String, partition_number: u32, new_end_bytes: u64 }, NoAction { reason: String } }`, `pub fn compute_plan(device: &str, entries: &[PartedEntry], partitions: &[PartitionInfo]) -> Plan`.

- [ ] **Step 1: Write the failing tests**

Append to `filesys-extender/src/disk.rs`:

```rust
#[cfg(test)]
mod plan_tests {
    use super::*;

    fn partition(path: &str, label: Option<&str>) -> PartitionInfo {
        PartitionInfo {
            path: path.to_string(),
            size_bytes: 0,
            mountpoint: None,
            label: label.map(str::to_string),
        }
    }

    fn entry(number: Option<u32>, start: u64, end: u64, fs_or_free: &str) -> PartedEntry {
        PartedEntry {
            number,
            start_bytes: start,
            end_bytes: end,
            size_bytes: end - start,
            fs_or_free: fs_or_free.to_string(),
        }
    }

    #[test]
    fn grows_existing_persistence_partition_into_trailing_free_space() {
        let entries = vec![
            entry(Some(1), 0, 1_000_000, "fat32"),
            entry(Some(2), 1_000_000, 2_000_000, "ext4"),
            entry(None, 2_000_000, 5_000_000, "free"),
        ];
        let partitions = vec![
            partition("/dev/sdb1", None),
            partition("/dev/sdb2", Some("persistence")),
        ];
        let plan = compute_plan("/dev/sdb", &entries, &partitions);
        assert_eq!(
            plan,
            Plan::Grow { device: "/dev/sdb".into(), partition_number: 2, new_end_bytes: 5_000_000 }
        );
    }

    #[test]
    fn creates_persistence_partition_when_none_exists_and_free_space_present() {
        let entries = vec![
            entry(Some(1), 0, 1_000_000, "fat32"),
            entry(None, 1_000_000, 5_000_000, "free"),
        ];
        let partitions = vec![partition("/dev/sdb1", None)];
        let plan = compute_plan("/dev/sdb", &entries, &partitions);
        assert_eq!(
            plan,
            Plan::Create { device: "/dev/sdb".into(), partition_number: 2, start_bytes: 1_000_000, end_bytes: 5_000_000 }
        );
    }

    #[test]
    fn no_action_when_persistence_exists_and_no_trailing_free_space() {
        let entries = vec![
            entry(Some(1), 0, 1_000_000, "fat32"),
            entry(Some(2), 1_000_000, 5_000_000, "ext4"),
        ];
        let partitions = vec![
            partition("/dev/sdb1", None),
            partition("/dev/sdb2", Some("persistence")),
        ];
        let plan = compute_plan("/dev/sdb", &entries, &partitions);
        assert!(matches!(plan, Plan::NoAction { .. }));
    }

    #[test]
    fn no_action_when_no_persistence_and_no_free_space() {
        let entries = vec![entry(Some(1), 0, 5_000_000, "fat32")];
        let partitions = vec![partition("/dev/sdb1", None)];
        let plan = compute_plan("/dev/sdb", &entries, &partitions);
        assert!(matches!(plan, Plan::NoAction { .. }));
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cd filesys-extender && cargo test disk::plan_tests
```
Expected: FAIL to compile (`Plan`/`compute_plan` not defined).

- [ ] **Step 3: Implement**

Append to `filesys-extender/src/disk.rs`:

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum Plan {
    Create {
        device: String,
        partition_number: u32,
        start_bytes: u64,
        end_bytes: u64,
    },
    Grow {
        device: String,
        partition_number: u32,
        new_end_bytes: u64,
    },
    NoAction {
        reason: String,
    },
}

pub fn compute_plan(device: &str, entries: &[PartedEntry], partitions: &[PartitionInfo]) -> Plan {
    let persistence_number = entries.iter().filter_map(|e| e.number).find(|&n| {
        let path = format!("{device}{n}");
        partitions
            .iter()
            .any(|p| p.path == path && p.label.as_deref() == Some("persistence"))
    });

    let free_at_end = entries
        .last()
        .filter(|e| e.fs_or_free == "free");

    match (persistence_number, free_at_end) {
        (Some(n), Some(free)) => Plan::Grow {
            device: device.to_string(),
            partition_number: n,
            new_end_bytes: free.end_bytes,
        },
        (None, Some(free)) => Plan::Create {
            device: device.to_string(),
            partition_number: entries.iter().filter_map(|e| e.number).max().unwrap_or(0) + 1,
            start_bytes: free.start_bytes,
            end_bytes: free.end_bytes,
        },
        (Some(_), None) => Plan::NoAction {
            reason: "the persistence partition already uses all available space".into(),
        },
        (None, None) => Plan::NoAction {
            reason: "no free space available on this disk".into(),
        },
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
cd filesys-extender && cargo test disk::plan_tests
```
Expected: 4 tests pass.

- [ ] **Step 5: Commit**

```bash
cd /home/fabio/dev/projects/dreamos-tools
git add filesys-extender/src/disk.rs
git commit -m "feat(filesys-extender): compute Create/Grow/NoAction plan from disk state"
```

---

### Task 5: Live-boot device detection

**Files:**
- Modify: `filesys-extender/src/disk.rs`

**Interfaces:**
- Consumes: `crate::exec::run_cmd` (Task 1).
- Produces: `pub fn parse_live_medium_source(findmnt_output: &str) -> Option<String>`, `pub fn strip_partition_suffix(partition_path: &str) -> String`, `pub fn detect_live_boot_disk() -> Option<String>`.

- [ ] **Step 1: Write the failing tests**

Append to `filesys-extender/src/disk.rs`:

```rust
#[cfg(test)]
mod live_boot_tests {
    use super::*;

    #[test]
    fn strips_simple_partition_suffix() {
        assert_eq!(strip_partition_suffix("/dev/sdb1"), "/dev/sdb");
        assert_eq!(strip_partition_suffix("/dev/sdb12"), "/dev/sdb");
    }

    #[test]
    fn strips_nvme_style_partition_suffix() {
        assert_eq!(strip_partition_suffix("/dev/nvme0n1p1"), "/dev/nvme0n1");
    }

    #[test]
    fn parse_live_medium_source_handles_empty_and_present_output() {
        assert_eq!(parse_live_medium_source(""), None);
        assert_eq!(parse_live_medium_source("\n"), None);
        assert_eq!(parse_live_medium_source("/dev/sdb1\n"), Some("/dev/sdb1".to_string()));
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cd filesys-extender && cargo test disk::live_boot_tests
```
Expected: FAIL to compile.

- [ ] **Step 3: Implement**

Append to `filesys-extender/src/disk.rs`:

```rust
pub fn strip_partition_suffix(partition_path: &str) -> String {
    let trimmed = partition_path.trim_end_matches(|c: char| c.is_ascii_digit());
    if let Some(base) = trimmed.strip_suffix('p') {
        if base.chars().last().map_or(false, |c| c.is_ascii_digit()) {
            return base.to_string();
        }
    }
    trimmed.to_string()
}

pub fn parse_live_medium_source(findmnt_output: &str) -> Option<String> {
    let src = findmnt_output.trim();
    if src.is_empty() {
        return None;
    }
    Some(src.to_string())
}

pub fn detect_live_boot_disk() -> Option<String> {
    let out = crate::exec::run_cmd(&["findmnt", "-no", "SOURCE", "/run/live/medium"]).ok()?;
    let partition = parse_live_medium_source(&out.stdout)?;
    Some(strip_partition_suffix(&partition))
}
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
cd filesys-extender && cargo test disk::live_boot_tests
```
Expected: 3 tests pass.

- [ ] **Step 5: Commit**

```bash
cd /home/fabio/dev/projects/dreamos-tools
git add filesys-extender/src/disk.rs
git commit -m "feat(filesys-extender): detect live-boot device via findmnt"
```

---

### Task 6: Step construction, execution, and `persistence.conf`

**Files:**
- Modify: `filesys-extender/src/exec.rs`

**Interfaces:**
- Consumes: `Plan` (Task 4), `run_cmd` (Task 1).
- Produces: `pub struct Step { pub description: String, pub argv: Vec<String> }`, `pub fn steps_for_plan(plan: &crate::disk::Plan) -> Vec<Step>`, `pub fn execute_steps<F: FnMut(&Step, Result<&CmdOutput, &DiskOpError>)>(steps: &[Step], on_step: F) -> Result<(), DiskOpError>`, `pub fn write_persistence_conf(mount_point: &str) -> Result<(), DiskOpError>`.

- [ ] **Step 1: Write the failing tests**

Append to `filesys-extender/src/exec.rs`:

```rust
#[cfg(test)]
mod step_tests {
    use super::*;
    use crate::disk::Plan;

    #[test]
    fn create_plan_produces_mkpart_then_mkfs() {
        let plan = Plan::Create {
            device: "/dev/sdb".into(),
            partition_number: 3,
            start_bytes: 100,
            end_bytes: 200,
        };
        let steps = steps_for_plan(&plan);
        assert_eq!(steps.len(), 4);
        assert!(steps[0].argv.contains(&"mkpart".to_string()));
        assert_eq!(steps[3].argv.last().unwrap(), "/dev/sdb3");
        assert!(steps[3].argv.contains(&"persistence".to_string()));
    }

    #[test]
    fn grow_plan_produces_resizepart_then_resize2fs() {
        let plan = Plan::Grow {
            device: "/dev/sdb".into(),
            partition_number: 2,
            new_end_bytes: 500,
        };
        let steps = steps_for_plan(&plan);
        assert_eq!(steps.len(), 4);
        assert_eq!(steps[0].argv[0], "parted");
        assert_eq!(steps[3].argv, vec!["resize2fs".to_string(), "/dev/sdb2".to_string()]);
    }

    #[test]
    fn no_action_plan_produces_no_steps() {
        let plan = Plan::NoAction { reason: "x".into() };
        assert!(steps_for_plan(&plan).is_empty());
    }

    #[test]
    fn execute_steps_calls_callback_per_step_and_stops_on_failure() {
        let steps = vec![
            Step { description: "ok".into(), argv: vec!["true".into()] },
            Step { description: "fail".into(), argv: vec!["false".into()] },
            Step { description: "never runs".into(), argv: vec!["true".into()] },
        ];
        let mut seen = Vec::new();
        let result = execute_steps(&steps, |step, res| {
            seen.push((step.description.clone(), res.is_ok()));
        });
        assert!(result.is_err());
        assert_eq!(seen, vec![
            ("ok".to_string(), true),
            ("fail".to_string(), false),
        ]);
    }

    #[test]
    fn write_persistence_conf_writes_expected_content() {
        let dir = std::env::temp_dir().join(format!("filesys-extender-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        write_persistence_conf(dir.to_str().unwrap()).unwrap();
        let content = std::fs::read_to_string(dir.join("persistence.conf")).unwrap();
        assert_eq!(content, "/ union\n");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cd filesys-extender && cargo test exec::step_tests
```
Expected: FAIL to compile (`Step`, `steps_for_plan`, `execute_steps`, `write_persistence_conf` not defined).

- [ ] **Step 3: Implement**

Append to `filesys-extender/src/exec.rs`:

```rust
use crate::disk::Plan;

#[derive(Debug, Clone, PartialEq)]
pub struct Step {
    pub description: String,
    pub argv: Vec<String>,
}

pub fn steps_for_plan(plan: &Plan) -> Vec<Step> {
    match plan {
        Plan::Create { device, partition_number, start_bytes, end_bytes } => vec![
            Step {
                description: "Create persistence partition".into(),
                argv: vec![
                    "parted".into(), "-s".into(), device.clone(), "unit".into(), "B".into(),
                    "mkpart".into(), "primary".into(), "ext4".into(),
                    format!("{start_bytes}B"), format!("{end_bytes}B"),
                ],
            },
            Step {
                description: "Re-read partition table".into(),
                argv: vec!["partprobe".into(), device.clone()],
            },
            Step {
                description: "Wait for device node".into(),
                argv: vec!["udevadm".into(), "settle".into()],
            },
            Step {
                description: "Format as ext4, label persistence".into(),
                argv: vec![
                    "mkfs.ext4".into(), "-F".into(), "-L".into(), "persistence".into(),
                    format!("{device}{partition_number}"),
                ],
            },
        ],
        Plan::Grow { device, partition_number, new_end_bytes } => vec![
            Step {
                description: "Grow partition".into(),
                argv: vec![
                    "parted".into(), "-s".into(), device.clone(), "unit".into(), "B".into(),
                    "resizepart".into(), partition_number.to_string(), format!("{new_end_bytes}B"),
                ],
            },
            Step {
                description: "Re-read partition table".into(),
                argv: vec!["partprobe".into(), device.clone()],
            },
            Step {
                description: "Wait for device node".into(),
                argv: vec!["udevadm".into(), "settle".into()],
            },
            Step {
                description: "Grow filesystem".into(),
                argv: vec!["resize2fs".into(), format!("{device}{partition_number}")],
            },
        ],
        Plan::NoAction { .. } => vec![],
    }
}

pub fn execute_steps<F>(steps: &[Step], mut on_step: F) -> Result<(), DiskOpError>
where
    F: FnMut(&Step, Result<&CmdOutput, &DiskOpError>),
{
    for step in steps {
        let argv_refs: Vec<&str> = step.argv.iter().map(String::as_str).collect();
        match run_cmd(&argv_refs) {
            Ok(out) => on_step(step, Ok(&out)),
            Err(e) => {
                on_step(step, Err(&e));
                return Err(e);
            }
        }
    }
    Ok(())
}

pub fn write_persistence_conf(mount_point: &str) -> Result<(), DiskOpError> {
    let path = format!("{mount_point}/persistence.conf");
    std::fs::write(&path, "/ union\n").map_err(|e| DiskOpError::Parse {
        what: "writing persistence.conf".into(),
        detail: e.to_string(),
    })
}
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
cd filesys-extender && cargo test exec::step_tests
```
Expected: 5 tests pass.

- [ ] **Step 5: Manual verification against a loopback device**

Not automated (destructive disk ops don't belong in CI - see spec's Testing
section). Run this by hand once, on the dev machine, to sanity-check the
exact argv sequences against real `parted`/`mkfs.ext4`/`resize2fs`:

```bash
truncate -s 200M /tmp/filesys-extender-loop.img
sudo losetup -fP /tmp/filesys-extender-loop.img
LOOPDEV=$(losetup -j /tmp/filesys-extender-loop.img | cut -d: -f1)
sudo parted -s "$LOOPDEV" mklabel msdos
sudo parted -s "$LOOPDEV" unit B mkpart primary ext4 1048576B 104857600B
sudo partprobe "$LOOPDEV"
sudo udevadm settle
sudo mkfs.ext4 -F -L persistence "${LOOPDEV}p1"
sudo parted -m -s "$LOOPDEV" unit B print free   # confirm output matches Task 3's fixture shape
sudo losetup -d "$LOOPDEV"
rm /tmp/filesys-extender-loop.img
```
Expected: each command succeeds; the `print free` output has the same
`BYT;` / `N:start:end:size:fs;` shape the fixture in Task 3 assumes.
Note any real-world formatting difference here and adjust `parse_parted_free`
if needed before moving on.

- [ ] **Step 6: Commit**

```bash
cd /home/fabio/dev/projects/dreamos-tools
git add filesys-extender/src/exec.rs
git commit -m "feat(filesys-extender): build and execute plan steps, write persistence.conf"
```

---

### Task 7: GTK4 application skeleton + self-elevation

**Files:**
- Modify: `filesys-extender/src/main.rs`

**Interfaces:**
- Consumes: nothing from `disk`/`exec` yet (wired in Tasks 8-10).
- Produces: `fn main()` (self-elevation + GTK startup), a `gtk4::Application` with a `gtk4::Stack` holding 5 named pages (`"disk_list"`, `"inspect"`, `"confirm"`, `"executing"`, `"result"`).

- [ ] **Step 1: Implement self-elevation**

Replace `filesys-extender/src/main.rs`:

```rust
use std::os::unix::process::CommandExt;
use std::process::Command;

fn ensure_root() {
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

fn run_app() {
    println!("filesys-extender: GTK UI scaffold (wizard pages added in Task 8+)");
}
```

- [ ] **Step 2: Verify it builds**

```bash
cd filesys-extender && cargo build
```
Expected: builds with no errors (GTK not wired in yet, so no display needed to build).

- [ ] **Step 3: Add the GTK4 Application + 5-page Stack skeleton**

Replace `run_app`:

```rust
use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow, Label, Stack};

fn run_app() {
    let app = Application::builder()
        .application_id("dev.dreamos.filesys-extender")
        .build();

    app.connect_activate(|app| {
        let stack = Stack::new();
        stack.add_titled(&Label::new(Some("Disk list (Task 8)")), Some("disk_list"), "Disk list");
        stack.add_titled(&Label::new(Some("Inspect (Task 9)")), Some("inspect"), "Inspect");
        stack.add_titled(&Label::new(Some("Confirm (Task 10)")), Some("confirm"), "Confirm");
        stack.add_titled(&Label::new(Some("Executing (Task 10)")), Some("executing"), "Executing");
        stack.add_titled(&Label::new(Some("Result (Task 10)")), Some("result"), "Result");
        stack.set_visible_child_name("disk_list");

        let window = ApplicationWindow::builder()
            .application(app)
            .title("dreamos - filesys-extender")
            .default_width(700)
            .default_height(500)
            .child(&stack)
            .build();
        window.present();
    });

    app.run();
}
```

- [ ] **Step 4: Manual verification**

```bash
cd filesys-extender && cargo build
sudo ./target/debug/filesys-extender
```
(Or, without root, temporarily comment out `ensure_root();` in `main()` to
test the window on a dev machine without a working `pkexec` prompt loop -
revert the comment before committing.)
Expected: a window opens titled "dreamos - filesys-extender" showing the
"Disk list (Task 8)" label. No crash.

- [ ] **Step 5: Commit**

```bash
cd /home/fabio/dev/projects/dreamos-tools
git add filesys-extender/src/main.rs
git commit -m "feat(filesys-extender): GTK4 app skeleton, 5-page stack, pkexec self-elevation"
```

---

### Task 8: Screen 1 - disk list

**Files:**
- Modify: `filesys-extender/src/main.rs`

**Interfaces:**
- Consumes: `filesys_extender::disk::{list_candidates, detect_live_boot_disk, Disk}` (Tasks 2, 5).
- Produces: a disk-list page showing each candidate `Disk` (path, size, model), the live-boot device pre-selected, and a "Next" button that stores the chosen `Disk` for Screen 2.

- [ ] **Step 1: Build the disk-list page as its own function**

In `filesys-extender/src/main.rs`, add (near the top, after imports):

```rust
use filesys_extender::disk::{detect_live_boot_disk, list_candidates, Disk};
use gtk4::{Box as GtkBox, Button, ListBox, ListBoxRow, Orientation};
use std::cell::RefCell;
use std::rc::Rc;

fn format_size(bytes: u64) -> String {
    const GIB: f64 = 1024.0 * 1024.0 * 1024.0;
    format!("{:.1} GiB", bytes as f64 / GIB)
}

fn build_disk_list_page(
    selected_disk: Rc<RefCell<Option<Disk>>>,
    on_next: impl Fn() + 'static,
) -> GtkBox {
    let container = GtkBox::new(Orientation::Vertical, 8);
    let list = ListBox::new();

    let disks = list_candidates().unwrap_or_default();
    let live_boot = detect_live_boot_disk();

    let disks_rc = Rc::new(disks);
    for (idx, disk) in disks_rc.iter().enumerate() {
        let label_text = format!(
            "{}  -  {}  -  {}",
            disk.path,
            format_size(disk.size_bytes),
            disk.model
        );
        let row = ListBoxRow::new();
        row.set_child(Some(&gtk4::Label::new(Some(&label_text))));
        row.set_widget_name(&idx.to_string());
        if Some(disk.path.clone()) == live_boot {
            list.select_row(Some(&row));
        }
        list.append(&row);
    }

    let next_button = Button::with_label("Next");
    next_button.set_sensitive(false);

    {
        let disks_rc = disks_rc.clone();
        let selected_disk = selected_disk.clone();
        let next_button_clone = next_button.clone();
        list.connect_row_selected(move |_, row| {
            if let Some(row) = row {
                let idx: usize = row.widget_name().parse().unwrap_or(usize::MAX);
                if let Some(disk) = disks_rc.get(idx) {
                    *selected_disk.borrow_mut() = Some(disk.clone());
                    next_button_clone.set_sensitive(true);
                }
            }
        });
    }

    next_button.connect_clicked(move |_| on_next());

    container.append(&list);
    container.append(&next_button);
    container
}
```

- [ ] **Step 2: Wire it into the Stack, replacing the Task 7 placeholder page**

In `run_app`, replace the `stack.add_titled(&Label::new(Some("Disk list (Task 8)")), ...)` line with:

```rust
let selected_disk: Rc<RefCell<Option<filesys_extender::disk::Disk>>> = Rc::new(RefCell::new(None));
let stack_for_nav = stack.clone();
let disk_list_page = build_disk_list_page(selected_disk.clone(), move || {
    stack_for_nav.set_visible_child_name("inspect");
});
stack.add_titled(&disk_list_page, Some("disk_list"), "Disk list");
```

Add `lib.rs`'s crate name to `main.rs`'s imports as shown (`filesys_extender::disk::...` - Cargo auto-generates the `filesys_extender` lib target name from the package name `filesys-extender`, hyphens become underscores).

- [ ] **Step 3: Manual verification**

```bash
cd filesys-extender && cargo build
```
Expected: builds without errors. Running it (as in Task 7 Step 4) on a
machine with a real USB stick plugged in shows it listed with the
live-boot device (if run from a live boot) pre-selected; on a regular dev
machine with no removable disk plugged in, the list is empty and "Next"
stays disabled - both are acceptable, this only gets a real device on the
actual live ISO.

- [ ] **Step 4: Commit**

```bash
cd /home/fabio/dev/projects/dreamos-tools
git add filesys-extender/src/main.rs
git commit -m "feat(filesys-extender): wire disk-list screen to list_candidates/detect_live_boot_disk"
```

---

### Task 9: Screen 2 - inspect + plan summary

**Files:**
- Modify: `filesys-extender/src/main.rs`

**Interfaces:**
- Consumes: `filesys_extender::disk::{inspect_free_space, compute_plan, Plan}` (Tasks 3, 4), `selected_disk` state from Task 8.
- Produces: an inspect page showing the selected disk's partition table + free space and the computed `Plan`, storing the `Plan` for Screen 3. If `Plan::NoAction`, shows the reason and disables "Next".

- [ ] **Step 1: Build the inspect page**

Add to `filesys-extender/src/main.rs`:

```rust
use filesys_extender::disk::{compute_plan, inspect_free_space, Plan};

fn describe_plan(plan: &Plan) -> String {
    match plan {
        Plan::Create { device, start_bytes, end_bytes, .. } => format!(
            "Will create a new ext4 'persistence' partition on {device}, using {} of free space.",
            format_size(end_bytes - start_bytes)
        ),
        Plan::Grow { device, new_end_bytes, .. } => format!(
            "Will grow the existing 'persistence' partition on {device} up to {}.",
            format_size(*new_end_bytes)
        ),
        Plan::NoAction { reason } => format!("Nothing to do: {reason}"),
    }
}

fn build_inspect_page(
    selected_disk: Rc<RefCell<Option<Disk>>>,
    plan: Rc<RefCell<Option<Plan>>>,
    on_next: impl Fn() + 'static,
) -> GtkBox {
    let container = GtkBox::new(Orientation::Vertical, 8);
    let summary_label = gtk4::Label::new(None);
    let next_button = Button::with_label("Next");
    next_button.set_sensitive(false);

    container.append(&summary_label);
    container.append(&next_button);

    {
        let selected_disk = selected_disk.clone();
        let plan = plan.clone();
        let summary_label = summary_label.clone();
        let next_button = next_button.clone();
        container.connect_map(move |_| {
            let Some(disk) = selected_disk.borrow().clone() else {
                summary_label.set_text("No disk selected.");
                return;
            };
            let entries = match inspect_free_space(&disk.path) {
                Ok(entries) => entries,
                Err(e) => {
                    summary_label.set_text(&format!("Failed to inspect {}: {e}", disk.path));
                    return;
                }
            };
            let computed = compute_plan(&disk.path, &entries, &disk.partitions);
            summary_label.set_text(&describe_plan(&computed));
            next_button.set_sensitive(!matches!(computed, Plan::NoAction { .. }));
            *plan.borrow_mut() = Some(computed);
        });
    }

    next_button.connect_clicked(move |_| on_next());
    container
}
```

- [ ] **Step 2: Wire it into the Stack**

In `run_app`, add alongside `selected_disk`:

```rust
let plan: Rc<RefCell<Option<Plan>>> = Rc::new(RefCell::new(None));
let stack_for_inspect_nav = stack.clone();
let inspect_page = build_inspect_page(selected_disk.clone(), plan.clone(), move || {
    stack_for_inspect_nav.set_visible_child_name("confirm");
});
stack.add_titled(&inspect_page, Some("inspect"), "Inspect");
```

Remove the Task 7 placeholder `stack.add_titled(&Label::new(Some("Inspect (Task 9)")), ...)` line.

- [ ] **Step 3: Manual verification**

```bash
cd filesys-extender && cargo build
```
Expected: builds without errors. `connect_map` fires `inspect_free_space`
each time the page becomes visible, so navigating Disk list -> Inspect
re-reads the disk's current state every time (no stale plan if the user
goes back and picks a different disk).

- [ ] **Step 4: Commit**

```bash
cd /home/fabio/dev/projects/dreamos-tools
git add filesys-extender/src/main.rs
git commit -m "feat(filesys-extender): wire inspect screen to inspect_free_space/compute_plan"
```

---

### Task 10: Screens 3-5 - typed confirm, executing, result

**Files:**
- Modify: `filesys-extender/src/main.rs`

**Interfaces:**
- Consumes: `filesys_extender::exec::{steps_for_plan, execute_steps, write_persistence_conf}` (Task 6), `plan`/`selected_disk` state (Tasks 8-9).
- Produces: confirm page (typed device-path gate), executing page (background-thread step execution with live log via a GLib channel), result page (final status + log).

- [ ] **Step 1: Build the confirm page**

Add to `filesys-extender/src/main.rs`:

```rust
use gtk4::Entry;

fn build_confirm_page(
    selected_disk: Rc<RefCell<Option<Disk>>>,
    plan: Rc<RefCell<Option<Plan>>>,
    on_apply: impl Fn() + 'static,
) -> GtkBox {
    let container = GtkBox::new(Orientation::Vertical, 8);
    let instructions = gtk4::Label::new(None);
    let entry = Entry::new();
    let apply_button = Button::with_label("Apply");
    apply_button.set_sensitive(false);

    container.append(&instructions);
    container.append(&entry);
    container.append(&apply_button);

    {
        let selected_disk = selected_disk.clone();
        let instructions = instructions.clone();
        container.connect_map(move |_| {
            if let Some(disk) = selected_disk.borrow().clone() {
                instructions.set_text(&format!(
                    "Type '{}' exactly to confirm this destructive operation:",
                    disk.path
                ));
            }
        });
    }

    {
        let selected_disk = selected_disk.clone();
        let apply_button = apply_button.clone();
        entry.connect_changed(move |entry| {
            let expected = selected_disk.borrow().as_ref().map(|d| d.path.clone());
            let matches = expected.as_deref() == Some(entry.text().as_str());
            apply_button.set_sensitive(matches);
        });
    }

    let _ = plan; // plan is read by the executing page, kept here for future use if needed
    apply_button.connect_clicked(move |_| on_apply());
    container
}
```

- [ ] **Step 2: Build the executing page (background thread + GLib channel)**

Add to `filesys-extender/src/main.rs`:

```rust
use filesys_extender::exec::{execute_steps, steps_for_plan, write_persistence_conf};
use glib::MainContext;
use gtk4::TextView;

#[derive(Debug)]
enum ExecMsg {
    StepStarted(String),
    StepOutput(String),
    StepFailed(String),
    AllDone,
}

fn build_executing_page(
    plan: Rc<RefCell<Option<Plan>>>,
    result_text: Rc<RefCell<String>>,
    on_finished: impl Fn(bool) + 'static,
) -> (GtkBox, impl Fn() + 'static) {
    let container = GtkBox::new(Orientation::Vertical, 8);
    let log_view = TextView::new();
    log_view.set_editable(false);
    container.append(&log_view);

    let (sender, receiver) = MainContext::channel::<ExecMsg>(glib::PRIORITY_DEFAULT);

    {
        let buf = log_view.buffer();
        let result_text = result_text.clone();
        receiver.attach(None, move |msg| {
            let mut end = buf.end_iter();
            match &msg {
                ExecMsg::StepStarted(desc) => buf.insert(&mut end, &format!("==> {desc}\n")),
                ExecMsg::StepOutput(line) => buf.insert(&mut end, &format!("{line}\n")),
                ExecMsg::StepFailed(err) => {
                    buf.insert(&mut end, &format!("FAILED: {err}\n"));
                    result_text.borrow_mut().push_str(&format!("FAILED: {err}\n"));
                }
                ExecMsg::AllDone => buf.insert(&mut end, "Done.\n"),
            }
            glib::Continue(true)
        });
    }

    let start = move || {
        let Some(current_plan) = plan.borrow().clone() else { return };
        let sender = sender.clone();
        let steps = steps_for_plan(&current_plan);
        let on_finished = on_finished.clone();

        std::thread::spawn(move || {
            let sender_for_cb = sender.clone();
            let outcome = execute_steps(&steps, move |step, res| {
                let _ = sender_for_cb.send(ExecMsg::StepStarted(step.description.clone()));
                match res {
                    Ok(out) => {
                        let _ = sender_for_cb.send(ExecMsg::StepOutput(out.stdout.clone()));
                    }
                    Err(e) => {
                        let _ = sender_for_cb.send(ExecMsg::StepFailed(e.to_string()));
                    }
                }
            });
            let _ = sender.send(ExecMsg::AllDone);
            on_finished(outcome.is_ok());
        });
    };

    (container, start)
}
```

`on_finished` is `Clone` because it's only ever called from the spawned
thread - wrap it in `Rc` at the call site in Step 4 below (a plain
closure captured by `move` into `std::thread::spawn` must be `Send`, so
`on_finished` is invoked via a `glib::MainContext` idle callback instead
of directly - see Step 4).

- [ ] **Step 3: Build the result page**

Add to `filesys-extender/src/main.rs`:

```rust
fn build_result_page(result_text: Rc<RefCell<String>>) -> GtkBox {
    let container = GtkBox::new(Orientation::Vertical, 8);
    let label = gtk4::Label::new(None);
    container.append(&label);
    container.connect_map(move |_| {
        let text = result_text.borrow();
        if text.is_empty() {
            label.set_text("Completed successfully.");
        } else {
            label.set_text(&text);
        }
    });
    container
}
```

- [ ] **Step 4: Wire screens 3-5 into the Stack and finish the flow**

In `run_app`, replace the Task 7 placeholder lines for `confirm`,
`executing`, and `result`:

```rust
let result_text: Rc<RefCell<String>> = Rc::new(RefCell::new(String::new()));

let stack_for_result = stack.clone();
let (executing_page, start_execution) = build_executing_page(
    plan.clone(),
    result_text.clone(),
    move |success| {
        let stack_for_result = stack_for_result.clone();
        glib::idle_add_local_once(move || {
            stack_for_result.set_visible_child_name(if success { "result" } else { "result" });
        });
    },
);
let start_execution = Rc::new(start_execution);
stack.add_titled(&executing_page, Some("executing"), "Executing");

let stack_for_confirm_nav = stack.clone();
let confirm_page = build_confirm_page(selected_disk.clone(), plan.clone(), move || {
    stack_for_confirm_nav.set_visible_child_name("executing");
    start_execution();
});
stack.add_titled(&confirm_page, Some("confirm"), "Confirm");

let result_page = build_result_page(result_text.clone());
stack.add_titled(&result_page, Some("result"), "Result");
```

Also, after a successful run, `write_persistence_conf` must be called
before showing the result page. Add this inside `build_executing_page`'s
spawned thread, right after `execute_steps` succeeds and before sending
`AllDone` - append to the `std::thread::spawn` closure body in Step 2:

```rust
if outcome.is_ok() {
    if let Plan::Create { device, partition_number, .. } | Plan::Grow { device, partition_number, .. } = &current_plan {
        let mount_dir = format!("/mnt/filesys-extender-{partition_number}");
        let _ = std::fs::create_dir_all(&mount_dir);
        let mount_result = execute_steps(
            &[filesys_extender::exec::Step {
                description: "Mount persistence partition".into(),
                argv: vec!["mount".into(), format!("{device}{partition_number}"), mount_dir.clone()],
            }],
            |_, _| {},
        );
        if mount_result.is_ok() {
            let _ = write_persistence_conf(&mount_dir);
            let _ = execute_steps(
                &[filesys_extender::exec::Step {
                    description: "Unmount".into(),
                    argv: vec!["umount".into(), mount_dir],
                }],
                |_, _| {},
            );
        }
    }
}
```

(Place this block right after the `let outcome = execute_steps(...)` line,
before `let _ = sender.send(ExecMsg::AllDone);`.)

- [ ] **Step 5: Manual verification**

```bash
cd filesys-extender && cargo build
```
Expected: builds without errors. Full manual walk-through happens in
Task 11's QEMU smoke test, since this needs a real removable disk with
free space to exercise end-to-end.

- [ ] **Step 6: Commit**

```bash
cd /home/fabio/dev/projects/dreamos-tools
git add filesys-extender/src/main.rs
git commit -m "feat(filesys-extender): wire confirm/executing/result screens, mount+write persistence.conf"
```

---

### Task 11: Release build, README, manual QEMU smoke test

**Files:**
- Create: `filesys-extender/README.md`
- Modify: `filesys-extender/Cargo.toml`

**Interfaces:**
- Consumes: nothing new.
- Produces: a documented build/run procedure for humans and for the future `dreamos` repo integration step (out of scope here per the spec).

- [ ] **Step 1: Add a release profile**

Append to `filesys-extender/Cargo.toml`:

```toml
[profile.release]
opt-level = 2
strip = true
```

- [ ] **Step 2: Write the README**

Create `filesys-extender/README.md`:

```markdown
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
  argv sequences: see Task 6 Step 5 of the implementation plan.
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
```

- [ ] **Step 3: Verify the release build**

```bash
cd filesys-extender && cargo build --release
```
Expected: builds successfully, `target/release/filesys-extender` exists.

- [ ] **Step 4: Run the full test suite one last time**

```bash
cd filesys-extender && cargo test
```
Expected: every test from Tasks 1-6 passes (20 tests total: 3 exec
run_cmd + 3 lsblk + 3 parted + 4 plan + 3 live-boot + 5 step/execute/conf
- 1 already counted in step_tests being 5 not 4, recount at run time and
treat any mismatch as a signal to re-check Task 6 before proceeding).

- [ ] **Step 5: Commit**

```bash
cd /home/fabio/dev/projects/dreamos-tools
git add filesys-extender/Cargo.toml filesys-extender/README.md
git commit -m "docs(filesys-extender): README, release profile"
```

- [ ] **Step 6: Manual QEMU smoke test (do this by hand, not scripted)**

This step depends on the separate `dreamos` repo staging the binary
(out of scope for this plan - see the spec). Once that staging exists:

1. Attach a second virtual disk to `start.sh`'s QEMU invocation larger
   than the ISO (e.g. `qemu-img create -f qcow2 /tmp/test-stick.qcow2 4G`
   then pass `-drive file=/tmp/test-stick.qcow2,if=virtio` as an extra
   arg to `./start.sh --`).
2. Boot, launch filesys-extender from the Openbox menu.
3. Confirm the second disk shows in the disk list, walk through
   Inspect -> Confirm (typing the device path) -> Executing -> Result.
4. Reboot the guest, confirm `/etc/persistence.conf`-driven persistence
   now survives a reboot (create a file in `$HOME`, reboot, check it's
   still there).

Record the outcome in `filesys-extender/README.md` under a new "Verified
on" note (date + dreamos build) once done - this is a manual follow-up,
not a plan step to check off here.

---

## Self-Review Notes

- **Spec coverage:** privilege model (Task 7), disk detection (Tasks
  2-3, 5), plan computation incl. Grow/Create/NoAction (Task 4), typed
  confirm gate (Task 10), execution + logging (Task 6, 10), error
  handling with verbatim command output (Task 1, 6), `persistence.conf`
  writing (Task 6, 10), crate layout (Task 1), testing strategy incl.
  manual loopback + QEMU steps (Tasks 6, 11) - all covered.
- **Type consistency checked:** `Plan`, `PartedEntry`, `PartitionInfo`,
  `Disk`, `Step`, `DiskOpError`, `CmdOutput` are defined once (Tasks 2-4,
  6) and referenced identically by name and field in every later task.
- **Known gap flagged explicitly (not a placeholder, a scoped
  decision):** dreamos-side ISO staging (package lists, menu.xml entry,
  binary staging script) is intentionally out of this plan per the
  spec's non-goals - Task 11 documents the interface only.
