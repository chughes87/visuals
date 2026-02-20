use std::sync::Arc;
use std::time::Instant;

use fractal_core::{patch::Patch, presets::Preset, EffectKind};
use fractal_gpu::{
    context::Uniforms,
    effect_pipeline::{EffectPass, PingPong},
    generator_pipeline::GeneratorPass,
    renderer::FULLSCREEN_WGSL,
};
use winit::event::WindowEvent;
use winit::window::Window;

use crate::input::{apply_zoom, clamp_iterations, InputAction, InputState, Key};

// ---------------------------------------------------------------------------
// FPS counter — tracks frame rate, exposes last known value for the HUD
// ---------------------------------------------------------------------------

struct FpsCounter {
    frames: u32,
    last_report: Instant,
    last_fps: f32,
}

impl FpsCounter {
    fn new() -> Self {
        Self {
            frames: 0,
            last_report: Instant::now(),
            last_fps: 0.0,
        }
    }

    /// Tick one frame.  Updates the stored FPS once per second and returns
    /// the new value so the caller can log it.
    fn tick(&mut self) -> Option<f32> {
        self.frames += 1;
        let elapsed = self.last_report.elapsed().as_secs_f32();
        if elapsed >= 1.0 {
            self.last_fps = self.frames as f32 / elapsed;
            self.frames = 0;
            self.last_report = Instant::now();
            Some(self.last_fps)
        } else {
            None
        }
    }

    fn fps(&self) -> f32 {
        self.last_fps
    }
}

// ---------------------------------------------------------------------------
// Short display name for an EffectKind (used in the HUD)
// ---------------------------------------------------------------------------

fn effect_name(kind: &EffectKind) -> &'static str {
    match kind {
        EffectKind::ColorMap { .. } => "Color Map",
        EffectKind::Ripple { .. } => "Ripple",
        EffectKind::Echo { .. } => "Echo",
        EffectKind::HueShift { .. } => "Hue Shift",
        EffectKind::BrightnessContrast { .. } => "Brightness/Contrast",
        EffectKind::MotionBlur { .. } => "Motion Blur",
    }
}

// ---------------------------------------------------------------------------
// App — Phase 11: egui HUD overlay
// ---------------------------------------------------------------------------

pub struct App {
    // Kept for egui-winit (take/handle input, scale factor)
    window: Arc<Window>,

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

    // egui
    egui_ctx: egui::Context,
    egui_state: egui_winit::State,
    egui_renderer: egui_wgpu::Renderer,
}

impl App {
    pub fn new(window: Arc<Window>) -> Self {
        let size = window.inner_size();
        let width = size.width.max(1);
        let height = size.height.max(1);

        // ---- Instance -------------------------------------------------------
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
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

        // ---- egui -----------------------------------------------------------
        let egui_ctx = egui::Context::default();
        let egui_state = egui_winit::State::new(
            egui_ctx.clone(),
            egui::ViewportId::ROOT,
            &*window,
            Some(window.scale_factor() as f32),
            None, // theme: use OS default
            Some(device.limits().max_texture_dimension_2d as usize),
        );
        let egui_renderer = egui_wgpu::Renderer::new(&device, format, None, 1, false);

        // ---- Patch (start with ClassicMandelbrot) ---------------------------
        let patch = Preset::ClassicMandelbrot.build();

        Self {
            window,
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
            egui_ctx,
            egui_state,
            egui_renderer,
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

    pub fn resize(&mut self, new_width: u32, new_height: u32) {
        if new_width == 0 || new_height == 0 {
            return;
        }
        self.surface_config.width = new_width;
        self.surface_config.height = new_height;
        self.surface.configure(&self.device, &self.surface_config);

        self.gen_pass = GeneratorPass::new(&self.device, new_width, new_height);
        self.pp = PingPong::new(&self.device, new_width, new_height);

        log::debug!("Surface resized to {}×{}", new_width, new_height);
    }

    // -------------------------------------------------------------------------
    // egui event forwarding
    // -------------------------------------------------------------------------

    /// Forward a `WindowEvent` to egui.  Returns `true` if egui consumed it
    /// (the caller should then skip game-input handling for that event).
    pub fn egui_on_window_event(&mut self, event: &WindowEvent) -> bool {
        self.egui_state
            .on_window_event(&self.window, event)
            .consumed
    }

    // -------------------------------------------------------------------------
    // Game input — called by main.rs after egui has had first look
    // -------------------------------------------------------------------------

    pub fn on_key_pressed(&self, key: Key) -> Option<InputAction> {
        self.input.on_key(key)
    }

    pub fn on_cursor_moved(&mut self, x: f64, y: f64) {
        self.cursor_pos = (x, y);
        let w = self.surface_config.width as f64;
        let h = self.surface_config.height as f64;
        self.patch.params.mouse_x = (x / w) as f32;
        self.patch.params.mouse_y = (y / h) as f32;
    }

    pub fn on_mouse_left_click(&self) -> InputAction {
        let w = self.surface_config.width as f64;
        let h = self.surface_config.height as f64;
        let norm_x = (self.cursor_pos.0 / w) as f32;
        let norm_y = (self.cursor_pos.1 / h) as f32;
        self.input.on_mouse_click(norm_x, norm_y)
    }

    /// Returns `true` if the app should exit.
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

        // --- Build uniforms --------------------------------------------------
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

        let gen_kind = self.patch.generator.kind();
        let effect_kinds: Vec<_> = self.patch.effects.iter().map(|e| e.kind(params)).collect();

        // --- egui frame (CPU side — must happen before GPU encoding) ---------
        // Collect HUD values before calling egui to avoid borrowing self inside
        // the closure.
        let preset_name = Preset::ALL[self.current_preset_idx].name();
        let zoom = self.patch.params.zoom;
        let max_iter = self.patch.params.max_iter;
        let fps_display = self.fps.fps();
        let effect_labels: Vec<&'static str> = effect_kinds.iter().map(effect_name).collect();

        let raw_input = self.egui_state.take_egui_input(&self.window);
        let full_output = self.egui_ctx.run(raw_input, |ctx| {
            egui::Window::new("Fractal Explorer")
                .anchor(egui::Align2::LEFT_TOP, [10.0, 10.0])
                .collapsible(false)
                .resizable(false)
                .frame(
                    egui::Frame::window(&ctx.style())
                        .fill(egui::Color32::from_rgba_unmultiplied(0, 0, 0, 200)),
                )
                .show(ctx, |ui| {
                    ui.label(format!("Preset:  {preset_name}"));
                    ui.label(format!("Zoom:    {zoom:.2}×"));
                    ui.label(format!("Iter:    {max_iter}"));
                    let fx = if effect_labels.is_empty() {
                        "none".to_string()
                    } else {
                        effect_labels.join(", ")
                    };
                    ui.label(format!("Effects: {fx}"));
                    ui.label(format!("FPS:     {fps_display:.1}"));
                    ui.separator();
                    ui.label("1–5  load preset   Space  cycle");
                    ui.label("+/-  iterations    R  reset");
                    ui.label("Click  zoom        Q/Esc  quit");
                });
        });
        self.egui_state
            .handle_platform_output(&self.window, full_output.platform_output);

        let primitives = self
            .egui_ctx
            .tessellate(full_output.shapes, full_output.pixels_per_point);
        let textures_delta = full_output.textures_delta;

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

        // --- 3. Fullscreen quad render pass (Clear → fractal) ----------------
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
            rpass.draw(0..6, 0..1);
        }

        // --- 4. egui render pass (Load → draw HUD on top) --------------------
        let screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [width, height],
            pixels_per_point: self.window.scale_factor() as f32,
        };

        // Upload any new/changed font/image textures required by egui
        for (id, image_delta) in &textures_delta.set {
            self.egui_renderer
                .update_texture(&self.device, &self.queue, *id, image_delta);
        }

        // update_buffers uploads vertex/index data and returns any extra
        // CommandBuffers produced by paint callbacks (typically empty).
        let user_cmds = self.egui_renderer.update_buffers(
            &self.device,
            &self.queue,
            &mut encoder,
            &primitives,
            &screen_descriptor,
        );

        {
            // egui-wgpu 0.29 requires RenderPass<'static>; forget_lifetime()
            // erases the borrow so we can pass it in.  The pass is dropped
            // before encoder.finish() is called, so the GPU contract holds.
            let mut egui_pass = encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("egui-pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &surface_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load, // composite on top of fractal
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                })
                .forget_lifetime();
            self.egui_renderer
                .render(&mut egui_pass, &primitives, &screen_descriptor);
        }

        // Free GPU resources for any textures egui no longer needs
        for id in &textures_delta.free {
            self.egui_renderer.free_texture(id);
        }

        // Submit paint-callback buffers first, then the main frame encoder
        self.queue
            .submit(user_cmds.into_iter().chain([encoder.finish()]));
        output.present();
        Ok(())
    }
}
