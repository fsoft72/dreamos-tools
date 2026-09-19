# DreamOS Machine Score - design spec

Date: 2026-09-19
Repo: `dreamos-tools` (this repo), folder `machine-score/`
Related: [`docs/dreamos.md`](../../dreamos.md) (dreamos distro context),
[`filesys-extender`'s spec](2026-09-19-filesys-extender-design.md) (this
repo's first tool, whose lib/bin split and asset-embedding pattern this
one follows).

## Problem

Users want a quick, no-install read on whether their PC is up to
gaming and to running game-development engines (Godot, Unreal Engine
5). DreamOS Machine Score detects the machine's CPU, GPU, RAM, and
storage, scores them against an embedded database of known hardware,
and presents a single composite score plus per-engine/per-use-case
readiness tiers.

## Goals

- Detect CPU, GPU, RAM, and storage on both Windows and Linux from one
  codebase.
- Score CPU and GPU by looking their detected model name up in a
  bundled database; fall back to a rough formula estimate when the
  model isn't found (very likely at first, since the shipped database
  starts as a small placeholder - a real dataset is a follow-up task,
  not part of this spec).
- Score RAM and storage from their detected quantity/type directly (no
  database - these aren't "models" to look up).
- Combine the four component scores into one composite score (0-100)
  and show tiered (Low/Medium/High/Ultra) readiness for three targets:
  Gaming, Godot, Unreal Engine 5.
- A three-screen wizard (Welcome -> Scoring -> Results) with a
  persistent header (`ball.png` logo + "DreamOS Machine Score" title)
  on every screen, matching `filesys-extender`'s visual pattern.

## Non-goals (v1)

- No live micro-benchmarking (no CPU stress loop, no disk read/write
  test) - detection + database/formula lookup only, per the "pure
  lookup" decision made during brainstorming.
- No per-target weight profiles - Gaming/Godot/UE5 all read the same
  composite score, just against different tier thresholds.
- No VRAM-based GPU scoring - `wgpu` has no reliable cross-vendor VRAM
  query, so VRAM is not a scoring input in v1.
- No real, tuned score database or tier thresholds - all numeric
  constants (weights, formula coefficients, tier cutoffs) are
  placeholders, explicitly marked for later tuning once a real dataset
  exists.
- No Windows testing in this development environment - the code is
  written to be cross-platform and builds for both targets, but actual
  verification here is Linux-only; Windows build/run is a documented
  follow-up.

## Repo layout

```
dreamos-tools/
  machine-score/
    Cargo.toml
    assets/
      ball.png                  # copied from repo-root assets/, embedded via include_bytes!
    src/
      data/
        cpu_scores.json          # placeholder CPU model -> score database
        gpu_scores.json          # placeholder GPU model -> score database
      lib.rs                     # re-exports hardware:: and scoring::
      hardware.rs                 # CPU/GPU/RAM/storage detection - no UI dep
      scoring.rs                   # database lookup, fallback formulas, composite, tiers - no UI dep
      main.rs                       # egui wizard
```

Single crate, same lib/bin split rationale as `filesys-extender`:
`hardware.rs` and `scoring.rs` are unit-testable without a display or
real hardware assumptions baked in; `main.rs` is the only place that
touches `eframe`/`egui`. No shared Cargo workspace with
`filesys-extender` yet (still just two sibling tool folders); revisit
if real code-sharing need appears later.

Dependencies: `eframe`/`egui` (UI), `sysinfo` (CPU/RAM/storage
detection), `wgpu` (GPU adapter enumeration), `serde`/`serde_json`
(embedded database format, matches `filesys-extender`'s existing use
of the same crates).

## Architecture

### No privilege elevation

Unlike `filesys-extender`, this tool only reads hardware information -
it never writes to disk or changes system state. No `pkexec`, no root
requirement, no polkit. This significantly simplifies the app compared
to `filesys-extender`: no self-elevation logic, no destructive-action
safety gates.

### Hardware detection (`hardware.rs`)

```rust
pub struct DetectedHardware {
    pub cpu_model: String,
    pub cpu_cores: usize,
    pub cpu_base_clock_mhz: u64,
    pub gpu_name: String,
    pub gpu_is_discrete: bool,
    pub ram_total_gb: f64,
    pub storage_kind: StorageKind, // Nvme | Ssd | Hdd | Unknown
}

pub enum StorageKind { Nvme, Ssd, Hdd, Unknown }
```

- CPU: `sysinfo::System` - `cpus()[0].brand()` for the model string,
  `cpus().len()` for physical core count (or logical, whichever
  `sysinfo`'s API most directly exposes - documented in the
  implementation, not re-litigated here), `cpus()[0].frequency()` for
  base clock in MHz.
- RAM: `sysinfo::System::total_memory()`, converted to GB.
- Storage: `sysinfo::Disks` - the boot/system disk's `kind()`
  (`sysinfo::DiskKind`) maps to `StorageKind`; `sysinfo` doesn't
  distinguish NVMe from SATA SSD directly, so NVMe is inferred from
  the device name containing `nvme` (Linux: `/dev/nvme0n1`-style
  names; Windows: physical drive model string) - falls back to `Ssd`
  if the name doesn't indicate NVMe.
- GPU: `wgpu::Instance::new(...)`, then `enumerate_adapters(...)` (or
  the closest available API in the pinned `wgpu` version - resolved
  during implementation) to get each adapter's `AdapterInfo` (`name`,
  `device_type`). The first discrete adapter is preferred if multiple
  exist (common on laptops with integrated + discrete GPUs); falls
  back to the first adapter found if none are discrete.

### Scoring (`scoring.rs`)

```rust
pub struct ComponentScores {
    pub cpu: ScoredComponent,
    pub gpu: ScoredComponent,
    pub ram: ScoredComponent,
    pub storage: ScoredComponent,
}

pub struct ScoredComponent {
    pub score: f64,       // 0-100
    pub estimated: bool,  // true when no database match was found
}

pub struct TargetReadiness {
    pub gaming: Tier,
    pub godot: Tier,
    pub unreal_engine_5: Tier,
}

pub enum Tier { Low, Medium, High, Ultra }
```

- `score_cpu(model: &str, cores: usize, base_clock_mhz: u64) -> ScoredComponent`:
  substring-match `model` (case-insensitive) against
  `src/data/cpu_scores.json` entries; on a match, `estimated: false`.
  On no match: `estimated: true`, score from
  `normalize(cores * base_clock_mhz)` (constants/normalization curve
  defined in code, tunable).
- `score_gpu(name: &str, is_discrete: bool) -> ScoredComponent`: same
  substring-match against `gpu_scores.json`; on no match, `estimated:
  true`, score = a fixed base value for discrete vs integrated (e.g.
  discrete defaults higher than integrated - exact placeholder values
  defined in code, tunable). This is a deliberately rough fallback,
  noted in the spec's non-goals as a known v1 limitation.
- `score_ram(total_gb: f64) -> ScoredComponent`: `min(total_gb / 32.0 * 100.0, 100.0)`,
  never estimated (it's a direct formula, not a lookup with a
  possible miss).
- `score_storage(kind: StorageKind) -> ScoredComponent`: fixed score
  by kind (Nvme highest, then Ssd, then Hdd, then Unknown as a
  middling default), never estimated.
- `composite_score(scores: &ComponentScores) -> f64`: weighted average
  with placeholder default weights (CPU 35%, GPU 35%, RAM 15%, Storage
  15%), defined as named constants so they're easy to find and tune.
- `target_readiness(composite: f64) -> TargetReadiness`: compares the
  same composite score against three independent sets of tier
  thresholds (Gaming, Godot, UE5), each a set of named constants
  (Godot's thresholds lowest, UE5's highest, Gaming in between).

### Embedded database format

`src/data/cpu_scores.json` / `gpu_scores.json`, shipped with a small
placeholder dataset (a handful of well-known CPU/GPU models) so the
lookup path is exercised in tests and in the running app, with a clear
comment at the top of each file noting it's a placeholder to be
replaced with a real, curated dataset later:

```json
[
  { "model_substring": "Ryzen 7 5800X", "score": 78 },
  { "model_substring": "Core i7-12700K", "score": 82 }
]
```

Loaded via `include_str!` (matches `filesys-extender`'s
`include_bytes!` pattern for its logo) and parsed with `serde_json` at
startup - no filesystem dependency at runtime, consistent with the
first tool's self-contained-binary approach.

## UI flow (egui, persistent header on every screen)

Header: `ball.png` logo (from `machine-score/assets/ball.png`, copied
from the repo-root `assets/ball.png` the same way `filesys-extender`
did) on the top-left, "DreamOS Machine Score" title next to it - drawn
by a shared header-drawing function called at the top of every
screen's `egui::CentralPanel` update, not a separate persistent
widget/window (egui is immediate-mode; "persistent header" means "the
same function runs every frame regardless of which screen is active").

1. **Welcome** - header, a welcome message explaining the tool's
   purpose (evaluating PC performance for gaming and for Godot/UE5
   game development), "Start" button advances to Scoring.
2. **Scoring** - header, "Run Scoring" button. On click, detection
   runs on a background thread (mirrors `filesys-extender`'s
   `execute_steps`-over-`mpsc`-channel pattern, since `wgpu` adapter
   creation can involve driver calls that shouldn't block the UI
   thread even though they're normally sub-second) sending progress
   messages (`DetectingCpu`, `DetectingGpu`, `DetectingRam`,
   `DetectingStorage`, `Done(DetectedHardware)`) back over an
   `std::sync::mpsc` channel; the main thread polls it each frame (via
   `egui::Context::request_repaint` while the background thread runs,
   not a GTK-style timeout source - the egui/eframe equivalent) and
   updates a 4-stage progress bar. On `Done`, auto-advances to Results.
3. **Results** - header, composite score (0-100) in a large font,
   per-target (Gaming / Godot / Unreal Engine 5) tier breakdown, plus
   a note next to any component whose score was `estimated: true`
   ("GPU score estimated - model not in database"). "Restart" button
   returns to Welcome and clears prior results (a fresh run
   re-detects everything, no caching). "Close" button quits the app.

## Testing

- `scoring.rs`: unit tests for database hit/miss on `score_cpu`/
  `score_gpu` (using a small fixture database, not the real bundled
  one, so tests don't churn when the placeholder data changes),
  formula-only `score_ram`/`score_storage` across their input ranges,
  `composite_score` weighting math, and `target_readiness` tier
  boundaries (exact edge values, one below/above each cutoff). All
  pure functions, no hardware or filesystem access needed.
- `hardware.rs`: not meaningfully unit-testable (depends on real
  hardware varying machine to machine) - verified manually by running
  the detection functions on the dev machine and sanity-checking the
  printed values look plausible (correct CPU model string, believable
  core count/clock, some GPU name, some plausible RAM/storage numbers).
- End-to-end UI: manual smoke test on Linux (build and run, click
  through all three screens, confirm the composite score and tier
  breakdown render and Restart/Close work). Windows build/run is a
  documented follow-up requiring a Windows machine or CI - out of
  scope for verification in this development environment.

## Open questions / follow-ups for the implementation plan

- Exact `wgpu` API surface for adapter enumeration depends on the
  pinned version at implementation time (`wgpu`'s adapter-enumeration
  API has changed across versions) - resolved when the dependency is
  actually added, not blocking the plan's task breakdown.
- Real CPU/GPU score datasets, real weight tuning, and real tier
  threshold tuning are explicitly deferred - this spec only commits to
  the lookup/fallback/composite *mechanism*, not accurate numbers.
- Windows-side verification (build + manual run) is a follow-up once a
  Windows environment or CI is available.
