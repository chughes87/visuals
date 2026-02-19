use std::sync::Arc;

use winit::window::Window;

// ---------------------------------------------------------------------------
// App — wgpu surface + device (Phase 8: black-screen checkpoint)
// ---------------------------------------------------------------------------

pub struct App {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface_config: wgpu::SurfaceConfiguration,
}

impl App {
    /// Initialise wgpu for a given window.  The window is wrapped in `Arc` so
    /// that the surface can safely hold a `'static` reference to it.
    pub fn new(window: Arc<Window>) -> Self {
        let size = window.inner_size();

        // ---- Instance -------------------------------------------------------
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        // ---- Surface --------------------------------------------------------
        // `create_surface` takes ownership of the Arc clone, giving the surface
        // a 'static lifetime tied to the Arc reference count.
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

        // Prefer an sRGB format for correct gamma; fall back to whatever is first.
        let format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::Fifo, // vsync
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

        Self {
            surface,
            device,
            queue,
            surface_config,
        }
    }

    // -------------------------------------------------------------------------
    // Resize
    // -------------------------------------------------------------------------

    /// Reconfigure the surface after a window resize.
    pub fn resize(&mut self, new_width: u32, new_height: u32) {
        if new_width == 0 || new_height == 0 {
            return;
        }
        self.surface_config.width = new_width;
        self.surface_config.height = new_height;
        self.surface.configure(&self.device, &self.surface_config);
        log::debug!("Surface resized to {}×{}", new_width, new_height);
    }

    // -------------------------------------------------------------------------
    // Render
    // -------------------------------------------------------------------------

    /// Clear the surface to black and present the frame.
    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("clear-encoder"),
            });

        // Single render pass that clears to opaque black.
        {
            let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("clear-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
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
        } // _pass dropped → render pass ends

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
        Ok(())
    }
}
