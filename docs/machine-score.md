# DreamOS Machine Score - user documentation

`machine-score` is a cross-platform (Windows/Linux) desktop app that
detects this PC's CPU, GPU, RAM, and storage, scores them against an
embedded database (with a rough formula fallback for unrecognized
hardware), and shows a single composite score plus readiness for
Gaming, Godot, and Unreal Engine 5.

Source: `machine-score/` in this repo (`dreamos-tools`). Design
rationale: [`docs/superpowers/specs/2026-09-19-machine-score-design.md`](superpowers/specs/2026-09-19-machine-score-design.md).
Developer-facing build/test notes: [`machine-score/README.md`](../machine-score/README.md).

## What it does

It answers one question: *is this PC ready for gaming, and for
building games with Godot or Unreal Engine 5?* It detects your
hardware, scores it, and shows a plain composite number (0-100) plus a
Low/Medium/High/Ultra readiness tier for each of the three targets.

It never writes anything, changes any setting, or needs elevated
privileges - it only reads hardware information.

## How scoring works, precisely

- **CPU and GPU** are identified by model name and looked up in a
  small embedded database. If the exact model isn't in the database
  (likely, since it currently ships a small placeholder dataset - see
  Limitations below), the score falls back to a rough estimate: CPU
  from core count x clock speed, GPU from whether it's a discrete or
  integrated card. Estimated scores are clearly labeled as such on the
  Results screen.
- **RAM** is scored from total capacity (scales up to 32 GiB, capped
  at 100 above that).
- **Storage** is scored from the type of your boot/system disk (NVMe
  highest, then SATA SSD, then HDD).
- These four component scores are combined into one **composite score
  (0-100)**, weighted toward CPU and GPU (they matter most for gaming
  and engine performance) over RAM and storage.
- The same composite score is compared against three separate sets of
  tier thresholds - one per target - to produce the Gaming / Godot /
  Unreal Engine 5 readiness shown on the Results screen. Godot's
  thresholds are the most lenient (it's the lightest engine), Unreal
  Engine 5's the strictest.

No live benchmarking happens - no CPU stress test, no disk speed test.
Everything is detection plus lookup/formula, so a run takes well under
a second.

## Installing / running

To build and run it directly:

```sh
cd machine-score
cargo build --release
./target/release/machine-score
```

(`machine-score.exe` on Windows.) No `sudo`/admin privileges needed.

For quick iteration from this repo's root:

```sh
./scripts/run-machine-score.sh
```

## Using the wizard

The window has three screens. Every screen shows the DreamOS logo and
"DreamOS Machine Score" title at the top (larger on the Welcome
screen, smaller on the others to leave room for content), and its
action button(s) pinned to the bottom-right of the window.

### 1. Welcome

A short description of what the tool does. Click **Start** to
continue.

### 2. Scoring

Click **Run Scoring** to begin. Detection runs in the background (CPU,
then GPU, then RAM, then storage) with a short progress label and
spinner; the screen automatically advances to Results once done - no
input needed while it runs.

### 3. Results

Shows, top to bottom:

- The **composite score** in large text (0-100).
- Detected CPU model, GPU name, and RAM size.
- Each component's individual score, with `(estimated - model not in
  database)` noted next to any CPU or GPU score that came from the
  fallback formula rather than a database match.
- **Readiness**: Gaming / Godot / Unreal Engine 5, each as Low /
  Medium / High / Ultra.

If the content is taller than the window, it scrolls - nothing is cut
off. **Restart** clears the result and returns to Welcome (a second
run always re-detects everything from scratch, nothing is cached).
**Close** quits the app.

## Known limitations (v1)

These are deliberate scope decisions for this first version, not bugs:

- **Placeholder score databases.** `src/data/cpu_scores.json` and
  `gpu_scores.json` contain only a small illustrative set of CPU/GPU
  models, not a curated real benchmark dataset. Most real-world
  hardware will currently show as "estimated." Swapping in a real
  dataset later needs no code changes - just replace the JSON files
  (same `model_substring`/`score` schema).
- **Untuned weights and tiers.** The composite score's component
  weights and each target's tier thresholds are placeholder constants
  in `scoring.rs`, not tuned against real-world accuracy data.
- **No VRAM in GPU scoring.** There's no reliable way to query VRAM
  size across both NVIDIA/AMD/Intel and Windows/Linux, so the GPU
  fallback estimate only considers discrete vs. integrated, not memory
  size.
- **Windows is untested in development.** The code targets Windows and
  Linux from one codebase (`eframe`, `sysinfo`, and `wgpu` all support
  both), but only Linux has actually been built and run so far. A real
  Windows build/run is a follow-up.

## Troubleshooting

- **A component's score looks wrong or oddly low**: check whether it's
  marked "estimated" - the fallback formulas are intentionally rough
  until a real database is in place.
- **The app appears to hang after "Run Scoring"**: detection normally
  completes in under a second; if it seems stuck, check whether a
  progress label is still updating (CPU -> GPU -> RAM -> storage) -
  if it's frozen at "Starting...", something in hardware detection is
  blocking (rare; GPU driver issues are the most likely cause since
  `wgpu` talks to the graphics driver).
- **Window looks cramped or text is cut off**: resize the window
  larger, or scroll - the Results screen especially can be taller than
  the default window size on some displays.
