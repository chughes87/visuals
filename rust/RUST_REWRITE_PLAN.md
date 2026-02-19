# Rust + wgpu Rewrite Plan

Rewriting `visuals` (currently a Clojure/Java2D app) as a Rust workspace
with GPU-compute rendering via **wgpu** and a **winit** window.

---

## Workspace layout

```
visuals-rs/
├── Cargo.toml                      ← workspace root
└── crates/
    ├── fractal-core/               ← pure-Rust types, traits, modulators
    ├── fractal-gpu/                ← wgpu pipelines + WGSL shaders
    └── fractal-app/                ← winit window + event loop (TBD)
```

---

## What's done (in the zip)

### `fractal-core`

| File | What it provides |
|------|-----------------|
| `src/lib.rs` | `Params`, `GeneratorKind`, `EffectKind`, `ColorScheme`, `Generator` / `Effect` / `Modulator` traits |
| `src/patch.rs` | `Patch` — owns a generator, effect chain, modulator list, and `Params`; has `tick(dt)` and `generator_dirty()` for cache-invalidation |
| `src/modulators.rs` | `Lfo` (sine/triangle/square/saw), `RandomWalk`, `MouseModulator`, `ModMatrix` |
| `src/presets.rs` | `Preset` enum with the five original presets named |

### `fractal-gpu`

| File | What it provides |
|------|-----------------|
| `src/context.rs` | `GpuContext::new_headless()` + `Uniforms` struct (repr C, bytemuck) |
| `src/generator_pipeline.rs` | `GeneratorPipelines` — one `ComputePipeline` per generator, loaded from inline WGSL |
| `src/effect_pipeline.rs` | `EffectPipelines` + `PingPong` ping-pong texture pair |
| `src/renderer.rs` | `FULLSCREEN_WGSL` — index-buffer-free full-screen quad for final blit |
| `shaders/mandelbrot.wgsl` | Mandelbrot escape-time compute shader |
| `shaders/julia.wgsl` | Julia set compute shader |
| `shaders/burning_ship.wgsl` | Burning Ship variant |
| `shaders/noise_field.wgsl` | Smooth noise field |
| `shaders/color_map.wgsl` | Palette / color-map effect |
| `shaders/ripple.wgsl` | UV-warping ripple effect |
| `shaders/echo.wgsl` | Multi-layer echo/smear |
| `shaders/hue_shift.wgsl` | HSV hue rotation |
| `shaders/brightness_contrast.wgsl` | Brightness + contrast adjustment |

### `fractal-app` (stub only)
Empty `Cargo.toml` — the winit event loop goes here next.

---

## What's next (step by step)

### Step 1 — Wire up `GeneratorPipelines` bind groups
Each compute shader needs a bind group layout:
- `@group(0) @binding(0)` → `Uniforms` uniform buffer
- `@group(0) @binding(1)` → output storage texture (`rgba32float`)

Create the bind group layout in `generator_pipeline.rs`, allocate the output
texture, and dispatch `ceil(W/8) × ceil(H/8)` workgroups.

### Step 2 — Wire up `EffectPipelines` with ping-pong
`PingPong` is already defined. Each effect pass:
1. Bind `ping_pong.read_view()` as input texture, `write_view()` as output storage texture.
2. Dispatch compute.
3. Call `ping_pong.swap()`.

### Step 3 — Frame loop in `fractal-gpu`
Add `Renderer::render_frame(patch, ping_pong)`:
1. `patch.tick(dt)`
2. If `patch.generator_dirty()` → dispatch generator compute pass.
3. For each effect in `patch.effects` → dispatch effect compute pass (ping-pong).
4. Submit `FULLSCREEN_WGSL` render pass blitting the final texture to the surface.

### Step 4 — `fractal-app`: winit window + event loop
- Create `winit` `EventLoop` + `Window`.
- Create wgpu `Surface` from the window.
- Build `GpuContext` with surface compatibility.
- Construct a default `Patch` (Classic Mandelbrot preset).
- Run the `winit` event loop, calling `render_frame` on `RedrawRequested`.
- Handle keyboard:  `1`–`5` → switch preset, `[`/`]` → cycle effects,
  mouse drag → pan, scroll → zoom.

### Step 5 — Preset `Patch` builders
One function per `Preset` that returns a ready-to-use `Patch` with the
right generator, effect chain, and modulators wired up, matching the
original Clojure presets.

### Step 6 — Motion blur effect shader + pipeline
`echo.wgsl` exists; `motion_blur` is referenced in `EffectPipelines::new`
but the shader file is missing. Write `shaders/motion_blur.wgsl` and add it.

### Step 7 — CI / build check
Add a `.github/workflows/ci.yml` that runs `cargo check` (no GPU needed
for a CI check-only build).

---

## Key design decisions

| Decision | Rationale |
|----------|-----------|
| Compute shaders for generators **and** effects | Uniform GPU dispatch path; easy to add new effects without changing CPU code |
| Ping-pong textures | Avoids read-write hazards without manual sync; simple `swap()` |
| `Patch::generator_dirty()` | Skips the expensive generator pass when only effects/time changed |
| `bytemuck::Pod` for `Uniforms` | Zero-copy cast to `&[u8]` for `queue.write_buffer` |
| Workspace crates | `fractal-core` can be tested headlessly; `fractal-gpu` can be swapped for a software renderer |
