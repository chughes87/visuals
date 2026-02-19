use std::sync::Arc;
use std::time::Instant;

use fractal_core::patch::Patch;
use fractal_core::presets::Preset;
use fractal_gpu::{
    context::Uniforms,
    effect_pipeline::{EffectPass, PingPong},
    generator_pipeline::GeneratorPass,
    renderer::FULLSCREEN_WGSL,
};
use winit::window::Window;

use crate::input::{apply_zoom, clamp_iterations, InputAction, InputState, Key};

// ---------------------------------------------------------------------------
// Simple FPS counter — logs to console once per second
// ---------------------------------------------------------------------------

struct FpsCounter {
    frames: u32,
    last_report: Instant,
}

impl FpsCounter {
    fn new() -> Self {
        Self {
            frames: 0,
            last_report: Instant::now(),
        }
    }

    /// Increment the frame count.  Returns the FPS value if a full second has
    /// elapsed since the last report (so the caller can log it).
    fn tick(&mut self) -> Option<f32> {
        self.frames += 1;
        let elapsed = self.last_report.elapsed().as_secs_f32();
        if elapsed >= 1.0 {
            let fps = self.frames as f32 / elapsed;
            self.frames = 0;
            self.last_report = Instant::now();
            Some(fps)
        } else {
            None
        }
    }
}

// ---------------------------------------------------------------------------
// App — Phase 10: input wired up
// ---------------------------------------------------------------------------

pub struct App {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface_config: wgpu::SurfaceConfiguration,

    // GPU passes (size-dependent resources rebuilt on resize)
    gen_pass: GeneratorPass,
    effect_pass: EffectPass,
    pp: PingPong,

    // Fullscreen quad render pipeline
    render_pipeline: wgpu::RenderPipeline,
    render_bgl: wgpu::BindGroupLayout,
    render_sampler: wgpu::Sampler,

    // Patch and preset tracking
    patch: Patch,
    current_preset_idx: usize,

    // Input
    input: InputState,
    /// Last known cursor position in physical pixels.
    cursor_pos: (f64, f64),

    // Frame timing
    last_frame: Instant,
    fps: FpsCounter,
}

impl App {
    /// Initialise wgpu for a given window.  The window is wrapped in `Arc` so
    /// that the surface can safely hold a `'static` reference to it.
    pub fn new(window: Arc<Window>) -> Self {
        let size = window.inner_size();
        let width = size.width.max(1);
        let height = size.height.max(1);

        // ---- Instance -------------------------------------------------------
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        // ---- Surface --------------------------------------------------------
        let surface = instance
            .create_surface(Arc::clone(&window))
            .expect("failed to create wgpu surface");

        // ---- Adapter --------------------------------------------------------
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .expect("no suitable GPU adapter found");

        log::info!("GPU adapter: {}", adapter.get_info().name);

        // ---- Device & Queue -------------------------------------------------
        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("fractal-app device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: Default::default(),
            },
            None,
        ))
        .expect("failed to create GPU device");

        // ---- Surface configuration ------------------------------------------
        let surface_caps = surface.get_capabilities(&adapter);

        let format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width,
            height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(&device, &surface_config);
        log::info!(
            "Surface configured: {}×{} {:?} Fifo",
            surface_config.width,
            surface_config.height,
            format
        );

        // ---- GPU passes -----------------------------------------------------
        let gen_pass = GeneratorPass::new(&device, width, height);
        let effect_pass = EffectPass::new(&device);
        let pp = PingPong::new(&device, width, height);

        // ---- Fullscreen quad render pipeline --------------------------------
        let (render_bgl, render_sampler, render_pipeline) =
            Self::build_render_pipeline(&device, format);

        // ---- Patch (start with ClassicMandelbrot) ---------------------------
        let patch = Preset::ClassicMandelbrot.build();

        Self {
            surface,
            device,
            queue,
            surface_config,
            gen_pass,
            effect_pass,
            pp,
            render_pipeline,
            render_bgl,
            render_sampler,
            patch,
            current_preset_idx: 0,
            input: InputState::new(),
            cursor_pos: (0.0, 0.0),
            last_frame: Instant::now(),
            fps: FpsCounter::new(),
        }
    }

    // -------------------------------------------------------------------------
    // Build the fullscreen-quad render pipeline (resolution-agnostic).
    // -------------------------------------------------------------------------

    fn build_render_pipeline(
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
    ) -> (wgpu::BindGroupLayout, wgpu::Sampler, wgpu::RenderPipeline) {
        let render_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("render_bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let render_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("render_sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            ..Default::default()
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("render_pl"),
            bind_group_layouts: &[&render_bgl],
            push_constant_ranges: &[],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("fullscreen"),
            source: wgpu::ShaderSource::Wgsl(FULLSCREEN_WGSL.into()),
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("render_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        (render_bgl, render_sampler, render_pipeline)
    }

    // -------------------------------------------------------------------------
    // Resize
    // -------------------------------------------------------------------------

    /// Reconfigure the surface and rebuild size-dependent GPU resources.
    pub fn resize(&mut self, new_width: u32, new_height: u32) {
        if new_width == 0 || new_height == 0 {
            return;
        }
        self.surface_config.width = new_width;
        self.surface_config.height = new_height;
        self.surface.configure(&self.device, &self.surface_config);

        // Generator output and ping-pong textures are tied to the resolution.
        self.gen_pass = GeneratorPass::new(&self.device, new_width, new_height);
        self.pp = PingPong::new(&self.device, new_width, new_height);

        log::debug!("Surface resized to {}×{}", new_width, new_height);
    }

    // -------------------------------------------------------------------------
    // Input — called by main.rs window_event handler
    // -------------------------------------------------------------------------

    /// Translate a key press and return the resulting action, if any.
    pub fn on_key_pressed(&self, key: Key) -> Option<InputAction> {
        self.input.on_key(key)
    }

    /// Track the cursor position in physical pixels and update patch mouse params.
    pub fn on_cursor_moved(&mut self, x: f64, y: f64) {
        self.cursor_pos = (x, y);
        // Normalise to [0, 1] for the MouseModulator
        let w = self.surface_config.width as f64;
        let h = self.surface_config.height as f64;
        self.patch.params.mouse_x = (x / w) as f32;
        self.patch.params.mouse_y = (y / h) as f32;
    }

    /// Produce a MouseZoom action from the last known cursor position.
    pub fn on_mouse_left_click(&self) -> InputAction {
        let w = self.surface_config.width as f64;
        let h = self.surface_config.height as f64;
        let norm_x = (self.cursor_pos.0 / w) as f32;
        let norm_y = (self.cursor_pos.1 / h) as f32;
        self.input.on_mouse_click(norm_x, norm_y)
    }

    /// Apply an action to the app state.
    ///
    /// Returns `true` if the app should exit (i.e. action was `Quit`).
    pub fn handle_action(&mut self, action: InputAction) -> bool {
        match action {
            InputAction::LoadPreset(preset) => {
                log::info!("Loading preset: {}", preset.name());
                if let Some(idx) = Preset::ALL.iter().position(|&p| p == preset) {
                    self.current_preset_idx = idx;
                }
                self.patch = preset.build();
            }

            InputAction::CycleNextPreset => {
                self.current_preset_idx = (self.current_preset_idx + 1) % Preset::ALL.len();
                let preset = Preset::ALL[self.current_preset_idx];
                log::info!("Cycling to preset: {}", preset.name());
                self.patch = preset.build();
            }

            InputAction::IterationsUp => {
                self.patch.params.max_iter =
                    clamp_iterations(self.patch.params.max_iter.saturating_add(10));
                log::debug!("max_iter → {}", self.patch.params.max_iter);
            }

            InputAction::IterationsDown => {
                self.patch.params.max_iter =
                    clamp_iterations(self.patch.params.max_iter.saturating_sub(10));
                log::debug!("max_iter → {}", self.patch.params.max_iter);
            }

            InputAction::Reset => {
                let preset = Preset::ALL[self.current_preset_idx];
                log::info!("Reset to preset defaults: {}", preset.name());
                self.patch = preset.build();
            }

            InputAction::MouseZoom { norm_x, norm_y } => {
                let w = self.surface_config.width as f32;
                let h = self.surface_config.height as f32;
                let aspect = w / h;
                let (cx, cy, zoom) = apply_zoom(
                    self.patch.params.center_x,
                    self.patch.params.center_y,
                    self.patch.params.zoom,
                    norm_x,
                    norm_y,
                    aspect,
                );
                self.patch.params.center_x = cx;
                self.patch.params.center_y = cy;
                self.patch.params.zoom = zoom;
                log::debug!("Zoom → {:.4}  center ({:.6}, {:.6})", zoom, cx, cy);
            }

            InputAction::Quit => return true,
        }
        false
    }

    // -------------------------------------------------------------------------
    // Render
    // -------------------------------------------------------------------------

    /// Run one full frame: tick the patch, dispatch generator + effects, draw.
    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        // --- Timing ----------------------------------------------------------
        let now = Instant::now();
        let dt = now.duration_since(self.last_frame).as_secs_f32();
        self.last_frame = now;
        self.patch.tick(dt);

        if let Some(fps) = self.fps.tick() {
            log::debug!(
                "FPS: {:.1}  preset: {}  zoom: {:.2}  iter: {}",
                fps,
                Preset::ALL[self.current_preset_idx].name(),
                self.patch.params.zoom,
                self.patch.params.max_iter,
            );
        }

        let width = self.surface_config.width;
        let height = self.surface_config.height;

        // --- Build uniforms from patch params --------------------------------
        let params = &self.patch.params;
        let uniforms = Uniforms {
            resolution: [width as f32, height as f32],
            center: [params.center_x, params.center_y],
            zoom: params.zoom,
            time: params.time,
            max_iter: params.max_iter,
            _pad: 0,
            julia_c: [params.get("julia_cx"), params.get("julia_cy")],
            _pad2: [0.0, 0.0],
        };

        // Snapshot kinds before GPU calls (avoids borrowing self.patch during dispatch).
        let gen_kind = self.patch.generator.kind();
        let effect_kinds: Vec<_> = self.patch.effects.iter().map(|e| e.kind(params)).collect();

        // --- Acquire surface texture -----------------------------------------
        let output = self.surface.get_current_texture()?;
        let surface_view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("frame-encoder"),
            });

        // --- 1. Generator compute pass ---------------------------------------
        self.gen_pass
            .dispatch(&self.device, &mut encoder, &self.queue, gen_kind, &uniforms);

        // --- 2. Effect chain -------------------------------------------------
        self.effect_pass.dispatch_chain(
            &self.device,
            &mut encoder,
            &self.queue,
            &effect_kinds,
            &uniforms,
            &self.gen_pass.output_view,
            &mut self.pp,
            width,
            height,
        );

        // --- 3. Fullscreen quad render pass ----------------------------------
        // Determine which texture holds the final image:
        //   * empty chain  → generator output
        //   * N effects    → ping-pong read target (last swap put result there)
        let final_view: &wgpu::TextureView = if effect_kinds.is_empty() {
            &self.gen_pass.output_view
        } else {
            self.pp.read_view()
        };

        let render_bg = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("render_bg"),
            layout: &self.render_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(final_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.render_sampler),
                },
            ],
        });

        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("fullscreen-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &surface_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            rpass.set_pipeline(&self.render_pipeline);
            rpass.set_bind_group(0, &render_bg, &[]);
            rpass.draw(0..6, 0..1); // two triangles, no vertex buffer
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
        Ok(())
    }
}
