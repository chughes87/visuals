use std::sync::Arc;

use winit::{
    application::ApplicationHandler,
    event::{ElementState, KeyEvent, MouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

mod app;
mod input;

use app::App;
use input::Key;

// ---------------------------------------------------------------------------
// Key mapping — winit PhysicalKey → input::Key
// ---------------------------------------------------------------------------

fn winit_to_key(code: KeyCode) -> Option<Key> {
    match code {
        KeyCode::Digit1 => Some(Key::Digit1),
        KeyCode::Digit2 => Some(Key::Digit2),
        KeyCode::Digit3 => Some(Key::Digit3),
        KeyCode::Digit4 => Some(Key::Digit4),
        KeyCode::Digit5 => Some(Key::Digit5),
        KeyCode::Space => Some(Key::Space),
        KeyCode::Equal => Some(Key::Equal),
        KeyCode::Minus => Some(Key::Minus),
        KeyCode::KeyR => Some(Key::R),
        KeyCode::KeyQ => Some(Key::Q),
        KeyCode::Escape => Some(Key::Escape),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Handler — winit ApplicationHandler (Phase 10: input wired up)
// ---------------------------------------------------------------------------

struct Handler {
    window: Option<Arc<Window>>,
    app: Option<App>,
}

impl ApplicationHandler for Handler {
    /// Called once on desktop when the event loop starts.
    /// Creates the window then initialises the wgpu surface.
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window_attrs = Window::default_attributes()
            .with_title("Fractal Explorer")
            .with_inner_size(winit::dpi::LogicalSize::new(800u32, 600u32));

        let window = Arc::new(
            event_loop
                .create_window(window_attrs)
                .expect("failed to create window"),
        );

        log::info!("Window created (800×600)");

        let gpu_app = App::new(Arc::clone(&window));
        self.window = Some(window);
        self.app = Some(gpu_app);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        // Feed every event to egui first; game input is skipped when egui
        // reports the event was consumed (e.g. a click inside the HUD panel).
        let egui_consumed = if let Some(app) = &mut self.app {
            app.egui_on_window_event(&event)
        } else {
            false
        };

        match event {
            // ----------------------------------------------------------------
            // Exit — always handled regardless of egui
            // ----------------------------------------------------------------
            WindowEvent::CloseRequested => {
                log::info!("Close requested — exiting");
                event_loop.exit();
            }

            // ----------------------------------------------------------------
            // Keyboard — skip if egui consumed the event
            // ----------------------------------------------------------------
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(code),
                        state: ElementState::Pressed,
                        ..
                    },
                ..
            } if !egui_consumed => {
                if let Some(key) = winit_to_key(code) {
                    if let Some(app) = &mut self.app {
                        if let Some(action) = app.on_key_pressed(key) {
                            if app.handle_action(action) {
                                event_loop.exit();
                            }
                        }
                    }
                }
            }

            // ----------------------------------------------------------------
            // Mouse — track cursor position (always; egui needs it too)
            // ----------------------------------------------------------------
            WindowEvent::CursorMoved { position, .. } => {
                if let Some(app) = &mut self.app {
                    app.on_cursor_moved(position.x, position.y);
                }
            }

            // ----------------------------------------------------------------
            // Mouse — left click → zoom (skip if egui consumed)
            // ----------------------------------------------------------------
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state: ElementState::Pressed,
                ..
            } if !egui_consumed => {
                if let Some(app) = &mut self.app {
                    let action = app.on_mouse_left_click();
                    if app.handle_action(action) {
                        event_loop.exit();
                    }
                }
            }

            // ----------------------------------------------------------------
            // Resize — always handled
            // ----------------------------------------------------------------
            WindowEvent::Resized(new_size) => {
                if let Some(app) = &mut self.app {
                    app.resize(new_size.width, new_size.height);
                }
            }

            // ----------------------------------------------------------------
            // Redraw — always handled
            // ----------------------------------------------------------------
            WindowEvent::RedrawRequested => {
                if let Some(app) = &mut self.app {
                    match app.render() {
                        Ok(()) => {}
                        Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                            if let Some(window) = &self.window {
                                let size = window.inner_size();
                                app.resize(size.width, size.height);
                            }
                        }
                        Err(wgpu::SurfaceError::OutOfMemory) => {
                            log::error!("GPU out of memory — exiting");
                            event_loop.exit();
                        }
                        Err(e) => log::warn!("render error: {e:?}"),
                    }
                }
            }

            _ => {}
        }
    }

    /// Drive continuous redraws (game-loop style).
    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

fn main() {
    env_logger::init();

    let event_loop = EventLoop::new().expect("failed to create event loop");
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut handler = Handler {
        window: None,
        app: None,
    };
    event_loop.run_app(&mut handler).expect("event loop error");
}
