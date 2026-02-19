use crate::{Effect, Generator, Modulator, Params};

pub struct Patch {
    pub generator: Box<dyn Generator>,
    pub effects: Vec<Box<dyn Effect>>,
    pub modulators: Vec<Box<dyn Modulator>>,
    pub params: Params,
    /// Snapshot of generator-relevant params from the last frame, used to
    /// decide whether the GPU generator pass can be skipped.
    pub last_gen_params: Option<Vec<(String, f32)>>,
}

impl Patch {
    pub fn new(generator: Box<dyn Generator>, params: Params) -> Self {
        Self {
            generator,
            effects: Vec::new(),
            modulators: Vec::new(),
            params,
            last_gen_params: None,
        }
    }

    pub fn add_effect(mut self, effect: Box<dyn Effect>) -> Self {
        self.effects.push(effect);
        self
    }

    pub fn add_modulator(mut self, modulator: Box<dyn Modulator>) -> Self {
        self.modulators.push(modulator);
        self
    }

    /// Apply all modulators, advancing params by one frame.
    pub fn tick(&mut self, dt: f32) {
        self.params.time += dt;
        self.params.frame += 1;
        for m in &self.modulators {
            m.modulate(&mut self.params);
        }
    }

    /// Returns true if the generator-relevant params have changed since the
    /// last call — i.e. the GPU compute pass must be re-dispatched.
    pub fn generator_dirty(&mut self) -> bool {
        let keys = self.generator.gen_param_keys();
        let current: Vec<(String, f32)> = keys
            .iter()
            .map(|&k| (k.to_string(), self.params.get(k)))
            .collect();

        // Also include the structural params that always affect generators.
        let structural = [
            ("zoom".to_string(), self.params.zoom),
            ("center_x".to_string(), self.params.center_x),
            ("center_y".to_string(), self.params.center_y),
            ("max_iter".to_string(), self.params.max_iter as f32),
        ];
        let mut full: Vec<(String, f32)> = current;
        full.extend_from_slice(&structural);

        let dirty = self.last_gen_params.as_deref() != Some(&full);
        if dirty {
            self.last_gen_params = Some(full);
        }
        dirty
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Effect, EffectKind, Generator, GeneratorKind};

    // --- Minimal stubs --------------------------------------------------------

    struct StubGen {
        keys: &'static [&'static str],
    }
    impl Generator for StubGen {
        fn kind(&self) -> GeneratorKind {
            GeneratorKind::Mandelbrot
        }
        fn gen_param_keys(&self) -> &[&'static str] {
            self.keys
        }
    }

    struct StubEffect;
    impl Effect for StubEffect {
        fn kind(&self, _: &Params) -> EffectKind {
            EffectKind::HueShift { amount: 0.0 }
        }
    }

    struct StubMod {
        key: &'static str,
        value: f32,
    }
    impl Modulator for StubMod {
        fn modulate(&self, params: &mut Params) {
            params.set(self.key, self.value);
        }
    }

    fn make_patch() -> Patch {
        Patch::new(Box::new(StubGen { keys: &[] }), Params::default())
    }

    // --- tick -----------------------------------------------------------------

    #[test]
    fn tick_advances_time_and_frame() {
        let mut patch = make_patch();
        patch.tick(0.016);
        assert!((patch.params.time - 0.016).abs() < 1e-6);
        assert_eq!(patch.params.frame, 1);
    }

    #[test]
    fn tick_accumulates_time() {
        let mut patch = make_patch();
        patch.tick(0.1);
        patch.tick(0.1);
        patch.tick(0.1);
        assert!((patch.params.time - 0.3).abs() < 1e-5);
        assert_eq!(patch.params.frame, 3);
    }

    #[test]
    fn tick_runs_modulators() {
        let mut patch = make_patch().add_modulator(Box::new(StubMod {
            key: "val",
            value: 99.0,
        }));
        patch.tick(0.016);
        assert_eq!(patch.params.get("val"), 99.0);
    }

    // --- generator_dirty ------------------------------------------------------

    #[test]
    fn generator_dirty_true_on_first_call() {
        let mut patch = make_patch();
        assert!(patch.generator_dirty());
    }

    #[test]
    fn generator_dirty_false_after_snapshot() {
        let mut patch = make_patch();
        patch.generator_dirty(); // take initial snapshot
        assert!(!patch.generator_dirty());
    }

    #[test]
    fn generator_dirty_after_zoom_change() {
        let mut patch = make_patch();
        patch.generator_dirty();
        patch.params.zoom = 3.5;
        assert!(patch.generator_dirty());
    }

    #[test]
    fn generator_dirty_after_center_change() {
        let mut patch = make_patch();
        patch.generator_dirty();
        patch.params.center_x = 0.25;
        assert!(patch.generator_dirty());
    }

    #[test]
    fn generator_dirty_after_max_iter_change() {
        let mut patch = make_patch();
        patch.generator_dirty();
        patch.params.max_iter = 500;
        assert!(patch.generator_dirty());
    }

    #[test]
    fn generator_dirty_ignores_time_change() {
        // `time` is NOT in the structural keys — only zoom / center / max_iter are,
        // plus whatever gen_param_keys() returns. StubGen returns [] so time
        // should NOT trigger dirty.
        let mut patch = make_patch();
        patch.generator_dirty();
        patch.params.time += 1.0;
        assert!(!patch.generator_dirty());
    }

    #[test]
    fn generator_dirty_tracks_custom_gen_key() {
        let mut patch = Patch::new(
            Box::new(StubGen {
                keys: &["julia_cx"],
            }),
            Params::default(),
        );
        patch.generator_dirty();
        patch.params.set("julia_cx", 0.42);
        assert!(patch.generator_dirty());
    }

    // --- add_effect / add_modulator -------------------------------------------

    #[test]
    fn add_effect_appends() {
        let patch = make_patch()
            .add_effect(Box::new(StubEffect))
            .add_effect(Box::new(StubEffect));
        assert_eq!(patch.effects.len(), 2);
    }

    #[test]
    fn add_modulator_appends() {
        let patch = make_patch()
            .add_modulator(Box::new(StubMod {
                key: "a",
                value: 0.0,
            }))
            .add_modulator(Box::new(StubMod {
                key: "b",
                value: 0.0,
            }));
        assert_eq!(patch.modulators.len(), 2);
    }
}
