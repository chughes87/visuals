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
