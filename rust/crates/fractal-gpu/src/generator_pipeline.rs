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

    /// rgba32float texture written by the active generator each frame.
    pub output_tex: Texture,
    pub output_view: TextureView,
    pub width: u32,
    pub height: u32,
}

impl GeneratorPass {
    pub fn new(device: &Device, width: u32, height: u32) -> Self {
        // --- bind group layout -------------------------------------------------
        // binding 0 : Uniforms uniform buffer
        // binding 1 : rgba32float storage texture (write-only)
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
                        format: wgpu::TextureFormat::Rgba32Float,
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
            format: wgpu::TextureFormat::Rgba32Float,
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
