# Fractal Explorer — Rust + GPU Rewrite Plan

## Overview

This document is the implementation plan for rewriting the Fractal Explorer
(currently Clojure + Quil) in Rust, with fractal generation and effect
processing offloaded to the GPU via compute shaders.

**Primary goals:**
- Re-implement all five presets with identical visual output
- Run fractal generation on the GPU (target: render at native resolution, 60 fps)
- Keep the modular Generator → Effect → Modulator → Patch architecture
- Retain interactive features: zoom, preset switching, iteration depth

---

## Technology Choices

| Concern | Choice | Rationale |
|---------|--------|-----------|
| Language | Rust (stable) | Safety + zero-cost abstractions; great GPU ecosystem |
| GPU API | **wgpu** | Cross-platform (Vulkan, Metal, DX12, WebGPU); first-class Rust support |
| Shader language | **WGSL** | Native to wgpu; statically typed; easier to debug than GLSL/SPIR-V by hand |
| Windowing | **winit** | De-facto standard; integrates cleanly with wgpu |
| Math | **glam** | Lightweight, GPU-compatible types (Vec2, Vec3, Mat4) |
| Noise | **fast-noise-lite** | CPU noise for NoiseGenerator; port to WGSL later if needed |
| UI overlay | **egui** + **egui-wgpu** | Immediate-mode GUI that renders over the wgpu surface |
| Build system | Cargo workspace | One repo, multiple crates; clear dependency boundaries |

No CUDA — wgpu keeps us vendor-neutral (NVIDIA, AMD, Intel, Apple Silicon).

---

## Repository Layout

```
visuals/
├── Cargo.toml                  # workspace root
├── crates/
│   ├── fractal-core/           # pure Rust types & CPU logic (no GPU deps)
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── patch.rs
│   │   │   ├── modulators.rs
│   │   │   └── presets.rs
│   │   └── Cargo.toml
│   ├── fractal-gpu/            # wgpu pipeline, shaders, GPU buffers
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── context.rs      # wgpu Device/Queue/Surface setup
│   │   │   ├── generator_pipeline.rs
│   │   │   ├── effect_pipeline.rs
│   │   │   └── renderer.rs
│   │   ├── shaders/
│   │   │   ├── mandelbrot.wgsl
│   │   │   ├── julia.wgsl
│   │   │   ├── burning_ship.wgsl
│   │   │   ├── noise_field.wgsl
│   │   │   ├── color_map.wgsl
│   │   │   ├── ripple.wgsl
│   │   │   ├── echo.wgsl
│   │   │   ├── hue_shift.wgsl
│   │   │   ├── brightness_contrast.wgsl
│   │   │   └── motion_blur.wgsl
│   │   └── Cargo.toml
│   └── fractal-app/            # winit event loop, egui, main binary
│       ├── src/
│       │   ├── main.rs
│       │   ├── app.rs
│       │   └── input.rs
│       └── Cargo.toml
└── RUST_GPU_PLAN.md
```

---

## Core Data Model (`fractal-core`)

These are plain Rust types — no GPU knowledge, easily testable.

```rust
// Equivalent to Clojure protocols
pub trait Generator: Send + Sync {
    /// Which param keys does this generator read?
    fn gen_params(&self) -> &[&'static str];
    /// Return a descriptor the GPU pipeline uses to configure its shader
    fn descriptor(&self) -> GeneratorDescriptor;
}

pub trait Effect: Send + Sync {
    fn descriptor(&self) -> EffectDescriptor;
}

pub trait Modulator: Send + Sync {
    /// Mutate `params` in place
    fn modulate(&self, params: &mut Params);
}

pub struct Params {
    pub fields: HashMap<String, f32>,
    pub time:   f32,
    pub frame:  u64,
    // ... zoom, center_x, center_y, max_iter, mouse_x, mouse_y
}

pub struct Patch {
    pub generator:  Box<dyn Generator>,
    pub effects:    Vec<Box<dyn Effect>>,
    pub modulators: Vec<Box<dyn Modulator>>,
    pub params:     Params,
}
```

`GeneratorDescriptor` and `EffectDescriptor` are plain enums/structs that the
GPU crate matches to select the right pipeline — keeping a clean boundary
between pure logic and GPU code.

---

## GPU Architecture (`fractal-gpu`)

### Data Flow

```
Params (CPU)
    │
    ▼
[Uniform Buffer]  ← uploaded every frame via queue.write_buffer()
    │
    ▼
┌─────────────────────────────────────┐
│  Generator Compute Pass              │
│  Dispatch: ceil(W/8) × ceil(H/8)    │
│  Workgroup: 8×8 = 64 threads        │
│  Output: storage texture (rgba32f)  │
└────────────────┬────────────────────┘
                 │ ping-pong textures
                 ▼
┌─────────────────────────────────────┐
│  Effect Compute Passes (chained)    │
│  One pass per active effect          │
│  Each reads input tex, writes output│
└────────────────┬────────────────────┘
                 │
                 ▼
┌─────────────────────────────────────┐
│  Render Pass                        │
│  Full-screen quad, samples final tex│
│  Outputs to wgpu Surface            │
└─────────────────────────────────────┘
```

### Uniform Buffer Layout

```wgsl
struct Uniforms {
    resolution:  vec2<f32>,
    center:      vec2<f32>,
    zoom:        f32,
    time:        f32,
    max_iter:    u32,
    // per-effect params follow as additional structs
}
```

### Generator Shaders (example: `mandelbrot.wgsl`)

```wgsl
@group(0) @binding(0) var<uniform> u: Uniforms;
@group(0) @binding(1) var output: texture_storage_2d<rgba32float, write>;

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let px = vec2<f32>(f32(gid.x), f32(gid.y));
    if px.x >= u.resolution.x || px.y >= u.resolution.y { return; }

    // Map pixel → complex plane
    let uv = (px - u.resolution * 0.5) / (u.zoom * u.resolution.y * 0.5);
    let c  = u.center + uv;

    var z = vec2<f32>(0.0);
    var i = 0u;
    loop {
        if i >= u.max_iter || dot(z, z) > 4.0 { break; }
        z = vec2<f32>(z.x*z.x - z.y*z.y + c.x, 2.0*z.x*z.y + c.y);
        i++;
    }

    // Store smooth iteration count
    let smooth_i = f32(i) - log2(log2(dot(z,z))) + 4.0;
    textureStore(output, vec2<i32>(gid.xy), vec4<f32>(smooth_i / f32(u.max_iter), 0.0, 0.0, 1.0));
}
```

Every pixel runs in its own GPU thread — the 800×600 = 480,000 pixels are
distributed across ~7,500 workgroups of 64 threads each.  This is the key
difference from the current `pmap` approach (which is limited to CPU core
count).

### Ping-Pong Texture Strategy for Effects

```
frame N:  gen_tex ──► effect_0 ──► tex_A ──► effect_1 ──► tex_B ──► render
frame N+1: cache hit → tex_B already valid, skip generator pass
```

Two `rgba32float` storage textures are allocated at startup and swapped between
effect passes.  The generator pass is skipped when the generator-relevant params
haven't changed (same caching logic as the Clojure implementation).

---

## Implementation Steps

### Phase 1 — Scaffolding

1. Create the Cargo workspace with the three crates.
2. Stand up a blank `winit` window with a wgpu surface — black screen, no crash.
3. Implement `context.rs`: `GpuContext` wrapping `Device`, `Queue`, `Surface`,
   `SurfaceConfiguration`.
4. Add the full-screen quad render pass that samples a placeholder texture.

### Phase 2 — Fractal Core (CPU types)

5. Port `Params`, `Patch`, and the `Generator`/`Effect`/`Modulator` traits to
   `fractal-core`.
6. Port all five modulators (`LFO`, `Envelope`, `MouseModulator`,
   `RandomWalk`, `ModMatrix`) — these stay on the CPU; output is uploaded as
   uniforms.
7. Port the `Patch` processing loop including the gen-cache invalidation logic.

### Phase 3 — Generator GPU Pipelines

8. Write `mandelbrot.wgsl`, `julia.wgsl`, `burning_ship.wgsl`,
   `noise_field.wgsl`.
9. Implement `generator_pipeline.rs`:
   - One `ComputePipeline` per generator variant.
   - Uniform buffer for `Uniforms`.
   - Output storage texture (full window resolution, no stride — GPU makes
     stride unnecessary).
10. Wire `Patch::descriptor()` → select & dispatch the right compute pipeline.
11. Verify visual correctness against Clojure output at matched params.

### Phase 4 — Effect GPU Pipelines

12. Write effect shaders: `color_map.wgsl`, `ripple.wgsl`, `echo.wgsl`,
    `hue_shift.wgsl`, `brightness_contrast.wgsl`, `motion_blur.wgsl`.
    Each reads a `texture_2d<f32>` and writes to a `texture_storage_2d`.
13. Implement `effect_pipeline.rs` with ping-pong texture management.
14. Chain effects in the order specified by the patch's `effects` vec.

### Phase 5 — Presets

15. Port all five presets from `presets.clj` to Rust in `fractal-core/src/presets.rs`.
16. Confirm each preset renders identically (or better) to the Clojure version.

### Phase 6 — Input & UI

Split into three sub-phases to keep each step independently verifiable.

#### Phase 6a — Input module (pure Rust, no GPU)

17. Create `fractal-app/src/input.rs`:
    - `InputAction` enum: `LoadPreset`, `CycleNextPreset`, `IterationsUp`,
      `IterationsDown`, `Reset`, `Quit`, `MouseZoom { norm_x, norm_y }`.
    - `InputState` struct with `on_key(PhysicalKey) -> Option<InputAction>` and
      `on_mouse_click(norm_x, norm_y) -> InputAction`.
    - Key map: `1`–`5` → load preset, `Space` → cycle, `=`/`+` → iter up,
      `-` → iter down, `R` → reset, `Q`/`Escape` → quit.
    - Unit tests (no GPU required): verify every key mapping, mouse zoom
      coordinate math, and iteration clamping logic.

#### Phase 6b — Windowed app (wgpu surface + input wired up, no egui)

18. Update `fractal-app/Cargo.toml`: add `winit = "0.30"`, `wgpu = "22"`,
    `pollster = "0.3"`, `log = "0.4"`, `env_logger = "0.11"`,
    `bytemuck = "1"`.
19. Create `fractal-app/src/app.rs`:
    - `App` struct owning `wgpu::Surface`, `Device`, `Queue`,
      `SurfaceConfiguration`, `GeneratorPass`, `EffectPass`, `PingPong`,
      fullscreen-quad `RenderPipeline`, `Patch`, `InputState`, cursor
      position, and FPS counter.
    - `App::new(Arc<Window>) -> impl Future<Output = App>` — sets up the
      wgpu surface-aware context (mirrors `GpuContext::new_headless` but
      with a surface).
    - `App::resize`, `App::render`, `App::handle_action`,
      `App::on_key_pressed`, `App::on_cursor_moved`,
      `App::on_mouse_left_click`.
20. Update `fractal-app/src/main.rs`: winit 0.30 `ApplicationHandler` event
    loop — `resumed` creates the window + App, `window_event` dispatches
    resize / redraw / input, `about_to_wait` requests continuous redraw.
21. Smoke-test: `cargo run -p fractal-app` opens an 800×600 window showing
    preset 1; keys 1–5 switch presets, `+`/`-` change iteration depth,
    mouse click zooms, `Q` quits.

#### Phase 6c — egui HUD overlay

22. Add to `fractal-app/Cargo.toml`: `egui = "0.29"`, `egui-wgpu = "0.29"`,
    `egui-winit = "0.29"`.
23. Extend `App` with `egui::Context`, `egui_winit::State`,
    `egui_wgpu::Renderer`.
24. Each frame: run egui to produce a semi-transparent HUD panel (top-left)
    showing preset name, zoom, iteration count, active effects, FPS, and
    control hints; tessellate and render it in the same render pass after
    the fullscreen quad.
25. Feed `WindowEvent`s to `egui_winit::State::on_window_event` first;
    skip game input if egui reports the event as consumed.

### Phase 7 — Polish & Performance

19. Benchmark: compare frame times between Clojure `pmap` and Rust GPU.
20. Enable smooth iteration count (already in the shader above) for nicer
    colour gradients at low iteration counts.
21. Remove the stride=2 limitation — render at true 1-pixel resolution since
    GPU cost per pixel is negligible.
22. Add `ParticleSystem` as a compute shader (particle positions in a storage
    buffer; one thread per particle).
23. Profile with `wgpu`'s built-in timestamp queries; optimize hot shaders.

---

## Key Improvements Over the Clojure Version

| Aspect | Clojure (current) | Rust + GPU (target) |
|--------|-------------------|---------------------|
| Fractal generation | CPU `pmap` (8-16 threads) | GPU compute (thousands of threads) |
| Pixel stride | 2 (half resolution) | 1 (full resolution) |
| Frame rate | 30 fps (target) | 60+ fps |
| Smooth colouring | No (hard iteration bands) | Yes (log-log escape smoothing) |
| Type safety | Runtime errors possible | Compile-time guarantees |
| Startup time | JVM cold start (~5s) | Near-instant |
| Memory | JVM heap + GC pauses | Stack/arena, no GC |
| Distribution | Requires JVM | Single static binary |

---

## Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| wgpu compute not supported on target GPU | Validate adapter features at startup; fall back to CPU path |
| WGSL shader debugging is hard | Use `wgpu`'s validation layer + `naga` IR dumps; test each shader in isolation |
| egui + wgpu integration boilerplate | Use `egui-wgpu` crate which handles render pass integration |
| Floating-point precision differences | Use `f64` on CPU for zoom math; `f32` only in shaders |
| Particle system harder to port to GPU | Implement CPU fallback first, GPU version in Phase 7 |

---

## What is NOT Changing

- The modular synthesizer metaphor (Generator → Effect → Modulator → Patch)
- The five presets and their parameter logic
- The interactive controls (same key bindings)
- The caching strategy (skip generator pass when gen-params unchanged)

---

## Next Action

Begin with **Phase 1** — create the Cargo workspace and get a blank wgpu window
running.  Each phase produces a working, runnable binary so progress is always
visible.
