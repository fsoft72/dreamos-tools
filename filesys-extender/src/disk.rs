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
