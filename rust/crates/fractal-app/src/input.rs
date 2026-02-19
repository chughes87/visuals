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
    /// Zoom in 2× centred on a normalised screen position.
    /// `norm_x` and `norm_y` are in \[0, 1\] (0 = left/top, 1 = right/bottom).
    MouseZoom {
        norm_x: f32,
        norm_y: f32,
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

    /// Produce a `MouseZoom` action from a normalised click position.
    pub fn on_mouse_click(&self, norm_x: f32, norm_y: f32) -> InputAction {
        InputAction::MouseZoom { norm_x, norm_y }
    }
}

// ---------------------------------------------------------------------------
// Zoom math (pure, testable)
// ---------------------------------------------------------------------------

/// Apply a zoom-in-2× action to the current view, returning
/// `(new_center_x, new_center_y, new_zoom)`.
///
/// Mirrors the Clojure `mouse-clicked` formula:
/// ```text
/// scale   = 4.0 / zoom
/// new_cx  = cx + (norm_x - 0.5) * scale * aspect
/// new_cy  = cy + (norm_y - 0.5) * scale
/// new_zoom = zoom * 2.0
/// ```
pub fn apply_zoom(
    cx: f32,
    cy: f32,
    zoom: f32,
    norm_x: f32,
    norm_y: f32,
    aspect: f32, // width / height
) -> (f32, f32, f32) {
    let scale = 4.0 / zoom;
    let new_cx = cx + (norm_x - 0.5) * scale * aspect;
    let new_cy = cy + (norm_y - 0.5) * scale;
    (new_cx, new_cy, zoom * 2.0)
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
        assert_eq!(input().on_key(Key::Space), Some(InputAction::CycleNextPreset));
    }

    #[test]
    fn equal_increases_iterations() {
        assert_eq!(
            input().on_key(Key::Equal),
            Some(InputAction::IterationsUp)
        );
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

    // --- Mouse click ----------------------------------------------------------

    #[test]
    fn mouse_click_produces_zoom_action() {
        assert_eq!(
            input().on_mouse_click(0.25, 0.75),
            InputAction::MouseZoom {
                norm_x: 0.25,
                norm_y: 0.75
            }
        );
    }

    #[test]
    fn mouse_click_preserves_coordinates() {
        let action = input().on_mouse_click(0.123, 0.456);
        if let InputAction::MouseZoom { norm_x, norm_y } = action {
            assert!((norm_x - 0.123).abs() < 1e-6);
            assert!((norm_y - 0.456).abs() < 1e-6);
        } else {
            panic!("expected MouseZoom");
        }
    }

    // --- Zoom math ------------------------------------------------------------

    #[test]
    fn zoom_at_center_does_not_pan() {
        // Clicking the exact screen centre must not shift the view centre.
        let (cx, cy, zoom) = apply_zoom(-0.5, 0.0, 1.0, 0.5, 0.5, 4.0 / 3.0);
        assert!((cx - (-0.5)).abs() < 1e-5, "cx={cx}");
        assert!(cy.abs() < 1e-5, "cy={cy}");
        assert!((zoom - 2.0).abs() < 1e-5, "zoom={zoom}");
    }

    #[test]
    fn zoom_doubles_each_click() {
        let (_, _, z1) = apply_zoom(0.0, 0.0, 1.0, 0.5, 0.5, 1.0);
        let (_, _, z2) = apply_zoom(0.0, 0.0, z1, 0.5, 0.5, 1.0);
        assert!((z1 - 2.0).abs() < 1e-5);
        assert!((z2 - 4.0).abs() < 1e-5);
    }

    #[test]
    fn zoom_top_left_shifts_center_left_and_up() {
        let (cx, cy, _) = apply_zoom(0.0, 0.0, 1.0, 0.0, 0.0, 1.0);
        assert!(cx < 0.0, "expected cx < 0, got {cx}");
        assert!(cy < 0.0, "expected cy < 0, got {cy}");
    }

    #[test]
    fn zoom_bottom_right_shifts_center_right_and_down() {
        let (cx, cy, _) = apply_zoom(0.0, 0.0, 1.0, 1.0, 1.0, 1.0);
        assert!(cx > 0.0, "expected cx > 0, got {cx}");
        assert!(cy > 0.0, "expected cy > 0, got {cy}");
    }

    #[test]
    fn zoom_higher_zoom_produces_smaller_pan() {
        // At 2× zoom the same off-centre click should move the centre
        // half as far as at 1× zoom.
        let (cx1, cy1, _) = apply_zoom(0.0, 0.0, 1.0, 0.0, 0.0, 1.0);
        let (cx2, cy2, _) = apply_zoom(0.0, 0.0, 2.0, 0.0, 0.0, 1.0);
        assert!((cx2 - cx1 / 2.0).abs() < 1e-5, "cx1={cx1} cx2={cx2}");
        assert!((cy2 - cy1 / 2.0).abs() < 1e-5, "cy1={cy1} cy2={cy2}");
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
