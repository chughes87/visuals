use fractal_core::presets::Preset;

// ---------------------------------------------------------------------------
// Key — windowing-library-independent key representation
// ---------------------------------------------------------------------------

/// A keyboard key, independent of any windowing library.
///
/// `main.rs` maps `winit::keyboard::PhysicalKey` → `Key`; everything else
/// in the input pipeline works purely with this enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Digit1,
    Digit2,
    Digit3,
    Digit4,
    Digit5,
    Space,
    Equal, // = / + (same physical key; Shift state ignored)
    Minus, // - / _ (same physical key; Shift state ignored)
    R,
    Q,
    Escape,
}

// ---------------------------------------------------------------------------
// InputAction — what the app does in response to input
// ---------------------------------------------------------------------------

/// High-level action produced by a key press or mouse click.
#[derive(Debug, Clone, PartialEq)]
pub enum InputAction {
    LoadPreset(Preset),
    CycleNextPreset,
    IterationsUp,
    IterationsDown,
    Reset,
    Quit,
    /// Zoom into a rubber-band selection rectangle.
    /// Coordinates are normalised screen fractions in \[0, 1\].
    BoxZoom {
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
    },
}

// ---------------------------------------------------------------------------
// InputState
// ---------------------------------------------------------------------------

pub struct InputState;

impl InputState {
    pub fn new() -> Self {
        Self
    }

    /// Translate a `Key` press into an `InputAction`, if the key is mapped.
    pub fn on_key(&self, key: Key) -> Option<InputAction> {
        match key {
            Key::Digit1 => Some(InputAction::LoadPreset(Preset::ClassicMandelbrot)),
            Key::Digit2 => Some(InputAction::LoadPreset(Preset::PsychedelicJulia)),
            Key::Digit3 => Some(InputAction::LoadPreset(Preset::TrippyMandelbrot)),
            Key::Digit4 => Some(InputAction::LoadPreset(Preset::BurningShipTrails)),
            Key::Digit5 => Some(InputAction::LoadPreset(Preset::NoiseField)),
            Key::Space => Some(InputAction::CycleNextPreset),
            Key::Equal => Some(InputAction::IterationsUp),
            Key::Minus => Some(InputAction::IterationsDown),
            Key::R => Some(InputAction::Reset),
            Key::Q | Key::Escape => Some(InputAction::Quit),
        }
    }
}

// ---------------------------------------------------------------------------
// Zoom math (pure, testable)
// ---------------------------------------------------------------------------

/// Zoom into a rubber-band selection box, returning
/// `(new_center_x, new_center_y, new_zoom)`.
///
/// `x1/y1` and `x2/y2` are the opposite corners of the selection in
/// normalised screen coords [0, 1].  The shader maps pixels via:
///   uv = (px - resolution*0.5) / (zoom * resolution.y * 0.5)
/// so the screen spans ±1/zoom vertically (±aspect/zoom horizontally).
/// The new center is the complex midpoint of the selection; the new zoom
/// is chosen so the larger normalised dimension of the box fills the screen.
pub fn apply_box_zoom(
    cx: f32,
    cy: f32,
    zoom: f32,
    x1: f32,
    y1: f32,
    x2: f32,
    y2: f32,
    aspect: f32, // width / height
) -> (f32, f32, f32) {
    let dx = (x2 - x1).abs().max(1e-6);
    let dy = (y2 - y1).abs().max(1e-6);
    let scale = 2.0 / zoom;
    let new_cx = cx + ((x1 + x2) * 0.5 - 0.5) * scale * aspect;
    let new_cy = cy + ((y1 + y2) * 0.5 - 0.5) * scale;
    let new_zoom = zoom / dx.max(dy);
    (new_cx, new_cy, new_zoom)
}

// ---------------------------------------------------------------------------
// Iteration clamping
// ---------------------------------------------------------------------------

/// Clamp an iteration count to the valid range \[20, 500\].
pub fn clamp_iterations(iter: u32) -> u32 {
    iter.clamp(20, 500)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn input() -> InputState {
        InputState::new()
    }

    // --- Digit keys load the correct preset -----------------------------------

    #[test]
    fn digit_1_loads_classic_mandelbrot() {
        assert_eq!(
            input().on_key(Key::Digit1),
            Some(InputAction::LoadPreset(Preset::ClassicMandelbrot))
        );
    }

    #[test]
    fn digit_2_loads_psychedelic_julia() {
        assert_eq!(
            input().on_key(Key::Digit2),
            Some(InputAction::LoadPreset(Preset::PsychedelicJulia))
        );
    }

    #[test]
    fn digit_3_loads_trippy_mandelbrot() {
        assert_eq!(
            input().on_key(Key::Digit3),
            Some(InputAction::LoadPreset(Preset::TrippyMandelbrot))
        );
    }

    #[test]
    fn digit_4_loads_burning_ship_trails() {
        assert_eq!(
            input().on_key(Key::Digit4),
            Some(InputAction::LoadPreset(Preset::BurningShipTrails))
        );
    }

    #[test]
    fn digit_5_loads_noise_field() {
        assert_eq!(
            input().on_key(Key::Digit5),
            Some(InputAction::LoadPreset(Preset::NoiseField))
        );
    }

    // --- Other key mappings ---------------------------------------------------

    #[test]
    fn space_cycles_next_preset() {
        assert_eq!(
            input().on_key(Key::Space),
            Some(InputAction::CycleNextPreset)
        );
    }

    #[test]
    fn equal_increases_iterations() {
        assert_eq!(input().on_key(Key::Equal), Some(InputAction::IterationsUp));
    }

    #[test]
    fn minus_decreases_iterations() {
        assert_eq!(
            input().on_key(Key::Minus),
            Some(InputAction::IterationsDown)
        );
    }

    #[test]
    fn r_resets() {
        assert_eq!(input().on_key(Key::R), Some(InputAction::Reset));
    }

    #[test]
    fn q_quits() {
        assert_eq!(input().on_key(Key::Q), Some(InputAction::Quit));
    }

    #[test]
    fn escape_quits() {
        assert_eq!(input().on_key(Key::Escape), Some(InputAction::Quit));
    }

    // --- All five digit keys are distinct ------------------------------------

    #[test]
    fn all_digit_keys_map_to_different_presets() {
        let presets: Vec<_> = [
            Key::Digit1,
            Key::Digit2,
            Key::Digit3,
            Key::Digit4,
            Key::Digit5,
        ]
        .iter()
        .map(|&k| input().on_key(k))
        .collect();

        for i in 0..presets.len() {
            for j in (i + 1)..presets.len() {
                assert_ne!(presets[i], presets[j], "keys {i} and {j} collide");
            }
        }
    }

    // --- Box zoom math --------------------------------------------------------

    #[test]
    fn box_zoom_full_screen_no_change() {
        // Selecting the entire screen should leave center and zoom unchanged.
        let (cx, cy, z) = apply_box_zoom(0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 1.0);
        assert!((cx - 0.0).abs() < 1e-5, "cx={cx}");
        assert!((cy - 0.0).abs() < 1e-5, "cy={cy}");
        assert!((z - 1.0).abs() < 1e-5, "z={z}");
    }

    #[test]
    fn box_zoom_center_half_screen_doubles_zoom() {
        // Selecting the centre quarter (half each axis) should double zoom.
        let (cx, cy, z) = apply_box_zoom(0.0, 0.0, 1.0, 0.25, 0.25, 0.75, 0.75, 1.0);
        assert!((cx - 0.0).abs() < 1e-5, "cx={cx}");
        assert!((cy - 0.0).abs() < 1e-5, "cy={cy}");
        assert!((z - 2.0).abs() < 1e-5, "z={z}");
    }

    #[test]
    fn box_zoom_top_left_quadrant_pans_left_and_up() {
        let (cx, cy, _) = apply_box_zoom(0.0, 0.0, 1.0, 0.0, 0.0, 0.5, 0.5, 1.0);
        assert!(cx < 0.0, "expected cx<0, got {cx}");
        assert!(cy < 0.0, "expected cy<0, got {cy}");
    }

    #[test]
    fn box_zoom_bottom_right_quadrant_pans_right_and_down() {
        let (cx, cy, _) = apply_box_zoom(0.0, 0.0, 1.0, 0.5, 0.5, 1.0, 1.0, 1.0);
        assert!(cx > 0.0, "expected cx>0, got {cx}");
        assert!(cy > 0.0, "expected cy>0, got {cy}");
    }

    #[test]
    fn box_zoom_limited_by_larger_dimension() {
        // dx=0.5, dy=0.25 → larger dim is dx → new_zoom = 1.0/0.5 = 2
        let (_, _, z) = apply_box_zoom(0.0, 0.0, 1.0, 0.25, 0.375, 0.75, 0.625, 1.0);
        assert!((z - 2.0).abs() < 1e-5, "z={z}");
    }

    #[test]
    fn box_zoom_higher_base_zoom_scales_proportionally() {
        // At 2× base zoom, same fractional selection gives twice the final zoom.
        let (_, _, z1) = apply_box_zoom(0.0, 0.0, 1.0, 0.25, 0.25, 0.75, 0.75, 1.0);
        let (_, _, z2) = apply_box_zoom(0.0, 0.0, 2.0, 0.25, 0.25, 0.75, 0.75, 1.0);
        assert!((z2 - z1 * 2.0).abs() < 1e-5, "z1={z1} z2={z2}");
    }

    // --- Iteration clamping ---------------------------------------------------

    #[test]
    fn clamp_iterations_enforces_minimum() {
        assert_eq!(clamp_iterations(0), 20);
        assert_eq!(clamp_iterations(1), 20);
        assert_eq!(clamp_iterations(19), 20);
        assert_eq!(clamp_iterations(20), 20);
    }

    #[test]
    fn clamp_iterations_enforces_maximum() {
        assert_eq!(clamp_iterations(500), 500);
        assert_eq!(clamp_iterations(501), 500);
        assert_eq!(clamp_iterations(9999), 500);
    }

    #[test]
    fn clamp_iterations_passes_through_valid_values() {
        assert_eq!(clamp_iterations(21), 21);
        assert_eq!(clamp_iterations(100), 100);
        assert_eq!(clamp_iterations(499), 499);
    }
}
