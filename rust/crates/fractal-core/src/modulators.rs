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
            Waveform::Triangle => {
                2.0 * (phase / TAU - (phase / TAU + 0.5).floor()).abs() * 2.0 - 1.0
            }
            Waveform::Square => {
                if phase.sin() >= 0.0 {
                    1.0
                } else {
                    -1.0
                }
            }
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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    fn params_at(time: f32) -> Params {
        let mut p = Params::default();
        p.time = time;
        p
    }

    // --- Lfo::Sine ------------------------------------------------------------

    #[test]
    fn lfo_sine_at_zero_time() {
        // sin(0) = 0  →  output = offset + 0 * amplitude = offset
        let lfo = Lfo {
            target: "v",
            waveform: Waveform::Sine,
            frequency: 1.0,
            amplitude: 2.0,
            offset: 5.0,
        };
        let mut p = params_at(0.0);
        lfo.modulate(&mut p);
        assert!((p.get("v") - 5.0).abs() < 1e-5, "got {}", p.get("v"));
    }

    #[test]
    fn lfo_sine_at_quarter_period() {
        // time = 0.25 s, freq = 1 Hz  →  phase = TAU*0.25 = π/2  →  sin = 1
        let lfo = Lfo {
            target: "v",
            waveform: Waveform::Sine,
            frequency: 1.0,
            amplitude: 1.0,
            offset: 0.0,
        };
        let mut p = params_at(0.25);
        lfo.modulate(&mut p);
        assert!((p.get("v") - 1.0).abs() < 1e-5, "got {}", p.get("v"));
    }

    #[test]
    fn lfo_sine_at_three_quarter_period() {
        // phase = TAU*0.75  →  sin ≈ -1
        let lfo = Lfo {
            target: "v",
            waveform: Waveform::Sine,
            frequency: 1.0,
            amplitude: 1.0,
            offset: 0.0,
        };
        let mut p = params_at(0.75);
        lfo.modulate(&mut p);
        assert!((p.get("v") - (-1.0)).abs() < 1e-5, "got {}", p.get("v"));
    }

    #[test]
    fn lfo_sine_amplitude_and_offset() {
        // At quarter period: output = offset + amplitude * 1.0
        let lfo = Lfo {
            target: "v",
            waveform: Waveform::Sine,
            frequency: 1.0,
            amplitude: 3.0,
            offset: 10.0,
        };
        let mut p = params_at(0.25);
        lfo.modulate(&mut p);
        assert!((p.get("v") - 13.0).abs() < 1e-4, "got {}", p.get("v"));
    }

    // --- Lfo::Square ----------------------------------------------------------

    #[test]
    fn lfo_square_positive_half() {
        // sin(TAU*0.1) > 0  →  raw = +1
        let lfo = Lfo {
            target: "v",
            waveform: Waveform::Square,
            frequency: 1.0,
            amplitude: 1.0,
            offset: 0.0,
        };
        let mut p = params_at(0.1);
        lfo.modulate(&mut p);
        assert!((p.get("v") - 1.0).abs() < 1e-5);
    }

    #[test]
    fn lfo_square_negative_half() {
        // sin(TAU*0.75) < 0  →  raw = -1
        let lfo = Lfo {
            target: "v",
            waveform: Waveform::Square,
            frequency: 1.0,
            amplitude: 1.0,
            offset: 0.0,
        };
        let mut p = params_at(0.75);
        lfo.modulate(&mut p);
        assert!((p.get("v") - (-1.0)).abs() < 1e-5);
    }

    // --- Lfo::Saw -------------------------------------------------------------

    #[test]
    fn lfo_saw_at_half_period() {
        // phase/TAU = 0.5  →  2*(0.5 - 0) - 1 = 0.0
        let lfo = Lfo {
            target: "v",
            waveform: Waveform::Saw,
            frequency: 1.0,
            amplitude: 1.0,
            offset: 0.0,
        };
        let mut p = params_at(0.5);
        lfo.modulate(&mut p);
        assert!((p.get("v")).abs() < 1e-5, "got {}", p.get("v"));
    }

    // --- Lfo::Triangle --------------------------------------------------------

    #[test]
    fn lfo_triangle_at_half_period() {
        // phase/TAU = 0.5 → (0.5 + 0.5).floor() = 1 → |0.5-1| = 0.5 → 2*0.5*2-1 = 1.0
        let lfo = Lfo {
            target: "v",
            waveform: Waveform::Triangle,
            frequency: 1.0,
            amplitude: 1.0,
            offset: 0.0,
        };
        let mut p = params_at(0.5);
        lfo.modulate(&mut p);
        assert!((p.get("v") - 1.0).abs() < 1e-5, "got {}", p.get("v"));
    }

    // --- MouseModulator -------------------------------------------------------

    #[test]
    fn mouse_modulator_maps_x() {
        let mm = MouseModulator {
            target_x: Some("mx"),
            target_y: None,
        };
        let mut p = Params::default();
        p.mouse_x = 1.0; // → 1.0*2 - 1 = 1.0
        mm.modulate(&mut p);
        assert!((p.get("mx") - 1.0).abs() < 1e-6);
    }

    #[test]
    fn mouse_modulator_maps_y() {
        let mm = MouseModulator {
            target_x: None,
            target_y: Some("my"),
        };
        let mut p = Params::default();
        p.mouse_y = 0.5; // → 0.5*2 - 1 = 0.0
        mm.modulate(&mut p);
        assert!((p.get("my")).abs() < 1e-6);
    }

    #[test]
    fn mouse_modulator_skips_none_targets() {
        let mm = MouseModulator {
            target_x: None,
            target_y: None,
        };
        let mut p = Params::default();
        mm.modulate(&mut p);
        assert_eq!(p.get("mx"), 0.0);
    }

    // --- RandomWalk -----------------------------------------------------------

    #[test]
    fn random_walk_sets_target() {
        let rw = RandomWalk {
            target: "drift",
            speed: 1.0,
        };
        let mut p = Params::default();
        p.time = 1.0;
        rw.modulate(&mut p);
        // Value is deterministic — just check it's in [-0.5, 0.5]
        let v = p.get("drift");
        assert!(v >= -0.5 && v <= 0.5, "out of range: {v}");
    }

    // --- ModMatrix ------------------------------------------------------------

    #[test]
    fn mod_matrix_scales_to_range() {
        // Inner Lfo outputs +1.0 at t=0.25  →  raw=1.0  →  scaled = min + (1.0*0.5+0.5)*(max-min) = min + 1*(max-min) = max
        let matrix = ModMatrix {
            routes: vec![Route {
                modulator: Box::new(Lfo {
                    target: "v",
                    waveform: Waveform::Sine,
                    frequency: 1.0,
                    amplitude: 1.0,
                    offset: 0.0,
                }),
                target: "v",
                min: 10.0,
                max: 20.0,
            }],
        };
        let mut p = params_at(0.25);
        matrix.modulate(&mut p);
        assert!((p.get("v") - 20.0).abs() < 1e-4, "got {}", p.get("v"));
    }

    #[test]
    fn mod_matrix_scales_min_at_negative_one() {
        // Lfo Sine at t=0.75  →  raw=-1.0  →  scaled = min + (-1*0.5+0.5)*(max-min) = min + 0 = min
        let matrix = ModMatrix {
            routes: vec![Route {
                modulator: Box::new(Lfo {
                    target: "v",
                    waveform: Waveform::Sine,
                    frequency: 1.0,
                    amplitude: 1.0,
                    offset: 0.0,
                }),
                target: "v",
                min: 10.0,
                max: 20.0,
            }],
        };
        let mut p = params_at(0.75);
        matrix.modulate(&mut p);
        assert!((p.get("v") - 10.0).abs() < 1e-4, "got {}", p.get("v"));
    }

    #[test]
    fn mod_matrix_multiple_routes() {
        // Two routes targeting different keys
        let matrix = ModMatrix {
            routes: vec![
                Route {
                    modulator: Box::new(Lfo {
                        target: "a",
                        waveform: Waveform::Sine,
                        frequency: 1.0,
                        amplitude: 1.0,
                        offset: 0.0,
                    }),
                    target: "a",
                    min: 0.0,
                    max: 1.0,
                },
                Route {
                    modulator: Box::new(Lfo {
                        target: "b",
                        waveform: Waveform::Sine,
                        frequency: 1.0,
                        amplitude: 1.0,
                        offset: 0.0,
                    }),
                    target: "b",
                    min: 5.0,
                    max: 10.0,
                },
            ],
        };
        let mut p = params_at(0.25); // both Lfos hit +1
        matrix.modulate(&mut p);
        assert!((p.get("a") - 1.0).abs() < 1e-4);
        assert!((p.get("b") - 10.0).abs() < 1e-4);
    }
}
