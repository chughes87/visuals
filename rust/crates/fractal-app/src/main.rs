use std::sync::Arc;

use winit::{
    application::ApplicationHandler,
    event::{ElementState, KeyEvent, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

mod input;

// ---------------------------------------------------------------------------
// App — winit ApplicationHandler (Phase 7: blank window only)
// ---------------------------------------------------------------------------

struct App {
    window: Option<Arc<Window>>,
}

impl ApplicationHandler for App {
    /// Called once on desktop (or on resume on mobile).
    /// Creates the 800×600 window.
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window_attrs = Window::default_attributes()
            .with_title("Fractal Explorer")
            .with_inner_size(winit::dpi::LogicalSize::new(800u32, 600u32));

        let window = event_loop
            .create_window(window_attrs)
            .expect("failed to create window");

        log::info!("Window created (800×600)");
        self.window = Some(Arc::new(window));
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            // ----------------------------------------------------------------
            // Exit on close button
            // ----------------------------------------------------------------
            WindowEvent::CloseRequested => {
                log::info!("Close requested — exiting");
                event_loop.exit();
            }

            // ----------------------------------------------------------------
            // Exit on Q or Escape
            // ----------------------------------------------------------------
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(code),
                        state: ElementState::Pressed,
                        ..
                    },
                ..
            } => match code {
                KeyCode::KeyQ | KeyCode::Escape => {
                    log::info!("Q/Escape pressed — exiting");
                    event_loop.exit();
                }
                _ => {}
            },

            // ----------------------------------------------------------------
            // Redraw — Phase 7: blank window, nothing to paint yet
            // ----------------------------------------------------------------
            WindowEvent::RedrawRequested => {}

            _ => {}
        }
    }

    /// Called when all pending events are processed.
    /// Requesting redraw here drives the continuous render loop.
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

    let mut app = App { window: None };
    event_loop.run_app(&mut app).expect("event loop error");
}
