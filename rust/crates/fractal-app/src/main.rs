use std::sync::Arc;

use winit::{
    application::ApplicationHandler,
    event::{ElementState, KeyEvent, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

mod app;
mod input;

use app::App;

// ---------------------------------------------------------------------------
// Handler — winit ApplicationHandler (Phase 8: black wgpu surface)
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
        match event {
            // ----------------------------------------------------------------
            // Exit
            // ----------------------------------------------------------------
            WindowEvent::CloseRequested => {
                log::info!("Close requested — exiting");
                event_loop.exit();
            }

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
            // Resize — reconfigure the wgpu surface
            // ----------------------------------------------------------------
            WindowEvent::Resized(new_size) => {
                if let Some(app) = &mut self.app {
                    app.resize(new_size.width, new_size.height);
                }
            }

            // ----------------------------------------------------------------
            // Redraw — clear to black and present
            // ----------------------------------------------------------------
            WindowEvent::RedrawRequested => {
                if let Some(app) = &mut self.app {
                    match app.render() {
                        Ok(()) => {}
                        // Surface lost / outdated: reconfigure and try again next frame.
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
