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

// ---------------------------------------------------------------------------
// App — Phase 9: Mandelbrot rendered to window
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

    // Patch — hardcoded to ClassicMandelbrot for Phase 9
    patch: Patch,

    // Frame timing
    last_frame: Instant,
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

        // ---- Patch ----------------------------------------------------------
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
            last_frame: Instant::now(),
        }
    }

    // -------------------------------------------------------------------------
    // Build (or rebuild) the fullscreen-quad render pipeline.
    // Called at init and not needed on resize (pipeline is resolution-agnostic).
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
    // Render
    // -------------------------------------------------------------------------

    /// Run one full frame: tick the patch, dispatch generator + effects, draw.
    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        // --- Timing ----------------------------------------------------------
        let now = Instant::now();
        let dt = now.duration_since(self.last_frame).as_secs_f32();
        self.last_frame = now;
        self.patch.tick(dt);

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

        // Snapshot what kind of generator and effects to dispatch (avoids
        // holding a borrow on self.patch during the GPU calls).
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
