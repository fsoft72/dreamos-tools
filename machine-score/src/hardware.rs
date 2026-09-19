use crate::scoring::StorageKind;

pub fn classify_storage(kind: sysinfo::DiskKind, device_name: &str) -> StorageKind {
    match kind {
        sysinfo::DiskKind::HDD => StorageKind::Hdd,
        sysinfo::DiskKind::SSD => {
            if device_name.to_lowercase().contains("nvme") {
                StorageKind::Nvme
            } else {
                StorageKind::Ssd
            }
        }
        sysinfo::DiskKind::Unknown(_) => StorageKind::Unknown,
    }
}

#[derive(Debug)]
pub struct CpuRamStorageInfo {
    pub cpu_model: String,
    pub cpu_cores: usize,
    pub cpu_base_clock_mhz: u64,
    pub ram_total_gb: f64,
    pub storage_kind: StorageKind,
}

pub fn detect_cpu_ram_storage() -> CpuRamStorageInfo {
    let sys = sysinfo::System::new_all();
    let cpus = sys.cpus();
    let cpu_model = cpus.first().map(|c| c.brand().to_string()).unwrap_or_default();
    let cpu_cores = cpus.len();
    let cpu_base_clock_mhz = cpus.first().map(|c| c.frequency()).unwrap_or(0);
    let ram_total_gb = sys.total_memory() as f64 / (1024.0 * 1024.0 * 1024.0);

    let disks = sysinfo::Disks::new_with_refreshed_list();
    let storage_kind = disks
        .list()
        .first()
        .map(|d| classify_storage(d.kind(), &d.name().to_string_lossy()))
        .unwrap_or(StorageKind::Unknown);

    CpuRamStorageInfo {
        cpu_model,
        cpu_cores,
        cpu_base_clock_mhz,
        ram_total_gb,
        storage_kind,
    }
}

#[cfg(test)]
mod classify_tests {
    use super::*;
    use sysinfo::DiskKind;

    #[test]
    fn hdd_is_always_hdd() {
        assert_eq!(classify_storage(DiskKind::HDD, "/dev/sda"), StorageKind::Hdd);
    }

    #[test]
    fn ssd_with_nvme_in_name_is_nvme() {
        assert_eq!(classify_storage(DiskKind::SSD, "/dev/nvme0n1"), StorageKind::Nvme);
        assert_eq!(classify_storage(DiskKind::SSD, "Samsung NVMe SSD 980"), StorageKind::Nvme);
    }

    #[test]
    fn ssd_without_nvme_in_name_is_plain_ssd() {
        assert_eq!(classify_storage(DiskKind::SSD, "/dev/sdb"), StorageKind::Ssd);
    }

    #[test]
    fn unknown_kind_is_unknown() {
        assert_eq!(classify_storage(DiskKind::Unknown(-1), "/dev/sdc"), StorageKind::Unknown);
    }

    #[test]
    fn nvme_detection_is_case_insensitive() {
        assert_eq!(classify_storage(DiskKind::SSD, "NVME0N1"), StorageKind::Nvme);
    }
}
