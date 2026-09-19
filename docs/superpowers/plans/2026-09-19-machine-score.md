# DreamOS Machine Score Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build `machine-score`, a cross-platform (Windows/Linux) egui
wizard that detects CPU/GPU/RAM/storage, scores them against an
embedded database (with formula fallback), and shows a composite score
plus Gaming/Godot/UE5 readiness tiers.

**Architecture:** Single Rust crate under `dreamos-tools/machine-score/`.
`scoring.rs` (database lookup, fallback formulas, composite, tiers) and
`hardware.rs` (CPU/RAM/storage via `sysinfo`, GPU via `wgpu`) hold all
non-UI logic, fully unit-testable. `main.rs` is a 3-screen `eframe`/`egui`
wizard (Welcome -> Scoring -> Results) with a persistent logo+title
header, calling into that logic. No privilege elevation anywhere - this
tool only reads hardware info.

**Tech Stack:** Rust (edition 2021), `eframe`/`egui` 0.33, `sysinfo`
0.38, `wgpu` 30, `pollster` 1 (sync-block on `wgpu`'s async adapter
enumeration), `image` 0.25 (PNG decode for the logo texture),
`serde`/`serde_json`.

**Spec:** [`docs/superpowers/specs/2026-09-19-machine-score-design.md`](../specs/2026-09-19-machine-score-design.md)

## Global Constraints

- Score scale: 0-100. Tiers: Low / Medium / High / Ultra (per spec's
  confirmed choice).
- Composite weights: CPU 0.35, GPU 0.35, RAM 0.15, Storage 0.15 -
  placeholder, defined as named constants for later tuning.
- CPU/GPU scored via database substring-match lookup with formula
  fallback (flagged `estimated: true`); RAM/storage are formula-only,
  never estimated.
- No live micro-benchmarking, no per-target weight profiles, no VRAM
  scoring input (`wgpu` has no reliable cross-vendor VRAM query) - all
  per spec's non-goals.
- No privilege elevation - read-only hardware detection.
- Crate layout: `src/lib.rs` (re-exports), `src/hardware.rs`,
  `src/scoring.rs` (no UI deps in either), `src/main.rs` (egui UI). No
  shared Cargo workspace with `filesys-extender` yet.
- Own `assets/ball.png` copy (copied from repo-root `assets/ball.png`),
  embedded via `include_bytes!` - self-contained, no filesystem
  dependency at runtime.

---

## File Structure

```
dreamos-tools/
  machine-score/
    Cargo.toml
    assets/
      ball.png
    src/
      data/
        cpu_scores.json     # placeholder CPU model -> score database
        gpu_scores.json     # placeholder GPU model -> score database
      lib.rs                 # re-exports hardware:: and scoring::
      hardware.rs             # CPU/GPU/RAM/storage detection
      scoring.rs                # database lookup, formulas, composite, tiers
      main.rs                    # egui wizard
    README.md
```

---

### Task 1: Crate scaffold

**Files:**
- Create: `machine-score/Cargo.toml`
- Create: `machine-score/src/lib.rs`
- Create: `machine-score/src/hardware.rs` (placeholder)
- Create: `machine-score/src/scoring.rs` (placeholder)
- Create: `machine-score/src/main.rs` (placeholder)
- Create: `machine-score/assets/ball.png` (copy of repo-root `assets/ball.png`)
- Create: `machine-score/.gitignore`

**Interfaces:**
- Produces: an empty-but-compiling crate with the module layout later tasks fill in.

- [ ] **Step 1: Create the crate and copy the logo**

```bash
cd /home/fabio/dev/projects/dreamos-tools
cargo new --bin machine-score
mkdir -p machine-score/assets
cp assets/ball.png machine-score/assets/ball.png
printf '/target\n' > machine-score/.gitignore
```

- [ ] **Step 2: Write `Cargo.toml`**

```toml
[package]
name = "machine-score"
version = "0.1.0"
edition = "2021"
rust-version = "1.91"

[dependencies]
eframe = "0.33"
sysinfo = "0.38"
wgpu = "30"
pollster = "1"
image = { version = "0.25", default-features = false, features = ["png"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"

[profile.release]
opt-level = 2
strip = true
```

- [ ] **Step 3: Write placeholder modules**

`machine-score/src/lib.rs`:

```rust
pub mod hardware;
pub mod scoring;
```

`machine-score/src/hardware.rs`:

```rust
// placeholder, filled in Tasks 6-7
```

`machine-score/src/scoring.rs`:

```rust
// placeholder, filled in Tasks 2-5
```

`machine-score/src/main.rs`:

```rust
fn main() {
    println!("machine-score: UI not implemented yet (see Task 8+)");
}
```

- [ ] **Step 4: Build**

```bash
cd machine-score && cargo build
```
Expected: builds with no errors (pulls in `eframe`/`sysinfo`/`wgpu`/etc; may take a minute the first time).

- [ ] **Step 5: Commit**

```bash
cd /home/fabio/dev/projects/dreamos-tools
git add machine-score/Cargo.toml machine-score/Cargo.lock machine-score/.gitignore machine-score/src machine-score/assets/ball.png
git commit -m "feat(machine-score): scaffold crate"
```

---

### Task 2: Scoring types, RAM/storage formulas

**Files:**
- Modify: `machine-score/src/scoring.rs`

**Interfaces:**
- Produces: `pub enum StorageKind { Nvme, Ssd, Hdd, Unknown }`,
  `pub struct ScoredComponent { pub score: f64, pub estimated: bool }`,
  `pub struct ComponentScores { pub cpu: ScoredComponent, pub gpu: ScoredComponent, pub ram: ScoredComponent, pub storage: ScoredComponent }`,
  `pub fn score_ram(total_gb: f64) -> ScoredComponent`,
  `pub fn score_storage(kind: StorageKind) -> ScoredComponent`.

- [ ] **Step 1: Write the failing tests**

Append to `machine-score/src/scoring.rs`:

```rust
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
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cd machine-score && cargo test formula_tests
```
Expected: FAIL to compile (types/functions not defined).

- [ ] **Step 3: Implement**

Replace the placeholder content of `machine-score/src/scoring.rs` with:

```rust
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
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
cd machine-score && cargo test formula_tests
```
Expected: 5 tests pass.

- [ ] **Step 5: Commit**

```bash
cd /home/fabio/dev/projects/dreamos-tools
git add machine-score/src/scoring.rs
git commit -m "feat(machine-score): RAM/storage scoring formulas"
```

---

### Task 3: CPU/GPU database lookup with fallback

**Files:**
- Modify: `machine-score/src/scoring.rs`

**Interfaces:**
- Consumes: `ScoredComponent` (Task 2).
- Produces: `pub struct DbEntry { pub model_substring: String, pub score: f64 }`,
  `pub fn score_cpu(model: &str, cores: usize, base_clock_mhz: u64, db: &[DbEntry]) -> ScoredComponent`,
  `pub fn score_gpu(name: &str, is_discrete: bool, db: &[DbEntry]) -> ScoredComponent`.

- [ ] **Step 1: Write the failing tests**

Append to `machine-score/src/scoring.rs`:

```rust
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
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cd machine-score && cargo test lookup_tests
```
Expected: FAIL to compile.

- [ ] **Step 3: Implement**

Append to `machine-score/src/scoring.rs`:

```rust
#[derive(Debug, Clone)]
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
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
cd machine-score && cargo test lookup_tests
```
Expected: 5 tests pass.

- [ ] **Step 5: Commit**

```bash
cd /home/fabio/dev/projects/dreamos-tools
git add machine-score/src/scoring.rs
git commit -m "feat(machine-score): CPU/GPU database lookup with fallback formulas"
```

---

### Task 4: Composite score and target readiness tiers

**Files:**
- Modify: `machine-score/src/scoring.rs`

**Interfaces:**
- Consumes: `ComponentScores` (Task 2).
- Produces: `pub enum Tier { Low, Medium, High, Ultra }`,
  `pub struct TargetReadiness { pub gaming: Tier, pub godot: Tier, pub unreal_engine_5: Tier }`,
  `pub fn composite_score(scores: &ComponentScores) -> f64`,
  `pub fn target_readiness(composite: f64) -> TargetReadiness`.

- [ ] **Step 1: Write the failing tests**

Append to `machine-score/src/scoring.rs`:

```rust
#[cfg(test)]
mod composite_tests {
    use super::*;

    fn scores(cpu: f64, gpu: f64, ram: f64, storage: f64) -> ComponentScores {
        ComponentScores {
            cpu: ScoredComponent { score: cpu, estimated: false },
            gpu: ScoredComponent { score: gpu, estimated: false },
            ram: ScoredComponent { score: ram, estimated: false },
            storage: ScoredComponent { score: storage, estimated: false },
        }
    }

    #[test]
    fn composite_of_all_100_is_100() {
        assert_eq!(composite_score(&scores(100.0, 100.0, 100.0, 100.0)), 100.0);
    }

    #[test]
    fn composite_of_all_0_is_0() {
        assert_eq!(composite_score(&scores(0.0, 0.0, 0.0, 0.0)), 0.0);
    }

    #[test]
    fn composite_weights_cpu_and_gpu_more_than_ram_and_storage() {
        let cpu_gpu_heavy = composite_score(&scores(100.0, 100.0, 0.0, 0.0));
        let ram_storage_heavy = composite_score(&scores(0.0, 0.0, 100.0, 100.0));
        assert!(cpu_gpu_heavy > ram_storage_heavy);
    }

    #[test]
    fn godot_reaches_higher_tiers_at_lower_scores_than_ue5() {
        // Same composite score, Godot's readiness must be >= UE5's at every point.
        for composite in [10.0, 30.0, 50.0, 70.0, 90.0] {
            let r = target_readiness(composite);
            assert!(tier_rank(r.godot) >= tier_rank(r.unreal_engine_5));
        }
    }

    #[test]
    fn tier_boundaries_are_exact() {
        // Gaming: Low<40, Medium<65, High<85, else Ultra (spec placeholder values).
        assert_eq!(target_readiness(39.9).gaming, Tier::Low);
        assert_eq!(target_readiness(40.0).gaming, Tier::Medium);
        assert_eq!(target_readiness(64.9).gaming, Tier::Medium);
        assert_eq!(target_readiness(65.0).gaming, Tier::High);
        assert_eq!(target_readiness(84.9).gaming, Tier::High);
        assert_eq!(target_readiness(85.0).gaming, Tier::Ultra);
    }

    fn tier_rank(t: Tier) -> u8 {
        match t {
            Tier::Low => 0,
            Tier::Medium => 1,
            Tier::High => 2,
            Tier::Ultra => 3,
        }
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cd machine-score && cargo test composite_tests
```
Expected: FAIL to compile.

- [ ] **Step 3: Implement**

Append to `machine-score/src/scoring.rs`:

```rust
/// Composite weights. Placeholder defaults, tunable once real-world
/// accuracy data exists.
const CPU_WEIGHT: f64 = 0.35;
const GPU_WEIGHT: f64 = 0.35;
const RAM_WEIGHT: f64 = 0.15;
const STORAGE_WEIGHT: f64 = 0.15;

pub fn composite_score(scores: &ComponentScores) -> f64 {
    scores.cpu.score * CPU_WEIGHT
        + scores.gpu.score * GPU_WEIGHT
        + scores.ram.score * RAM_WEIGHT
        + scores.storage.score * STORAGE_WEIGHT
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tier {
    Low,
    Medium,
    High,
    Ultra,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TargetReadiness {
    pub gaming: Tier,
    pub godot: Tier,
    pub unreal_engine_5: Tier,
}

fn tier_for(score: f64, low_max: f64, medium_max: f64, high_max: f64) -> Tier {
    if score < low_max {
        Tier::Low
    } else if score < medium_max {
        Tier::Medium
    } else if score < high_max {
        Tier::High
    } else {
        Tier::Ultra
    }
}

// Placeholder tier thresholds, tunable once real-world accuracy data
// exists. Godot's thresholds are lowest (lightest engine), UE5's
// highest (heaviest), Gaming in between.
const GAMING_LOW_MAX: f64 = 40.0;
const GAMING_MEDIUM_MAX: f64 = 65.0;
const GAMING_HIGH_MAX: f64 = 85.0;

const GODOT_LOW_MAX: f64 = 25.0;
const GODOT_MEDIUM_MAX: f64 = 50.0;
const GODOT_HIGH_MAX: f64 = 75.0;

const UE5_LOW_MAX: f64 = 50.0;
const UE5_MEDIUM_MAX: f64 = 70.0;
const UE5_HIGH_MAX: f64 = 90.0;

pub fn target_readiness(composite: f64) -> TargetReadiness {
    TargetReadiness {
        gaming: tier_for(composite, GAMING_LOW_MAX, GAMING_MEDIUM_MAX, GAMING_HIGH_MAX),
        godot: tier_for(composite, GODOT_LOW_MAX, GODOT_MEDIUM_MAX, GODOT_HIGH_MAX),
        unreal_engine_5: tier_for(composite, UE5_LOW_MAX, UE5_MEDIUM_MAX, UE5_HIGH_MAX),
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
cd machine-score && cargo test composite_tests
```
Expected: 5 tests pass.

- [ ] **Step 5: Commit**

```bash
cd /home/fabio/dev/projects/dreamos-tools
git add machine-score/src/scoring.rs
git commit -m "feat(machine-score): composite score and per-target readiness tiers"
```

---

### Task 5: Embedded placeholder databases

**Files:**
- Create: `machine-score/src/data/cpu_scores.json`
- Create: `machine-score/src/data/gpu_scores.json`
- Modify: `machine-score/src/scoring.rs`

**Interfaces:**
- Consumes: `DbEntry` (Task 3).
- Produces: `pub fn load_cpu_database() -> Vec<DbEntry>`, `pub fn load_gpu_database() -> Vec<DbEntry>`.

- [ ] **Step 1: Write the placeholder databases**

`machine-score/src/data/cpu_scores.json`:

```json
[
  { "model_substring": "Ryzen 9 7950X", "score": 95 },
  { "model_substring": "Ryzen 7 5800X", "score": 78 },
  { "model_substring": "Ryzen 5 5600X", "score": 68 },
  { "model_substring": "Core i9-13900K", "score": 96 },
  { "model_substring": "Core i7-12700K", "score": 82 },
  { "model_substring": "Core i5-12400", "score": 62 }
]
```

`machine-score/src/data/gpu_scores.json`:

```json
[
  { "model_substring": "RTX 4090", "score": 100 },
  { "model_substring": "RTX 4070", "score": 85 },
  { "model_substring": "RTX 3060", "score": 65 },
  { "model_substring": "RX 7900 XT", "score": 92 },
  { "model_substring": "RX 6600", "score": 55 },
  { "model_substring": "Iris Xe", "score": 20 }
]
```

Both files carry a leading comment noting the placeholder status - JSON
has no comment syntax, so this is instead documented in Step 3's
loader doc-comment and in the plan/spec, not in the files themselves.

- [ ] **Step 2: Write the failing tests**

Append to `machine-score/src/scoring.rs`:

```rust
#[cfg(test)]
mod database_tests {
    use super::*;

    #[test]
    fn cpu_database_loads_and_parses() {
        let db = load_cpu_database();
        assert!(!db.is_empty());
    }

    #[test]
    fn gpu_database_loads_and_parses() {
        let db = load_gpu_database();
        assert!(!db.is_empty());
    }

    #[test]
    fn loaded_cpu_database_is_usable_by_score_cpu() {
        let db = load_cpu_database();
        let result = score_cpu("AMD Ryzen 7 5800X 8-Core Processor", 8, 3800, &db);
        assert!(!result.estimated);
    }
}
```

- [ ] **Step 3: Run tests to verify they fail**

```bash
cd machine-score && cargo test database_tests
```
Expected: FAIL to compile (`load_cpu_database`/`load_gpu_database` not defined).

- [ ] **Step 4: Implement**

Append to `machine-score/src/scoring.rs`:

```rust
// Placeholder datasets - a small, illustrative set of known CPU/GPU
// models, not a curated real benchmark database. Swap the JSON files
// under src/data/ for a real dataset later; no code changes needed
// as long as the schema (model_substring, score) stays the same.
const CPU_DATABASE_JSON: &str = include_str!("data/cpu_scores.json");
const GPU_DATABASE_JSON: &str = include_str!("data/gpu_scores.json");

pub fn load_cpu_database() -> Vec<DbEntry> {
    serde_json::from_str(CPU_DATABASE_JSON).expect("embedded cpu_scores.json is valid")
}

pub fn load_gpu_database() -> Vec<DbEntry> {
    serde_json::from_str(GPU_DATABASE_JSON).expect("embedded gpu_scores.json is valid")
}
```

`DbEntry` needs `serde::Deserialize` - add the derive where it's
defined (Task 3's struct):

```rust
#[derive(Debug, Clone, serde::Deserialize)]
pub struct DbEntry {
    pub model_substring: String,
    pub score: f64,
}
```

- [ ] **Step 5: Run tests to verify they pass**

```bash
cd machine-score && cargo test
```
Expected: all tests so far pass (formula_tests: 5, lookup_tests: 5, composite_tests: 5, database_tests: 3 = 18 total).

- [ ] **Step 6: Commit**

```bash
cd /home/fabio/dev/projects/dreamos-tools
git add machine-score/src/scoring.rs machine-score/src/data/cpu_scores.json machine-score/src/data/gpu_scores.json
git commit -m "feat(machine-score): embed placeholder CPU/GPU score databases"
```

---

### Task 6: CPU/RAM/storage detection

**Files:**
- Modify: `machine-score/src/hardware.rs`

**Interfaces:**
- Consumes: `StorageKind` (Task 2, `crate::scoring::StorageKind`).
- Produces: `pub fn classify_storage(kind: sysinfo::DiskKind, device_name: &str) -> StorageKind`,
  `pub struct CpuRamStorageInfo { pub cpu_model: String, pub cpu_cores: usize, pub cpu_base_clock_mhz: u64, pub ram_total_gb: f64, pub storage_kind: StorageKind }`,
  `pub fn detect_cpu_ram_storage() -> CpuRamStorageInfo`.

- [ ] **Step 1: Write the failing test for the pure classification logic**

Replace the placeholder content of `machine-score/src/hardware.rs` with:

```rust
use crate::scoring::StorageKind;

pub fn classify_storage(kind: sysinfo::DiskKind, device_name: &str) -> StorageKind {
    todo!("implemented in step 3")
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
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cd machine-score && cargo test classify_tests
```
Expected: FAIL (unimplemented `todo!()`).

- [ ] **Step 3: Implement `classify_storage`**

```rust
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
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
cd machine-score && cargo test classify_tests
```
Expected: 5 tests pass.

- [ ] **Step 5: Implement real detection (not unit-tested - depends on real hardware)**

Append to `machine-score/src/hardware.rs`:

```rust
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
```

- [ ] **Step 6: Manual verification**

Add `#[derive(Debug)]` above `pub struct CpuRamStorageInfo` in
`hardware.rs`. Then temporarily replace `src/main.rs`'s `main()` body
with a one-off print (do not commit this swap - revert it in Step 7's
commit):

```rust
fn main() {
    println!("{:#?}", machine_score::hardware::detect_cpu_ram_storage());
}
```

```bash
cd machine-score && cargo run 2>&1 | tail -20
```
Expected: a `CpuRamStorageInfo` printed with a plausible CPU model
string, a believable core count and clock speed, a believable RAM size
in GB, and some `StorageKind` variant. Revert `main.rs` back to its
Task 1 placeholder content afterward (the real UI arrives in Task 8+).

- [ ] **Step 7: Commit**

```bash
cd /home/fabio/dev/projects/dreamos-tools
git add machine-score/src/hardware.rs
git commit -m "feat(machine-score): detect CPU/RAM/storage via sysinfo"
```

---

### Task 7: GPU detection

**Files:**
- Modify: `machine-score/src/hardware.rs`

**Interfaces:**
- Produces: `pub struct GpuAdapterInfo { pub name: String, pub is_discrete: bool }`,
  `pub fn pick_gpu_adapter(adapters: &[GpuAdapterInfo]) -> Option<GpuAdapterInfo>`,
  `pub fn detect_gpu() -> GpuAdapterInfo`.

- [ ] **Step 1: Write the failing test for the pure selection logic**

Append to `machine-score/src/hardware.rs`:

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct GpuAdapterInfo {
    pub name: String,
    pub is_discrete: bool,
}

pub fn pick_gpu_adapter(adapters: &[GpuAdapterInfo]) -> Option<GpuAdapterInfo> {
    todo!("implemented in step 3")
}

#[cfg(test)]
mod pick_gpu_tests {
    use super::*;

    #[test]
    fn prefers_first_discrete_adapter_over_integrated() {
        let adapters = vec![
            GpuAdapterInfo { name: "Intel Iris Xe".into(), is_discrete: false },
            GpuAdapterInfo { name: "NVIDIA RTX 4070".into(), is_discrete: true },
        ];
        let picked = pick_gpu_adapter(&adapters).unwrap();
        assert_eq!(picked.name, "NVIDIA RTX 4070");
    }

    #[test]
    fn falls_back_to_first_adapter_when_none_discrete() {
        let adapters = vec![
            GpuAdapterInfo { name: "Intel Iris Xe".into(), is_discrete: false },
        ];
        let picked = pick_gpu_adapter(&adapters).unwrap();
        assert_eq!(picked.name, "Intel Iris Xe");
    }

    #[test]
    fn returns_none_for_empty_adapter_list() {
        assert_eq!(pick_gpu_adapter(&[]), None);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cd machine-score && cargo test pick_gpu_tests
```
Expected: FAIL (`todo!()`).

- [ ] **Step 3: Implement `pick_gpu_adapter`**

```rust
pub fn pick_gpu_adapter(adapters: &[GpuAdapterInfo]) -> Option<GpuAdapterInfo> {
    adapters
        .iter()
        .find(|a| a.is_discrete)
        .or_else(|| adapters.first())
        .cloned()
}
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
cd machine-score && cargo test pick_gpu_tests
```
Expected: 3 tests pass.

- [ ] **Step 5: Implement real detection (not unit-tested - depends on real hardware/drivers)**

Append to `machine-score/src/hardware.rs`:

```rust
pub fn detect_gpu() -> GpuAdapterInfo {
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        ..Default::default()
    });
    let adapters = pollster::block_on(instance.enumerate_adapters(wgpu::Backends::all()));
    let mapped: Vec<GpuAdapterInfo> = adapters
        .iter()
        .map(|a| {
            let info = a.get_info();
            GpuAdapterInfo {
                name: info.name,
                is_discrete: info.device_type == wgpu::DeviceType::DiscreteGpu,
            }
        })
        .collect();
    pick_gpu_adapter(&mapped).unwrap_or(GpuAdapterInfo {
        name: "Unknown GPU".into(),
        is_discrete: false,
    })
}
```

- [ ] **Step 6: Manual verification**

Temporarily swap `src/main.rs`'s `main()` body (same approach as Task
6 Step 6, not committed):

```rust
fn main() {
    println!("{:#?}", machine_score::hardware::detect_gpu());
}
```

```bash
cd machine-score && cargo run 2>&1 | tail -20
```
Expected: a `GpuAdapterInfo` with a plausible GPU name (the actual GPU
or a software/llvmpipe renderer if running headless/in a VM - either
is fine, this just confirms the `wgpu`/`pollster` wiring works) and a
sensible `is_discrete` value. Revert `main.rs` afterward.

- [ ] **Step 7: Commit**

```bash
cd /home/fabio/dev/projects/dreamos-tools
git add machine-score/src/hardware.rs
git commit -m "feat(machine-score): detect GPU via wgpu adapter enumeration"
```

---

### Task 8: egui application skeleton, header, Welcome screen

**Files:**
- Modify: `machine-score/src/main.rs`

**Interfaces:**
- Consumes: nothing from `hardware`/`scoring` yet (wired in Tasks 9-10).
- Produces: an `eframe::App` with a `Screen` enum (`Welcome`, `Scoring`, `Results`), a shared header-drawing function, and a working Welcome screen.

- [ ] **Step 1: Implement the app skeleton with the persistent header and Welcome screen**

Replace `machine-score/src/main.rs`:

```rust
const LOGO_BYTES: &[u8] = include_bytes!("../assets/ball.png");

fn load_logo_texture(ctx: &egui::Context) -> egui::TextureHandle {
    let img = image::load_from_memory(LOGO_BYTES)
        .expect("embedded assets/ball.png is a valid PNG")
        .to_rgba8();
    let size = [img.width() as usize, img.height() as usize];
    let color_image = egui::ColorImage::from_rgba_unmultiplied(size, img.as_raw());
    ctx.load_texture("logo", color_image, egui::TextureOptions::default())
}

fn draw_header(ui: &mut egui::Ui, logo: &egui::TextureHandle) {
    ui.horizontal(|ui| {
        ui.image((logo.id(), egui::vec2(48.0, 48.0)));
        ui.heading("DreamOS Machine Score");
    });
    ui.separator();
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Screen {
    Welcome,
    Scoring,
    Results,
}

struct MachineScoreApp {
    logo: Option<egui::TextureHandle>,
    screen: Screen,
}

impl MachineScoreApp {
    fn new() -> Self {
        Self { logo: None, screen: Screen::Welcome }
    }
}

impl eframe::App for MachineScoreApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let logo = self.logo.get_or_insert_with(|| load_logo_texture(ctx)).clone();

        egui::CentralPanel::default().show(ctx, |ui| {
            draw_header(ui, &logo);

            match self.screen {
                Screen::Welcome => {
                    ui.label(
                        "DreamOS Machine Score evaluates whether this PC is ready for \
                         gaming and for game development with Godot or Unreal Engine 5. \
                         It detects your CPU, GPU, RAM, and storage, scores them, and \
                         shows a readiness breakdown for each target.",
                    );
                    ui.add_space(12.0);
                    if ui.button("Start").clicked() {
                        self.screen = Screen::Scoring;
                    }
                }
                Screen::Scoring => {
                    ui.label("Scoring screen - Task 9");
                }
                Screen::Results => {
                    ui.label("Results screen - Task 10");
                }
            }
        });
    }
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([700.0, 500.0]),
        ..Default::default()
    };
    eframe::run_native(
        "DreamOS Machine Score",
        options,
        Box::new(|_cc| Ok(Box::new(MachineScoreApp::new()))),
    )
}
```

- [ ] **Step 2: Build**

```bash
cd machine-score && cargo build 2>&1 | tail -80
```
Expected: builds with no errors. If `eframe::Result`/`NativeOptions`/
`ViewportBuilder` field names differ slightly from what's pinned
(`eframe` 0.33 API surface, confirmed during planning but re-check
here), fix based on the compiler's exact error - the shape above is
correct for `eframe` 0.33 as verified when this plan was written.

- [ ] **Step 3: Manual verification**

```bash
cd machine-score && timeout 4 env DISPLAY=:0 cargo run 2>&1 | tail -60
```
Expected: no crash, no panic in the 4s window. If a graphical session
is available, this opens a window titled "DreamOS Machine Score"
showing the logo, title, welcome text, and a "Start" button; clicking
it should (based on code review, since the window closes on timeout)
switch to the Scoring placeholder screen.

- [ ] **Step 4: Commit**

```bash
cd /home/fabio/dev/projects/dreamos-tools
git add machine-score/src/main.rs
git commit -m "feat(machine-score): egui app skeleton, persistent header, Welcome screen"
```

---

### Task 9: Scoring screen - background detection with progress

**Files:**
- Modify: `machine-score/src/main.rs`

**Interfaces:**
- Consumes: `hardware::{detect_cpu_ram_storage, detect_gpu, CpuRamStorageInfo, GpuAdapterInfo}` (Tasks 6-7), `scoring::{score_cpu, score_gpu, score_ram, score_storage, composite_score, target_readiness, load_cpu_database, load_gpu_database, ComponentScores, TargetReadiness}` (Tasks 2-5).
- Produces: `ScoringMsg` enum, background-thread wiring, a `ResultData` struct carried into Task 10's Results screen.

- [ ] **Step 1: Add the message type and result data**

Add to `machine-score/src/main.rs`, above `struct MachineScoreApp`:

```rust
enum ScoringMsg {
    DetectingCpu,
    DetectingGpu,
    DetectingRam,
    DetectingStorage,
    Done(ResultData),
}

#[derive(Clone)]
struct ResultData {
    cpu_model: String,
    gpu_name: String,
    ram_total_gb: f64,
    scores: machine_score::scoring::ComponentScores,
    composite: f64,
    readiness: machine_score::scoring::TargetReadiness,
}
```

- [ ] **Step 2: Add scoring state to `MachineScoreApp` and a `start_scoring` method**

Modify the `MachineScoreApp` struct and `new()`:

```rust
struct MachineScoreApp {
    logo: Option<egui::TextureHandle>,
    screen: Screen,
    scoring_rx: Option<std::sync::mpsc::Receiver<ScoringMsg>>,
    progress_label: String,
    result: Option<ResultData>,
}

impl MachineScoreApp {
    fn new() -> Self {
        Self {
            logo: None,
            screen: Screen::Welcome,
            scoring_rx: None,
            progress_label: String::new(),
            result: None,
        }
    }

    fn start_scoring(&mut self) {
        let (tx, rx) = std::sync::mpsc::channel();
        self.scoring_rx = Some(rx);
        self.progress_label = "Starting...".into();

        std::thread::spawn(move || {
            use machine_score::{hardware, scoring};

            let _ = tx.send(ScoringMsg::DetectingCpu);
            let cpu_ram_storage = hardware::detect_cpu_ram_storage();

            let _ = tx.send(ScoringMsg::DetectingGpu);
            let gpu = hardware::detect_gpu();

            let _ = tx.send(ScoringMsg::DetectingRam);
            let ram_score = scoring::score_ram(cpu_ram_storage.ram_total_gb);

            let _ = tx.send(ScoringMsg::DetectingStorage);
            let storage_score = scoring::score_storage(cpu_ram_storage.storage_kind);

            let cpu_db = scoring::load_cpu_database();
            let gpu_db = scoring::load_gpu_database();
            let cpu_score = scoring::score_cpu(
                &cpu_ram_storage.cpu_model,
                cpu_ram_storage.cpu_cores,
                cpu_ram_storage.cpu_base_clock_mhz,
                &cpu_db,
            );
            let gpu_score = scoring::score_gpu(&gpu.name, gpu.is_discrete, &gpu_db);

            let scores = scoring::ComponentScores {
                cpu: cpu_score,
                gpu: gpu_score,
                ram: ram_score,
                storage: storage_score,
            };
            let composite = scoring::composite_score(&scores);
            let readiness = scoring::target_readiness(composite);

            let _ = tx.send(ScoringMsg::Done(ResultData {
                cpu_model: cpu_ram_storage.cpu_model,
                gpu_name: gpu.name,
                ram_total_gb: cpu_ram_storage.ram_total_gb,
                scores,
                composite,
                readiness,
            }));
        });
    }
}
```

- [ ] **Step 3: Wire the Scoring screen's UI and per-frame poll**

Replace the `Screen::Scoring` arm inside `update()`:

```rust
Screen::Scoring => {
    if self.scoring_rx.is_none() {
        if ui.button("Run Scoring").clicked() {
            self.start_scoring();
        }
    } else {
        ui.label(&self.progress_label);
        ui.add(egui::widgets::Spinner::new());

        let mut finished = false;
        if let Some(rx) = &self.scoring_rx {
            for msg in rx.try_iter() {
                match msg {
                    ScoringMsg::DetectingCpu => self.progress_label = "Detecting CPU...".into(),
                    ScoringMsg::DetectingGpu => self.progress_label = "Detecting GPU...".into(),
                    ScoringMsg::DetectingRam => self.progress_label = "Detecting RAM...".into(),
                    ScoringMsg::DetectingStorage => self.progress_label = "Detecting storage...".into(),
                    ScoringMsg::Done(data) => {
                        self.result = Some(data);
                        finished = true;
                    }
                }
            }
        }
        if finished {
            self.scoring_rx = None;
            self.screen = Screen::Results;
        } else {
            ctx.request_repaint();
        }
    }
}
```

- [ ] **Step 4: Build**

```bash
cd machine-score && cargo build 2>&1 | tail -100
```
Expected: builds with no errors.

- [ ] **Step 5: Run the full test suite (regression check)**

```bash
cd machine-score && cargo test 2>&1 | tail -20
```
Expected: all prior tests (Tasks 2-7) still pass; no new tests here
since this task is UI wiring, not new pure logic.

- [ ] **Step 6: Manual verification**

```bash
cd machine-score && timeout 6 env DISPLAY=:0 cargo run 2>&1 | tail -60
```
Expected: no crash. Based on code review (window closes on timeout):
clicking Start -> Run Scoring shows the progress label cycling through
detection stages, then auto-advances to the Results placeholder.

- [ ] **Step 7: Commit**

```bash
cd /home/fabio/dev/projects/dreamos-tools
git add machine-score/src/main.rs
git commit -m "feat(machine-score): Scoring screen with background detection and progress"
```

---

### Task 10: Results screen

**Files:**
- Modify: `machine-score/src/main.rs`

**Interfaces:**
- Consumes: `ResultData` (Task 9), `scoring::Tier` (Task 4).
- Produces: a Results screen showing the composite score, per-target tiers, estimated-component notes, and Restart/Close buttons.

- [ ] **Step 1: Add a `Tier` label helper**

Add to `machine-score/src/main.rs`:

```rust
fn tier_label(tier: machine_score::scoring::Tier) -> &'static str {
    use machine_score::scoring::Tier;
    match tier {
        Tier::Low => "Low",
        Tier::Medium => "Medium",
        Tier::High => "High",
        Tier::Ultra => "Ultra",
    }
}
```

- [ ] **Step 2: Replace the `Screen::Results` arm**

```rust
Screen::Results => {
    if let Some(result) = self.result.clone() {
        ui.add_space(8.0);
        ui.label(egui::RichText::new(format!("{:.0}", result.composite)).size(64.0).strong());
        ui.label("Overall Score (0-100)");
        ui.add_space(16.0);

        ui.label(format!("CPU: {}", result.cpu_model));
        ui.label(format!("GPU: {}", result.gpu_name));
        ui.label(format!("RAM: {:.0} GiB", result.ram_total_gb));
        ui.add_space(8.0);

        for (label, component) in [
            ("CPU", result.scores.cpu),
            ("GPU", result.scores.gpu),
            ("RAM", result.scores.ram),
            ("Storage", result.scores.storage),
        ] {
            let note = if component.estimated { " (estimated - model not in database)" } else { "" };
            ui.label(format!("{label} score: {:.0}{note}", component.score));
        }

        ui.add_space(16.0);
        ui.heading("Readiness");
        ui.label(format!("Gaming: {}", tier_label(result.readiness.gaming)));
        ui.label(format!("Godot: {}", tier_label(result.readiness.godot)));
        ui.label(format!("Unreal Engine 5: {}", tier_label(result.readiness.unreal_engine_5)));

        ui.add_space(16.0);
        ui.horizontal(|ui| {
            if ui.button("Restart").clicked() {
                self.result = None;
                self.screen = Screen::Welcome;
            }
            if ui.button("Close").clicked() {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        });
    } else {
        ui.label("No result yet.");
    }
}
```

- [ ] **Step 3: Build**

```bash
cd machine-score && cargo build 2>&1 | tail -100
```
Expected: builds with no errors. If `egui::ViewportCommand::Close`
differs from the pinned `egui` 0.33 API, fix per the compiler's exact
error (this is the documented way to close an eframe window as of the
version pinned when this plan was written).

- [ ] **Step 4: Run the full test suite (regression check)**

```bash
cd machine-score && cargo test 2>&1 | tail -20
```
Expected: all 18 tests from Tasks 2-7 still pass.

- [ ] **Step 5: Manual end-to-end verification**

```bash
cd machine-score && timeout 8 env DISPLAY=:0 cargo run 2>&1 | tail -80
```
Expected: no crash across the full Welcome -> Start -> Run Scoring ->
(auto) Results flow within the timeout window. If a graphical session
is available for interactive testing, click through manually and
confirm: the composite score renders large, CPU/GPU/RAM lines show
plausible detected values, any `estimated` component shows the note,
all three tier labels render, Restart returns to Welcome and a second
run re-detects from scratch, Close quits the app.

- [ ] **Step 6: Commit**

```bash
cd /home/fabio/dev/projects/dreamos-tools
git add machine-score/src/main.rs
git commit -m "feat(machine-score): Results screen with score, tiers, Restart/Close"
```

---

### Task 11: README and final verification

**Files:**
- Create: `machine-score/README.md`

**Interfaces:**
- Consumes: nothing new.
- Produces: documented build/run/test procedure and the Windows-verification follow-up noted explicitly.

- [ ] **Step 1: Write the README**

Create `machine-score/README.md`:

```markdown
# machine-score

Cross-platform (Windows/Linux) egui wizard that detects this PC's
CPU/GPU/RAM/storage, scores them against an embedded database (with a
rough formula fallback for unrecognized models), and shows a composite
0-100 score plus Gaming/Godot/Unreal Engine 5 readiness tiers. See
[the design spec](../docs/superpowers/specs/2026-09-19-machine-score-design.md)
for the full rationale and the placeholder-dataset/threshold caveats.

## Build

    cargo build --release

Output: `target/release/machine-score` (or `machine-score.exe` on
Windows).

## Run

    ./target/release/machine-score

No elevated privileges needed - this tool only reads hardware info.

## Test

    cargo test

18 unit tests cover `scoring.rs` (database lookup, fallback formulas,
composite weighting, tier boundaries) and `hardware.rs`'s pure
classification/selection logic (`classify_storage`, `pick_gpu_adapter`).
Real detection (`detect_cpu_ram_storage`, `detect_gpu`) is not
unit-tested - it depends on real, varying hardware - and was instead
manually verified during development by printing its output and
sanity-checking the values (see the implementation plan's Task 6/7
manual-verification steps).

## Known limitations (v1)

- The CPU/GPU score databases (`src/data/*.json`) are small
  placeholder datasets, not a curated real benchmark source. Swap the
  JSON files for a real dataset later - same schema
  (`model_substring`, `score`), no code changes needed.
- Composite weights and per-target tier thresholds are placeholder
  constants in `scoring.rs`, not tuned against real-world accuracy data.
- No VRAM-based GPU scoring - `wgpu` has no reliable cross-vendor VRAM
  query, so the GPU fallback formula only considers discrete vs
  integrated.
- Windows has not been built or run in this development environment
  (Linux-only here) - the code targets both platforms via
  `eframe`/`sysinfo`/`wgpu`, all of which support Windows, but an
  actual Windows build/run is a follow-up requiring a Windows machine
  or CI.
```

- [ ] **Step 2: Full test suite + release build**

```bash
cd machine-score && cargo test 2>&1 | tail -20 && cargo build --release 2>&1 | tail -40
```
Expected: 18 tests pass; release binary builds successfully at
`target/release/machine-score`.

- [ ] **Step 3: Commit**

```bash
cd /home/fabio/dev/projects/dreamos-tools
git add machine-score/README.md
git commit -m "docs(machine-score): README with build/run/test/limitations"
```

---

## Self-Review Notes

- **Spec coverage:** cross-platform egui UI (Task 8), persistent
  header (Task 8), CPU/GPU/RAM/storage detection (Tasks 6-7), database
  lookup + fallback (Tasks 3, 5), composite + per-target tiers (Task
  4), 3-screen wizard with background-threaded scoring + progress
  (Tasks 8-10), Restart/Close (Task 10), no privilege elevation
  (never added, correctly - this tool doesn't need it), Windows
  follow-up documented (Task 11) - all covered.
- **Type consistency checked:** `StorageKind`, `ScoredComponent`,
  `ComponentScores`, `DbEntry`, `Tier`, `TargetReadiness`,
  `CpuRamStorageInfo`, `GpuAdapterInfo`, `ResultData`, `ScoringMsg` are
  each defined once and referenced identically by name and field
  across every task that uses them.
- **Known gaps flagged explicitly (scoped decisions, not
  placeholders):** real score datasets, tuned weights/thresholds, and
  Windows verification are all explicitly out of scope per the spec's
  non-goals, and Task 11's README states this plainly for future
  readers.
- **API-version risk noted:** `eframe`/`egui` 0.33 and `wgpu` 30 API
  shapes used in Tasks 7-10 were verified against the actual pinned
  crate source during planning; Tasks 8 and 10 each include a note to
  fix based on the compiler's exact error if a patch-version API
  change has occurred by implementation time.
