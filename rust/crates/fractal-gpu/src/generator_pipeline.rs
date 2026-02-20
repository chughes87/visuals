use fractal_core::GeneratorKind;
use wgpu::{BindGroupLayout, Buffer, ComputePipeline, Device, Queue, Texture, TextureView};

use crate::context::Uniforms;

/// Holds one compute pipeline per generator variant plus the GPU resources
/// shared across all of them: a uniform buffer, a bind group layout, and the
/// output texture that every pipeline writes into.
pub struct GeneratorPass {
    pub mandelbrot: ComputePipeline,
    pub julia: ComputePipeline,
    pub burning_ship: ComputePipeline,
    pub noise_field: ComputePipeline,

    bind_group_layout: BindGroupLayout,
    uniform_buf: Buffer,

    /// rgba16float texture written by the active generator each frame.
    pub output_tex: Texture,
    pub output_view: TextureView,
    pub width: u32,
    pub height: u32,
}

impl GeneratorPass {
    pub fn new(device: &Device, width: u32, height: u32) -> Self {
        // --- bind group layout -------------------------------------------------
        // binding 0 : Uniforms uniform buffer
        // binding 1 : rgba16float storage texture (write-only)
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("gen_bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: wgpu::TextureFormat::Rgba16Float,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("gen_pl"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        // --- uniform buffer ----------------------------------------------------
        let uniform_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("gen_uniforms"),
            size: std::mem::size_of::<Uniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // --- output texture ----------------------------------------------------
        let output_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("gen_output"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let output_view = output_tex.create_view(&Default::default());

        // --- pipelines --------------------------------------------------------
        let make = |label: &str, src: &str| {
            let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(label),
                source: wgpu::ShaderSource::Wgsl(src.into()),
            });
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(label),
                layout: Some(&pipeline_layout),
                module: &module,
                entry_point: "main",
                compilation_options: Default::default(),
                cache: None,
            })
        };

        Self {
            mandelbrot: make("mandelbrot", include_str!("../shaders/mandelbrot.wgsl")),
            julia: make("julia", include_str!("../shaders/julia.wgsl")),
            burning_ship: make("burning_ship", include_str!("../shaders/burning_ship.wgsl")),
            noise_field: make("noise_field", include_str!("../shaders/noise_field.wgsl")),
            bind_group_layout,
            uniform_buf,
            output_tex,
            output_view,
            width,
            height,
        }
    }

    /// Upload uniforms and record the generator compute pass into `encoder`.
    /// The result lands in `self.output_tex`, ready for the effect chain.
    pub fn dispatch(
        &self,
        device: &Device,
        encoder: &mut wgpu::CommandEncoder,
        queue: &Queue,
        kind: GeneratorKind,
        uniforms: &Uniforms,
    ) {
        queue.write_buffer(&self.uniform_buf, 0, bytemuck::bytes_of(uniforms));

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("gen_bg"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.uniform_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&self.output_view),
                },
            ],
        });

        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("gen_pass"),
            timestamp_writes: None,
        });
        pass.set_pipeline(self.pipeline_for(kind));
        pass.set_bind_group(0, &bind_group, &[]);

        let wg = 8u32;
        pass.dispatch_workgroups(self.width.div_ceil(wg), self.height.div_ceil(wg), 1);
    }

    fn pipeline_for(&self, kind: GeneratorKind) -> &ComputePipeline {
        match kind {
            GeneratorKind::Mandelbrot => &self.mandelbrot,
            GeneratorKind::Julia => &self.julia,
            GeneratorKind::BurningShip => &self.burning_ship,
            GeneratorKind::NoiseField => &self.noise_field,
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    // --- WGSL validation (CPU-only, no GPU required) -------------------------

    /// Parse and type-check a WGSL shader using naga, the same validator that
    /// wgpu uses internally.  Catches struct layout mismatches, undefined
    /// builtins, type errors, and binding mismatches without needing a device.
    fn validate_wgsl(label: &str, src: &str) {
        let module = naga::front::wgsl::parse_str(src)
            .unwrap_or_else(|e| panic!("{label}: WGSL parse failed\n{e}"));
        let mut validator = naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        );
        validator
            .validate(&module)
            .unwrap_or_else(|e| panic!("{label}: WGSL validation failed\n{e:?}"));
    }

    #[test]
    fn mandelbrot_wgsl_is_valid() {
        validate_wgsl("mandelbrot", include_str!("../shaders/mandelbrot.wgsl"));
    }

    #[test]
    fn julia_wgsl_is_valid() {
        validate_wgsl("julia", include_str!("../shaders/julia.wgsl"));
    }

    #[test]
    fn burning_ship_wgsl_is_valid() {
        validate_wgsl("burning_ship", include_str!("../shaders/burning_ship.wgsl"));
    }

    #[test]
    fn noise_field_wgsl_is_valid() {
        validate_wgsl("noise_field", include_str!("../shaders/noise_field.wgsl"));
    }

    // --- Coordinate mapping (Rust mirror of the WGSL UV formula) -------------
    //
    // let uv = (px - resolution * 0.5) / (zoom * resolution.y * 0.5);
    // let c  = center + uv;

    fn complex_for_pixel(
        px: f32,
        py: f32,
        width: f32,
        height: f32,
        zoom: f32,
        cx: f32,
        cy: f32,
    ) -> (f32, f32) {
        let scale = zoom * height * 0.5;
        let ux = (px - width * 0.5) / scale;
        let uy = (py - height * 0.5) / scale;
        (cx + ux, cy + uy)
    }

    #[test]
    fn center_pixel_maps_to_center_coordinate() {
        // The centre pixel should land exactly on `center`.
        let (rx, ry) = complex_for_pixel(400.0, 300.0, 800.0, 600.0, 1.0, -0.5, 0.0);
        assert!((rx - (-0.5)).abs() < 1e-6, "x={rx}");
        assert!(ry.abs() < 1e-6, "y={ry}");
    }

    #[test]
    fn top_left_pixel_at_zoom1_center0() {
        // At zoom=1, center=(0,0) the top-left pixel should be at
        // (-(width/height), -1) = (-800/600, -1) ≈ (-1.333, -1).
        let (rx, ry) = complex_for_pixel(0.0, 0.0, 800.0, 600.0, 1.0, 0.0, 0.0);
        assert!((rx - (-800.0 / 600.0)).abs() < 1e-5, "x={rx}");
        assert!((ry - (-1.0)).abs() < 1e-5, "y={ry}");
    }

    #[test]
    fn doubling_zoom_halves_the_view_span() {
        // At zoom=2 the same pixel should be half as far from center as at zoom=1.
        let (rx1, _) = complex_for_pixel(0.0, 300.0, 800.0, 600.0, 1.0, 0.0, 0.0);
        let (rx2, _) = complex_for_pixel(0.0, 300.0, 800.0, 600.0, 2.0, 0.0, 0.0);
        assert!(
            (rx2 - rx1 / 2.0).abs() < 1e-6,
            "zoom=1 edge={rx1}, zoom=2 edge={rx2}"
        );
    }

    // --- Mandelbrot iteration (mirrors shader loop) --------------------------

    fn mandelbrot_iter(cx: f32, cy: f32, max_iter: u32) -> (u32, f32, f32) {
        let (mut x, mut y) = (0.0f32, 0.0f32);
        let mut i = 0u32;
        while i < max_iter {
            if x * x + y * y > 4.0 {
                break;
            }
            let xn = x * x - y * y + cx;
            y = 2.0 * x * y + cy;
            x = xn;
            i += 1;
        }
        (i, x, y)
    }

    #[test]
    fn mandelbrot_origin_is_interior() {
        let (i, _, _) = mandelbrot_iter(0.0, 0.0, 100);
        assert_eq!(i, 100, "c=(0,0) must be interior");
    }

    #[test]
    fn mandelbrot_far_point_escapes_on_first_iteration() {
        // z₁ = (2.1)² + 2.1 at first step — actually: z₀=0 → z₁=(2.1,0) → |z₁|²=4.41>4
        let (i, _, _) = mandelbrot_iter(2.1, 0.0, 100);
        assert_eq!(i, 1, "c=(2.1,0) should escape at iter 1");
    }

    #[test]
    fn mandelbrot_exterior_point_escapes() {
        // c = (0.5, 0.5) is well outside the Mandelbrot set; the orbit diverges
        // within a handful of steps.
        let (i, _, _) = mandelbrot_iter(0.5, 0.5, 100);
        assert!(i < 10, "c=(0.5,0.5) should escape quickly; got i={i}");
    }

    // --- Julia iteration (c fixed, z starts at pixel) ------------------------

    fn julia_iter(zx: f32, zy: f32, cx: f32, cy: f32, max_iter: u32) -> (u32, f32, f32) {
        let (mut x, mut y) = (zx, zy);
        let mut i = 0u32;
        while i < max_iter {
            if x * x + y * y > 4.0 {
                break;
            }
            let xn = x * x - y * y + cx;
            y = 2.0 * x * y + cy;
            x = xn;
            i += 1;
        }
        (i, x, y)
    }

    #[test]
    fn julia_psychedelic_c_origin_escapes_late() {
        // The PsychedelicJulia preset uses c = (-0.7, 0.27015).  z=(0,0) lies
        // very close to the Julia set boundary and escapes only after many
        // iterations (empirically at i=96 for max_iter=100), confirming that
        // the iteration loop runs the full depth for near-boundary points.
        let (i, _, _) = julia_iter(0.0, 0.0, -0.7, 0.27015, 100);
        assert!(i > 50, "z=(0,0) should escape late (>50 iters); got i={i}");
    }

    #[test]
    fn julia_point_outside_radius_2_escapes_immediately() {
        // |z|² = 9 > 4 → the while condition breaks at i=0 before any iteration.
        let (i, _, _) = julia_iter(3.0, 0.0, -0.7, 0.27015, 100);
        assert_eq!(i, 0, "z=(3,0) should escape at i=0");
    }

    // --- Burning Ship iteration (mirrors shader loop) ------------------------

    fn burning_ship_iter(cx: f32, cy: f32, max_iter: u32) -> (u32, f32, f32) {
        let (mut x, mut y) = (0.0f32, 0.0f32);
        let mut i = 0u32;
        while i < max_iter {
            if x * x + y * y > 4.0 {
                break;
            }
            let xn = x * x - y * y + cx;
            y = 2.0 * x.abs() * y.abs() + cy;
            x = xn;
            i += 1;
        }
        (i, x, y)
    }

    #[test]
    fn burning_ship_origin_is_interior() {
        let (i, _, _) = burning_ship_iter(0.0, 0.0, 100);
        assert_eq!(i, 100, "c=(0,0) must be interior");
    }

    #[test]
    fn burning_ship_far_point_escapes() {
        let (i, _, _) = burning_ship_iter(3.0, 3.0, 100);
        assert!(i < 100, "c=(3,3) should escape; got i={i}");
    }

    #[test]
    fn burning_ship_differs_from_mandelbrot_for_same_c() {
        // For c = (-1.76, -0.02) the two fractals produce different escape counts,
        // proving the abs() transform has a real effect.
        let (mi, _, _) = mandelbrot_iter(-1.76, -0.02, 200);
        let (bi, _, _) = burning_ship_iter(-1.76, -0.02, 200);
        assert_ne!(
            mi, bi,
            "Mandelbrot and BurningShip should differ at c=(-1.76,-0.02)"
        );
    }

    // --- GPU smoke test (requires adapter, skipped in CI) --------------------

    /// Verify GeneratorPass::new compiles all four shaders on the actual device.
    /// Run with:  cargo test -p fractal-gpu -- --ignored
    #[test]
    #[ignore = "requires GPU adapter"]
    fn generator_pass_new_does_not_panic() {
        pollster::block_on(async {
            let ctx = crate::context::GpuContext::new_headless().await;
            let _pass = super::GeneratorPass::new(&ctx.device, 64, 64);
        });
    }
}
