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

    let free_at_end = entries.last().filter(|e| e.fs_or_free == "free");

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
