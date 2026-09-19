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

#[derive(Debug, Clone, serde::Deserialize)]
pub struct DbEntry {
    pub model_substring: String,
    pub score: f64,
}

fn lookup(model: &str, db: &[DbEntry]) -> Option<f64> {
    let model_lower = model.to_lowercase();
    db.iter()
        .find(|entry| model_lower.contains(&entry.model_substring.to_lowercase()))
        .map(|entry| entry.score)
}

/// Fallback CPU estimate when no database match is found: normalizes
/// cores * base_clock_mhz against an 8-core/4GHz reference point.
/// Placeholder constant, tunable once a real dataset exists.
const CPU_FALLBACK_NORMALIZER: f64 = 8.0 * 4000.0;

pub fn score_cpu(model: &str, cores: usize, base_clock_mhz: u64, db: &[DbEntry]) -> ScoredComponent {
    if let Some(score) = lookup(model, db) {
        return ScoredComponent { score, estimated: false };
    }
    let raw = (cores as f64) * (base_clock_mhz as f64);
    let score = (raw / CPU_FALLBACK_NORMALIZER * 100.0).min(100.0);
    ScoredComponent { score, estimated: true }
}

/// Fallback GPU estimate when no database match is found: no reliable
/// cross-vendor VRAM signal is available (see spec non-goals), so this
/// is a deliberately rough base value by device type. Placeholder
/// constants, tunable once a real dataset exists.
const GPU_FALLBACK_DISCRETE: f64 = 45.0;
const GPU_FALLBACK_INTEGRATED: f64 = 15.0;

pub fn score_gpu(name: &str, is_discrete: bool, db: &[DbEntry]) -> ScoredComponent {
    if let Some(score) = lookup(name, db) {
        return ScoredComponent { score, estimated: false };
    }
    let score = if is_discrete { GPU_FALLBACK_DISCRETE } else { GPU_FALLBACK_INTEGRATED };
    ScoredComponent { score, estimated: true }
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

#[cfg(test)]
mod lookup_tests {
    use super::*;

    fn fixture_db() -> Vec<DbEntry> {
        vec![
            DbEntry { model_substring: "Ryzen 7 5800X".into(), score: 78.0 },
            DbEntry { model_substring: "Core i7-12700K".into(), score: 82.0 },
        ]
    }

    #[test]
    fn cpu_lookup_matches_substring_case_insensitively() {
        let db = fixture_db();
        let result = score_cpu("AMD Ryzen 7 5800X 8-Core Processor", 8, 3800, &db);
        assert_eq!(result.score, 78.0);
        assert!(!result.estimated);
    }

    #[test]
    fn cpu_lookup_falls_back_to_formula_when_no_match() {
        let db = fixture_db();
        let result = score_cpu("Some Unknown CPU", 4, 2000, &db);
        assert!(result.estimated);
        assert!(result.score > 0.0);
    }

    #[test]
    fn cpu_fallback_scales_with_cores_and_clock() {
        let db: Vec<DbEntry> = vec![];
        let weak = score_cpu("Unknown A", 2, 1000, &db);
        let strong = score_cpu("Unknown B", 8, 4000, &db);
        assert!(strong.score > weak.score);
    }

    #[test]
    fn gpu_lookup_matches_substring() {
        let db = vec![DbEntry { model_substring: "RTX 4070".into(), score: 85.0 }];
        let result = score_gpu("NVIDIA GeForce RTX 4070", true, &db);
        assert_eq!(result.score, 85.0);
        assert!(!result.estimated);
    }

    #[test]
    fn gpu_fallback_favors_discrete_over_integrated() {
        let db: Vec<DbEntry> = vec![];
        let discrete = score_gpu("Unknown GPU", true, &db);
        let integrated = score_gpu("Unknown iGPU", false, &db);
        assert!(discrete.estimated && integrated.estimated);
        assert!(discrete.score > integrated.score);
    }
}
