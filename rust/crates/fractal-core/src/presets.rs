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
            Preset::PsychedelicJulia  => "Psychedelic Julia",
            Preset::TrippyMandelbrot  => "Trippy Mandelbrot",
            Preset::BurningShipTrails => "Burning Ship Trails",
            Preset::NoiseField        => "Noise Field",
        }
    }
}
