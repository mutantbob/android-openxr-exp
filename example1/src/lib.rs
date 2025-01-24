// This code is loosely based on
// glium's examples/triangle.rs
// and
// android-activity's examples-na-winit-glutin

use android_activity::AndroidApp;
use drawcore::ActiveRenderer;
use gl_thin::gl_helper::initialize_gl_using_egli;
use std::ops::Add;
use std::time::{Duration, Instant};
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopBuilder};
use winit::platform::android::EventLoopBuilderExtAndroid;

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

fn event_loop_one_pass<T: Drawable, X: std::fmt::Debug, E: std::fmt::Debug>(
    event: Event<X>,
    event_loop: &ActiveEventLoop,
    // control_flow: &mut ControlFlow,
    app: &mut AppState<T>,
    factory: impl Fn(&ActiveEventLoop) -> Result<T, E>,
) {
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
        Event::Resumed => {
            log::debug!("resume");
            match factory(event_loop) {
                Ok(x) => {
                    *app = AppState::Active(x);
                }
                Err(e) => {
                    log::error!("malfunction building drawable {:?}", e)
                }
            }
        }
        Event::Suspended => {
            log::debug!("suspend");
            if let AppState::Active(app) = app {
                app.suspend();
            }
            // log::trace!("Suspended, dropping surface state...");
            // app.surface_state = None;
            *app = AppState::Paused;
        }
        Event::WindowEvent { event: we, .. } => {
            control_flow = window_event_loop_one_pass(we, event_loop, app);
        }
        Event::NewEvents(_) => {
            if let AppState::Active(app) = app {
                app.handle_events_and_draw();
            }
        }
        _ => {}
    }

    event_loop.set_control_flow(control_flow);
}

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

    let mut app = AppState::<ActiveRenderer>::default();
    event_loop
        .run(move |evt, e_loop| {
            event_loop_one_pass(evt, e_loop, &mut app, |event_loop| {
                initialize_gl_using_egli();

                ActiveRenderer::new(event_loop)
            })
        })
        .unwrap();
}
