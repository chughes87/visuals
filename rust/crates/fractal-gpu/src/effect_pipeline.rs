use fractal_core::{ColorScheme, EffectKind};
use wgpu::{BindGroupLayout, Buffer, ComputePipeline, Device, Queue, Sampler};

use crate::context::Uniforms;

/// Shared per-effect params buffer size.
/// 16 bytes fits every effect's parameter struct.
const PARAMS_SIZE: u64 = 16;

/// Ping-pong texture set — two `rgba32float` storage textures that swap
/// roles each effect pass to avoid read-write hazards.
pub struct PingPong {
    pub tex_a:   wgpu::Texture,
    pub tex_b:   wgpu::Texture,
    pub view_a:  wgpu::TextureView,
    pub view_b:  wgpu::TextureView,
    /// `false` = A is the current read target, `true` = B is.
    pub current: bool,
}

impl PingPong {
    pub fn new(device: &Device, width: u32, height: u32) -> Self {
        let desc = wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba32Float,
            usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        };
        let tex_a  = device.create_texture(&wgpu::TextureDescriptor { label: Some("ping"), ..desc });
        let tex_b  = device.create_texture(&wgpu::TextureDescriptor { label: Some("pong"), ..desc });
        let view_a = tex_a.create_view(&Default::default());
        let view_b = tex_b.create_view(&Default::default());
        Self { tex_a, tex_b, view_a, view_b, current: false }
    }

    pub fn read_view(&self) -> &wgpu::TextureView {
        if self.current { &self.view_b } else { &self.view_a }
    }
    pub fn write_view(&self) -> &wgpu::TextureView {
        if self.current { &self.view_a } else { &self.view_b }
    }
    pub fn swap(&mut self) {
        self.current = !self.current;
    }
}

// ---------------------------------------------------------------------------
// EffectPass
// ---------------------------------------------------------------------------

/// Owns all effect compute pipelines and the GPU resources shared across
/// every effect dispatch: two uniform buffers (Uniforms + per-effect params),
/// two bind group layouts (with / without a sampler), and a linear sampler.
pub struct EffectPass {
    pub color_map:           ComputePipeline,
    pub ripple:              ComputePipeline,
    pub echo:                ComputePipeline,
    pub hue_shift:           ComputePipeline,
    pub brightness_contrast: ComputePipeline,
    pub motion_blur:         ComputePipeline,

    /// BGL for effects that sample via UV warp (ripple, echo):
    ///   binding 0: Uniforms · binding 1: params · binding 2: input ·
    ///   binding 3: output · binding 4: sampler
    bgl_sampler: BindGroupLayout,
    /// BGL for effects that use textureLoad (color_map, hue_shift,
    /// brightness_contrast, motion_blur):
    ///   binding 0: Uniforms · binding 1: params · binding 2: input ·
    ///   binding 3: output
    bgl: BindGroupLayout,

    uniform_buf: Buffer,
    params_buf:  Buffer,
    sampler:     Sampler,
}

impl EffectPass {
    pub fn new(device: &Device) -> Self {
        // --- bind group layouts -----------------------------------------------
        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("effect_bgl"),
            entries: &[
                uniform_entry(0),
                uniform_entry(1),
                texture_entry(2),
                storage_tex_entry(3),
            ],
        });

        let bgl_sampler = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("effect_bgl_sampler"),
            entries: &[
                uniform_entry(0),
                uniform_entry(1),
                texture_entry(2),
                storage_tex_entry(3),
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("effect_pl"),
            bind_group_layouts: &[&bgl],
            push_constant_ranges: &[],
        });
        let pl_sampler = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("effect_pl_sampler"),
            bind_group_layouts: &[&bgl_sampler],
            push_constant_ranges: &[],
        });

        // --- shared buffers + sampler -----------------------------------------
        let uniform_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("effect_uniforms"),
            size: std::mem::size_of::<Uniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let params_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("effect_params"),
            size: PARAMS_SIZE,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("effect_sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        // --- pipelines --------------------------------------------------------
        let make = |label: &str, src: &str, layout: &wgpu::PipelineLayout| {
            let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(label),
                source: wgpu::ShaderSource::Wgsl(src.into()),
            });
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(label),
                layout: Some(layout),
                module: &module,
                entry_point: "main",
                compilation_options: Default::default(),
                cache: None,
            })
        };

        Self {
            color_map:           make("color_map",           include_str!("../shaders/color_map.wgsl"),           &pl),
            ripple:              make("ripple",              include_str!("../shaders/ripple.wgsl"),              &pl_sampler),
            echo:                make("echo",                include_str!("../shaders/echo.wgsl"),                &pl_sampler),
            hue_shift:           make("hue_shift",           include_str!("../shaders/hue_shift.wgsl"),           &pl),
            brightness_contrast: make("brightness_contrast", include_str!("../shaders/brightness_contrast.wgsl"), &pl),
            motion_blur:         make("motion_blur",         include_str!("../shaders/motion_blur.wgsl"),         &pl),
            bgl,
            bgl_sampler,
            uniform_buf,
            params_buf,
            sampler,
        }
    }

    /// Upload uniforms + per-effect params, record one compute pass into
    /// `encoder`, then call `pp.swap()` so the next pass reads the result.
    pub fn dispatch(
        &self,
        device: &Device,
        encoder: &mut wgpu::CommandEncoder,
        queue: &Queue,
        kind: &EffectKind,
        uniforms: &Uniforms,
        pp: &mut PingPong,
        width: u32,
        height: u32,
    ) {
        queue.write_buffer(&self.uniform_buf, 0, bytemuck::bytes_of(uniforms));
        queue.write_buffer(&self.params_buf,  0, &effect_params_bytes(kind));

        let uses_sampler = matches!(kind, EffectKind::Ripple { .. } | EffectKind::Echo { .. });

        let bind_group = if uses_sampler {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("effect_bg"),
                layout: &self.bgl_sampler,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: self.uniform_buf.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 1, resource: self.params_buf.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(pp.read_view()) },
                    wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(pp.write_view()) },
                    wgpu::BindGroupEntry { binding: 4, resource: wgpu::BindingResource::Sampler(&self.sampler) },
                ],
            })
        } else {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("effect_bg"),
                layout: &self.bgl,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: self.uniform_buf.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 1, resource: self.params_buf.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(pp.read_view()) },
                    wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(pp.write_view()) },
                ],
            })
        };

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("effect_pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(self.pipeline_for(kind));
            pass.set_bind_group(0, &bind_group, &[]);
            let wg = 8u32;
            pass.dispatch_workgroups(
                (width  + wg - 1) / wg,
                (height + wg - 1) / wg,
                1,
            );
        }

        pp.swap();
    }

    fn pipeline_for(&self, kind: &EffectKind) -> &ComputePipeline {
        match kind {
            EffectKind::ColorMap { .. }           => &self.color_map,
            EffectKind::Ripple { .. }             => &self.ripple,
            EffectKind::Echo { .. }               => &self.echo,
            EffectKind::HueShift { .. }           => &self.hue_shift,
            EffectKind::BrightnessContrast { .. } => &self.brightness_contrast,
            EffectKind::MotionBlur { .. }         => &self.motion_blur,
        }
    }
}

// ---------------------------------------------------------------------------
// Serialise EffectKind → 16-byte params buffer (matches each WGSL params struct)
// ---------------------------------------------------------------------------

pub(crate) fn effect_params_bytes(kind: &EffectKind) -> [u8; 16] {
    let mut buf = [0u8; 16];
    match kind {
        EffectKind::ColorMap { scheme } => {
            let v: u32 = match scheme {
                ColorScheme::Classic     => 0,
                ColorScheme::Fire        => 1,
                ColorScheme::Ocean       => 2,
                ColorScheme::Psychedelic => 3,
            };
            buf[..4].copy_from_slice(&v.to_ne_bytes());
        }
        EffectKind::Ripple { frequency, amplitude, speed } => {
            buf[0..4].copy_from_slice(&frequency.to_ne_bytes());
            buf[4..8].copy_from_slice(&amplitude.to_ne_bytes());
            buf[8..12].copy_from_slice(&speed.to_ne_bytes());
        }
        EffectKind::Echo { layers, offset, decay } => {
            buf[0..4].copy_from_slice(&layers.to_ne_bytes());
            buf[4..8].copy_from_slice(&offset.to_ne_bytes());
            buf[8..12].copy_from_slice(&decay.to_ne_bytes());
        }
        EffectKind::HueShift { amount } => {
            buf[0..4].copy_from_slice(&amount.to_ne_bytes());
        }
        EffectKind::BrightnessContrast { brightness, contrast } => {
            buf[0..4].copy_from_slice(&brightness.to_ne_bytes());
            buf[4..8].copy_from_slice(&contrast.to_ne_bytes());
        }
        EffectKind::MotionBlur { opacity } => {
            buf[0..4].copy_from_slice(&opacity.to_ne_bytes());
        }
    }
    buf
}

// ---------------------------------------------------------------------------
// BGL entry helpers
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use fractal_core::{ColorScheme, EffectKind};

    fn f32_at(buf: &[u8; 16], offset: usize) -> f32 {
        f32::from_ne_bytes(buf[offset..offset + 4].try_into().unwrap())
    }
    fn u32_at(buf: &[u8; 16], offset: usize) -> u32 {
        u32::from_ne_bytes(buf[offset..offset + 4].try_into().unwrap())
    }

    // --- effect_params_bytes --------------------------------------------------

    #[test]
    fn params_bytes_color_map_classic() {
        let buf = effect_params_bytes(&EffectKind::ColorMap { scheme: ColorScheme::Classic });
        assert_eq!(u32_at(&buf, 0), 0);
    }

    #[test]
    fn params_bytes_color_map_fire() {
        let buf = effect_params_bytes(&EffectKind::ColorMap { scheme: ColorScheme::Fire });
        assert_eq!(u32_at(&buf, 0), 1);
    }

    #[test]
    fn params_bytes_color_map_ocean() {
        let buf = effect_params_bytes(&EffectKind::ColorMap { scheme: ColorScheme::Ocean });
        assert_eq!(u32_at(&buf, 0), 2);
    }

    #[test]
    fn params_bytes_color_map_psychedelic() {
        let buf = effect_params_bytes(&EffectKind::ColorMap { scheme: ColorScheme::Psychedelic });
        assert_eq!(u32_at(&buf, 0), 3);
    }

    #[test]
    fn params_bytes_ripple() {
        let buf = effect_params_bytes(&EffectKind::Ripple {
            frequency: 0.5, amplitude: 3.0, speed: 2.0,
        });
        assert!((f32_at(&buf, 0) - 0.5).abs() < 1e-6);
        assert!((f32_at(&buf, 4) - 3.0).abs() < 1e-6);
        assert!((f32_at(&buf, 8) - 2.0).abs() < 1e-6);
    }

    #[test]
    fn params_bytes_echo() {
        let buf = effect_params_bytes(&EffectKind::Echo {
            layers: 4, offset: 1.5, decay: 0.7,
        });
        assert_eq!(u32_at(&buf, 0), 4);
        assert!((f32_at(&buf, 4) - 1.5).abs() < 1e-6);
        assert!((f32_at(&buf, 8) - 0.7).abs() < 1e-6);
    }

    #[test]
    fn params_bytes_hue_shift() {
        let buf = effect_params_bytes(&EffectKind::HueShift { amount: 1.047 });
        assert!((f32_at(&buf, 0) - 1.047).abs() < 1e-5);
        // padding bytes should be zero
        assert_eq!(&buf[4..16], &[0u8; 12]);
    }

    #[test]
    fn params_bytes_brightness_contrast() {
        let buf = effect_params_bytes(&EffectKind::BrightnessContrast {
            brightness: 0.2, contrast: 1.5,
        });
        assert!((f32_at(&buf, 0) - 0.2).abs() < 1e-6);
        assert!((f32_at(&buf, 4) - 1.5).abs() < 1e-6);
        assert_eq!(&buf[8..16], &[0u8; 8]);
    }

    #[test]
    fn params_bytes_motion_blur() {
        let buf = effect_params_bytes(&EffectKind::MotionBlur { opacity: 0.85 });
        assert!((f32_at(&buf, 0) - 0.85).abs() < 1e-6);
        assert_eq!(&buf[4..16], &[0u8; 12]);
    }

    #[test]
    fn params_bytes_always_16_bytes() {
        let kinds = [
            EffectKind::ColorMap { scheme: ColorScheme::Classic },
            EffectKind::Ripple { frequency: 1.0, amplitude: 1.0, speed: 1.0 },
            EffectKind::Echo { layers: 1, offset: 0.0, decay: 0.5 },
            EffectKind::HueShift { amount: 0.0 },
            EffectKind::BrightnessContrast { brightness: 0.0, contrast: 1.0 },
            EffectKind::MotionBlur { opacity: 1.0 },
        ];
        for kind in &kinds {
            assert_eq!(effect_params_bytes(kind).len(), 16);
        }
    }

    // --- Uniforms layout ------------------------------------------------------

    #[test]
    fn uniforms_size_is_48_bytes() {
        // Uniforms must be 48 bytes to satisfy wgpu's min uniform buffer alignment
        // and match the WGSL struct: 2+2+1+1+1+1 f32/u32 + 2+2 padding f32 = 12 × 4
        assert_eq!(std::mem::size_of::<crate::context::Uniforms>(), 48);
    }

    // --- GPU smoke tests (require a GPU — skipped in CI) ----------------------

    /// Verify EffectPass and PingPong can be constructed without panicking.
    /// Run with:  cargo test -p fractal-gpu -- --ignored
    #[test]
    #[ignore = "requires GPU adapter"]
    fn effect_pass_new_does_not_panic() {
        pollster::block_on(async {
            let ctx = crate::context::GpuContext::new_headless().await;
            let _pass = EffectPass::new(&ctx.device);
            let _pp   = PingPong::new(&ctx.device, 64, 64);
        });
    }

    #[test]
    #[ignore = "requires GPU adapter"]
    fn ping_pong_swap_alternates_views() {
        pollster::block_on(async {
            let ctx = crate::context::GpuContext::new_headless().await;
            let mut pp = PingPong::new(&ctx.device, 64, 64);

            assert!(!pp.current);
            let read_before  = pp.read_view()  as *const _;
            let write_before = pp.write_view() as *const _;

            pp.swap();

            assert!(pp.current);
            let read_after  = pp.read_view()  as *const _;
            let write_after = pp.write_view() as *const _;

            // After swap, what was the write target is now the read target
            assert_eq!(read_after,  write_before);
            assert_eq!(write_after, read_before);
        });
    }
}

fn uniform_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

fn texture_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        count: None,
    }
}

fn storage_tex_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::StorageTexture {
            access: wgpu::StorageTextureAccess::WriteOnly,
            format: wgpu::TextureFormat::Rgba32Float,
            view_dimension: wgpu::TextureViewDimension::D2,
        },
        count: None,
    }
}
