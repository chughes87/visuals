# Fractal Explorer — Rust/wgpu GPU Rewrite

A GPU-accelerated fractal explorer built with Rust, [wgpu](https://github.com/gfx-rs/wgpu), and [winit](https://github.com/rust-windowing/winit). This is a rewrite of the original Clojure/Quil implementation in the parent directory, targeting 60+ fps at native resolution via compute shaders.

## Features

- **4 fractal generators** — Mandelbrot, Julia, Burning Ship, Noise Field
- **6 real-time effects** — Color mapping, ripple, echo, hue shift, brightness/contrast, motion blur
- **5 presets** — each with its own generator, effect chain, and LFO modulators
- **Smooth iteration colouring** — log-log escape smoothing (no hard colour bands)
- **Live parameter modulation** — sine LFOs, random walks, and mouse-driven modulators
- **egui HUD** — overlay showing preset, zoom level, iterations, active effects, and FPS
- **Cross-platform** — runs on Vulkan, Metal, DX12, and WebGPU via wgpu

## Prerequisites

### Rust toolchain

Install Rust via [rustup](https://rustup.rs/):

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Rust 1.75+ is recommended (the workspace uses the 2021 edition).

### System graphics dependencies

wgpu uses the platform's native graphics API. No extra drivers are needed on most systems, but ensure your GPU drivers are up to date.

| Platform | Backend used |
|----------|-------------|
| Linux    | Vulkan       |
| macOS    | Metal        |
| Windows  | DX12 / Vulkan |

On Linux, you may need Vulkan loader libraries:

```sh
# Debian/Ubuntu
sudo apt install libvulkan1 vulkan-tools

# Fedora
sudo dnf install vulkan-loader vulkan-tools
```

## Building

All commands below are run from the `rust/` directory:

```sh
cd rust/
cargo build -p fractal-app          # debug build
cargo build -p fractal-app --release  # optimised build (recommended for performance)
```

## Running

```sh
cd rust/
cargo run -p fractal-app            # debug
cargo run -p fractal-app --release  # release (60+ fps target)
```

## Controls

| Key / Input        | Action                          |
|--------------------|---------------------------------|
| `1` – `5`          | Load preset 1–5                 |
| `Space`            | Cycle to next preset            |
| `+` / `=`          | Increase max iterations         |
| `-`                | Decrease max iterations         |
| `R`                | Reset to default view           |
| `Q` / `Escape`     | Quit                            |
| Left-click         | Zoom in 2× at clicked location  |

## Presets

| # | Name                  | Generator    | Highlight effects              |
|---|----------------------|--------------|-------------------------------|
| 1 | Classic Mandelbrot   | Mandelbrot   | Classic colour map             |
| 2 | Psychedelic Julia    | Julia        | Psychedelic palette, LFO hue  |
| 3 | Trippy Mandelbrot    | Mandelbrot   | Ripple, hue shift, echo        |
| 4 | Burning Ship Trails  | Burning Ship | Echo, motion blur              |
| 5 | Noise Field          | Noise Field  | Fire palette, ripple           |

## Project Structure

```
rust/
├── Cargo.toml                  # workspace root
├── RUST_GPU_PLAN.md            # phased implementation roadmap
└── crates/
    ├── fractal-core/           # pure Rust CPU types, no GPU deps
    │   └── src/
    │       ├── lib.rs          # Params, Generator/Effect/Modulator traits
    │       ├── patch.rs        # Patch: owns generator, effects, modulators
    │       ├── modulators.rs   # LFO, RandomWalk, MouseModulator, ModMatrix
    │       └── presets.rs      # 5 Preset definitions
    ├── fractal-gpu/            # wgpu compute pipelines and WGSL shaders
    │   ├── src/
    │   │   ├── context.rs      # GpuContext, Uniforms struct
    │   │   ├── generator_pipeline.rs  # 4 generator compute passes
    │   │   ├── effect_pipeline.rs     # 6 effect passes, ping-pong buffers
    │   │   └── renderer.rs     # fullscreen-quad render pass
    │   └── shaders/            # 10 WGSL compute/fragment shaders
    └── fractal-app/            # winit event loop, main binary
        └── src/
            ├── main.rs         # winit ApplicationHandler
            ├── app.rs          # GPU state, render loop, egui HUD
            └── input.rs        # key mappings, mouse zoom, iteration clamping
```

## Architecture

Each frame follows this GPU pipeline:

```
CPU: Params → Uniforms
         ↓
[Generator Compute Pass]   — mandelbrot / julia / burning_ship / noise_field
         ↓  rgba32float texture
[Effect Compute Passes]    — ping-pong through 0–N effects
         ↓  rgba32float texture
[Fullscreen Render Pass]   — blit to surface
         ↓
[egui Render Pass]         — HUD overlay
         ↓
Present
```

Shaders are embedded in the binary at compile time via `include_str!()`.

## Testing

```sh
cd rust/
cargo test                          # all unit tests
cargo test -p fractal-core          # core types only
cargo test -p fractal-gpu           # shader validation + coordinate tests
cargo test -p fractal-app           # input handling tests

# GPU smoke test (requires a real GPU adapter)
cargo test -p fractal-gpu -- --ignored
```

## Formatting

The project enforces `rustfmt` formatting. Always run before committing:

```sh
cd rust/
cargo fmt           # auto-format all crates
cargo fmt --check   # verify clean (run in CI)
```
