// This code is loosely based on
// glium's examples/triangle.rs
// and
// android-activity's examples-na-winit-glutin

use android_activity::AndroidApp;
use drawcore::ActiveRenderer;
use gl_thin::gl_helper::initialize_gl_using_egli;
use std::ops::Add;
use std::time::{Duration, Instant};
use winit::application::ApplicationHandler;
use winit::event::{StartCause, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopBuilder};
use winit::platform::android::EventLoopBuilderExtAndroid;
use winit::window::WindowId;

pub mod drawcore;
pub mod rainbow_triangle;
pub mod scene;
pub mod suzanne;
pub mod text_painting;
pub mod textured_quad;
pub mod xr_input;

//

pub trait Drawable {
    fn handle_events_and_draw(&mut self);

    fn suspend(&mut self);
}

pub enum AppState<T: Drawable> {
    Paused,
    Active(T),
}

impl<T: Drawable> Default for AppState<T> {
    fn default() -> Self {
        Self::Paused
    }
}

pub struct MyApp<T: Drawable, F, E: std::fmt::Debug>
where
    F: Fn(&ActiveEventLoop) -> Result<T, E>,
{
    state: AppState<T>,
    factory: F,
}

impl<T: Drawable, F, E: std::fmt::Debug> ApplicationHandler for MyApp<T, F, E>
where
    F: Fn(&ActiveEventLoop) -> Result<T, E>,
{
    fn new_events(&mut self, _event_loop: &ActiveEventLoop, _cause: StartCause) {
        if let AppState::Active(app) = &mut self.state {
            app.handle_events_and_draw();
        }
    }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        match (self.factory)(event_loop) {
            Ok(x) => {
                self.state = AppState::Active(x);
            }
            Err(e) => {
                log::error!("malfunction building drawable {:?}", e)
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        let control_flow = window_event_loop_one_pass(event, event_loop, &mut self.state);
        event_loop.set_control_flow(control_flow);
    }

    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        log::debug!("suspend");
        if let AppState::Active(app) = &mut self.state {
            app.suspend();
        }
        // log::trace!("Suspended, dropping surface state...");
        // app.surface_state = None;
        self.state = AppState::Paused;
    }
}

//

fn window_event_loop_one_pass<T: Drawable>(
    event: WindowEvent,
    event_loop: &ActiveEventLoop,
    app: &mut AppState<T>,
) -> ControlFlow {
    log::trace!("Received Winit event: {event:?}");

    let static_graphics = false;

    let mut control_flow = match app {
        AppState::Paused => ControlFlow::Wait,
        AppState::Active(_) => {
            if static_graphics {
                ControlFlow::Poll
            } else {
                // trigger redraws every 6 milliseconds
                ControlFlow::WaitUntil(Instant::now().add(Duration::from_millis(6)))
            }
        }
    };

    match event {
        WindowEvent::Resized(_size) => {
            // Winit: doesn't currently implicitly request a redraw
            // for a resize which may be required on some platforms...
            if let AppState::Active(_) = app {
                control_flow = ControlFlow::Poll; // this should trigger a redraw via NewEvents
            }
        }
        WindowEvent::RedrawRequested => {
            log::trace!("Handling Redraw Request");
            if let AppState::Active(app) = app {
                app.handle_events_and_draw();
            }
        }
        WindowEvent::CloseRequested => event_loop.exit(),
        _ => {}
    }

    control_flow
}

//

//#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(android_app: AndroidApp) {
    android_logger::init_once(
        android_logger::Config::default().with_max_level(log::LevelFilter::Trace),
    );

    unsafe {
        std::env::set_var("RUST_BACKTRACE", "1");
    }

    log::debug!("bob test");

    let mut builder: EventLoopBuilder<_> = EventLoop::builder();
    let event_loop: EventLoop<()> = builder.with_android_app(android_app).build().unwrap();

    log::debug!("got event loop");

    let app = AppState::<ActiveRenderer>::default();
    let mut app = MyApp {
        state: app,
        factory: |event_loop| {
            initialize_gl_using_egli();

            ActiveRenderer::new(event_loop)
        },
    };
    event_loop.run_app(&mut app).unwrap();
}
