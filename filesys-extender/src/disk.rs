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
