#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageKind {
    Nvme,
    Ssd,
    Hdd,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScoredComponent {
    pub score: f64,
    pub estimated: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ComponentScores {
    pub cpu: ScoredComponent,
    pub gpu: ScoredComponent,
    pub ram: ScoredComponent,
    pub storage: ScoredComponent,
}

/// Score caps at 100 for 32 GiB or more; scales linearly below that.
/// Placeholder constant, tunable once a real dataset informs a better curve.
const RAM_SCORE_FULL_GB: f64 = 32.0;

pub fn score_ram(total_gb: f64) -> ScoredComponent {
    let score = (total_gb / RAM_SCORE_FULL_GB * 100.0).min(100.0);
    ScoredComponent { score, estimated: false }
}

const STORAGE_SCORE_NVME: f64 = 100.0;
const STORAGE_SCORE_SSD: f64 = 70.0;
const STORAGE_SCORE_HDD: f64 = 30.0;
const STORAGE_SCORE_UNKNOWN: f64 = 50.0;

pub fn score_storage(kind: StorageKind) -> ScoredComponent {
    let score = match kind {
        StorageKind::Nvme => STORAGE_SCORE_NVME,
        StorageKind::Ssd => STORAGE_SCORE_SSD,
        StorageKind::Hdd => STORAGE_SCORE_HDD,
        StorageKind::Unknown => STORAGE_SCORE_UNKNOWN,
    };
    ScoredComponent { score, estimated: false }
}

#[cfg(test)]
mod formula_tests {
    use super::*;

    #[test]
    fn ram_score_caps_at_100_for_32gb_or_more() {
        assert_eq!(score_ram(32.0).score, 100.0);
        assert_eq!(score_ram(64.0).score, 100.0);
    }

    #[test]
    fn ram_score_scales_linearly_below_32gb() {
        assert_eq!(score_ram(16.0).score, 50.0);
        assert_eq!(score_ram(8.0).score, 25.0);
    }

    #[test]
    fn ram_score_is_never_estimated() {
        assert!(!score_ram(8.0).estimated);
    }

    #[test]
    fn storage_score_ranks_nvme_above_ssd_above_hdd() {
        assert!(score_storage(StorageKind::Nvme).score > score_storage(StorageKind::Ssd).score);
        assert!(score_storage(StorageKind::Ssd).score > score_storage(StorageKind::Hdd).score);
    }

    #[test]
    fn storage_score_is_never_estimated() {
        assert!(!score_storage(StorageKind::Unknown).estimated);
    }
}
