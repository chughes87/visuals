use fractal_core::EffectKind;
use wgpu::{ComputePipeline, Device};

/// Ping-pong texture set — two storage textures that swap roles each effect pass.
pub struct PingPong {
    pub tex_a: wgpu::Texture,
    pub tex_b: wgpu::Texture,
    pub view_a: wgpu::TextureView,
    pub view_b: wgpu::TextureView,
    /// Which texture holds the current output (false = A, true = B).
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
        let tex_a = device.create_texture(&wgpu::TextureDescriptor { label: Some("ping"), ..desc });
        let tex_b = device.create_texture(&wgpu::TextureDescriptor { label: Some("pong"), ..desc });
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

pub struct EffectPipelines {
    pub color_map: ComputePipeline,
    pub ripple: ComputePipeline,
    pub echo: ComputePipeline,
    pub hue_shift: ComputePipeline,
    pub brightness_contrast: ComputePipeline,
    pub motion_blur: ComputePipeline,
}

impl EffectPipelines {
    pub fn new(device: &Device) -> Self {
        Self {
            color_map:           make_pipeline(device, "color_map",           include_str!("../shaders/color_map.wgsl")),
            ripple:              make_pipeline(device, "ripple",              include_str!("../shaders/ripple.wgsl")),
            echo:                make_pipeline(device, "echo",                include_str!("../shaders/echo.wgsl")),
            hue_shift:           make_pipeline(device, "hue_shift",           include_str!("../shaders/hue_shift.wgsl")),
            brightness_contrast: make_pipeline(device, "brightness_contrast", include_str!("../shaders/brightness_contrast.wgsl")),
            motion_blur:         make_pipeline(device, "motion_blur",         include_str!("../shaders/motion_blur.wgsl")),
        }
    }

    pub fn get(&self, kind: &EffectKind) -> &ComputePipeline {
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

fn make_pipeline(device: &Device, label: &str, wgsl: &str) -> ComputePipeline {
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(label),
        source: wgpu::ShaderSource::Wgsl(wgsl.into()),
    });
    device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some(label),
        layout: None,
        module: &module,
        entry_point: Some("main"),
        compilation_options: Default::default(),
        cache: None,
    })
}
