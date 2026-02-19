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

impl Default for Params {
    fn default() -> Self {
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
}

impl Params {
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
    ColorMap {
        scheme: ColorScheme,
    },
    Ripple {
        frequency: f32,
        amplitude: f32,
        speed: f32,
    },
    Echo {
        layers: u32,
        offset: f32,
        decay: f32,
    },
    HueShift {
        amount: f32,
    },
    BrightnessContrast {
        brightness: f32,
        contrast: f32,
    },
    MotionBlur {
        opacity: f32,
    },
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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- Params ----------------------------------------------------------------

    #[test]
    fn params_default_values() {
        let p = Params::default();
        assert_eq!(p.zoom, 1.0);
        assert_eq!(p.center_x, -0.5);
        assert_eq!(p.center_y, 0.0);
        assert_eq!(p.max_iter, 100);
        assert_eq!(p.time, 0.0);
        assert_eq!(p.frame, 0);
        assert_eq!(p.mouse_x, 0.0);
        assert_eq!(p.mouse_y, 0.0);
        assert!(p.fields.is_empty());
    }

    #[test]
    fn params_set_and_get() {
        let mut p = Params::default();
        p.set("foo", 3.14);
        assert!((p.get("foo") - 3.14).abs() < 1e-6);
    }

    #[test]
    fn params_get_missing_returns_zero() {
        let p = Params::default();
        assert_eq!(p.get("nonexistent"), 0.0);
    }

    #[test]
    fn params_set_overwrites() {
        let mut p = Params::default();
        p.set("x", 1.0);
        p.set("x", 2.0);
        assert_eq!(p.get("x"), 2.0);
    }

    // --- GeneratorKind ---------------------------------------------------------

    #[test]
    fn generator_kind_eq() {
        assert_eq!(GeneratorKind::Mandelbrot, GeneratorKind::Mandelbrot);
        assert_ne!(GeneratorKind::Julia, GeneratorKind::BurningShip);
        assert_ne!(GeneratorKind::NoiseField, GeneratorKind::Mandelbrot);
    }

    // --- EffectKind ------------------------------------------------------------

    #[test]
    fn effect_kind_matches() {
        let e = EffectKind::HueShift { amount: 1.5 };
        assert!(matches!(e, EffectKind::HueShift { .. }));

        let e2 = EffectKind::Ripple {
            frequency: 0.1,
            amplitude: 5.0,
            speed: 1.0,
        };
        assert!(matches!(e2, EffectKind::Ripple { .. }));
    }

    #[test]
    fn effect_kind_echo_fields() {
        let e = EffectKind::Echo {
            layers: 3,
            offset: 0.5,
            decay: 0.8,
        };
        if let EffectKind::Echo {
            layers,
            offset,
            decay,
        } = e
        {
            assert_eq!(layers, 3);
            assert!((offset - 0.5).abs() < 1e-6);
            assert!((decay - 0.8).abs() < 1e-6);
        } else {
            panic!("wrong variant");
        }
    }

    // --- ColorScheme -----------------------------------------------------------

    #[test]
    fn color_scheme_eq() {
        assert_eq!(ColorScheme::Classic, ColorScheme::Classic);
        assert_ne!(ColorScheme::Fire, ColorScheme::Ocean);
        assert_ne!(ColorScheme::Psychedelic, ColorScheme::Classic);
    }
}
