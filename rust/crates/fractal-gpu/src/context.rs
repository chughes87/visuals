use wgpu::{Device, Instance, Queue};

pub struct GpuContext {
    pub instance: Instance,
    pub device: Device,
    pub queue: Queue,
}

impl GpuContext {
    /// Create a headless GPU context (no surface). Used for compute-only work
    /// and testing. A surface-aware variant is created by `fractal-app`.
    pub async fn new_headless() -> Self {
        let instance = Instance::default();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
            .expect("No suitable GPU adapter found");

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("fractal-gpu device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    memory_hints: Default::default(),
                },
                None,
            )
            .await
            .expect("Failed to create GPU device");

        Self {
            instance,
            device,
            queue,
        }
    }
}

/// All per-frame data uploaded to the GPU as a single uniform buffer.
/// Must match the `Uniforms` struct in every WGSL shader.
/// `repr(C)` + `bytemuck` ensures safe casting to `&[u8]`.
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Uniforms {
    pub resolution: [f32; 2],
    pub center: [f32; 2],
    pub zoom: f32,
    pub time: f32,
    pub max_iter: u32,
    pub _pad: u32, // keep 16-byte alignment
    // Julia-set specific (unused for other generators — zero them out)
    pub julia_c: [f32; 2],
    pub _pad2: [f32; 2],
}
