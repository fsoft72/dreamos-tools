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

26 unit tests cover `scoring.rs` (database lookup, fallback formulas,
composite weighting, tier boundaries) and `hardware.rs`'s pure
classification/selection logic (`classify_storage`, `pick_gpu_adapter`).
Real detection (`detect_cpu_ram_storage`, `detect_gpu`) is not
unit-tested - it depends on real, varying hardware - and was instead
manually verified during development by printing its output and
sanity-checking the values (see the implementation plan's Task 6/7
manual-verification steps). On the dev machine this produced a correct
CPU model string, core count, RAM size, and correctly identified a
discrete NVIDIA GPU.

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
