use std::f32::consts::TAU;

use crate::{
    modulators::{Lfo, ModMatrix, Route, Waveform},
    patch::Patch,
    BrightnessContrastEffect, BurningShipGen, ColorMapEffect, ColorScheme, EchoEffect,
    HueShiftEffect, JuliaGen, MandelbrotGen, MotionBlurEffect, NoiseFieldGen, Params, RippleEffect,
};

/// Preset names, matching the five from the original Clojure implementation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Preset {
    ClassicMandelbrot,
    PsychedelicJulia,
    TrippyMandelbrot,
    BurningShipTrails,
    NoiseField,
}

impl Preset {
    pub const ALL: [Preset; 5] = [
        Preset::ClassicMandelbrot,
        Preset::PsychedelicJulia,
        Preset::TrippyMandelbrot,
        Preset::BurningShipTrails,
        Preset::NoiseField,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Preset::ClassicMandelbrot => "Classic Mandelbrot",
            Preset::PsychedelicJulia => "Psychedelic Julia",
            Preset::TrippyMandelbrot => "Trippy Mandelbrot",
            Preset::BurningShipTrails => "Burning Ship Trails",
            Preset::NoiseField => "Noise Field",
        }
    }

    /// Construct a fully-configured [`Patch`] for this preset, mirroring the
    /// corresponding Clojure patch factory in `presets.clj`.
    pub fn build(self) -> Patch {
        match self {
            // -----------------------------------------------------------------
            // 1. Classic Mandelbrot
            //    Clojure: mandelbrot + color-mapper(:classic) + no modulators
            // -----------------------------------------------------------------
            Preset::ClassicMandelbrot => {
                let mut params = Params::default();
                params.center_x = -0.5;
                params.center_y = 0.0;
                params.zoom = 1.0;
                params.max_iter = 100;

                Patch::new(Box::new(MandelbrotGen), params)
                    .add_effect(Box::new(ColorMapEffect(ColorScheme::Classic)))
            }

            // -----------------------------------------------------------------
            // 2. Psychedelic Julia
            //    Clojure: julia(-0.7, 0.27015) + psychedelic color-map +
            //             hue-shift driven by LFO(0.5 Hz, sine) → [0, TAU].
            //
            //    julia_cx / julia_cy are stored in Params::fields so the GPU
            //    layer can read them into Uniforms::julia_c each frame.
            // -----------------------------------------------------------------
            Preset::PsychedelicJulia => {
                let mut params = Params::default();
                params.center_x = 0.0;
                params.center_y = 0.0;
                params.zoom = 1.0;
                params.max_iter = 100;
                params.set("julia_cx", -0.7_f32);
                params.set("julia_cy", 0.27015_f32);
                params.set("hue_shift_amount", 0.0_f32);

                Patch::new(Box::new(JuliaGen), params)
                    .add_effect(Box::new(ColorMapEffect(ColorScheme::Psychedelic)))
                    .add_effect(Box::new(HueShiftEffect("hue_shift_amount")))
                    .add_modulator(Box::new(ModMatrix {
                        routes: vec![Route {
                            modulator: Box::new(Lfo {
                                target: "hue_shift_amount",
                                waveform: Waveform::Sine,
                                frequency: 0.5,
                                amplitude: 1.0,
                                offset: 0.0,
                            }),
                            target: "hue_shift_amount",
                            min: 0.0,
                            max: TAU,
                        }],
                    }))
            }

            // -----------------------------------------------------------------
            // 3. Trippy Mandelbrot
            //    Clojure: mandelbrot + ocean color-map + ripple(0.05, 10, 2) +
            //             echo(3, 5, 5, 2.0) + particles(skipped, Phase 7) +
            //             LFO(0.3 Hz) → ripple_amplitude [5, 15].
            // -----------------------------------------------------------------
            Preset::TrippyMandelbrot => {
                let mut params = Params::default();
                params.center_x = -0.5;
                params.center_y = 0.0;
                params.zoom = 1.0;
                params.max_iter = 100;
                params.set("ripple_amplitude", 10.0_f32);

                Patch::new(Box::new(MandelbrotGen), params)
                    .add_effect(Box::new(ColorMapEffect(ColorScheme::Ocean)))
                    .add_effect(Box::new(RippleEffect {
                        frequency: 0.05,
                        amplitude_key: "ripple_amplitude",
                        speed: 2.0,
                    }))
                    .add_effect(Box::new(EchoEffect {
                        layers: 3,
                        offset: 5.0,
                        decay: 2.0,
                    }))
                    // ParticleSystem effect deferred to Phase 7 (GPU compute particles).
                    .add_modulator(Box::new(ModMatrix {
                        routes: vec![Route {
                            modulator: Box::new(Lfo {
                                target: "ripple_amplitude",
                                waveform: Waveform::Sine,
                                frequency: 0.3,
                                amplitude: 1.0,
                                offset: 0.0,
                            }),
                            target: "ripple_amplitude",
                            min: 5.0,
                            max: 15.0,
                        }],
                    }))
            }

            // -----------------------------------------------------------------
            // 4. Burning Ship Trails
            //    Clojure: burning-ship + fire color-map + motion-blur(0.15)
            // -----------------------------------------------------------------
            Preset::BurningShipTrails => {
                let mut params = Params::default();
                params.center_x = -0.5;
                params.center_y = -0.5;
                params.zoom = 1.0;
                params.max_iter = 100;

                Patch::new(Box::new(BurningShipGen), params)
                    .add_effect(Box::new(ColorMapEffect(ColorScheme::Fire)))
                    .add_effect(Box::new(MotionBlurEffect(0.15)))
            }

            // -----------------------------------------------------------------
            // 5. Noise Field
            //    Clojure: noise(0.01, 4) + psychedelic color-map +
            //             brightness-contrast(20, 1.5) driven by LFO(0.2 Hz)
            //             → brightness [0, 40/255].
            //
            //    Brightness values are normalised to [0, 1] (Rust shader uses
            //    float channels), so the Clojure range 0–40 (out of 255)
            //    maps to 0.0–0.157.
            // -----------------------------------------------------------------
            Preset::NoiseField => {
                let mut params = Params::default();
                // Initial midpoint ≈ Clojure's brightness=20 on 0-255 scale
                params.set("brightness_amount", 20.0_f32 / 255.0);

                Patch::new(Box::new(NoiseFieldGen), params)
                    .add_effect(Box::new(ColorMapEffect(ColorScheme::Psychedelic)))
                    .add_effect(Box::new(BrightnessContrastEffect {
                        brightness_key: "brightness_amount",
                        contrast: 1.5,
                    }))
                    .add_modulator(Box::new(ModMatrix {
                        routes: vec![Route {
                            modulator: Box::new(Lfo {
                                target: "brightness_amount",
                                waveform: Waveform::Sine,
                                frequency: 0.2,
                                amplitude: 1.0,
                                offset: 0.0,
                            }),
                            target: "brightness_amount",
                            min: 0.0,
                            max: 40.0 / 255.0,
                        }],
                    }))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{EffectKind, GeneratorKind};

    // Helper: get the EffectKind slice for a built patch at its initial params.
    fn effect_kinds(preset: Preset) -> Vec<EffectKind> {
        let patch = preset.build();
        patch
            .effects
            .iter()
            .map(|e| e.kind(&patch.params))
            .collect()
    }

    // --- Enum basics ----------------------------------------------------------

    #[test]
    fn all_contains_five_presets() {
        assert_eq!(Preset::ALL.len(), 5);
    }

    #[test]
    fn all_names_are_nonempty() {
        for p in Preset::ALL {
            assert!(!p.name().is_empty(), "{p:?} has empty name");
        }
    }

    #[test]
    fn all_names_are_unique() {
        let names: Vec<_> = Preset::ALL.iter().map(|p| p.name()).collect();
        let mut seen = std::collections::HashSet::new();
        for name in &names {
            assert!(seen.insert(*name), "duplicate preset name: {name}");
        }
    }

    #[test]
    fn preset_eq() {
        assert_eq!(Preset::ClassicMandelbrot, Preset::ClassicMandelbrot);
        assert_ne!(Preset::ClassicMandelbrot, Preset::NoiseField);
    }

    #[test]
    fn preset_names_match_expected() {
        assert_eq!(Preset::ClassicMandelbrot.name(), "Classic Mandelbrot");
        assert_eq!(Preset::PsychedelicJulia.name(), "Psychedelic Julia");
        assert_eq!(Preset::TrippyMandelbrot.name(), "Trippy Mandelbrot");
        assert_eq!(Preset::BurningShipTrails.name(), "Burning Ship Trails");
        assert_eq!(Preset::NoiseField.name(), "Noise Field");
    }

    // --- ClassicMandelbrot ---------------------------------------------------

    #[test]
    fn classic_mandelbrot_generator() {
        let patch = Preset::ClassicMandelbrot.build();
        assert_eq!(patch.generator.kind(), GeneratorKind::Mandelbrot);
    }

    #[test]
    fn classic_mandelbrot_effects() {
        let kinds = effect_kinds(Preset::ClassicMandelbrot);
        assert_eq!(kinds.len(), 1);
        assert!(matches!(
            kinds[0],
            EffectKind::ColorMap {
                scheme: ColorScheme::Classic
            }
        ));
    }

    #[test]
    fn classic_mandelbrot_no_modulators() {
        assert_eq!(Preset::ClassicMandelbrot.build().modulators.len(), 0);
    }

    #[test]
    fn classic_mandelbrot_params() {
        let p = Preset::ClassicMandelbrot.build().params;
        assert!((p.center_x - (-0.5)).abs() < 1e-6);
        assert!(p.center_y.abs() < 1e-6);
        assert!((p.zoom - 1.0).abs() < 1e-6);
        assert_eq!(p.max_iter, 100);
    }

    // --- PsychedelicJulia ----------------------------------------------------

    #[test]
    fn psychedelic_julia_generator() {
        let patch = Preset::PsychedelicJulia.build();
        assert_eq!(patch.generator.kind(), GeneratorKind::Julia);
    }

    #[test]
    fn psychedelic_julia_gen_param_keys_include_julia_c() {
        let patch = Preset::PsychedelicJulia.build();
        let keys = patch.generator.gen_param_keys();
        assert!(keys.contains(&"julia_cx"), "missing julia_cx");
        assert!(keys.contains(&"julia_cy"), "missing julia_cy");
    }

    #[test]
    fn psychedelic_julia_c_values() {
        let p = Preset::PsychedelicJulia.build().params;
        assert!((p.get("julia_cx") - (-0.7)).abs() < 1e-5);
        assert!((p.get("julia_cy") - 0.27015).abs() < 1e-5);
    }

    #[test]
    fn psychedelic_julia_effects() {
        let kinds = effect_kinds(Preset::PsychedelicJulia);
        assert_eq!(kinds.len(), 2);
        assert!(matches!(
            kinds[0],
            EffectKind::ColorMap {
                scheme: ColorScheme::Psychedelic
            }
        ));
        assert!(matches!(kinds[1], EffectKind::HueShift { .. }));
    }

    #[test]
    fn psychedelic_julia_has_one_modulator() {
        assert_eq!(Preset::PsychedelicJulia.build().modulators.len(), 1);
    }

    #[test]
    fn psychedelic_julia_hue_shift_driven_by_lfo() {
        // After one tick the LFO should change hue_shift_amount away from 0.
        let mut patch = Preset::PsychedelicJulia.build();
        let before = patch.params.get("hue_shift_amount");
        patch.tick(0.5); // half second — LFO at 0.5 Hz should be near TAU/4
        let after = patch.params.get("hue_shift_amount");
        assert!(
            (after - before).abs() > 1e-3,
            "hue_shift_amount did not change after tick"
        );
        // Value must be within the [0, TAU] range.
        assert!(after >= 0.0 && after <= TAU + 1e-4, "out of range: {after}");
    }

    // --- TrippyMandelbrot ----------------------------------------------------

    #[test]
    fn trippy_mandelbrot_generator() {
        let patch = Preset::TrippyMandelbrot.build();
        assert_eq!(patch.generator.kind(), GeneratorKind::Mandelbrot);
    }

    #[test]
    fn trippy_mandelbrot_effects() {
        let kinds = effect_kinds(Preset::TrippyMandelbrot);
        assert_eq!(kinds.len(), 3);
        assert!(matches!(
            kinds[0],
            EffectKind::ColorMap {
                scheme: ColorScheme::Ocean
            }
        ));
        assert!(matches!(kinds[1], EffectKind::Ripple { .. }));
        assert!(matches!(kinds[2], EffectKind::Echo { .. }));
    }

    #[test]
    fn trippy_mandelbrot_ripple_initial_amplitude() {
        let kinds = effect_kinds(Preset::TrippyMandelbrot);
        if let EffectKind::Ripple {
            frequency,
            amplitude,
            speed,
        } = kinds[1]
        {
            assert!((frequency - 0.05).abs() < 1e-6);
            assert!((amplitude - 10.0).abs() < 1e-6);
            assert!((speed - 2.0).abs() < 1e-6);
        } else {
            panic!("expected Ripple");
        }
    }

    #[test]
    fn trippy_mandelbrot_echo_params() {
        let kinds = effect_kinds(Preset::TrippyMandelbrot);
        if let EffectKind::Echo {
            layers,
            offset,
            decay,
        } = kinds[2]
        {
            assert_eq!(layers, 3);
            assert!((offset - 5.0).abs() < 1e-6);
            assert!((decay - 2.0).abs() < 1e-6);
        } else {
            panic!("expected Echo");
        }
    }

    #[test]
    fn trippy_mandelbrot_ripple_driven_by_lfo() {
        let mut patch = Preset::TrippyMandelbrot.build();
        let before = patch.params.get("ripple_amplitude");
        patch.tick(1.0);
        let after = patch.params.get("ripple_amplitude");
        assert!(
            (after - before).abs() > 1e-3,
            "ripple_amplitude did not change"
        );
        assert!(
            after >= 5.0 - 1e-4 && after <= 15.0 + 1e-4,
            "ripple_amplitude out of [5, 15]: {after}"
        );
    }

    #[test]
    fn trippy_mandelbrot_has_one_modulator() {
        assert_eq!(Preset::TrippyMandelbrot.build().modulators.len(), 1);
    }

    // --- BurningShipTrails ---------------------------------------------------

    #[test]
    fn burning_ship_trails_generator() {
        let patch = Preset::BurningShipTrails.build();
        assert_eq!(patch.generator.kind(), GeneratorKind::BurningShip);
    }

    #[test]
    fn burning_ship_trails_effects() {
        let kinds = effect_kinds(Preset::BurningShipTrails);
        assert_eq!(kinds.len(), 2);
        assert!(matches!(
            kinds[0],
            EffectKind::ColorMap {
                scheme: ColorScheme::Fire
            }
        ));
        assert!(
            matches!(kinds[1], EffectKind::MotionBlur { opacity } if (opacity - 0.15).abs() < 1e-6)
        );
    }

    #[test]
    fn burning_ship_trails_no_modulators() {
        assert_eq!(Preset::BurningShipTrails.build().modulators.len(), 0);
    }

    #[test]
    fn burning_ship_trails_center() {
        let p = Preset::BurningShipTrails.build().params;
        assert!((p.center_x - (-0.5)).abs() < 1e-6);
        assert!((p.center_y - (-0.5)).abs() < 1e-6);
    }

    // --- NoiseField ----------------------------------------------------------

    #[test]
    fn noise_field_generator() {
        let patch = Preset::NoiseField.build();
        assert_eq!(patch.generator.kind(), GeneratorKind::NoiseField);
    }

    #[test]
    fn noise_field_effects() {
        let kinds = effect_kinds(Preset::NoiseField);
        assert_eq!(kinds.len(), 2);
        assert!(matches!(
            kinds[0],
            EffectKind::ColorMap {
                scheme: ColorScheme::Psychedelic
            }
        ));
        assert!(matches!(kinds[1], EffectKind::BrightnessContrast { .. }));
    }

    #[test]
    fn noise_field_brightness_contrast_params() {
        let kinds = effect_kinds(Preset::NoiseField);
        if let EffectKind::BrightnessContrast {
            brightness,
            contrast,
        } = kinds[1]
        {
            // Initial brightness ≈ 20/255 ≈ 0.078
            assert!(brightness >= 0.0 && brightness <= 40.0 / 255.0 + 1e-4);
            assert!((contrast - 1.5).abs() < 1e-6);
        } else {
            panic!("expected BrightnessContrast");
        }
    }

    #[test]
    fn noise_field_brightness_driven_by_lfo() {
        let mut patch = Preset::NoiseField.build();
        let before = patch.params.get("brightness_amount");
        patch.tick(1.25); // enough for LFO at 0.2 Hz to move
        let after = patch.params.get("brightness_amount");
        assert!(
            (after - before).abs() > 1e-4,
            "brightness_amount did not change"
        );
        assert!(
            after >= 0.0 - 1e-4 && after <= 40.0 / 255.0 + 1e-4,
            "brightness_amount out of range: {after}"
        );
    }

    #[test]
    fn noise_field_has_one_modulator() {
        assert_eq!(Preset::NoiseField.build().modulators.len(), 1);
    }

    // --- build() is idempotent (returns a fresh Patch each call) -------------

    #[test]
    fn build_returns_independent_patches() {
        let mut p1 = Preset::ClassicMandelbrot.build();
        let p2 = Preset::ClassicMandelbrot.build();
        p1.params.zoom = 99.0;
        // p2 must be unaffected
        assert!((p2.params.zoom - 1.0).abs() < 1e-6);
    }
}
