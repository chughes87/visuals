use crate::{Modulator, Params};
use std::f32::consts::TAU;

// ---------------------------------------------------------------------------
// LFO
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
pub enum Waveform {
    Sine,
    Triangle,
    Square,
    Saw,
}

pub struct Lfo {
    pub target: &'static str,
    pub waveform: Waveform,
    pub frequency: f32,
    pub amplitude: f32,
    pub offset: f32,
}

impl Modulator for Lfo {
    fn modulate(&self, params: &mut Params) {
        let phase = params.time * self.frequency * TAU;
        let raw = match self.waveform {
            Waveform::Sine => phase.sin(),
            Waveform::Triangle => 2.0 * (phase / TAU - (phase / TAU + 0.5).floor()).abs() * 2.0 - 1.0,
            Waveform::Square => if phase.sin() >= 0.0 { 1.0 } else { -1.0 },
            Waveform::Saw => 2.0 * (phase / TAU - (phase / TAU).floor()) - 1.0,
        };
        params.set(self.target, self.offset + raw * self.amplitude);
    }
}

// ---------------------------------------------------------------------------
// RandomWalk  (exponential smoothing toward a new target each period)
// ---------------------------------------------------------------------------

pub struct RandomWalk {
    pub target: &'static str,
    pub speed: f32,
    // Internal state — for a real implementation this would use interior
    // mutability; left simple here as a placeholder.
}

impl Modulator for RandomWalk {
    fn modulate(&self, params: &mut Params) {
        // Placeholder: smooth drift using a sine of a large prime offset
        let drift = (params.time * self.speed * 0.37 + 1.618).sin() * 0.5;
        params.set(self.target, drift);
    }
}

// ---------------------------------------------------------------------------
// MouseModulator
// ---------------------------------------------------------------------------

pub struct MouseModulator {
    pub target_x: Option<&'static str>,
    pub target_y: Option<&'static str>,
}

impl Modulator for MouseModulator {
    fn modulate(&self, params: &mut Params) {
        if let Some(key) = self.target_x {
            params.set(key, params.mouse_x * 2.0 - 1.0);
        }
        if let Some(key) = self.target_y {
            params.set(key, params.mouse_y * 2.0 - 1.0);
        }
    }
}

// ---------------------------------------------------------------------------
// ModMatrix  — routes multiple modulators to params with min/max scaling
// ---------------------------------------------------------------------------

pub struct Route {
    pub modulator: Box<dyn Modulator>,
    pub target: &'static str,
    pub min: f32,
    pub max: f32,
}

pub struct ModMatrix {
    pub routes: Vec<Route>,
}

impl Modulator for ModMatrix {
    fn modulate(&self, params: &mut Params) {
        for route in &self.routes {
            // Run the inner modulator into a temporary params, read back the
            // raw [-1, 1] output, then scale to [min, max].
            let mut tmp = params.clone();
            route.modulator.modulate(&mut tmp);
            let raw = tmp.get(route.target);
            let scaled = route.min + (raw * 0.5 + 0.5) * (route.max - route.min);
            params.set(route.target, scaled);
        }
    }
}
