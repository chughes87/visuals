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
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
