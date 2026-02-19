pub mod modulators;
pub mod patch;
pub mod presets;

use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Params — the shared mutable state passed through the pipeline every frame
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct Params {
    pub fields: HashMap<String, f32>,
    pub time: f32,
    pub frame: u64,
    pub zoom: f32,
    pub center_x: f32,
    pub center_y: f32,
    pub max_iter: u32,
    pub mouse_x: f32,
    pub mouse_y: f32,
}

impl Params {
    pub fn default() -> Self {
        Self {
            fields: HashMap::new(),
            time: 0.0,
            frame: 0,
            zoom: 1.0,
            center_x: -0.5,
            center_y: 0.0,
            max_iter: 100,
            mouse_x: 0.0,
            mouse_y: 0.0,
        }
    }

    pub fn get(&self, key: &str) -> f32 {
        *self.fields.get(key).unwrap_or(&0.0)
    }

    pub fn set(&mut self, key: impl Into<String>, value: f32) {
        self.fields.insert(key.into(), value);
    }
}

// ---------------------------------------------------------------------------
// Traits
// ---------------------------------------------------------------------------

/// Describes which generator to use and the GPU shader it maps to.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GeneratorKind {
    Mandelbrot,
    Julia,
    BurningShip,
    NoiseField,
}

/// Describes which effect to apply and its configuration.
#[derive(Debug, Clone)]
pub enum EffectKind {
    ColorMap { scheme: ColorScheme },
    Ripple { frequency: f32, amplitude: f32, speed: f32 },
    Echo { layers: u32, offset: f32, decay: f32 },
    HueShift { amount: f32 },
    BrightnessContrast { brightness: f32, contrast: f32 },
    MotionBlur { opacity: f32 },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ColorScheme {
    Classic,
    Fire,
    Ocean,
    Psychedelic,
}

pub trait Generator: Send + Sync {
    fn kind(&self) -> GeneratorKind;
    /// Which Params fields affect the generator output (used for cache invalidation).
    fn gen_param_keys(&self) -> &[&'static str];
}

pub trait Effect: Send + Sync {
    fn kind(&self) -> EffectKind;
}

pub trait Modulator: Send + Sync {
    fn modulate(&self, params: &mut Params);
}
